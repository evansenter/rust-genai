use crate::GenaiError;
use crate::internal::response_processing::process_response_parts;
use crate::types::GenerateContentResponse;
use async_stream::try_stream;
use futures_util::StreamExt;
use genai_client::ApiVersion;
use genai_client::construct_url;
use genai_client::error_helpers::check_response;
use genai_client::models::request::GenerateContentRequest as InternalGenerateContentRequest;
use reqwest::Client as ReqwestClient;
use std::str;

/// Logs a request body at debug level, preferring JSON format when possible.
fn log_request_body<T: std::fmt::Debug + serde::Serialize>(body: &T) {
    match serde_json::to_string_pretty(body) {
        Ok(json) => log::debug!("Request Body (JSON):\n{json}"),
        Err(_) => log::debug!("Request Body: {body:#?}"),
    }
}

/// The main client for interacting with the Google Generative AI API.
#[derive(Debug, Clone)]
pub struct Client {
    pub(crate) api_key: String,
    #[allow(clippy::struct_field_names)]
    pub(crate) http_client: ReqwestClient,
    pub(crate) api_version: ApiVersion,
}

/// Builder for `Client` instances.
#[derive(Debug)]
pub struct ClientBuilder {
    api_key: String,
    api_version: Option<ApiVersion>,
}

impl ClientBuilder {
    /// Sets the API version for the client.
    /// If not called, defaults to `ApiVersion::V1Beta`.
    #[must_use]
    pub const fn api_version(mut self, version: ApiVersion) -> Self {
        self.api_version = Some(version);
        self
    }

    /// Builds the `Client`.
    #[must_use]
    pub fn build(self) -> Client {
        Client {
            api_key: self.api_key,
            http_client: ReqwestClient::new(),
            api_version: self.api_version.unwrap_or(ApiVersion::V1Beta),
        }
    }
}

impl Client {
    /// Creates a new builder for `Client` instances.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Your Google AI API key.
    #[must_use]
    pub const fn builder(api_key: String) -> ClientBuilder {
        ClientBuilder {
            api_key,
            api_version: None,
        }
    }

    /// Creates a new `GenAI` client with specified or default API version.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Your Google AI API key.
    /// * `api_version` - Optional API version to use. Defaults to `V1Beta`.
    #[must_use]
    pub fn new(api_key: String, api_version: Option<ApiVersion>) -> Self {
        Self {
            api_key,
            http_client: ReqwestClient::new(),
            api_version: api_version.unwrap_or(ApiVersion::V1Beta),
        }
    }

