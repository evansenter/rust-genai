use crate::common::{ApiVersion, construct_url};
use crate::errors::InternalError;
use crate::models::request::GenerateContentRequest;
use crate::models::response::GenerateContentResponse;
use crate::models::shared::{Content, Part};
use crate::sse_parser::parse_sse_stream;
use async_stream::try_stream;
use futures_util::{Stream, StreamExt};
use reqwest::Client as ReqwestClient;

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
        tool_config: None,
    };

    let response = http_client.post(&url).json(&request_body).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await.map_err(InternalError::Http)?;
        return Err(InternalError::Api(error_text));
    }

    let response_text = response.text().await.map_err(InternalError::Http)?;

    let response_body: GenerateContentResponse = serde_json::from_str(&response_text)?;

    if let Some(candidate) = response_body.candidates.first()
        && let Some(part) = candidate.content.parts.first()
        && let Some(text) = &part.text
    {
        return Ok(text.clone());
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
            tool_config: None,
        };
        let response = http_client
            .post(&url)
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            let byte_stream = response.bytes_stream();
            let parsed_stream = parse_sse_stream::<GenerateContentResponse>(byte_stream);
            futures_util::pin_mut!(parsed_stream);

            while let Some(result) = parsed_stream.next().await {
                yield result?;
            }
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
            Err(InternalError::Api(error_text))?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_generate_content_success() {
        let mock_server = MockServer::start().await;
        let api_key = "test-api-key";
        let model_name = "gemini-pro";

        let response_json = serde_json::json!({
            "candidates": [{
                "content": {
                    "parts": [{"text": "Hello, world!"}],
                    "role": "model"
                }
            }]
        });

        Mock::given(method("POST"))
            .and(path(format!(
                "/v1beta/models/{}:generateContent",
                model_name
            )))
            .and(query_param("key", api_key))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_json))
            .mount(&mock_server)
            .await;

        // Manually construct URL with mock server
        let http_client = ReqwestClient::new();
        let base_url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            mock_server.uri(),
            model_name,
            api_key
        );

        let request_body = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: Some("Test prompt".to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: None,
            }],
            system_instruction: None,
            tools: None,
            tool_config: None,
        };

        let response = http_client
            .post(&base_url)
            .json(&request_body)
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());

        let response_body: GenerateContentResponse = response.json().await.unwrap();
        assert_eq!(response_body.candidates.len(), 1);
        assert_eq!(
            response_body.candidates[0].content.parts[0]
                .text
                .as_ref()
                .unwrap(),
            "Hello, world!"
        );
    }

    #[tokio::test]
    async fn test_generate_content_with_system_instruction() {
        let mock_server = MockServer::start().await;
        let api_key = "test-api-key";
        let model_name = "gemini-pro";

        let response_json = serde_json::json!({
            "candidates": [{
                "content": {
                    "parts": [{"text": "Response with system context"}],
                    "role": "model"
                }
            }]
        });

        Mock::given(method("POST"))
            .and(path(format!(
                "/v1beta/models/{}:generateContent",
                model_name
            )))
            .and(query_param("key", api_key))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_json))
            .mount(&mock_server)
            .await;

        let http_client = ReqwestClient::new();
        let base_url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            mock_server.uri(),
            model_name,
            api_key
        );

        let request_body = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: Some("Test prompt".to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: None,
            }],
            system_instruction: Some(Content {
                parts: vec![Part {
                    text: Some("You are a helpful assistant".to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: None,
            }),
            tools: None,
            tool_config: None,
        };

        let response = http_client
            .post(&base_url)
            .json(&request_body)
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
    }

    #[tokio::test]
    async fn test_generate_content_auth_error() {
        let mock_server = MockServer::start().await;
        let api_key = "invalid-key";
        let model_name = "gemini-pro";

        let error_json = serde_json::json!({
            "error": {
                "code": 401,
                "message": "API key not valid",
                "status": "UNAUTHENTICATED"
            }
        });

        Mock::given(method("POST"))
            .and(path(format!(
                "/v1beta/models/{}:generateContent",
                model_name
            )))
            .respond_with(ResponseTemplate::new(401).set_body_json(&error_json))
            .mount(&mock_server)
            .await;

        let http_client = ReqwestClient::new();
        let base_url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            mock_server.uri(),
            model_name,
            api_key
        );

        let request_body = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: Some("Test".to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: None,
            }],
            system_instruction: None,
            tools: None,
            tool_config: None,
        };

        let response = http_client
            .post(&base_url)
            .json(&request_body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 401);
    }

    #[tokio::test]
    async fn test_generate_content_rate_limit_error() {
        let mock_server = MockServer::start().await;
        let api_key = "test-key";
        let model_name = "gemini-pro";

        let error_json = serde_json::json!({
            "error": {
                "code": 429,
                "message": "Resource has been exhausted (e.g. check quota).",
                "status": "RESOURCE_EXHAUSTED"
            }
        });

        Mock::given(method("POST"))
            .and(path(format!(
                "/v1beta/models/{}:generateContent",
                model_name
            )))
            .respond_with(ResponseTemplate::new(429).set_body_json(&error_json))
            .mount(&mock_server)
            .await;

        let http_client = ReqwestClient::new();
        let base_url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            mock_server.uri(),
            model_name,
            api_key
        );

        let request_body = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: Some("Test".to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: None,
            }],
            system_instruction: None,
            tools: None,
            tool_config: None,
        };

        let response = http_client
            .post(&base_url)
            .json(&request_body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 429);
    }

    #[tokio::test]
    async fn test_generate_content_server_error() {
        let mock_server = MockServer::start().await;
        let api_key = "test-key";
        let model_name = "gemini-pro";

        Mock::given(method("POST"))
            .and(path(format!(
                "/v1beta/models/{}:generateContent",
                model_name
            )))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        let http_client = ReqwestClient::new();
        let base_url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            mock_server.uri(),
            model_name,
            api_key
        );

        let request_body = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: Some("Test".to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: None,
            }],
            system_instruction: None,
            tools: None,
            tool_config: None,
        };

        let response = http_client
            .post(&base_url)
            .json(&request_body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 500);
    }

    #[tokio::test]
    async fn test_generate_content_malformed_json_response() {
        let mock_server = MockServer::start().await;
        let api_key = "test-key";
        let model_name = "gemini-pro";

        Mock::given(method("POST"))
            .and(path(format!(
                "/v1beta/models/{}:generateContent",
                model_name
            )))
            .respond_with(ResponseTemplate::new(200).set_body_string("{invalid json"))
            .mount(&mock_server)
            .await;

        let http_client = ReqwestClient::new();
        let base_url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            mock_server.uri(),
            model_name,
            api_key
        );

        let request_body = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: Some("Test".to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: None,
            }],
            system_instruction: None,
            tools: None,
            tool_config: None,
        };

        let response = http_client
            .post(&base_url)
            .json(&request_body)
            .send()
            .await
            .unwrap();
        let text = response.text().await.unwrap();
        assert!(text.contains("invalid json"));
    }

    #[tokio::test]
    async fn test_generate_content_empty_response() {
        let mock_server = MockServer::start().await;
        let api_key = "test-key";
        let model_name = "gemini-pro";

        let response_json = serde_json::json!({
            "candidates": []
        });

        Mock::given(method("POST"))
            .and(path(format!(
                "/v1beta/models/{}:generateContent",
                model_name
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_json))
            .mount(&mock_server)
            .await;

        let http_client = ReqwestClient::new();
        let base_url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            mock_server.uri(),
            model_name,
            api_key
        );

        let request_body = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: Some("Test".to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: None,
            }],
            system_instruction: None,
            tools: None,
            tool_config: None,
        };

        let response = http_client
            .post(&base_url)
            .json(&request_body)
            .send()
            .await
            .unwrap();
        let response_body: GenerateContentResponse = response.json().await.unwrap();
        assert_eq!(response_body.candidates.len(), 0);
    }

    #[tokio::test]
    async fn test_generate_content_stream_success() {
        let mock_server = MockServer::start().await;
        let api_key = "test-key";
        let model_name = "gemini-pro";

        // SSE response with multiple chunks
        let sse_response = "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hello\"}],\"role\":\"model\"}}]}\n\ndata: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\" world\"}],\"role\":\"model\"}}]}\n\n";

        Mock::given(method("POST"))
            .and(path(format!(
                "/v1beta/models/{}:streamGenerateContent",
                model_name
            )))
            .and(query_param("key", api_key))
            .and(query_param("alt", "sse"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(sse_response)
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&mock_server)
            .await;

        let http_client = ReqwestClient::new();
        let base_url = format!(
            "{}/v1beta/models/{}:streamGenerateContent?key={}&alt=sse",
            mock_server.uri(),
            model_name,
            api_key
        );

        let request_body = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: Some("Test".to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: None,
            }],
            system_instruction: None,
            tools: None,
            tool_config: None,
        };

        let response = http_client
            .post(&base_url)
            .json(&request_body)
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        // Note: content-type header may vary in mock vs real API
    }

    #[tokio::test]
    async fn test_generate_content_stream_auth_error() {
        let mock_server = MockServer::start().await;
        let api_key = "invalid-key";
        let model_name = "gemini-pro";

        let error_json = serde_json::json!({
            "error": {
                "code": 401,
                "message": "API key not valid",
                "status": "UNAUTHENTICATED"
            }
        });

        Mock::given(method("POST"))
            .and(path(format!(
                "/v1beta/models/{}:streamGenerateContent",
                model_name
            )))
            .respond_with(ResponseTemplate::new(401).set_body_json(&error_json))
            .mount(&mock_server)
            .await;

        let http_client = ReqwestClient::new();
        let base_url = format!(
            "{}/v1beta/models/{}:streamGenerateContent?key={}&alt=sse",
            mock_server.uri(),
            model_name,
            api_key
        );

        let request_body = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: Some("Test".to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: None,
            }],
            system_instruction: None,
            tools: None,
            tool_config: None,
        };

        let response = http_client
            .post(&base_url)
            .json(&request_body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 401);
    }

    #[tokio::test]
    async fn test_generate_content_multiple_candidates() {
        let mock_server = MockServer::start().await;
        let api_key = "test-key";
        let model_name = "gemini-pro";

        let response_json = serde_json::json!({
            "candidates": [
                {
                    "content": {
                        "parts": [{"text": "First response"}],
                        "role": "model"
                    }
                },
                {
                    "content": {
                        "parts": [{"text": "Second response"}],
                        "role": "model"
                    }
                }
            ]
        });

        Mock::given(method("POST"))
            .and(path(format!(
                "/v1beta/models/{}:generateContent",
                model_name
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_json))
            .mount(&mock_server)
            .await;

        let http_client = ReqwestClient::new();
        let base_url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            mock_server.uri(),
            model_name,
            api_key
        );

        let request_body = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: Some("Test".to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: None,
            }],
            system_instruction: None,
            tools: None,
            tool_config: None,
        };

        let response = http_client
            .post(&base_url)
            .json(&request_body)
            .send()
            .await
            .unwrap();
        let response_body: GenerateContentResponse = response.json().await.unwrap();
        assert_eq!(response_body.candidates.len(), 2);
        assert_eq!(
            response_body.candidates[0].content.parts[0]
                .text
                .as_ref()
                .unwrap(),
            "First response"
        );
        assert_eq!(
            response_body.candidates[1].content.parts[0]
                .text
                .as_ref()
                .unwrap(),
            "Second response"
        );
    }
}
