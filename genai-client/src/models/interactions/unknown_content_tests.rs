//! Tests for Evergreen unknown content handling.
//!
//! This module tests the Unknown variant handling in InteractionContent,
//! ensuring graceful degradation when the API returns unrecognized content types.
//!
//! Following the Evergreen spec philosophy:
//! - Unknown data should be preserved, not rejected
//! - Unknown variants serialize back with original data intact
//! - Known types continue to work correctly
//!
//! See the crate documentation on Evergreen-Inspired Soft-Typing for more details.

use super::*;

// =============================================================================
// Unknown Variant Deserialization Tests
// =============================================================================

#[test]
fn test_deserialize_unknown_interaction_content() {
    // Simulate a new API content type that this library doesn't know about
    // Note: code_execution_result is now a known type, so use a truly unknown type
    let unknown_json = r#"{"type": "future_api_feature", "data_field": "some_value", "count": 42}"#;

    let content: InteractionContent =
        serde_json::from_str(unknown_json).expect("Should deserialize as Unknown");

    match &content {
        InteractionContent::Unknown { type_name, data } => {
            assert_eq!(type_name, "future_api_feature");
            assert_eq!(data["data_field"], "some_value");
            assert_eq!(data["count"], 42);
        }
        _ => panic!("Expected Unknown variant, got {:?}", content),
    }

    assert!(content.is_unknown());
    assert_eq!(content.unknown_type(), Some("future_api_feature"));
    assert!(content.unknown_data().is_some());
}

#[test]
fn test_deserialize_unknown_streaming_content() {
    // Simulate a new streaming content type that this library doesn't know about
    let unknown_json = r#"{"type": "new_feature_delta", "data": "some_value"}"#;

    let content: InteractionContent =
        serde_json::from_str(unknown_json).expect("Should deserialize as Unknown");

    assert!(content.is_unknown());
    assert_eq!(content.unknown_type(), Some("new_feature_delta"));

    match &content {
        InteractionContent::Unknown { type_name, data } => {
            assert_eq!(type_name, "new_feature_delta");
            assert_eq!(data["data"], "some_value");
        }
        _ => panic!("Expected Unknown variant"),
    }
}

#[test]
fn test_known_types_still_work() {
    // Ensure adding Unknown doesn't break known types
    let text_json = r#"{"type": "text", "text": "Hello"}"#;
    let content: InteractionContent = serde_json::from_str(text_json).unwrap();
    assert!(matches!(content, InteractionContent::Text { .. }));
    assert!(!content.is_unknown());

    let thought_json = r#"{"type": "thought", "text": "Thinking..."}"#;
    let content: InteractionContent = serde_json::from_str(thought_json).unwrap();
    assert!(matches!(content, InteractionContent::Thought { .. }));
    assert!(!content.is_unknown());

    let signature_json = r#"{"type": "thought_signature", "signature": "sig123"}"#;
    let content: InteractionContent = serde_json::from_str(signature_json).unwrap();
    assert!(matches!(
        content,
        InteractionContent::ThoughtSignature { .. }
    ));
    assert!(!content.is_unknown());

    let function_json = r#"{"type": "function_call", "name": "test", "arguments": {}}"#;
    let content: InteractionContent = serde_json::from_str(function_json).unwrap();
    assert!(matches!(content, InteractionContent::FunctionCall { .. }));
    assert!(!content.is_unknown());
}

// =============================================================================
// Response Unknown Detection Tests
// =============================================================================

#[test]
fn test_interaction_response_has_unknown() {
    let response = InteractionResponse {
        id: "test_id".to_string(),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::Text {
                text: Some("Here's the result:".to_string()),
            },
            InteractionContent::Unknown {
                type_name: "code_execution_result".to_string(),
                data: serde_json::json!({
                    "type": "code_execution_result",
                    "outcome": "success",
                    "output": "42"
                }),
            },
        ],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert!(response.has_unknown());
    assert!(response.has_text());

    let unknowns = response.unknown_content();
    assert_eq!(unknowns.len(), 1);
    assert_eq!(unknowns[0].0, "code_execution_result");
    assert_eq!(unknowns[0].1["outcome"], "success");
}

