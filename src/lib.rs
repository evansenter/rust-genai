use futures_util::StreamExt; // For .boxed()
use reqwest::Client as ReqwestClient; // Alias to avoid name clash
use serde_json::Value;
use thiserror::Error;

pub use genai_client::ApiVersion;

/// Defines errors that can occur when interacting with the `GenAI` API.
#[derive(Debug, Error)]
pub enum GenaiError {
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("SSE parsing error: {0}")]
    Parse(String),
    #[error("JSON deserialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("UTF-8 decoding error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("API Error returned by Google: {0}")]
    Api(String),
    #[error("Internal client error: {0}")] // Variant to wrap internal errors
    Internal(String),
}

// Implement conversion from internal error to public error
impl From<genai_client::InternalError> for GenaiError {
    fn from(internal_err: genai_client::InternalError) -> Self {
        match internal_err {
            genai_client::InternalError::Http(e) => Self::Http(e),
            genai_client::InternalError::Parse(s) => Self::Parse(s),
            genai_client::InternalError::Json(e) => Self::Json(e),
            genai_client::InternalError::Utf8(e) => Self::Utf8(e),
            genai_client::InternalError::Api(s) => Self::Api(s),
        }
    }
}

/// Represents a successful response from a generate content request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateContentResponse {
    /// The generated text content, if any.
    pub text: Option<String>,
    /// The function call, if any.
    pub function_call: Option<FunctionCall>,
}

/// Represents a function call in the response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionCall {
    /// The name of the function to call.
    pub name: String,
    /// The arguments to pass to the function.
    pub args: Value,
}

/// Represents a function declaration that can be used by the model.
#[derive(Debug, Clone)]
pub struct FunctionDeclaration {
    /// The name of the function.
    pub name: String,
    /// A description of what the function does.
    pub description: String,
    /// The JSON Schema for the function's parameters.
    pub parameters: Value,
    /// The names of required parameters.
    pub required: Vec<String>,
}

/// The main client for interacting with the Google Generative AI API.
#[derive(Debug, Clone)]
pub struct Client {
    api_key: String,
    #[allow(clippy::struct_field_names)] // Allow to keep descriptive name
    http_client: ReqwestClient,
    api_version: ApiVersion,
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

/// Builder for generating content with optional system instructions and function calling.
#[derive(Debug)]
pub struct GenerateContentBuilder<'a> {
    client: &'a Client,
    model_name: &'a str,
    prompt_text: Option<&'a str>,
    system_instruction: Option<&'a str>,
    tools: Option<Vec<genai_client::models::request::Tool>>,
}

impl<'a> GenerateContentBuilder<'a> {
    /// Creates a new builder for generating content.
    const fn new(client: &'a Client, model_name: &'a str) -> Self {
        Self {
            client,
            model_name,
            prompt_text: None,
            system_instruction: None,
            tools: None,
        }
    }

    /// Helper function to convert public `FunctionDeclaration` to internal Tool.
    fn convert_public_fn_decl_to_tool(function: FunctionDeclaration) -> genai_client::models::request::Tool {
        let schema_properties = function.parameters.get("properties").cloned().unwrap_or(Value::Null);
        let schema_type = function.parameters.get("type").and_then(Value::as_str).unwrap_or("object").to_string();

        let internal_function_parameters = genai_client::models::request::FunctionParameters {
            type_: schema_type,
            properties: schema_properties,
            required: function.required,
        };

        genai_client::models::request::Tool {
            function_declarations: vec![genai_client::models::request::FunctionDeclaration {
                name: function.name,
                description: function.description,
                parameters: internal_function_parameters,
            }],
        }
    }

    /// Sets the prompt text for the request.
    #[must_use]
    pub const fn with_prompt(mut self, prompt: &'a str) -> Self {
        self.prompt_text = Some(prompt);
        self
    }

    /// Sets the system instruction for the request.
    #[must_use]
    pub const fn with_system_instruction(mut self, instruction: &'a str) -> Self {
        self.system_instruction = Some(instruction);
        self
    }

