use crate::common::{API_KEY_HEADER, Endpoint, construct_endpoint_url};
use crate::error_helpers::{check_response, deserialize_with_context};
use crate::errors::GenaiError;
use crate::loud_wire;
use crate::models::interactions::{
    CreateInteractionRequest, InteractionResponse, InteractionStreamEvent, StreamChunk,
};
use crate::sse_parser::parse_sse_stream;
use async_stream::try_stream;
use futures_util::{Stream, StreamExt};
use log::{debug, warn};
use reqwest::Client as ReqwestClient;

/// Creates a new interaction with the Gemini API.
///
/// This is the unified interface for interacting with both models and agents.
/// Supports function calling, structured outputs, and more.
///
/// # Errors
///
/// Returns an error if:
/// - The HTTP request fails
/// - The response status is not successful
/// - The response cannot be parsed as JSON
pub async fn create_interaction(
    http_client: &ReqwestClient,
    api_key: &str,
    request: CreateInteractionRequest,
) -> Result<InteractionResponse, GenaiError> {
    let endpoint = Endpoint::CreateInteraction { stream: false };
    let url = construct_endpoint_url(endpoint);

    // LOUD_WIRE: Log outgoing request
    let request_id = loud_wire::next_request_id();
    let request_body = match serde_json::to_string(&request) {
        Ok(body) => Some(body),
        Err(e) => {
            log::warn!("LOUD_WIRE: Failed to serialize request body: {}", e);
            None
        }
    };
    loud_wire::log_request(request_id, "POST", &url, request_body.as_deref());

    let response = http_client
        .post(&url)
        .header(API_KEY_HEADER, api_key)
        .json(&request)
        .send()
        .await?;

    // LOUD_WIRE: Log response status
    loud_wire::log_response_status(request_id, response.status().as_u16());

    let response = check_response(response).await?;
    let response_text = response.text().await.map_err(GenaiError::Http)?;

    // LOUD_WIRE: Log response body
    loud_wire::log_response_body(request_id, &response_text);

    let interaction_response: InteractionResponse =
        deserialize_with_context(&response_text, "InteractionResponse from create")?;

    Ok(interaction_response)
}

/// Creates a new interaction with streaming responses.
///
/// Returns a stream of `StreamChunk` items as they arrive from the server.
/// Each chunk can be either:
/// - `StreamChunk::Delta`: Incremental content (text or thought)
/// - `StreamChunk::Complete`: The final complete interaction response
///
/// # Example
/// ```ignore
/// let stream = create_interaction_stream(&client, &api_key, request);
/// while let Some(chunk) = stream.next().await {
///     match chunk? {
///         StreamChunk::Delta(delta) => {
///             if let Some(text) = delta.text() {
///                 print!("{}", text);
///             }
///         }
///         StreamChunk::Complete(response) => {
///             println!("\nComplete: {} tokens", response.usage.map(|u| u.total_tokens).flatten().unwrap_or(0));
///         }
///     }
/// }
/// ```
pub fn create_interaction_stream<'a>(
    http_client: &'a ReqwestClient,
    api_key: &'a str,
    request: CreateInteractionRequest,
) -> impl Stream<Item = Result<StreamChunk, GenaiError>> + Send + 'a {
    let endpoint = Endpoint::CreateInteraction { stream: true };
    let url = construct_endpoint_url(endpoint);

    // LOUD_WIRE: Log outgoing request (before try_stream! to capture request_id)
    let request_id = loud_wire::next_request_id();
    let request_body = match serde_json::to_string(&request) {
        Ok(body) => Some(body),
        Err(e) => {
            log::warn!("LOUD_WIRE: Failed to serialize request body: {}", e);
            None
        }
    };
    loud_wire::log_request(request_id, "POST (stream)", &url, request_body.as_deref());

    try_stream! {
        let response = http_client
            .post(&url)
            .header(API_KEY_HEADER, api_key)
            .json(&request)
            .send()
            .await?;

        // LOUD_WIRE: Log response status
        loud_wire::log_response_status(request_id, response.status().as_u16());

        let response = check_response(response).await?;
        let byte_stream = response.bytes_stream();
        let parsed_stream = parse_sse_stream::<InteractionStreamEvent>(byte_stream, request_id);
        futures_util::pin_mut!(parsed_stream);

        while let Some(result) = parsed_stream.next().await {
            let event = result?;
            debug!(
                "SSE event received: event_type={:?}, has_delta={}, has_interaction={}, interaction_id={:?}",
                event.event_type,
                event.delta.is_some(),
                event.interaction.is_some(),
                event.interaction_id
            );

            // Handle different event types from the Interactions API:
            // - interaction.start: Initial event with interaction data (yields Start)
            // - interaction.status_update: Status changes (yields StatusUpdate)
            // - content.start: Content block begins (yields ContentStart)
            // - content.delta: Incremental content updates (yields Delta)
            // - content.stop: Content block ends (yields ContentStop)
            // - interaction.complete: Final response (yields Complete)
            // - error: Error occurred (yields Error)
            match event.event_type.as_str() {
                "interaction.start" => {
                    // Interaction has started - provides early access to interaction ID
                    if let Some(interaction) = event.interaction {
                        yield StreamChunk::Start { interaction };
                    }
                }
                "interaction.status_update" => {
                    // Status change during processing
                    match (event.interaction_id, event.status) {
                        (Some(interaction_id), Some(status)) => {
                            yield StreamChunk::StatusUpdate {
                                interaction_id,
                                status,
                            };
                        }
                        (has_id, has_status) => {
                            debug!(
                                "interaction.status_update missing required fields: interaction_id={:?}, status={:?}",
                                has_id.is_some(),
                                has_status.is_some()
                            );
                        }
                    }
                }
                "content.start" => {
                    // Content generation begins
                    if let Some(index) = event.index {
                        // Try to get content type from the content field if present
                        let content_type = event.content.as_ref().and_then(|c| {
                            // Get the content type name from the variant
                            match c {
                                crate::models::interactions::InteractionContent::Text { .. } => Some("text".to_string()),
                                crate::models::interactions::InteractionContent::Thought { .. } => Some("thought".to_string()),
                                crate::models::interactions::InteractionContent::FunctionCall { .. } => Some("function_call".to_string()),
                                _ => None,
                            }
                        });
                        yield StreamChunk::ContentStart { index, content_type };
                    } else {
                        debug!("content.start event missing index field");
                    }
                }
                "content.delta" => {
                    // Incremental content update
                    if let Some(delta) = event.delta {
                        yield StreamChunk::Delta(delta);
                    }
                }
                "content.stop" => {
                    // Content generation ends
                    if let Some(index) = event.index {
                        yield StreamChunk::ContentStop { index };
                    } else {
                        debug!("content.stop event missing index field");
                    }
                }
                "interaction.complete" => {
                    // Final complete response
                    if let Some(interaction) = event.interaction {
                        yield StreamChunk::Complete(interaction);
                    }
                }
                "error" => {
                    // Error occurred during streaming
                    if let Some(error) = event.error {
                        yield StreamChunk::Error {
                            message: error.message,
                            code: error.code,
                        };
                    } else {
                        // If no error object, treat as unknown error
                        yield StreamChunk::Error {
                            message: "Unknown streaming error".to_string(),
                            code: None,
                        };
                    }
                }
                _ => {
                    // For unknown event types, only yield if they have delta content.
                    // Do NOT yield Complete for other event types that happen to have
                    // an interaction field - only interaction.complete is the final response.
                    if let Some(delta) = event.delta {
                        yield StreamChunk::Delta(delta);
                    } else if event.interaction.is_some() {
                        // Warn about unknown event types with interaction fields - this could
                        // indicate API version drift that needs attention
                        warn!(
                            "Unknown event type '{}' has interaction field but is not 'interaction.complete' - skipping",
                            event.event_type
                        );
                    }
                    // Skip events without useful content
                }
            }
        }
    }
}

