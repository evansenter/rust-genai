//! Unknown variant preservation tests
//!
//! Tests that all types with Unknown variants properly:
//! 1. Deserialize unrecognized values into Unknown variants
//! 2. Preserve the original type string and data
//! 3. Roundtrip correctly (serialize back to original form)
//!
//! These tests are SKIPPED when `strict-unknown` is enabled because
//! strict mode causes deserialization errors instead of Unknown variants.
//!
//! Run tests:
//! - Default: `cargo test --test unknown_variant_tests`
//! - Strict (should skip): `cargo test --test unknown_variant_tests --features strict-unknown`

#![cfg(not(feature = "strict-unknown"))]

use genai_rs::{
    Content, FunctionCallingMode, InteractionStatus, Resolution, Role, StreamChunk, ThinkingLevel,
    ThinkingSummaries,
};
use serde_json::json;

// =============================================================================
// Resolution Unknown Variant Tests
// =============================================================================

mod resolution {
    use super::*;

    #[test]
    fn unknown_resolution_deserializes() {
        let json = json!("super_ultra_high");
        let value: Resolution = serde_json::from_value(json).unwrap();
        assert!(value.is_unknown());
        assert_eq!(value.unknown_resolution_type(), Some("super_ultra_high"));
    }

    #[test]
    fn unknown_resolution_roundtrips() {
        let json = json!("future_resolution");
        let value: Resolution = serde_json::from_value(json.clone()).unwrap();
        let back = serde_json::to_value(&value).unwrap();
        assert_eq!(back, "future_resolution");
    }
}

// =============================================================================
// Content Unknown Variant Tests
// =============================================================================

mod interaction_content {
    use super::*;

    #[test]
    fn unknown_content_type_deserializes() {
        let json = json!({
            "type": "future_content_type",
            "some_field": "value",
            "nested": {"a": 1}
        });
        let content: Content = serde_json::from_value(json).unwrap();
        assert!(content.is_unknown());
        assert_eq!(content.unknown_content_type(), Some("future_content_type"));
    }

    #[test]
    fn unknown_content_preserves_data() {
        let json = json!({
            "type": "new_feature",
            "field1": "value1",
            "field2": 42,
            "nested": {"key": "value"}
        });
        let content: Content = serde_json::from_value(json).unwrap();

        let data = content.unknown_data().unwrap();
        assert_eq!(data["field1"], "value1");
        assert_eq!(data["field2"], 42);
        assert_eq!(data["nested"]["key"], "value");
    }

    #[test]
    fn unknown_content_roundtrips() {
        let json = json!({
            "type": "experimental_type",
            "payload": {"data": "test"}
        });
        let content: Content = serde_json::from_value(json.clone()).unwrap();
        let back = serde_json::to_value(&content).unwrap();

        assert_eq!(back["type"], "experimental_type");
        assert_eq!(back["payload"]["data"], "test");
    }

    #[test]
    fn missing_type_field_becomes_unknown() {
        let json = json!({"foo": "bar", "baz": 42});
        let content: Content = serde_json::from_value(json).unwrap();
        assert!(content.is_unknown());
    }
}

// =============================================================================
// Role Unknown Variant Tests
// =============================================================================

mod role {
    use super::*;

    #[test]
    fn unknown_role_deserializes() {
        let json = json!("assistant");
        let value: Role = serde_json::from_value(json).unwrap();
        assert!(value.is_unknown());
        assert_eq!(value.unknown_role_type(), Some("assistant"));
    }

    #[test]
    fn unknown_role_roundtrips() {
        let json = json!("supervisor");
        let value: Role = serde_json::from_value(json.clone()).unwrap();
        let back = serde_json::to_value(&value).unwrap();
        assert_eq!(back, "supervisor");
    }
}

// =============================================================================
// ThinkingLevel Unknown Variant Tests
// =============================================================================

mod thinking_level {
    use super::*;

    #[test]
    fn unknown_level_deserializes() {
        let json = json!("ultra_high");
        let value: ThinkingLevel = serde_json::from_value(json).unwrap();
        assert!(value.is_unknown());
        assert_eq!(value.unknown_level_type(), Some("ultra_high"));
    }