    /// Adds a function that the model can call.
    #[must_use]
    pub fn with_function(mut self, function: FunctionDeclaration) -> Self {
        let tool = Self::convert_public_fn_decl_to_tool(function);
        self.tools.get_or_insert_with(Vec::new).push(tool);
        self
    }

    #[must_use]
    pub fn with_functions(mut self, functions: Vec<FunctionDeclaration>) -> Self {
        let tools_vec = functions
            .into_iter()
            .map(Self::convert_public_fn_decl_to_tool)
            .collect::<Vec<_>>();

        if !tools_vec.is_empty() {
            self.tools.get_or_insert_with(Vec::new).extend(tools_vec);
        }
        self
    }

    /// Generates content based on the configured parameters.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails, response parsing fails, or the API returns an error.
    // TODO: consider also having a generate_parts that returns 1+ Parts directly.
    pub async fn generate(self) -> Result<GenerateContentResponse, GenaiError> {
        let prompt_text = self.prompt_text.ok_or_else(|| {
            GenaiError::Internal("Prompt text is required for content generation".to_string())
        })?;

        let request_body = genai_client::models::request::GenerateContentRequest {
            contents: vec![genai_client::models::request::Content {
                parts: vec![genai_client::models::request::Part {
                    text: Some(prompt_text.to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: None,
            }],
            system_instruction: self.system_instruction.map(|text| {
                genai_client::models::request::Content {
                    parts: vec![genai_client::models::request::Part {
                        text: Some(text.to_string()),
                        function_call: None,
                        function_response: None,
                    }],
                    role: None,
                }
            }),
            tools: self.tools,
        };

        let url = genai_client::construct_url(self.model_name, &self.client.api_key, false, self.client.api_version);

        let response = self.client.http_client.post(&url).json(&request_body).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(GenaiError::Api(error_text));
        }

        let response_text = response.text().await?;
        let response_body: genai_client::models::response::GenerateContentResponse =
            serde_json::from_str(&response_text)?;

        // TODO: Currently processes only the first candidate and first part.
        // Future enhancements could allow selecting a candidate or handling multiple parts if the API supports it.
        if response_body.candidates.len() > 1 {
            log::warn!("Multiple candidates received, processing only the first.");
        }

        if let Some(candidate) = response_body.candidates.first() {
            if candidate.content.parts.len() > 1 {
                log::warn!("Multiple parts received, processing only the first.");
            }
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

    /// Generates content as a stream based on the configured parameters.
    pub fn stream(
        self,
    ) -> impl futures_util::Stream<Item = Result<GenerateContentResponse, GenaiError>> + Send + 'a
    {
        let Some(prompt_text) = self.prompt_text else {
            return futures_util::stream::once(async move {
                Err(GenaiError::Internal(
                    "Prompt text is required for content generation".to_string(),
                ))
            })
            .boxed();
        };

        let request_body = genai_client::models::request::GenerateContentRequest {
            contents: vec![genai_client::models::request::Content {
                parts: vec![genai_client::models::request::Part {
                    text: Some(prompt_text.to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: None,
            }],
            system_instruction: self.system_instruction.map(|text| {
                genai_client::models::request::Content {
                    parts: vec![genai_client::models::request::Part {
                        text: Some(text.to_string()),
                        function_call: None,
                        function_response: None,
                    }],
                    role: None,
                }
            }),
            tools: self.tools,
        };

        let url = genai_client::construct_url(self.model_name, &self.client.api_key, true, self.client.api_version);

        let stream = async_stream::try_stream! {
            let response = self.client.http_client.post(&url).json(&request_body).send().await?;

            let status = response.status();
            if status.is_success() {
                let mut byte_stream = response.bytes_stream();
                let mut buffer = Vec::new();

                while let Some(chunk_result) = byte_stream.next().await {
                    let chunk = chunk_result?;
                    buffer.extend_from_slice(&chunk);

                    while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                        let line_bytes = buffer.drain(..=newline_pos).collect::<Vec<u8>>();
                        let line = std::str::from_utf8(&line_bytes)?.trim_end_matches(|c| c == '\n' || c == '\r');

                        if line.starts_with("data:") {
                            let json_data = line.strip_prefix("data:").unwrap_or("").trim_start();
                            if !json_data.is_empty() {
                                let chunk_response: genai_client::models::response::GenerateContentResponse =
                                    serde_json::from_str(json_data)?;

                                // TODO: Currently processes only the first candidate and first part from streamed chunks.
                                // Future enhancements could align with unary response handling.
                                if chunk_response.candidates.len() > 1 {
                                    log::warn!("Multiple candidates received in stream, processing only the first.");
                                }
                                if let Some(candidate) = chunk_response.candidates.first() {
                                    if candidate.content.parts.len() > 1 {
                                        log::warn!("Multiple parts received in stream, processing only the first.");
                                    }
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

        stream.boxed()
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
        request_body: genai_client::models::request::GenerateContentRequest,
    ) -> Result<GenerateContentResponse, GenaiError> {
        let url = genai_client::construct_url(model_name, &self.api_key, false, self.api_version);

        let response = self.http_client.post(&url).json(&request_body).send().await?;

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
    pub fn stream_from_request<'a>(
        &'a self,
        model_name: &'a str,
        request_body: genai_client::models::request::GenerateContentRequest,
    ) -> impl futures_util::Stream<Item = Result<GenerateContentResponse, GenaiError>> + Send + 'a {
        let url = genai_client::construct_url(model_name, &self.api_key, true, self.api_version);

        let stream = async_stream::try_stream! {
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
                        let line = std::str::from_utf8(&line_bytes)?.trim_end_matches(|c| c == '\n' || c == '\r');

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
        stream.boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genai_client::InternalError;

    #[test]
    fn test_internal_error_to_genai_error_conversion() {
        // Test Parse variant
        let internal_parse = InternalError::Parse("parse error".to_string());
        let public_parse: GenaiError = internal_parse.into();
        assert!(matches!(public_parse, GenaiError::Parse(s) if s == "parse error"));

        // Test Http variant - we'll skip this test since creating a reqwest::Error is complex
        // and the #[from] attribute is well-tested in the reqwest crate itself
        // If we need to test this in the future, we can use a mock HTTP client

        // Test Json variant
        let invalid_json = "{invalid json";
        let json_error = serde_json::from_str::<serde_json::Value>(invalid_json).unwrap_err();
        let internal_json = InternalError::Json(json_error);
        let public_json: GenaiError = internal_json.into();
        assert!(matches!(public_json, GenaiError::Json(_)));

        // Test Utf8 variant - using a dynamic approach to create invalid UTF-8
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"valid");
        bytes.push(0xFF); // Add an invalid byte
        let utf8_error = std::str::from_utf8(&bytes).unwrap_err();
        let internal_utf8 = InternalError::Utf8(utf8_error);
        let public_utf8: GenaiError = internal_utf8.into();
        assert!(matches!(public_utf8, GenaiError::Utf8(_)));

        // Test Api variant
        let internal_api = InternalError::Api("api error".to_string());
        let public_api: GenaiError = internal_api.into();
        assert!(matches!(public_api, GenaiError::Api(s) if s == "api error"));
    }

    #[test]
    fn test_public_response_struct() {
        let response = GenerateContentResponse {
            text: Some("test".to_string()),
            function_call: None,
        };
        assert_eq!(response.text.as_deref(), Some("test"));
        assert!(response.function_call.is_none());

        let response = GenerateContentResponse {
            text: None,
            function_call: Some(FunctionCall {
                name: "test_function".to_string(),
                args: serde_json::json!({"arg": "value"}),
            }),
        };
        assert!(response.text.is_none());
        assert_eq!(
            response.function_call.as_ref().unwrap().name,
            "test_function"
        );
        assert_eq!(
            response.function_call.as_ref().unwrap().args,
            serde_json::json!({"arg": "value"})
        );
    }
}
