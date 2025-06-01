use crate::GenaiError;
use crate::request_builder::GenerateContentBuilder;
use crate::types::{FunctionCall, GenerateContentResponse};
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
}

/// Builder for `Client` instances.
#[derive(Debug)]
pub struct ClientBuilder {
    api_key: String,
    api_version: Option<ApiVersion>,
}

impl ClientBuilder {
    /// Sets the API version for the client.
    /// If not called, defaults to `ApiVersion::V1Alpha`.
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
            api_version: self.api_version.unwrap_or(ApiVersion::V1Alpha),
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
    /// This method is kept for direct instantiation if preferred over the builder.
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
        }
    }

    /// Starts building a content generation request using a specific model.
    ///
    /// # Arguments
    ///
    /// * `model_name` - The name of the model to use (e.g., "gemini-1.5-flash-latest")
    #[must_use]
    pub const fn with_model<'a>(&'a self, model_name: &'a str) -> GenerateContentBuilder<'a> {
        GenerateContentBuilder::new(self, model_name)
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

        let response = self
            .http_client
            .post(&url)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(GenaiError::Api(error_text));
        }

        let response_text = response.text().await?;
        let response_body: genai_client::models::response::GenerateContentResponse =
            serde_json::from_str(&response_text)?;

        if let Some(candidate) = response_body.candidates.first() {
            if let Some(part) = candidate.content.parts.first() {
                if let Some(function_call) = &part.function_call {
                    return Ok(GenerateContentResponse {
                        text: None,
                        function_call: Some(FunctionCall {
                            name: function_call.name.clone(),
                            args: function_call.args.clone(),
                        }),
                    });
                } else if let Some(text) = &part.text {
                    return Ok(GenerateContentResponse {
                        text: Some(text.clone()),
                        function_call: None,
                    });
                }
            }
        }
        Err(GenaiError::Parse(
            "No text content or function call found in response structure".to_string(),
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

        let stream_val = try_stream! {
            let response = self.http_client.post(&url).json(&request_body).send().await?;
            let status = response.status();

            if status.is_success() {
                let mut byte_stream = response.bytes_stream();
                let mut buffer = Vec::new();

                while let Some(chunk_result) = byte_stream.next().await {
                    let chunk = chunk_result?; // Assuming Bytes
                    buffer.extend_from_slice(&chunk);

                    while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                        let line_bytes = buffer.drain(..=newline_pos).collect::<Vec<u8>>();
                        let line = str::from_utf8(&line_bytes)?.trim_end_matches(|c| c == '\n' || c == '\r');

                        if line.starts_with("data:") {
                            let json_data = line.strip_prefix("data:").unwrap_or("").trim_start();
                            if !json_data.is_empty() {
                                let chunk_response: genai_client::models::response::GenerateContentResponse =
                                    serde_json::from_str(json_data)?;

                                if let Some(candidate) = chunk_response.candidates.first() {
                                    if let Some(part) = candidate.content.parts.first() {
                                        if let Some(function_call) = &part.function_call {
                                            yield GenerateContentResponse {
                                                text: None,
                                                function_call: Some(FunctionCall {
                                                    name: function_call.name.clone(),
                                                    args: function_call.args.clone(),
                                                }),
                                            };
                                        } else if let Some(text) = &part.text {
                                            yield GenerateContentResponse {
                                                text: Some(text.clone()),
                                                function_call: None,
                                            };
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                let error_text = response.text().await?;
                Err(GenaiError::Api(error_text))?;
            }
        };
        stream_val.boxed()
    }
}
