use crate::common::{Endpoint, construct_endpoint_url};
use crate::error_helpers::read_error_with_context;
use crate::errors::InternalError;
use crate::models::interactions::{CreateInteractionRequest, InteractionResponse};
use crate::sse_parser::parse_sse_stream;
use async_stream::try_stream;
use futures_util::{Stream, StreamExt};
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
) -> Result<InteractionResponse, InternalError> {
    let endpoint = Endpoint::CreateInteraction { stream: false };
    let url = construct_endpoint_url(endpoint, api_key);

    let response = http_client.post(&url).json(&request).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await.map_err(InternalError::Http)?;
        return Err(InternalError::Api(error_text));
    }

    let response_text = response.text().await.map_err(InternalError::Http)?;
    let interaction_response: InteractionResponse = serde_json::from_str(&response_text)?;

    Ok(interaction_response)
}

/// Creates a new interaction with streaming responses.
///
/// Returns a stream of InteractionResponse chunks as they arrive from the server.
/// Useful for long-running interactions or agents where you want incremental updates.
pub fn create_interaction_stream<'a>(
    http_client: &'a ReqwestClient,
    api_key: &'a str,
    request: CreateInteractionRequest,
) -> impl Stream<Item = Result<InteractionResponse, InternalError>> + Send + 'a {
    let endpoint = Endpoint::CreateInteraction { stream: true };
    let url = construct_endpoint_url(endpoint, api_key);

    try_stream! {
        let response = http_client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            let byte_stream = response.bytes_stream();
            let parsed_stream = parse_sse_stream::<InteractionResponse>(byte_stream);
            futures_util::pin_mut!(parsed_stream);

            while let Some(result) = parsed_stream.next().await {
                yield result?;
            }
        } else {
            Err(read_error_with_context(response).await)?;
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
) -> Result<InteractionResponse, InternalError> {
    let endpoint = Endpoint::GetInteraction { id: interaction_id };
    let url = construct_endpoint_url(endpoint, api_key);

    let response = http_client.get(&url).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await.map_err(InternalError::Http)?;
        return Err(InternalError::Api(error_text));
    }

    let response_text = response.text().await.map_err(InternalError::Http)?;
    let interaction_response: InteractionResponse = serde_json::from_str(&response_text)?;

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
) -> Result<(), InternalError> {
    let endpoint = Endpoint::DeleteInteraction { id: interaction_id };
    let url = construct_endpoint_url(endpoint, api_key);

    let response = http_client.delete(&url).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await.map_err(InternalError::Http)?;
        return Err(InternalError::Api(error_text));
    }

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