#[test]
fn test_interaction_response_no_unknown() {
    let response = InteractionResponse {
        id: "test_id".to_string(),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![InteractionContent::Text {
            text: Some("Normal response".to_string()),
        }],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert!(!response.has_unknown());
    assert!(response.unknown_content().is_empty());
}

// =============================================================================
// ContentSummary Tests (includes unknown type tracking)
// =============================================================================

#[test]
fn test_content_summary() {
    let response = InteractionResponse {
        id: "test_id".to_string(),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::Text {
                text: Some("Text 1".to_string()),
            },
            InteractionContent::Text {
                text: Some("Text 2".to_string()),
            },
            InteractionContent::Thought {
                text: Some("Thinking".to_string()),
            },
            InteractionContent::FunctionCall {
                id: Some("call_1".to_string()),
                name: "test_fn".to_string(),
                args: serde_json::json!({}),
                thought_signature: None,
            },
            InteractionContent::Unknown {
                type_name: "type_a".to_string(),
                data: serde_json::json!({"type": "type_a"}),
            },
            InteractionContent::Unknown {
                type_name: "type_b".to_string(),
                data: serde_json::json!({"type": "type_b"}),
            },
            InteractionContent::Unknown {
                type_name: "type_a".to_string(), // Duplicate type
                data: serde_json::json!({"type": "type_a", "extra": true}),
            },
        ],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    let summary = response.content_summary();

    assert_eq!(summary.text_count, 2);
    assert_eq!(summary.thought_count, 1);
    assert_eq!(summary.function_call_count, 1);
    assert_eq!(summary.unknown_count, 3);
    // Unknown types should be deduplicated and sorted
    assert_eq!(summary.unknown_types.len(), 2);
    assert_eq!(summary.unknown_types, vec!["type_a", "type_b"]);
}

#[test]
fn test_content_summary_empty() {
    let response = InteractionResponse {
        id: "test_id".to_string(),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    let summary = response.content_summary();

    assert_eq!(summary.text_count, 0);
    assert_eq!(summary.unknown_count, 0);
    assert!(summary.unknown_types.is_empty());
}

#[test]
fn test_content_summary_display() {
    // Test Display for ContentSummary with various counts
    let summary = ContentSummary {
        text_count: 2,
        thought_count: 1,
        code_execution_call_count: 1,
        code_execution_result_count: 1,
        ..Default::default()
    };
    let display = format!("{}", summary);
    assert!(display.contains("2 text"));
    assert!(display.contains("1 thought"));
    assert!(display.contains("1 code_execution_call"));
    assert!(display.contains("1 code_execution_result"));
    // Should not contain zero-count items
    assert!(!display.contains("image"));
    assert!(!display.contains("audio"));
}

#[test]
fn test_content_summary_display_empty() {
    let summary = ContentSummary::default();
    assert_eq!(format!("{}", summary), "empty");
}

#[test]
fn test_content_summary_display_with_unknown() {
    let summary = ContentSummary {
        unknown_count: 2,
        unknown_types: vec!["new_type_a".to_string(), "new_type_b".to_string()],
        ..Default::default()
    };
    let display = format!("{}", summary);
    assert!(display.contains("2 unknown"));
    assert!(display.contains("new_type_a"));
    assert!(display.contains("new_type_b"));
}

// =============================================================================
// Unknown Serialization Roundtrip Tests
// =============================================================================

#[test]
fn test_serialize_unknown_content_roundtrip() {
    // Create an Unknown content (simulating what we'd receive from API)
    let unknown = InteractionContent::Unknown {
        type_name: "code_execution_result".to_string(),
        data: serde_json::json!({
            "outcome": "success",
            "output": "42"
        }),
    };

    // Serialize it
    let json = serde_json::to_string(&unknown).expect("Serialization should work");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Verify the structure: type field + flattened data
    assert_eq!(value["type"], "code_execution_result");
    assert_eq!(value["outcome"], "success");
    assert_eq!(value["output"], "42");
}

#[test]
fn test_deserialize_response_with_built_in_tool_outputs() {
    // Test deserializing a full response that contains built-in tool content
    // Note: code_execution_call and code_execution_result are now known types
    let response_json = r#"{
        "id": "interaction_789",
        "model": "gemini-3-flash-preview",
        "input": [{"type": "text", "text": "Execute some code"}],
        "outputs": [
            {"type": "text", "text": "Here's the result:"},
            {"type": "code_execution_call", "id": "call_abc", "arguments": {"code": "print(42)", "language": "python"}},
            {"type": "code_execution_result", "call_id": "call_abc", "is_error": false, "result": "42"}
        ],
        "status": "completed"
    }"#;

    let response: InteractionResponse =
        serde_json::from_str(response_json).expect("Should deserialize with built-in tool types");

    assert_eq!(response.id, "interaction_789");
    assert_eq!(response.outputs.len(), 3);
    assert!(response.has_text());
    assert!(response.has_code_execution_calls());
    assert!(response.has_code_execution_results());
    assert!(!response.has_unknown()); // These are now known types

    let summary = response.content_summary();
    assert_eq!(summary.text_count, 1);
    assert_eq!(summary.code_execution_call_count, 1);
    assert_eq!(summary.code_execution_result_count, 1);
    assert_eq!(summary.unknown_count, 0);
}

#[test]
fn test_deserialize_response_with_unknown_in_outputs() {
    // Test deserializing a full response that contains truly unknown content
    let response_json = r#"{
        "id": "interaction_789",
        "model": "gemini-3-flash-preview",
        "input": [{"type": "text", "text": "Do something"}],
        "outputs": [
            {"type": "text", "text": "Result:"},
            {"type": "future_tool_result", "data": "some_data"},
            {"type": "another_unknown_type", "value": 123}
        ],
        "status": "completed"
    }"#;

    let response: InteractionResponse =
        serde_json::from_str(response_json).expect("Should deserialize with unknown types");

    assert_eq!(response.id, "interaction_789");
    assert_eq!(response.outputs.len(), 3);
    assert!(response.has_text());
    assert!(response.has_unknown());

    let summary = response.content_summary();
    assert_eq!(summary.text_count, 1);
    assert_eq!(summary.unknown_count, 2);
    assert!(
        summary
            .unknown_types
            .contains(&"future_tool_result".to_string())
    );
    assert!(
        summary
            .unknown_types
            .contains(&"another_unknown_type".to_string())
    );
}

