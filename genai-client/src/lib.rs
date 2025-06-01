use reqwest::Client as ReqwestClient;
// use std::error::Error; // Removed unused import
use async_stream::try_stream;
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use std::str;
use thiserror::Error; // Keep thiserror here too for InternalError

// Declare the models module
pub mod models;

// Import and selectively re-export the necessary structs from the models module
pub use models::request::Content;
pub use models::request::FunctionCall;
pub use models::request::FunctionCallingConfig;
pub use models::request::FunctionCallingMode;
pub use models::request::FunctionDeclaration;
pub use models::request::FunctionParameters;
pub use models::request::FunctionResponse;
pub use models::request::GenerateContentRequest;
pub use models::request::Part;
pub use models::request::Tool;
pub use models::request::ToolConfig;

pub use models::response::Candidate;
pub use models::response::ContentResponse;
pub use models::response::FunctionCallResponse;
pub use models::response::GenerateContentResponse;
pub use models::response::PartResponse;

/// Represents the API version to target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiVersion {
    V1Alpha,
    V1Beta,
}

impl ApiVersion {
    const fn as_str(self) -> &'static str {
        match self {
            Self::V1Alpha => "v1alpha",
            Self::V1Beta => "v1beta",
        }
    }
}

// Define an INTERNAL error type for this crate
#[derive(Debug, Error)]
pub enum InternalError {
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("SSE parsing error: {0}")]
    Parse(String),
    #[error("JSON deserialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("UTF-8 decoding error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("API Error: {0}")]
    Api(String),
}

// --- URL Construction ---
const BASE_URL_PREFIX: &str = "https://generativelanguage.googleapis.com";

#[must_use]
pub fn construct_url(model_name: &str, api_key: &str, stream: bool, version: ApiVersion) -> String {
    let action = if stream { "streamGenerateContent" } else { "generateContent" };
    let sse_param = if stream { "&alt=sse" } else { "" };
    format!(
        "{BASE_URL_PREFIX}/{version_str}/models/{model_name}:{action}?key={api_key}{sse_param}",
        version_str = version.as_str()
    )
}

// --- Internal Helper Functions ---

/// Sends a content generation request to the Google Generative AI API.
///
/// # Errors
/// Returns an error if the HTTP request fails, the response status is not successful, the response cannot be parsed as JSON, or if no text content is found in the response structure.
pub async fn generate_content_internal(
    http_client: &ReqwestClient,
    api_key: &str,
    model_name: &str,
    prompt_text: &str,
    system_instruction: Option<&str>,
    version: ApiVersion,
) -> Result<String, InternalError> {
    let url = construct_url(model_name, api_key, false, version);
    let request_body = GenerateContentRequest {
        contents: vec![Content {
            parts: vec![Part {
                text: Some(prompt_text.to_string()),
                function_call: None,
                function_response: None,
            }],
            role: None,
        }],
        system_instruction: system_instruction.map(|text| Content {
            parts: vec![Part {
                text: Some(text.to_string()),
                function_call: None,
                function_response: None,
            }],
            role: None,
        }),
        tools: None,
    };

    let response = http_client.post(&url).json(&request_body).send().await?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            // Use function directly instead of closure
            .map_err(InternalError::Http)?;
        return Err(InternalError::Api(error_text));
    }

    // Use function directly instead of closure
    let response_text = response.text().await.map_err(InternalError::Http)?;

    let response_body: GenerateContentResponse = serde_json::from_str(&response_text)?;

    // Use .first() instead of .get(0)
    if let Some(candidate) = response_body.candidates.first() {
        if let Some(part) = candidate.content.parts.first() {
            if let Some(text) = &part.text {
                return Ok(text.clone());
            }
        }
    }
    Err(InternalError::Parse(
        "No text content found in response structure".to_string(),
    ))
}

// Make pub so rust-genai can call them
pub fn generate_content_stream_internal<'a>(
    http_client: &'a ReqwestClient,
    api_key: &'a str,
    model_name: &'a str,
    prompt_text: &'a str,
    system_instruction: Option<&'a str>,
    version: ApiVersion,
) -> impl Stream<Item = Result<GenerateContentResponse, InternalError>> + Send + 'a {
    let url = construct_url(model_name, api_key, true, version);
    let prompt_text = prompt_text.to_string();

    try_stream! {
        let request_body = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: Some(prompt_text.to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: None,
            }],
            system_instruction: system_instruction.map(|text| Content {
                parts: vec![Part {
                    text: Some(text.to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: None,
            }),
            tools: None,
        };
        let response = http_client
            .post(&url)
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            let mut byte_stream = response.bytes_stream();
            let mut buffer = Vec::new();
            while let Some(chunk_result) = byte_stream.next().await {
                let chunk: Bytes = chunk_result?;
                buffer.extend_from_slice(&chunk);
                while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                    let line_bytes_with_newline = buffer.drain(..=newline_pos).collect::<Vec<u8>>();
                    let line = str::from_utf8(&line_bytes_with_newline)?.trim_end_matches(|c| c == '\n' || c == '\r');
                    if line.starts_with("data:") {
                        let json_data = line.strip_prefix("data:").unwrap_or("").trim_start();
                        if !json_data.is_empty() {
                            let chunk_response: GenerateContentResponse = serde_json::from_str(json_data)?;
                            yield chunk_response;
                        }
                    }
                }
            }
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
            Err(InternalError::Api(error_text))?;
        }
    }
}
