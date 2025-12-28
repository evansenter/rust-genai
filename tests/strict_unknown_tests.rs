//! Tests for strict-unknown feature flag behavior
//!
//! These tests verify that the `strict-unknown` feature flag correctly modifies
//! deserialization behavior for unknown content types.
//!
//! When `strict-unknown` is DISABLED (default):
//! - Unknown content types are captured in `InteractionContent::Unknown` variants
//! - Deserialization succeeds even for unrecognized types
//! - Unknown variants can be serialized back (round-trip support)
//!
//! When `strict-unknown` is ENABLED:
//! - Unknown content types cause deserialization errors
//! - Error messages clearly indicate the unknown type and strict mode
//!
//! # Running Tests
//!
//! Default mode (graceful handling):
//! ```sh
//! cargo test --test strict_unknown_tests
//! ```
//!
//! Strict mode (fail on unknown):
//! ```sh
//! cargo test --test strict_unknown_tests --features strict-unknown
//! ```

use genai_client::InteractionContent;
use serde_json::json;

// =============================================================================
// Tests for DEFAULT behavior (strict-unknown DISABLED)
// =============================================================================

/// Verify that unknown content types deserialize into Unknown variants
/// when strict-unknown is disabled.
#[cfg(not(feature = "strict-unknown"))]
#[test]
fn test_unknown_type_deserializes_gracefully() {
    let json = r#"{"type": "future_feature", "data": "test", "extra_field": 42}"#;
    let result: Result<InteractionContent, _> = serde_json::from_str(json);

    assert!(
        result.is_ok(),
        "Unknown type should deserialize successfully"
    );

    let content = result.unwrap();
    assert!(
        matches!(&content, InteractionContent::Unknown { type_name, .. } if type_name == "future_feature"),
        "Should be Unknown variant with correct type_name"
    );
}

/// Verify that the Unknown variant preserves the original JSON data.
#[cfg(not(feature = "strict-unknown"))]
#[test]
fn test_unknown_variant_preserves_data() {
    let json = json!({
        "type": "new_api_feature",
        "field1": "value1",
        "field2": 42,
        "nested": {"a": 1, "b": 2}
    });

    let content: InteractionContent = serde_json::from_value(json.clone()).unwrap();

    if let InteractionContent::Unknown { type_name, data } = content {
        assert_eq!(type_name, "new_api_feature");
        assert_eq!(data["field1"], "value1");
        assert_eq!(data["field2"], 42);
        assert_eq!(data["nested"]["a"], 1);
    } else {
        panic!("Expected Unknown variant");
    }
}

/// Verify that Unknown variants can be serialized back to JSON (round-trip).
#[cfg(not(feature = "strict-unknown"))]
#[test]
fn test_unknown_variant_roundtrip() {
    let original_json = json!({
        "type": "experimental_type",
        "value": 123,
        "metadata": {"version": "1.0"}
    });

    // Deserialize
    let content: InteractionContent = serde_json::from_value(original_json.clone()).unwrap();

    // Serialize back
    let serialized = serde_json::to_value(&content).unwrap();

    // Verify key fields are preserved
    assert_eq!(serialized["type"], "experimental_type");
    assert_eq!(serialized["value"], 123);
    assert_eq!(serialized["metadata"]["version"], "1.0");
}

/// Verify that multiple unknown types in a response are all captured.
#[cfg(not(feature = "strict-unknown"))]
#[test]
fn test_multiple_unknown_types() {
    let items: Vec<InteractionContent> = serde_json::from_value(json!([
        {"type": "unknown_type_a", "data": "a"},
        {"type": "text", "text": "Hello"},
        {"type": "unknown_type_b", "data": "b"}
    ]))
    .unwrap();

    assert_eq!(items.len(), 3);

    // First is unknown
    assert!(matches!(
        &items[0],
        InteractionContent::Unknown { type_name, .. } if type_name == "unknown_type_a"
    ));

    // Second is known (Text)
    assert!(matches!(&items[1], InteractionContent::Text { .. }));

    // Third is unknown
    assert!(matches!(
        &items[2],
        InteractionContent::Unknown { type_name, .. } if type_name == "unknown_type_b"
    ));
}

/// Verify that is_unknown() helper method works correctly.
#[cfg(not(feature = "strict-unknown"))]
#[test]
fn test_is_unknown_method() {
    let unknown: InteractionContent =
        serde_json::from_value(json!({"type": "new_type", "data": 1})).unwrap();

    let known: InteractionContent =
        serde_json::from_value(json!({"type": "text", "text": "hello"})).unwrap();

    assert!(unknown.is_unknown());
    assert!(!known.is_unknown());
}