    #[test]
    fn unknown_level_roundtrips() {
        let json = json!("extreme");
        let value: ThinkingLevel = serde_json::from_value(json.clone()).unwrap();
        let back = serde_json::to_value(&value).unwrap();
        assert_eq!(back, "extreme");
    }
}

// =============================================================================
// ThinkingSummaries Unknown Variant Tests
// =============================================================================

mod thinking_summaries {
    use super::*;

    #[test]
    fn unknown_summaries_deserializes() {
        let json = json!("THINKING_SUMMARIES_VERBOSE");
        let value: ThinkingSummaries = serde_json::from_value(json).unwrap();
        assert!(value.is_unknown());
        assert_eq!(
            value.unknown_summaries_type(),
            Some("THINKING_SUMMARIES_VERBOSE")
        );
    }

    #[test]
    fn unknown_summaries_roundtrips() {
        let json = json!("THINKING_SUMMARIES_DETAILED");
        let value: ThinkingSummaries = serde_json::from_value(json.clone()).unwrap();
        let back = serde_json::to_value(&value).unwrap();
        assert_eq!(back, "THINKING_SUMMARIES_DETAILED");
    }
}

// =============================================================================
// FunctionCallingMode Unknown Variant Tests
// =============================================================================

mod function_calling_mode {
    use super::*;

    #[test]
    fn unknown_mode_deserializes() {
        let json = json!("REQUIRED");
        let value: FunctionCallingMode = serde_json::from_value(json).unwrap();
        assert!(value.is_unknown());
        assert_eq!(value.unknown_mode_type(), Some("REQUIRED"));
    }

    #[test]
    fn unknown_mode_roundtrips() {
        let json = json!("FORCED");
        let value: FunctionCallingMode = serde_json::from_value(json.clone()).unwrap();
        let back = serde_json::to_value(&value).unwrap();
        assert_eq!(back, "FORCED");
    }
}

// =============================================================================
// InteractionStatus Unknown Variant Tests
// =============================================================================

mod interaction_status {
    use super::*;

    #[test]
    fn unknown_status_deserializes() {
        let json = json!("pending_review");
        let value: InteractionStatus = serde_json::from_value(json).unwrap();
        assert!(value.is_unknown());
        assert_eq!(value.unknown_status_type(), Some("pending_review"));
    }

    #[test]
    fn unknown_status_roundtrips() {
        let json = json!("queued");
        let value: InteractionStatus = serde_json::from_value(json.clone()).unwrap();
        let back = serde_json::to_value(&value).unwrap();
        assert_eq!(back, "queued");
    }
}

// =============================================================================
// StreamChunk Unknown Variant Tests
// =============================================================================

mod stream_chunk {
    use super::*;

    #[test]
    fn unknown_chunk_deserializes() {
        // StreamChunk uses "chunk_type" field, not "event_type"
        let json = json!({
            "chunk_type": "future_chunk",
            "data": {"key": "value"}
        });
        let value: StreamChunk = serde_json::from_value(json).unwrap();
        assert!(value.is_unknown());
        assert_eq!(value.unknown_chunk_type(), Some("future_chunk"));
    }

    #[test]
    fn unknown_chunk_roundtrips() {
        let json = json!({
            "chunk_type": "new_chunk_type",
            "data": {"payload": "test"}
        });
        let value: StreamChunk = serde_json::from_value(json.clone()).unwrap();
        let back = serde_json::to_value(&value).unwrap();
        assert_eq!(back["chunk_type"], "new_chunk_type");
    }
}

// =============================================================================
// Comprehensive: All Unknown Variants Have Helper Methods
// =============================================================================

mod helper_methods {
    use super::*;

    #[test]
    fn all_unknown_variants_have_is_unknown() {
        // Resolution
        let resolution: Resolution = serde_json::from_value(json!("future")).unwrap();
        assert!(resolution.is_unknown());

        // Content
        let content: Content = serde_json::from_value(json!({"type": "future"})).unwrap();
        assert!(content.is_unknown());

        // Role
        let role: Role = serde_json::from_value(json!("future")).unwrap();
        assert!(role.is_unknown());

        // ThinkingLevel
        let level: ThinkingLevel = serde_json::from_value(json!("future")).unwrap();
        assert!(level.is_unknown());

        // ThinkingSummaries
        let summaries: ThinkingSummaries = serde_json::from_value(json!("future")).unwrap();
        assert!(summaries.is_unknown());

        // FunctionCallingMode
        let mode: FunctionCallingMode = serde_json::from_value(json!("FUTURE")).unwrap();
        assert!(mode.is_unknown());

        // InteractionStatus
        let status: InteractionStatus = serde_json::from_value(json!("future")).unwrap();
        assert!(status.is_unknown());

        // StreamChunk (uses "chunk_type" field)
        let chunk: StreamChunk = serde_json::from_value(json!({"chunk_type": "future"})).unwrap();
        assert!(chunk.is_unknown());
    }

