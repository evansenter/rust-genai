use reqwest::Client as ReqwestClient;
// use std::error::Error; // Removed unused import
use async_stream::try_stream;
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use std::str;

// Declare the models module
mod models;

// Import the necessary structs from the models module
use models::request::{Content, GenerateContentRequest, Part};
// use models::response::GenerateContentResponse; // REMOVED - Re-exported below instead

// Make model structs publicly accessible if needed by the main crate
pub use models::response::GenerateContentResponse;
// pub use models::request::{Content, GenerateContentRequest, Part}; // Only if needed publicly

// Define a concrete error type for better handling
#[derive(Debug, thiserror::Error)]
pub enum GenaiError {
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

// --- Internal Helper Functions ---

// Make pub so rust-genai can call them
pub async fn generate_content_internal(
    http_client: &ReqwestClient,
    api_key: &str,
    model_name: &str,
    prompt_text: &str,
) -> Result<String, GenaiError> {
    // Return GenaiError
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model_name, api_key
    );
    let request_body = GenerateContentRequest {
        contents: vec![Content {
            parts: vec![Part {
                text: prompt_text.to_string(),
            }],
        }],
    };

    let response = http_client
        .post(&url)
        .json(&request_body)
        .send()
        .await?; // Automatically maps reqwest::Error to GenaiError::Http

    if !response.status().is_success() {
        let error_text = response.text().await
            // Use function directly instead of closure
            .map_err(GenaiError::Http)?; 
        return Err(GenaiError::Api(error_text));
    }
    
    // Use function directly instead of closure
    let response_text = response.text().await.map_err(GenaiError::Http)?;

    let response_body: GenerateContentResponse = serde_json::from_str(&response_text)?;

    // Use .first() instead of .get(0)
    if let Some(candidate) = response_body.candidates.first() {
        if let Some(part) = candidate.content.parts.first() {
            return Ok(part.text.clone());
        }
    }
    Err(GenaiError::Parse("No text content found in response structure".to_string()))
}

// Make pub so rust-genai can call them
pub fn generate_content_stream_internal<'a>(
    http_client: &'a ReqwestClient,
    api_key: &'a str,
    model_name: &'a str,
    prompt_text: &'a str,
) -> impl Stream<Item = Result<GenerateContentResponse, GenaiError>> + Send + 'a {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}&alt=sse",
        model_name, api_key
    );
    let prompt_text = prompt_text.to_string();

    try_stream! {
        let request_body = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part { text: prompt_text }],
            }],
        };
        let response = http_client // Use passed client
            .post(&url)
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
            Err(GenaiError::Api(error_text))?;
        } else {
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
        }
    }
}
