//! Streaming types for SSE responses.

use serde::{Deserialize, Serialize};

use super::content::InteractionContent;
use super::response::{InteractionResponse, InteractionStatus};

/// A chunk from the streaming API
///
/// During streaming, the API sends different types of events:
/// - `Delta`: Incremental content updates (text, thought, function_call, etc.)
/// - `Complete`: The final complete interaction response
///
/// Both variants implement `Serialize` and `Deserialize` for logging,
/// persistence, and replay of streaming events.
///
/// # Forward Compatibility
///
/// This enum uses `#[non_exhaustive]` to allow adding new chunk types in future
/// versions without breaking existing code. Always include a wildcard arm in
/// match statements. Unknown chunk types deserialize to the `Unknown` variant.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "chunk_type", content = "data", rename_all = "snake_case")]
#[non_exhaustive]
pub enum StreamChunk {
    /// Incremental content update
    Delta(InteractionContent),
    /// Complete interaction response (final event)
    Complete(InteractionResponse),
    /// Unknown chunk type (for forward compatibility).
    ///
    /// This variant is used when deserializing JSON that contains an unrecognized
    /// `chunk_type`. This allows the library to gracefully handle new chunk types
    /// added by the API in future versions without failing deserialization.
    ///
    /// When encountering an `Unknown` variant, code should typically log a warning
    /// and continue processing, as the stream may still contain useful events.
    #[serde(other)]
    Unknown,
}

/// Wrapper for SSE streaming events from the Interactions API
///
/// The API returns different event types during streaming:
/// - `content.delta`: Contains incremental content in the `delta` field
/// - `interaction.complete`: Contains the full interaction in the `interaction` field
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct InteractionStreamEvent {
    /// Event type (e.g., "content.delta", "interaction.complete")
    pub event_type: String,

    /// The full interaction data (present in "interaction.complete" events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction: Option<InteractionResponse>,

    /// Incremental content delta (present in "content.delta" events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<InteractionContent>,

    /// Interaction ID (present in various events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction_id: Option<String>,

    /// Status (present in status update events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<InteractionStatus>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_chunk_delta_roundtrip() {
        let chunk = StreamChunk::Delta(InteractionContent::Text {
            text: Some("Hello, world!".to_string()),
        });

        let json = serde_json::to_string(&chunk).expect("Serialization should succeed");
        assert!(json.contains("chunk_type"), "Should have chunk_type tag");
        assert!(json.contains("delta"), "Should have delta variant");
        assert!(json.contains("Hello, world!"), "Should have content");

        let deserialized: StreamChunk =
            serde_json::from_str(&json).expect("Deserialization should succeed");

        match deserialized {
            StreamChunk::Delta(content) => {
                assert_eq!(content.text(), Some("Hello, world!"));
            }
            _ => panic!("Expected Delta variant"),
        }
    }

    #[test]
    fn test_stream_chunk_complete_roundtrip() {
        let response = InteractionResponse {
            id: "test-interaction-123".to_string(),
            model: Some("gemini-3-flash-preview".to_string()),
            agent: None,
            input: vec![InteractionContent::Text {
                text: Some("What is 2+2?".to_string()),
            }],
            outputs: vec![InteractionContent::Text {
                text: Some("The answer is 4.".to_string()),
            }],
            status: InteractionStatus::Completed,
            usage: None,
            tools: None,
            grounding_metadata: None,
            url_context_metadata: None,
            previous_interaction_id: None,
        };

        let chunk = StreamChunk::Complete(response);

        let json = serde_json::to_string(&chunk).expect("Serialization should succeed");
        assert!(json.contains("chunk_type"), "Should have chunk_type tag");
        assert!(json.contains("complete"), "Should have complete variant");
        assert!(
            json.contains("test-interaction-123"),
            "Should have interaction id"
        );
        assert!(
            json.contains("The answer is 4"),
            "Should have response text"
        );

        let deserialized: StreamChunk =
            serde_json::from_str(&json).expect("Deserialization should succeed");

        match deserialized {
            StreamChunk::Complete(response) => {
                assert_eq!(response.id, "test-interaction-123");
                assert_eq!(response.status, InteractionStatus::Completed);
                assert_eq!(response.text(), Some("The answer is 4."));
            }
            _ => panic!("Expected Complete variant"),
        }
    }

    #[test]
    fn test_stream_chunk_unknown_forward_compatibility() {
        // Simulate a future chunk type that doesn't exist yet
        // Note: With adjacently tagged enums and #[serde(other)], the Unknown
        // variant must not have a data field (it's a unit variant)
        let unknown_json = r#"{"chunk_type": "future_chunk_type"}"#;
        let deserialized: StreamChunk =
            serde_json::from_str(unknown_json).expect("Should deserialize unknown variant");
        assert!(matches!(deserialized, StreamChunk::Unknown));
    }
}