    #[test]
    fn all_unknown_variants_have_type_getter() {
        // Resolution
        let resolution: Resolution = serde_json::from_value(json!("test_res")).unwrap();
        assert_eq!(resolution.unknown_resolution_type(), Some("test_res"));

        // Content
        let content: Content = serde_json::from_value(json!({"type": "test_content"})).unwrap();
        assert_eq!(content.unknown_content_type(), Some("test_content"));

        // Role
        let role: Role = serde_json::from_value(json!("test_role")).unwrap();
        assert_eq!(role.unknown_role_type(), Some("test_role"));

        // ThinkingLevel
        let level: ThinkingLevel = serde_json::from_value(json!("test_level")).unwrap();
        assert_eq!(level.unknown_level_type(), Some("test_level"));

        // ThinkingSummaries
        let summaries: ThinkingSummaries = serde_json::from_value(json!("test_summaries")).unwrap();
        assert_eq!(summaries.unknown_summaries_type(), Some("test_summaries"));

        // FunctionCallingMode
        let mode: FunctionCallingMode = serde_json::from_value(json!("TEST_MODE")).unwrap();
        assert_eq!(mode.unknown_mode_type(), Some("TEST_MODE"));

        // InteractionStatus
        let status: InteractionStatus = serde_json::from_value(json!("test_status")).unwrap();
        assert_eq!(status.unknown_status_type(), Some("test_status"));

        // StreamChunk (uses "chunk_type" field)
        let chunk: StreamChunk =
            serde_json::from_value(json!({"chunk_type": "test_chunk"})).unwrap();
        assert_eq!(chunk.unknown_chunk_type(), Some("test_chunk"));
    }

    #[test]
    fn all_unknown_variants_have_data_getter() {
        // Resolution
        let resolution: Resolution = serde_json::from_value(json!("test")).unwrap();
        assert!(resolution.unknown_data().is_some());

        // Content
        let content: Content =
            serde_json::from_value(json!({"type": "test", "extra": 42})).unwrap();
        let data = content.unknown_data().unwrap();
        assert_eq!(data["extra"], 42);

        // StreamChunk (uses "chunk_type" field, data in "data" field)
        let chunk: StreamChunk =
            serde_json::from_value(json!({"chunk_type": "test", "data": {"payload": "data"}}))
                .unwrap();
        let data = chunk.unknown_data().unwrap();
        assert!(data.get("payload").is_some() || data.get("chunk_type").is_some());
    }
}

// =============================================================================
// Edge Cases
// =============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn empty_type_string_becomes_unknown() {
        let json = json!("");
        let role: Role = serde_json::from_value(json).unwrap();
        assert!(role.is_unknown());
        assert_eq!(role.unknown_role_type(), Some(""));
    }

    #[test]
    fn whitespace_type_string_becomes_unknown() {
        let json = json!("   ");
        let level: ThinkingLevel = serde_json::from_value(json).unwrap();
        assert!(level.is_unknown());
    }

    #[test]
    fn special_characters_preserved() {
        let json = json!("type-with-dashes_and_underscores.and.dots");
        let role: Role = serde_json::from_value(json.clone()).unwrap();
        assert!(role.is_unknown());

        let back = serde_json::to_value(&role).unwrap();
        assert_eq!(back, "type-with-dashes_and_underscores.and.dots");
    }

    #[test]
    fn unicode_type_string_preserved() {
        let json = json!("タイプ");
        let role: Role = serde_json::from_value(json.clone()).unwrap();
        assert!(role.is_unknown());

        let back = serde_json::to_value(&role).unwrap();
        assert_eq!(back, "タイプ");
    }
}
