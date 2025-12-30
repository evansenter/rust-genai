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
/// match statements. Unknown chunk types deserialize to the `Unknown` variant
/// with their data preserved.
#[derive(Clone, Debug)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
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
    /// The `chunk_type` field contains the unrecognized type string, and `data`
    /// contains the full JSON data for inspection or debugging.
    Unknown {
        /// The unrecognized chunk type from the API
        chunk_type: String,
        /// The raw JSON data, preserved for debugging and roundtrip serialization
        data: serde_json::Value,
    },
}

impl StreamChunk {
    /// Check if this is an unknown chunk type.
    #[must_use]
    pub const fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }

    /// Returns the chunk type name if this is an unknown chunk type.
    ///
    /// Returns `None` for known chunk types.
    #[must_use]
    pub fn unknown_chunk_type(&self) -> Option<&str> {
        match self {
            Self::Unknown { chunk_type, .. } => Some(chunk_type),
            _ => None,
        }
    }

    /// Returns the raw JSON data if this is an unknown chunk type.
    ///
    /// Returns `None` for known chunk types.
    #[must_use]
    pub fn unknown_data(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Unknown { data, .. } => Some(data),
            _ => None,
        }
    }
}

impl Serialize for StreamChunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        match self {
            Self::Delta(content) => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("chunk_type", "delta")?;
                map.serialize_entry("data", content)?;
                map.end()
            }
            Self::Complete(response) => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("chunk_type", "complete")?;
                map.serialize_entry("data", response)?;
                map.end()
            }
            Self::Unknown { chunk_type, data } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("chunk_type", chunk_type)?;
                if !data.is_null() {
                    map.serialize_entry("data", data)?;
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for StreamChunk {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        let chunk_type = match value.get("chunk_type") {
            Some(serde_json::Value::String(s)) => s.as_str(),
            Some(other) => {
                log::warn!(
                    "StreamChunk received non-string chunk_type: {}. \
                     This may indicate a malformed API response.",
                    other
                );
                "<non-string chunk_type>"
            }
            None => {
                log::warn!(
                    "StreamChunk is missing required chunk_type field. \
                     This may indicate a malformed API response."
                );
                "<missing chunk_type>"
            }
        };

        match chunk_type {
            "delta" => {
                let data = match value.get("data").cloned() {
                    Some(d) => d,
                    None => {
                        log::warn!(
                            "StreamChunk::Delta is missing the 'data' field. \
                             This may indicate a malformed API response."
                        );
                        serde_json::Value::Null
                    }
                };
                let content: InteractionContent = serde_json::from_value(data).map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Failed to deserialize StreamChunk::Delta data: {}",
                        e
                    ))
                })?;
                Ok(Self::Delta(content))
            }
            "complete" => {
                let data = match value.get("data").cloned() {
                    Some(d) => d,
                    None => {
                        log::warn!(
                            "StreamChunk::Complete is missing the 'data' field. \
                             This may indicate a malformed API response."
                        );
                        serde_json::Value::Null
                    }
                };
                let response: InteractionResponse = serde_json::from_value(data).map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Failed to deserialize StreamChunk::Complete data: {}",
                        e
                    ))
                })?;
                Ok(Self::Complete(response))
            }
            other => {
                log::warn!(
                    "Encountered unknown StreamChunk type '{}'. \
                     This may indicate a new API feature. \
                     The chunk will be preserved in the Unknown variant.",
                    other
                );
                let data = value
                    .get("data")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                Ok(Self::Unknown {
                    chunk_type: other.to_string(),
                    data,
                })
            }
        }
    }
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
            id: Some("test-interaction-123".to_string()),
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
                assert_eq!(response.id.as_deref(), Some("test-interaction-123"));
                assert_eq!(response.status, InteractionStatus::Completed);
                assert_eq!(response.text(), Some("The answer is 4."));
            }
            _ => panic!("Expected Complete variant"),
        }
    }

    #[test]
    fn test_stream_chunk_unknown_forward_compatibility() {
        // Simulate a future chunk type that doesn't exist yet
        let unknown_json = r#"{"chunk_type": "future_chunk_type", "data": {"key": "value"}}"#;
        let deserialized: StreamChunk =
            serde_json::from_str(unknown_json).expect("Should deserialize unknown variant");

        // Verify it's an Unknown variant
        assert!(deserialized.is_unknown());
        assert_eq!(deserialized.unknown_chunk_type(), Some("future_chunk_type"));

        // Verify data is preserved
        let data = deserialized.unknown_data().expect("Should have data");
        assert_eq!(data["key"], "value");

        // Verify roundtrip serialization
        let reserialized = serde_json::to_string(&deserialized).expect("Should serialize");
        assert!(reserialized.contains("future_chunk_type"));
        assert!(reserialized.contains("value"));
    }

    #[test]
    fn test_stream_chunk_unknown_without_data() {
        // Test unknown chunk type without data field
        let unknown_json = r#"{"chunk_type": "no_data_chunk"}"#;
        let deserialized: StreamChunk =
            serde_json::from_str(unknown_json).expect("Should deserialize unknown variant");

        assert!(deserialized.is_unknown());
        assert_eq!(deserialized.unknown_chunk_type(), Some("no_data_chunk"));

        // Data should be null when not provided
        let data = deserialized.unknown_data().expect("Should have data field");
        assert!(data.is_null());
    }
}