    /// Starts building a content generation request using a specific model.
    ///
    /// # Arguments
    ///
    /// * `model_name` - The name of the model to use (e.g., "gemini-3-flash-preview")
    #[must_use]
    pub const fn with_model<'a>(
        &'a self,
        model_name: &'a str,
    ) -> crate::request_builder::GenerateContentBuilder<'a> {
        crate::request_builder::GenerateContentBuilder::new(self, model_name)
    }

    /// Generates content directly from a pre-constructed request body.
    ///
    /// This method is useful for advanced scenarios where you need to manually build the
    /// `GenerateContentRequest`, for example, in multi-turn conversations with function calls.
    ///
    /// # Arguments
    ///
    /// * `model_name` - The name of the model to use.
    /// * `request_body` - The fully constructed `genai_client::models::request::GenerateContentRequest`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The HTTP request fails
    /// - Response parsing fails
    /// - The API returns an error
    pub async fn generate_from_request(
        &self,
        model_name: &str,
        request_body: InternalGenerateContentRequest,
    ) -> Result<GenerateContentResponse, GenaiError> {
        let url = construct_url(model_name, &self.api_key, false, self.api_version);

        log::debug!("Request URL: {url}");
        log_request_body(&request_body);

        let response = self
            .http_client
            .post(&url)
            .json(&request_body)
            .send()
            .await?;
        let response = check_response(response).await?;
        let response_text = response.text().await?;
        log::debug!("Response Text: {response_text}");
        let response_body: genai_client::models::response::GenerateContentResponse =
            serde_json::from_str(&response_text)?;

        if let Some(candidate) = response_body.candidates.first() {
            let processed_parts = process_response_parts(&candidate.content.parts);

            if !processed_parts.function_calls.is_empty()
                || !processed_parts.code_execution_results.is_empty()
                || processed_parts.text.is_some()
            {
                return Ok(GenerateContentResponse {
                    text: processed_parts.text,
                    function_calls: if processed_parts.function_calls.is_empty() {
                        None
                    } else {
                        Some(processed_parts.function_calls)
                    },
                    code_execution_results: if processed_parts.code_execution_results.is_empty() {
                        None
                    } else {
                        Some(processed_parts.code_execution_results)
                    },
                    thought_signatures: if processed_parts.thought_signatures.is_empty() {
                        None
                    } else {
                        Some(processed_parts.thought_signatures)
                    },
                });
            }
        }
        Err(GenaiError::Parse(
            "No text, function calls, or actionable tool code results found in response structure (from client::generate_from_request)".to_string(),
        ))
    }

    /// Generates content as a stream directly from a pre-constructed request body.
    ///
    /// This method is useful for advanced scenarios where you need to manually build the
    /// `GenerateContentRequest` for streaming, e.g., in multi-turn conversations.
    ///
    /// # Arguments
    ///
    /// * `model_name` - The name of the model to use.
    /// * `request_body` - The fully constructed `genai_client::models::request::GenerateContentRequest`.
    pub fn stream_from_request<'b>(
        &'b self,
        model_name: &'b str,
        request_body: InternalGenerateContentRequest,
    ) -> impl futures_util::stream::Stream<Item = Result<GenerateContentResponse, GenaiError>> + Send + 'b
    {
        let url = construct_url(model_name, &self.api_key, true, self.api_version);

        log::debug!("Streaming Request URL: {url}");
        log_request_body(&request_body);

        let stream_val = try_stream! {
            let response = self.http_client.post(&url).json(&request_body).send().await?;
            let status = response.status();

            if status.is_success() {
                let byte_stream = response.bytes_stream();
                let parsed_stream = genai_client::sse_parser::parse_sse_stream::<genai_client::models::response::GenerateContentResponse>(byte_stream);
                futures_util::pin_mut!(parsed_stream);

                while let Some(result) = parsed_stream.next().await {
                    let chunk_response_internal = result?;

                    log::debug!("Stream Chunk: {chunk_response_internal:#?}");

                    if let Some(candidate) = chunk_response_internal.candidates.first() {
                        let processed_parts = process_response_parts(&candidate.content.parts);

                        if processed_parts.text.is_some() || !processed_parts.function_calls.is_empty() || !processed_parts.code_execution_results.is_empty() {
                            yield GenerateContentResponse {
                                text: processed_parts.text,
                                function_calls: if processed_parts.function_calls.is_empty() { None } else { Some(processed_parts.function_calls) },
                                code_execution_results: if processed_parts.code_execution_results.is_empty() { None } else { Some(processed_parts.code_execution_results) },
                                thought_signatures: if processed_parts.thought_signatures.is_empty() { None } else { Some(processed_parts.thought_signatures) },
                            };
                        }
                    }
                }
            } else {
                let error_text = response.text().await?;
                log::debug!("Streaming Error: {error_text}");
                Err(GenaiError::Api(error_text))?;
            }
        };
        stream_val.boxed()
    }

    // --- Interactions API methods ---

    /// Creates a builder for constructing an interaction request.
    ///
    /// This provides a fluent interface for building interactions with models or agents.
    /// Use this method for a more ergonomic API compared to manually constructing
    /// `CreateInteractionRequest`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use rust_genai::Client;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder("api_key".to_string()).build();
    ///
    /// // Simple interaction
    /// let response = client.interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Hello, world!")
    ///     .create()
    ///     .await?;
    ///
    /// // Stateful conversation
    /// let response2 = client.interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("What did I just say?")
    ///     .with_previous_interaction(&response.id)
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn interaction(&self) -> crate::request_builder::InteractionBuilder<'_> {
        crate::request_builder::InteractionBuilder::new(self)
    }

    /// Creates a new interaction using the Gemini Interactions API.
    ///
    /// The Interactions API provides a unified interface for working with models and agents,
    /// with built-in support for stateful conversations, function calling, and long-running tasks.
    ///
    /// # Arguments
    ///
    /// * `request` - The interaction request with model/agent, input, and optional configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The HTTP request fails
    /// - Response parsing fails
    /// - The API returns an error
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::{Client, ApiVersion};
    /// use genai_client::{CreateInteractionRequest, InteractionInput};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("your-api-key".to_string(), Some(ApiVersion::V1Beta));
    ///
    /// let request = CreateInteractionRequest {
    ///     model: Some("gemini-3-flash-preview".to_string()),
    ///     agent: None,
    ///     input: InteractionInput::Text("Hello, world!".to_string()),
    ///     previous_interaction_id: None,
    ///     tools: None,
    ///     response_modalities: None,
    ///     response_format: None,
    ///     generation_config: None,
    ///     stream: None,
    ///     background: None,
    ///     store: None,
    ///     system_instruction: None,
    /// };
    ///
    /// let response = client.create_interaction(request).await?;
    /// println!("Interaction ID: {}", response.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_interaction(
        &self,
        request: genai_client::CreateInteractionRequest,
    ) -> Result<genai_client::InteractionResponse, GenaiError> {
        log::debug!("Creating interaction");
        log_request_body(&request);

        let response =
            genai_client::create_interaction(&self.http_client, &self.api_key, request).await?;

        log::debug!("Interaction created: ID={}", response.id);

        Ok(response)
    }

    /// Creates a new interaction with streaming responses.
    ///
    /// Returns a stream of `InteractionResponse` updates as they arrive from the server.
    /// Useful for long-running interactions or agents where you want incremental updates.
    ///
    /// # Arguments
    ///
    /// * `request` - The interaction request with streaming enabled.
    ///
    /// # Returns
    /// A boxed stream that yields `InteractionResponse` items.
    pub fn create_interaction_stream(
        &self,
        request: genai_client::CreateInteractionRequest,
    ) -> futures_util::stream::BoxStream<'_, Result<genai_client::InteractionResponse, GenaiError>>
    {
        use futures_util::StreamExt;

        log::debug!("Creating streaming interaction");
        log_request_body(&request);

        let stream =
            genai_client::create_interaction_stream(&self.http_client, &self.api_key, request);

        stream
            .map(move |result| {
                result
                    .inspect(|response| {
                        log::debug!("Received interaction update: status={:?}", response.status);
                    })
                    .map_err(GenaiError::from)
            })
            .boxed()
    }

    /// Retrieves an existing interaction by its ID.
    ///
    /// Useful for checking the status of long-running interactions or agents,
    /// or for retrieving the full conversation history.
    ///
    /// # Arguments
    ///
    /// * `interaction_id` - The unique identifier of the interaction to retrieve.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The HTTP request fails
    /// - Response parsing fails
    /// - The API returns an error
    pub async fn get_interaction(
        &self,
        interaction_id: &str,
    ) -> Result<genai_client::InteractionResponse, GenaiError> {
        log::debug!("Getting interaction: ID={interaction_id}");

        let response =
            genai_client::get_interaction(&self.http_client, &self.api_key, interaction_id).await?;

        log::debug!("Retrieved interaction: status={:?}", response.status);

        Ok(response)
    }

    /// Deletes an interaction by its ID.
    ///
    /// Removes the interaction from the server, freeing up storage and making it
    /// unavailable for future reference via `previous_interaction_id`.
    ///
    /// # Arguments
    ///
    /// * `interaction_id` - The unique identifier of the interaction to delete.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The HTTP request fails
    /// - The API returns an error
    pub async fn delete_interaction(&self, interaction_id: &str) -> Result<(), GenaiError> {
        log::debug!("Deleting interaction: ID={interaction_id}");

        genai_client::delete_interaction(&self.http_client, &self.api_key, interaction_id).await?;

        log::debug!("Interaction deleted successfully");

        Ok(())
    }
}
