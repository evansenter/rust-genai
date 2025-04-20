use reqwest::Client;
use std::error::Error;
use futures_util::{Stream, StreamExt};
use async_stream::try_stream;
use bytes::Bytes;
use std::str;

// Declare the models module
mod models;

// Import the necessary structs from the models module
use models::request::{Content, GenerateContentRequest, Part};
use models::response::GenerateContentResponse;

// Define a concrete error type for better handling
#[derive(Debug, thiserror::Error)]
pub enum StreamingError {
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

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

pub async fn generate_content(
    api_key: &str,
    model_name: &str,
    prompt_text: &str,
) -> Result<String, Box<dyn Error>> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model_name,
        api_key
    );

    let request_body = GenerateContentRequest {
        contents: vec![Content {
            parts: vec![Part { text: prompt_text.to_string() }],
        }],
    };

    let client = Client::new();
    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("API Error: {}", error_text).into());
    }

    let response_body = response.json::<GenerateContentResponse>().await?;

    if let Some(candidate) = response_body.candidates.get(0) {
        if let Some(part) = candidate.content.parts.get(0) {
            return Ok(part.text.clone());
        }
    }

    Err("No text content found in the response".into())
}

pub fn generate_content_stream<'a>(
    api_key: &'a str,
    model_name: &'a str,
    prompt_text: &'a str,
) -> impl Stream<Item = Result<GenerateContentResponse, StreamingError>> + Send + 'a {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}&alt=sse",
        model_name,
        api_key
    );
    let prompt_text = prompt_text.to_string();

    try_stream! {
        let client = Client::new();
        let request_body = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part { text: prompt_text }],
            }],
        };

        let response = client
            .post(&url)
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
            Err(StreamingError::Api(error_text))?;
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