// =============================================================================
// Known Variant Serialization Edge Cases
// =============================================================================

#[test]
fn test_serialize_known_variant_with_none_fields() {
    // Test that known variants with None fields serialize correctly (omit None fields)
    let text = InteractionContent::Text { text: None };
    let json = serde_json::to_string(&text).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["type"], "text");
    assert!(value.get("text").is_none());

    let image = InteractionContent::Image {
        data: Some("base64data".to_string()),
        uri: None,
        mime_type: None,
    };
    let json = serde_json::to_string(&image).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["type"], "image");
    assert_eq!(value["data"], "base64data");
    assert!(value.get("uri").is_none());
    assert!(value.get("mime_type").is_none());

    let fc = InteractionContent::FunctionCall {
        id: None,
        name: "test_fn".to_string(),
        args: serde_json::json!({"arg": "value"}),
        thought_signature: None,
    };
    let json = serde_json::to_string(&fc).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["type"], "function_call");
    assert_eq!(value["name"], "test_fn");
    assert!(value.get("id").is_none());
    assert!(value.get("thoughtSignature").is_none());
}

// =============================================================================
// Unknown Variant Serialization Edge Cases
// =============================================================================

#[test]
fn test_serialize_unknown_with_non_object_data() {
    // Test that Unknown with non-object data (array, string, number) is preserved
    let unknown_array = InteractionContent::Unknown {
        type_name: "weird_type".to_string(),
        data: serde_json::json!([1, 2, 3]),
    };
    let json = serde_json::to_string(&unknown_array).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["type"], "weird_type");
    assert_eq!(value["data"], serde_json::json!([1, 2, 3]));

    let unknown_string = InteractionContent::Unknown {
        type_name: "string_type".to_string(),
        data: serde_json::json!("just a string"),
    };
    let json = serde_json::to_string(&unknown_string).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["type"], "string_type");
    assert_eq!(value["data"], "just a string");

    let unknown_null = InteractionContent::Unknown {
        type_name: "null_type".to_string(),
        data: serde_json::Value::Null,
    };
    let json = serde_json::to_string(&unknown_null).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["type"], "null_type");
    // Null data should be omitted
    assert!(value.get("data").is_none());
}