/// Verify unknown_type() returns the type name for Unknown variants.
#[cfg(not(feature = "strict-unknown"))]
#[test]
fn test_unknown_type_accessor() {
    let content: InteractionContent =
        serde_json::from_value(json!({"type": "brand_new_type", "x": 1})).unwrap();

    assert_eq!(content.unknown_type(), Some("brand_new_type"));

    let text: InteractionContent =
        serde_json::from_value(json!({"type": "text", "text": "hi"})).unwrap();

    assert_eq!(text.unknown_type(), None);
}

/// Verify unknown_data() returns the raw JSON for Unknown variants.
#[cfg(not(feature = "strict-unknown"))]
#[test]
fn test_unknown_data_accessor() {
    let content: InteractionContent =
        serde_json::from_value(json!({"type": "custom_type", "value": 42})).unwrap();

    let data = content.unknown_data().expect("Should have data");
    assert_eq!(data["value"], 42);

    let text: InteractionContent =
        serde_json::from_value(json!({"type": "text", "text": "hi"})).unwrap();

    assert!(text.unknown_data().is_none());
}

/// Verify that content with missing type field is handled as unknown.
#[cfg(not(feature = "strict-unknown"))]
#[test]
fn test_missing_type_field_handled_gracefully() {
    let json = json!({"no_type_field": "value"});
    let result: Result<InteractionContent, _> = serde_json::from_value(json);

    // Should succeed but result in Unknown with "<missing type>" marker
    assert!(result.is_ok());
    if let InteractionContent::Unknown { type_name, .. } = result.unwrap() {
        assert_eq!(type_name, "<missing type>");
    } else {
        panic!("Expected Unknown variant for missing type");
    }
}

// =============================================================================
// Tests for STRICT behavior (strict-unknown ENABLED)
// =============================================================================

/// Verify that unknown content types cause deserialization errors
/// when strict-unknown is enabled.
#[cfg(feature = "strict-unknown")]
#[test]
fn test_strict_unknown_fails_on_unknown_type() {
    let json = r#"{"type": "future_feature", "data": "test"}"#;
    let result: Result<InteractionContent, _> = serde_json::from_str(json);

    assert!(
        result.is_err(),
        "Unknown type should fail deserialization in strict mode"
    );
}

/// Verify that the error message in strict mode mentions the unknown type.
#[cfg(feature = "strict-unknown")]
#[test]
fn test_strict_unknown_error_message_contains_type() {
    let json = r#"{"type": "experimental_api_type", "data": "test"}"#;
    let result: Result<InteractionContent, _> = serde_json::from_str(json);

    let err = result.expect_err("Should fail in strict mode");
    let err_msg = err.to_string();

    assert!(
        err_msg.contains("experimental_api_type"),
        "Error message should contain the unknown type name. Got: {}",
        err_msg
    );
}

/// Verify that the error message mentions strict mode is enabled.
#[cfg(feature = "strict-unknown")]
#[test]
fn test_strict_unknown_error_message_mentions_strict_mode() {
    let json = r#"{"type": "new_type", "data": "test"}"#;
    let result: Result<InteractionContent, _> = serde_json::from_str(json);

    let err = result.expect_err("Should fail in strict mode");
    let err_msg = err.to_string();

    assert!(
        err_msg.contains("strict") || err_msg.contains("Strict"),
        "Error message should mention strict mode. Got: {}",
        err_msg
    );
}

/// Verify that known types still deserialize correctly in strict mode.
#[cfg(feature = "strict-unknown")]
#[test]
fn test_strict_known_types_still_work() {
    // Text
    let text: InteractionContent = serde_json::from_value(json!({"type": "text", "text": "hello"}))
        .expect("Text should deserialize in strict mode");
    assert!(matches!(text, InteractionContent::Text { .. }));

    // Thought
    let thought: InteractionContent =
        serde_json::from_value(json!({"type": "thought", "text": "thinking..."}))
            .expect("Thought should deserialize in strict mode");
    assert!(matches!(thought, InteractionContent::Thought { .. }));

    // Image
    let image: InteractionContent = serde_json::from_value(json!({
        "type": "image",
        "data": "base64data",
        "mime_type": "image/png"
    }))
    .expect("Image should deserialize in strict mode");
    assert!(matches!(image, InteractionContent::Image { .. }));

    // FunctionCall
    let func_call: InteractionContent = serde_json::from_value(json!({
        "type": "function_call",
        "name": "test_func",
        "arguments": {}
    }))
    .expect("FunctionCall should deserialize in strict mode");
    assert!(matches!(func_call, InteractionContent::FunctionCall { .. }));
}

