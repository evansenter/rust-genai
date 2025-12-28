use crate::common::{Endpoint, construct_endpoint_url};
use crate::error_helpers::{check_response, deserialize_with_context};
use crate::errors::GenaiError;
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
    let url = construct_endpoint_url(endpoint, api_key);

    let response = http_client.post(&url).json(&request).send().await?;
    let response = check_response(response).await?;
    let response_text = response.text().await.map_err(GenaiError::Http)?;
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
    let url = construct_endpoint_url(endpoint, api_key);

    try_stream! {
        let response = http_client
            .post(&url)
            .json(&request)
            .send()
            .await?;
        let response = check_response(response).await?;
        let byte_stream = response.bytes_stream();
        let parsed_stream = parse_sse_stream::<InteractionStreamEvent>(byte_stream);
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

            // Handle different event types
            // Known event types from the Interactions API:
            // - content.delta: Incremental content updates (yields Delta)
            // - interaction.complete: Final response with full content (yields Complete)
            // - interaction.start: Lifecycle signal, has interaction but NOT final
            // - interaction.status_update: Status changes (no content)
            // - content.start/content.stop: Content block boundaries (no content)
            match event.event_type.as_str() {
                "content.delta" => {
                    // Incremental content update
                    if let Some(delta) = event.delta {
                        yield StreamChunk::Delta(delta);
                    }
                }
                "interaction.complete" => {
                    // Final complete response - only yield Complete for this event type
                    if let Some(interaction) = event.interaction {
                        yield StreamChunk::Complete(interaction);
                    }
                }
                "interaction.start" | "interaction.status_update" | "content.start" | "content.stop" => {
                    // Known lifecycle events - skip silently (they don't contain useful content)
                    // - interaction.start: Signals interaction has started (has interaction field but not final)
                    // - interaction.status_update: Status changes during processing
                    // - content.start/content.stop: Content block boundaries
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
    let url = construct_endpoint_url(endpoint, api_key);

    let response = http_client.get(&url).send().await?;
    let response = check_response(response).await?;
    let response_text = response.text().await.map_err(GenaiError::Http)?;
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
    let url = construct_endpoint_url(endpoint, api_key);

    let response = http_client.delete(&url).send().await?;
    check_response(response).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::interactions::{InteractionContent, InteractionInput, InteractionStatus};

    #[test]
    fn test_endpoint_url_construction() {
        // Test that we can construct proper URLs for each endpoint
        let endpoint_create = Endpoint::CreateInteraction { stream: false };
        let url = construct_endpoint_url(endpoint_create, "test_key");
        assert!(url.contains("/v1beta/interactions"));
        assert!(url.contains("key=test_key"));

        let endpoint_get = Endpoint::GetInteraction { id: "test_id_123" };
        let url = construct_endpoint_url(endpoint_get, "test_key");
        assert!(url.contains("/v1beta/interactions/test_id_123"));

        let endpoint_delete = Endpoint::DeleteInteraction { id: "test_id_456" };
        let url = construct_endpoint_url(endpoint_delete, "test_key");
        assert!(url.contains("/v1beta/interactions/test_id_456"));
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

        assert_eq!(response.id, "test_interaction_123");
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
}