#[test]
fn test_serialize_unknown_with_duplicate_type_field() {
    // When data contains a "type" field, it should be ignored in serialization
    // (the type_name takes precedence)
    let unknown = InteractionContent::Unknown {
        type_name: "correct_type".to_string(),
        data: serde_json::json!({
            "type": "should_be_ignored",
            "field1": "value1",
            "field2": 42
        }),
    };

    let json = serde_json::to_string(&unknown).expect("Serialization should work");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    // The type should be from type_name, not from data
    assert_eq!(value["type"], "correct_type");
    // Other fields should be preserved
    assert_eq!(value["field1"], "value1");
    assert_eq!(value["field2"], 42);
    // There should be exactly one "type" field, not two
    let obj = value.as_object().unwrap();
    let type_count = obj.keys().filter(|k| *k == "type").count();
    assert_eq!(type_count, 1);
}

#[test]
fn test_serialize_unknown_with_empty_type_name() {
    // Empty type_name is allowed but not recommended
    let unknown = InteractionContent::Unknown {
        type_name: String::new(),
        data: serde_json::json!({"field": "value"}),
    };

    let json = serde_json::to_string(&unknown).expect("Serialization should work");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["type"], "");
    assert_eq!(value["field"], "value");
}

#[test]
fn test_serialize_unknown_with_special_characters() {
    // Type names with special characters should be preserved
    let unknown = InteractionContent::Unknown {
        type_name: "special/type:with.chars-and_underscores".to_string(),
        data: serde_json::json!({"key": "value"}),
    };

    let json = serde_json::to_string(&unknown).expect("Serialization should work");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["type"], "special/type:with.chars-and_underscores");
}

#[test]
fn test_unknown_manual_construction_roundtrip() {
    // Test that manually constructed Unknown variants can round-trip through JSON
    let original = InteractionContent::Unknown {
        type_name: "manual_test".to_string(),
        data: serde_json::json!({
            "nested": {"deeply": {"nested": "value"}},
            "array": [1, 2, 3],
            "number": 42,
            "boolean": true,
            "null_field": null
        }),
    };

    // Serialize
    let json = serde_json::to_string(&original).expect("Serialization should work");

    // Deserialize back
    let deserialized: InteractionContent =
        serde_json::from_str(&json).expect("Deserialization should work");

    // Verify it's still Unknown with same type
    assert!(deserialized.is_unknown());
    assert_eq!(deserialized.unknown_type(), Some("manual_test"));

    // Verify the data was preserved (check a few fields)
    if let InteractionContent::Unknown { data, .. } = deserialized {
        assert_eq!(data["nested"]["deeply"]["nested"], "value");
        assert_eq!(data["array"], serde_json::json!([1, 2, 3]));
        assert_eq!(data["number"], 42);
        assert_eq!(data["boolean"], true);
        // null_field should be present with null value (not stripped during serialization)
        assert_eq!(data.get("null_field"), Some(&serde_json::Value::Null));
    } else {
        panic!("Expected Unknown variant");
    }
}

// =============================================================================
// Edge Cases: Missing or Malformed Type Field
// =============================================================================

#[test]
fn test_deserialize_unknown_with_missing_type() {
    // Edge case: JSON object without a type field
    let malformed_json = r#"{"foo": "bar", "baz": 42}"#;
    let content: InteractionContent = serde_json::from_str(malformed_json).unwrap();
    match content {
        InteractionContent::Unknown { type_name, data } => {
            assert_eq!(type_name, "<missing type>");
            assert_eq!(data["foo"], "bar");
            assert_eq!(data["baz"], 42);
        }
        _ => panic!("Expected Unknown variant"),
    }
}

#[test]
fn test_deserialize_unknown_with_null_type() {
    // Edge case: JSON object with null type field
    let null_type_json = r#"{"type": null, "content": "test"}"#;
    let content: InteractionContent = serde_json::from_str(null_type_json).unwrap();
    match content {
        InteractionContent::Unknown { type_name, data } => {
            assert_eq!(type_name, "<missing type>");
            assert_eq!(data["content"], "test");
        }
        _ => panic!("Expected Unknown variant"),
    }
}
