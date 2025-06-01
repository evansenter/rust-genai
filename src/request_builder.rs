use crate::GenaiError;
use crate::client::Client;
use crate::types::{FunctionCall, FunctionDeclaration, GenerateContentResponse};
use futures_util::StreamExt;
use genai_client::{self, models};
use serde_json::Value;

use async_stream::try_stream;
use futures_util::stream;
use std::str;

/// Builder for generating content with optional system instructions and function calling.
#[derive(Debug)]
pub struct GenerateContentBuilder<'a> {
    pub(crate) client: &'a Client,
    pub(crate) model_name: &'a str,
    pub(crate) prompt_text: Option<&'a str>,
    pub(crate) system_instruction: Option<&'a str>,
    pub(crate) tools: Option<Vec<genai_client::models::request::Tool>>,
}

impl<'a> GenerateContentBuilder<'a> {
    /// Creates a new builder for generating content.
    pub(crate) const fn new(client: &'a Client, model_name: &'a str) -> Self {
        Self {
            client,
            model_name,
            prompt_text: None,
            system_instruction: None,
            tools: None,
        }
    }

    /// Helper function to convert public `FunctionDeclaration` to internal Tool.
    fn convert_public_fn_decl_to_tool(
        function: FunctionDeclaration,
    ) -> genai_client::models::request::Tool {
        let schema_properties = function
            .parameters
            .get("properties")
            .cloned()
            .unwrap_or(Value::Null);
        let schema_type = function
            .parameters
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("object")
            .to_string();

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

        let url = genai_client::construct_url(
            self.model_name,
            &self.client.api_key,
            false,
            self.client.api_version,
        );

        let response = self
            .client
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
        let response_body: models::response::GenerateContentResponse = // Using models directly here
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
                        // This is crate::types::GenerateContentResponse
                        text: None,
                        function_call: Some(FunctionCall {
                            // This is crate::types::FunctionCall
                            name: function_call.name.clone(),
                            args: function_call.args.clone(),
                        }),
                    });
                } else if let Some(text) = &part.text {
                    return Ok(GenerateContentResponse {
                        // This is crate::types::GenerateContentResponse
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
    ) -> impl stream::Stream<Item = Result<GenerateContentResponse, GenaiError>> + Send + 'a {
        let Some(prompt_text) = self.prompt_text else {
            return stream::once(async move {
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

        let url = genai_client::construct_url(
            self.model_name,
            &self.client.api_key,
            true,
            self.client.api_version,
        );

        let stream_val = try_stream! { // Renamed to stream_val to avoid conflict with futures_util::stream
            let response = self.client.http_client.post(&url).json(&request_body).send().await?;

            let status = response.status();
            if status.is_success() {
                let mut byte_stream = response.bytes_stream();
                let mut buffer = Vec::new();

                while let Some(chunk_result) = byte_stream.next().await {
                    let chunk = chunk_result?; // Assuming chunk_result is Result<Bytes, Error>
                    buffer.extend_from_slice(&chunk);

                    while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                        let line_bytes = buffer.drain(..=newline_pos).collect::<Vec<u8>>();
                        let line = str::from_utf8(&line_bytes)?.trim_end_matches(|c| c == '\n' || c == '\r');

                        if line.starts_with("data:") {
                            let json_data = line.strip_prefix("data:").unwrap_or("").trim_start();
                            if !json_data.is_empty() {
                                let chunk_response: models::response::GenerateContentResponse = // Using models directly
                                    serde_json::from_str(json_data)?;

                                // TODO: Currently processes only the first candidate and first part from streamed chunks.
                                // Future enhancements could align with unary response handling.
                                if chunk_response.candidates.len() > 1 {
                                    // log::warn!("Multiple candidates received in stream, processing only the first.");
                                }
                                if let Some(candidate) = chunk_response.candidates.first() {
                                    if candidate.content.parts.len() > 1 {
                                        // log::warn!("Multiple parts received in stream, processing only the first.");
                                    }
                                    if let Some(part) = candidate.content.parts.first() {
                                        if let Some(function_call) = &part.function_call {
                                            yield GenerateContentResponse { // This is crate::types::GenerateContentResponse
                                                text: None,
                                                function_call: Some(FunctionCall { // This is crate::types::FunctionCall
                                                    name: function_call.name.clone(),
                                                    args: function_call.args.clone(),
                                                }),
                                            };
                                        } else if let Some(text) = &part.text {
                                            yield GenerateContentResponse { // This is crate_types::GenerateContentResponse
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