/// Retrieves an existing interaction by its ID.
///
/// Useful for checking the status of long-running interactions or agents,
/// or for retrieving the full conversation history.
///
/// # Errors
///
/// Returns an error if:
/// - The HTTP request fails
/// - The response status is not successful
/// - The response cannot be parsed as JSON
pub async fn get_interaction(
    http_client: &ReqwestClient,
    api_key: &str,
    interaction_id: &str,
) -> Result<InteractionResponse, GenaiError> {
    let endpoint = Endpoint::GetInteraction { id: interaction_id };
    let url = construct_endpoint_url(endpoint);

    // LOUD_WIRE: Log outgoing request
    let request_id = loud_wire::next_request_id();
    loud_wire::log_request(request_id, "GET", &url, None);

    let response = http_client
        .get(&url)
        .header(API_KEY_HEADER, api_key)
        .send()
        .await?;

    // LOUD_WIRE: Log response status
    loud_wire::log_response_status(request_id, response.status().as_u16());

    let response = check_response(response).await?;
    let response_text = response.text().await.map_err(GenaiError::Http)?;

    // LOUD_WIRE: Log response body
    loud_wire::log_response_body(request_id, &response_text);

    let interaction_response: InteractionResponse =
        deserialize_with_context(&response_text, "InteractionResponse from get")?;

    Ok(interaction_response)
}

/// Deletes an interaction by its ID.
///
/// Removes the interaction from the server, freeing up storage and making it
/// unavailable for future reference via `previous_interaction_id`.
///
/// # Errors
///
/// Returns an error if:
/// - The HTTP request fails
/// - The response status is not successful
pub async fn delete_interaction(
    http_client: &ReqwestClient,
    api_key: &str,
    interaction_id: &str,
) -> Result<(), GenaiError> {
    let endpoint = Endpoint::DeleteInteraction { id: interaction_id };
    let url = construct_endpoint_url(endpoint);

    // LOUD_WIRE: Log outgoing request
    let request_id = loud_wire::next_request_id();
    loud_wire::log_request(request_id, "DELETE", &url, None);

    let response = http_client
        .delete(&url)
        .header(API_KEY_HEADER, api_key)
        .send()
        .await?;

    // LOUD_WIRE: Log response status
    loud_wire::log_response_status(request_id, response.status().as_u16());

    check_response(response).await?;
    Ok(())
}

