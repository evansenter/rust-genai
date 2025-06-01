use crate::GenaiError;
use crate::internal::response_processing::process_response_parts;
use crate::types::GenerateContentResponse;
use async_stream::try_stream;
use futures_util::StreamExt;
use genai_client::ApiVersion;
use genai_client::construct_url;
use genai_client::models::request::GenerateContentRequest as InternalGenerateContentRequest;
use reqwest::Client as ReqwestClient;
use std::str;

/// The main client for interacting with the Google Generative AI API.
#[derive(Debug, Clone)]
pub struct Client {
    pub(crate) api_key: String,
    #[allow(clippy::struct_field_names)]
    pub(crate) http_client: ReqwestClient,
    pub(crate) api_version: ApiVersion,
    pub(crate) debug: bool,
}

/// Builder for `Client` instances.
#[derive(Debug)]
pub struct ClientBuilder {
    api_key: String,
    api_version: Option<ApiVersion>,
    debug: bool,
}

impl ClientBuilder {
    /// Sets the API version for the client.
    /// If not called, defaults to `ApiVersion::V1Alpha`.
    #[must_use]
    pub const fn api_version(mut self, version: ApiVersion) -> Self {
        self.api_version = Some(version);
        self
    }

    /// Enables debug mode for the client.
    /// If not called, defaults to `false`.
    #[must_use]
    pub const fn debug(mut self) -> Self {
        self.debug = true;
        self
    }

    /// Builds the `Client`.
    #[must_use]
    pub fn build(self) -> Client {
        Client {
            api_key: self.api_key,
            http_client: ReqwestClient::new(),
            api_version: self.api_version.unwrap_or(ApiVersion::V1Alpha),
            debug: self.debug,
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
            debug: false,
        }
    }

    /// Creates a new `GenAI` client with specified or default API version.
    /// Debug mode is disabled by default. Use the builder to enable it.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Your Google AI API key.
    /// * `api_version` - Optional API version to use. Defaults to `V1Alpha`.
    #[must_use]
    pub fn new(api_key: String, api_version: Option<ApiVersion>) -> Self {
        Self {
            api_key,
            http_client: ReqwestClient::new(),
            api_version: api_version.unwrap_or(ApiVersion::V1Alpha),
            debug: false,
        }
    }

    /// Starts building a content generation request using a specific model.
    ///
    /// # Arguments
    ///
    /// * `model_name` - The name of the model to use (e.g., "gemini-1.5-flash-latest")
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
    /// Returns an error if the HTTP request fails, response parsing fails, or the API returns an error.
    pub async fn generate_from_request(
        &self,
        model_name: &str,
        request_body: InternalGenerateContentRequest,
    ) -> Result<GenerateContentResponse, GenaiError> {
        let url = construct_url(model_name, &self.api_key, false, self.api_version);

        if self.debug {
            println!("[DEBUG] Request URL: {url}");
            match serde_json::to_string_pretty(&request_body) {
                Ok(json) => println!("[DEBUG] Request Body (JSON):\n{json}"),
                Err(_) => println!("[DEBUG] Request Body: {request_body:#?}"),
            }
        }

        let response = self
            .http_client
            .post(&url)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            if self.debug {
                println!("[DEBUG] Response Text: {error_text}");
            }
            return Err(GenaiError::Api(error_text));
        }

        let response_text = response.text().await?;
        if self.debug {
            println!("[DEBUG] Response Text: {response_text}");
        }
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

        if self.debug {
            println!("[DEBUG] Streaming Request URL: {url}");
            match serde_json::to_string_pretty(&request_body) {
                Ok(json) => println!("[DEBUG] Streaming Request Body (JSON):\n{json}"),
                Err(_) => println!("[DEBUG] Streaming Request Body: {request_body:#?}"),
            }
        }

        let stream_val = try_stream! {
            let response = self.http_client.post(&url).json(&request_body).send().await?;
            let status = response.status();

            if status.is_success() {
                let mut byte_stream = response.bytes_stream();
                let mut buffer = Vec::new();

                while let Some(chunk_result) = byte_stream.next().await {
                    let chunk = chunk_result?;
                    buffer.extend_from_slice(&chunk);

                    while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                        let line_bytes = buffer.drain(..=newline_pos).collect::<Vec<u8>>();
                        let line = str::from_utf8(&line_bytes)?.trim_end_matches(|c| c == '\n' || c == '\r');

                        if self.debug {
                            println!("[DEBUG] Raw Stream Line: {line}");
                        }

                        if line.starts_with("data:") {
                            let json_data = line.strip_prefix("data:").unwrap_or("").trim_start();
                            if !json_data.is_empty() {
                                let chunk_response_internal: genai_client::models::response::GenerateContentResponse =
                                    serde_json::from_str(json_data)?;

                                if let Some(candidate) = chunk_response_internal.candidates.first() {
                                    let processed_parts = process_response_parts(&candidate.content.parts);

                                    if processed_parts.text.is_some() || !processed_parts.function_calls.is_empty() || !processed_parts.code_execution_results.is_empty() {
                                        yield GenerateContentResponse {
                                            text: processed_parts.text,
                                            function_calls: if processed_parts.function_calls.is_empty() { None } else { Some(processed_parts.function_calls) },
                                            code_execution_results: if processed_parts.code_execution_results.is_empty() { None } else { Some(processed_parts.code_execution_results) },
                                        };
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                let error_text = response.text().await?;
                if self.debug {
                    println!("[DEBUG] Streaming Error: {error_text}");
                }
                Err(GenaiError::Api(error_text))?;
            }
        };
        stream_val.boxed()
    }
}
