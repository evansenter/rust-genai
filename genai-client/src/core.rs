use crate::common::{ApiVersion, construct_url};
use crate::errors::InternalError;
use crate::models::request::{Content, GenerateContentRequest, Part};
use crate::models::response::GenerateContentResponse;
use async_stream::try_stream;
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use reqwest::Client as ReqwestClient;
use std::str;

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
        let error_text = response.text().await.map_err(InternalError::Http)?;
        return Err(InternalError::Api(error_text));
    }

    let response_text = response.text().await.map_err(InternalError::Http)?;

    let response_body: GenerateContentResponse = serde_json::from_str(&response_text)?;

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