/// Cancels a background interaction by its ID.
///
/// Halts an in-progress background interaction. Only applicable to interactions
/// created with `background: true` that are still in `InProgress` status.
///
/// # Errors
///
/// Returns an error if:
/// - The HTTP request fails
/// - The response status is not successful
/// - The response cannot be parsed as JSON
/// - The interaction is not in a cancellable state
pub async fn cancel_interaction(
    http_client: &ReqwestClient,
    api_key: &str,
    interaction_id: &str,
) -> Result<InteractionResponse, GenaiError> {
    let endpoint = Endpoint::CancelInteraction { id: interaction_id };
    let url = construct_endpoint_url(endpoint);

    // LOUD_WIRE: Log outgoing request
    let request_id = loud_wire::next_request_id();
    loud_wire::log_request(request_id, "POST", &url, Some("{}"));

    // Send empty JSON body - the API requires Content-Length header
    let response = http_client
        .post(&url)
        .header(API_KEY_HEADER, api_key)
        .json(&serde_json::json!({}))
        .send()
        .await?;

    // LOUD_WIRE: Log response status
    loud_wire::log_response_status(request_id, response.status().as_u16());

    let response = check_response(response).await?;
    let response_text = response.text().await.map_err(GenaiError::Http)?;

    // LOUD_WIRE: Log response body
    loud_wire::log_response_body(request_id, &response_text);

    let interaction_response: InteractionResponse =
        deserialize_with_context(&response_text, "InteractionResponse from cancel")?;

    Ok(interaction_response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::interactions::{InteractionContent, InteractionInput, InteractionStatus};

    #[test]
    fn test_endpoint_url_construction() {
        // Test that we can construct proper URLs for each endpoint
        // API key is now passed via header, not in URL
        let endpoint_create = Endpoint::CreateInteraction { stream: false };
        let url = construct_endpoint_url(endpoint_create);
        assert!(url.contains("/v1beta/interactions"));
        assert!(!url.contains("key=")); // API key should not be in URL

        let endpoint_get = Endpoint::GetInteraction { id: "test_id_123" };
        let url = construct_endpoint_url(endpoint_get);
        assert!(url.contains("/v1beta/interactions/test_id_123"));
        assert!(!url.contains("key=")); // API key should not be in URL

        let endpoint_delete = Endpoint::DeleteInteraction { id: "test_id_456" };
        let url = construct_endpoint_url(endpoint_delete);
        assert!(url.contains("/v1beta/interactions/test_id_456"));
        assert!(!url.contains("key=")); // API key should not be in URL

        let endpoint_cancel = Endpoint::CancelInteraction { id: "test_id_789" };
        let url = construct_endpoint_url(endpoint_cancel);
        assert!(url.contains("/v1beta/interactions/test_id_789/cancel"));
        assert!(!url.contains("key=")); // API key should not be in URL
    }

    #[test]
    fn test_create_interaction_request_serialization() {
        // Verify request serialization works correctly
        let request = CreateInteractionRequest {
            model: Some("gemini-3-flash-preview".to_string()),
            agent: None,
            input: InteractionInput::Text("Hello".to_string()),
            previous_interaction_id: None,
            tools: None,
            response_modalities: None,
            response_format: None,
            response_mime_type: None,
            generation_config: None,
            stream: None,
            background: None,
            store: None,
            system_instruction: None,
        };

        let json = serde_json::to_string(&request).expect("Serialization should work");
        assert!(json.contains("gemini-3-flash-preview"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_interaction_response_deserialization() {
        // Verify we can deserialize a typical response
        let response_json = r#"{
            "id": "test_interaction_123",
            "model": "gemini-3-flash-preview",
            "input": [{"type": "text", "text": "Hello"}],
            "outputs": [{"type": "text", "text": "Hi there!"}],
            "status": "completed"
        }"#;

        let response: InteractionResponse =
            serde_json::from_str(response_json).expect("Deserialization should work");

        assert_eq!(response.id.as_deref(), Some("test_interaction_123"));
        assert_eq!(response.status, InteractionStatus::Completed);
        assert_eq!(response.outputs.len(), 1);

        // Verify we can access the text content
        match &response.outputs[0] {
            InteractionContent::Text { text } => {
                assert_eq!(text.as_ref().unwrap(), "Hi there!")
            }
            _ => panic!("Expected Text content"),
        }
    }

    #[test]
    fn test_cancelled_interaction_response_deserialization() {
        // Verify we can deserialize a cancelled interaction response
        let response_json = r#"{
            "id": "cancelled_interaction_123",
            "model": "deep-research-pro-preview-12-2025",
            "input": [{"type": "text", "text": "Research topic"}],
            "outputs": [],
            "status": "cancelled"
        }"#;

        let response: InteractionResponse =
            serde_json::from_str(response_json).expect("Deserialization should work");

        assert_eq!(response.id.as_deref(), Some("cancelled_interaction_123"));
        assert_eq!(response.status, InteractionStatus::Cancelled);
        assert!(response.outputs.is_empty());
    }
}