/// Verify multiple unknown types all fail in strict mode (not just the first).
#[cfg(feature = "strict-unknown")]
#[test]
fn test_strict_fails_on_any_unknown_in_array() {
    // Array with unknown type in the middle
    let result: Result<Vec<InteractionContent>, _> = serde_json::from_value(json!([
        {"type": "text", "text": "Hello"},
        {"type": "unknown_middle", "data": "x"},
        {"type": "text", "text": "World"}
    ]));

    assert!(
        result.is_err(),
        "Array containing unknown type should fail in strict mode"
    );
}

// =============================================================================
// Tests that work in BOTH modes
// =============================================================================

/// Verify that all known content types deserialize correctly regardless of mode.
#[test]
fn test_all_known_types_deserialize() {
    // Text
    let _: InteractionContent =
        serde_json::from_value(json!({"type": "text", "text": "hello"})).unwrap();

    // Thought
    let _: InteractionContent =
        serde_json::from_value(json!({"type": "thought", "text": "thinking"})).unwrap();

    // ThoughtSignature
    let _: InteractionContent =
        serde_json::from_value(json!({"type": "thought_signature", "signature": "sig123"}))
            .unwrap();

    // Image
    let _: InteractionContent =
        serde_json::from_value(json!({"type": "image", "data": "x", "mime_type": "image/png"}))
            .unwrap();

    // Audio
    let _: InteractionContent =
        serde_json::from_value(json!({"type": "audio", "data": "x", "mime_type": "audio/mp3"}))
            .unwrap();

    // Video
    let _: InteractionContent =
        serde_json::from_value(json!({"type": "video", "data": "x", "mime_type": "video/mp4"}))
            .unwrap();

    // Document
    let _: InteractionContent = serde_json::from_value(
        json!({"type": "document", "data": "x", "mime_type": "application/pdf"}),
    )
    .unwrap();

    // FunctionCall
    let _: InteractionContent =
        serde_json::from_value(json!({"type": "function_call", "name": "fn", "arguments": {}}))
            .unwrap();

    // FunctionResult
    let _: InteractionContent = serde_json::from_value(
        json!({"type": "function_result", "name": "fn", "call_id": "1", "result": {}}),
    )
    .unwrap();

    // CodeExecutionCall
    let _: InteractionContent = serde_json::from_value(
        json!({"type": "code_execution_call", "id": "1", "language": "PYTHON", "code": "print(1)"}),
    )
    .unwrap();

    // CodeExecutionResult
    let _: InteractionContent = serde_json::from_value(json!({
        "type": "code_execution_result",
        "call_id": "1",
        "outcome": "OUTCOME_OK",
        "output": "1"
    }))
    .unwrap();

    // GoogleSearchCall
    let _: InteractionContent =
        serde_json::from_value(json!({"type": "google_search_call", "query": "test"})).unwrap();

    // GoogleSearchResult
    let _: InteractionContent =
        serde_json::from_value(json!({"type": "google_search_result", "results": []})).unwrap();

    // UrlContextCall
    let _: InteractionContent =
        serde_json::from_value(json!({"type": "url_context_call", "url": "https://example.com"}))
            .unwrap();

    // UrlContextResult
    let _: InteractionContent = serde_json::from_value(
        json!({"type": "url_context_result", "url": "https://example.com", "content": "page"}),
    )
    .unwrap();
}

/// Verify that known types serialize back correctly regardless of mode.
#[test]
fn test_known_types_roundtrip() {
    let text = InteractionContent::Text {
        text: Some("hello".to_string()),
    };
    let json = serde_json::to_value(&text).unwrap();
    assert_eq!(json["type"], "text");
    assert_eq!(json["text"], "hello");

    let thought = InteractionContent::Thought {
        text: Some("thinking".to_string()),
    };
    let json = serde_json::to_value(&thought).unwrap();
    assert_eq!(json["type"], "thought");
    assert_eq!(json["text"], "thinking");

    let image = InteractionContent::Image {
        data: Some("b64".to_string()),
        uri: None,
        mime_type: Some("image/png".to_string()),
    };
    let json = serde_json::to_value(&image).unwrap();
    assert_eq!(json["type"], "image");
    assert_eq!(json["data"], "b64");
    assert_eq!(json["mime_type"], "image/png");
}
