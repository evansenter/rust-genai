//! Unit tests for InteractionContent types, serialization, and Unknown variant handling.

use super::*;

// --- Basic Content Serialization/Deserialization ---

#[test]
fn test_serialize_interaction_content() {
    let content = InteractionContent::Text {
        text: Some("Hello".to_string()),
        annotations: None,
    };

    let json = serde_json::to_string(&content).expect("Serialization failed");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["type"], "text");
    assert_eq!(value["text"], "Hello");
}

#[test]
fn test_deserialize_function_call_content() {
    let content_json =
        r#"{"type": "function_call", "name": "get_weather", "arguments": {"location": "Paris"}}"#;

    let content: InteractionContent =
        serde_json::from_str(content_json).expect("Deserialization failed");

    match content {
        InteractionContent::FunctionCall { name, args, .. } => {
            assert_eq!(name, "get_weather");
            assert_eq!(args["location"], "Paris");
        }
        _ => panic!("Expected FunctionCall variant"),
    }
}

#[test]
fn test_content_empty_text_returns_none() {
    let content = InteractionContent::Text {
        text: Some(String::new()),
        annotations: None,
    };
    assert_eq!(content.text(), None);

    let content_none = InteractionContent::Text {
        text: None,
        annotations: None,
    };
    assert_eq!(content_none.text(), None);
}

#[test]
fn test_content_thought_accessor() {
    // Non-empty thought returns Some
    let content = InteractionContent::Thought {
        text: Some("reasoning about the problem".to_string()),
    };
    assert_eq!(content.thought(), Some("reasoning about the problem"));

    // Empty thought returns None
    let empty = InteractionContent::Thought {
        text: Some(String::new()),
    };
    assert_eq!(empty.thought(), None);

    // None thought returns None
    let none = InteractionContent::Thought { text: None };
    assert_eq!(none.thought(), None);

    // Text variant returns None for thought()
    let text_content = InteractionContent::Text {
        text: Some("hello".to_string()),
        annotations: None,
    };
    assert_eq!(text_content.thought(), None);
}

// --- Unknown Variant Tests ---
// Note: Tests that rely on graceful unknown handling are disabled when strict-unknown is enabled,
// since strict mode causes deserialization errors for unknown types instead of capturing them.

#[cfg(not(feature = "strict-unknown"))]
#[test]
fn test_deserialize_unknown_interaction_content() {
    // Simulate a new API content type that this library doesn't know about
    // Note: code_execution_result is now a known type, so use a truly unknown type
    let unknown_json = r#"{"type": "future_api_feature", "data_field": "some_value", "count": 42}"#;

    let content: InteractionContent =
        serde_json::from_str(unknown_json).expect("Should deserialize as Unknown");

    match &content {
        InteractionContent::Unknown { content_type, data } => {
            assert_eq!(content_type, "future_api_feature");
            assert_eq!(data["data_field"], "some_value");
            assert_eq!(data["count"], 42);
        }
        _ => panic!("Expected Unknown variant, got {:?}", content),
    }

    assert!(content.is_unknown());
    assert_eq!(content.unknown_content_type(), Some("future_api_feature"));
    assert!(content.unknown_data().is_some());
}

#[cfg(not(feature = "strict-unknown"))]
#[test]
fn test_deserialize_unknown_streaming_content() {
    // Simulate a new streaming content type that this library doesn't know about
    let unknown_json = r#"{"type": "new_feature_delta", "data": "some_value"}"#;

    let content: InteractionContent =
        serde_json::from_str(unknown_json).expect("Should deserialize as Unknown");

    assert!(content.is_unknown());
    assert_eq!(content.unknown_content_type(), Some("new_feature_delta"));

    match &content {
        InteractionContent::Unknown { content_type, data } => {
            assert_eq!(content_type, "new_feature_delta");
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

#[test]
fn test_serialize_unknown_content_roundtrip() {
    // Create an Unknown content (simulating what we'd receive from API)
    let unknown = InteractionContent::Unknown {
        content_type: "code_execution_result".to_string(),
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
fn test_serialize_known_variant_with_none_fields() {
    // Test that known variants with None fields serialize correctly (omit None fields)
    let text = InteractionContent::Text {
        text: None,
        annotations: None,
    };
    let json = serde_json::to_string(&text).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["type"], "text");
    assert!(value.get("text").is_none());
    assert!(value.get("annotations").is_none());

    let image = InteractionContent::Image {
        data: Some("base64data".to_string()),
        uri: None,
        mime_type: None,
        resolution: None,
    };
    let json = serde_json::to_string(&image).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["type"], "image");
    assert_eq!(value["data"], "base64data");
    assert!(value.get("uri").is_none());
    assert!(value.get("mime_type").is_none());
    assert!(value.get("resolution").is_none());

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

#[test]
fn test_serialize_unknown_with_non_object_data() {
    // Test that Unknown with non-object data (array, string, number) is preserved
    let unknown_array = InteractionContent::Unknown {
        content_type: "weird_type".to_string(),
        data: serde_json::json!([1, 2, 3]),
    };
    let json = serde_json::to_string(&unknown_array).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["type"], "weird_type");
    assert_eq!(value["data"], serde_json::json!([1, 2, 3]));

    let unknown_string = InteractionContent::Unknown {
        content_type: "string_type".to_string(),
        data: serde_json::json!("just a string"),
    };
    let json = serde_json::to_string(&unknown_string).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["type"], "string_type");
    assert_eq!(value["data"], "just a string");

    let unknown_null = InteractionContent::Unknown {
        content_type: "null_type".to_string(),
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
    // (the content_type takes precedence)
    let unknown = InteractionContent::Unknown {
        content_type: "correct_type".to_string(),
        data: serde_json::json!({
            "type": "should_be_ignored",
            "field1": "value1",
            "field2": 42
        }),
    };

    let json = serde_json::to_string(&unknown).expect("Serialization should work");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    // The type should be from content_type, not from data
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
fn test_serialize_unknown_with_empty_content_type() {
    // Empty content_type is allowed but not recommended
    let unknown = InteractionContent::Unknown {
        content_type: String::new(),
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
        content_type: "special/type:with.chars-and_underscores".to_string(),
        data: serde_json::json!({"key": "value"}),
    };

    let json = serde_json::to_string(&unknown).expect("Serialization should work");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["type"], "special/type:with.chars-and_underscores");
}

#[cfg(not(feature = "strict-unknown"))]
#[test]
fn test_unknown_manual_construction_roundtrip() {
    // Test that manually constructed Unknown variants can round-trip through JSON
    let original = InteractionContent::Unknown {
        content_type: "manual_test".to_string(),
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
    assert_eq!(deserialized.unknown_content_type(), Some("manual_test"));

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

#[cfg(not(feature = "strict-unknown"))]
#[test]
fn test_deserialize_unknown_with_missing_type() {
    // Edge case: JSON object without a type field
    let malformed_json = r#"{"foo": "bar", "baz": 42}"#;
    let content: InteractionContent = serde_json::from_str(malformed_json).unwrap();
    match content {
        InteractionContent::Unknown { content_type, data } => {
            assert_eq!(content_type, "<missing type>");
            assert_eq!(data["foo"], "bar");
            assert_eq!(data["baz"], 42);
        }
        _ => panic!("Expected Unknown variant"),
    }
}

#[cfg(not(feature = "strict-unknown"))]
#[test]
fn test_deserialize_unknown_with_null_type() {
    // Edge case: JSON object with null type field
    let null_type_json = r#"{"type": null, "content": "test"}"#;
    let content: InteractionContent = serde_json::from_str(null_type_json).unwrap();
    match content {
        InteractionContent::Unknown { content_type, data } => {
            assert_eq!(content_type, "<missing type>");
            assert_eq!(data["content"], "test");
        }
        _ => panic!("Expected Unknown variant"),
    }
}

// --- Built-in Tool Content Tests ---

#[test]
fn test_deserialize_code_execution_call() {
    // Test deserialization from the API format (arguments object)
    let json = r#"{"type": "code_execution_call", "id": "call_123", "arguments": {"code": "print(42)", "language": "PYTHON"}}"#;
    let content: InteractionContent = serde_json::from_str(json).expect("Should deserialize");

    match &content {
        InteractionContent::CodeExecutionCall { id, language, code } => {
            assert_eq!(id, "call_123");
            assert_eq!(*language, CodeExecutionLanguage::Python);
            assert_eq!(code, "print(42)");
        }
        _ => panic!("Expected CodeExecutionCall variant, got {:?}", content),
    }

    assert!(content.is_code_execution_call());
    assert!(!content.is_unknown());
}

#[test]
fn test_deserialize_code_execution_call_direct_fields() {
    // Test deserialization from direct fields (new format)
    let json = r#"{"type": "code_execution_call", "id": "call_123", "language": "PYTHON", "code": "print(42)"}"#;
    let content: InteractionContent = serde_json::from_str(json).expect("Should deserialize");

    match &content {
        InteractionContent::CodeExecutionCall { id, language, code } => {
            assert_eq!(id, "call_123");
            assert_eq!(*language, CodeExecutionLanguage::Python);
            assert_eq!(code, "print(42)");
        }
        _ => panic!("Expected CodeExecutionCall variant, got {:?}", content),
    }
}

#[test]
fn test_deserialize_code_execution_call_malformed_becomes_unknown() {
    // Issue #186: Malformed CodeExecutionCall (missing both direct fields and arguments)
    // should become Unknown variant per Evergreen philosophy, not silently fall back to empty code.
    let json =
        r#"{"type": "code_execution_call", "id": "call_malformed", "extra_field": "unexpected"}"#;
    let content: InteractionContent = serde_json::from_str(json).expect("Should deserialize");

    // Should be Unknown, not a CodeExecutionCall with empty code
    match &content {
        InteractionContent::Unknown { content_type, data } => {
            assert_eq!(content_type, "code_execution_call");
            // Verify the original data is preserved for debugging
            assert_eq!(data["id"], "call_malformed");
            assert_eq!(data["extra_field"], "unexpected");
            assert_eq!(data["type"], "code_execution_call");
        }
        InteractionContent::CodeExecutionCall { code, .. } => {
            panic!(
                "Should NOT be CodeExecutionCall with empty code (got code={:?}). \
                 Malformed responses should become Unknown variant.",
                code
            );
        }
        _ => panic!("Expected Unknown variant, got {:?}", content),
    }

    assert!(content.is_unknown());
    assert!(!content.is_code_execution_call());
    assert_eq!(content.unknown_content_type(), Some("code_execution_call"));
}

#[test]
fn test_deserialize_code_execution_call_malformed_roundtrip() {
    // Verify malformed CodeExecutionCall can roundtrip through Unknown variant
    let json =
        r#"{"type": "code_execution_call", "id": "call_malformed", "custom": {"nested": true}}"#;
    let content: InteractionContent = serde_json::from_str(json).expect("Should deserialize");

    assert!(content.is_unknown());

    // Serialize back
    let reserialized = serde_json::to_string(&content).expect("Should serialize");
    let value: serde_json::Value =
        serde_json::from_str(&reserialized).expect("Should parse as JSON");

    // Verify key fields are preserved
    assert_eq!(value["type"], "code_execution_call");
    assert_eq!(value["id"], "call_malformed");
    assert_eq!(value["custom"]["nested"], true);
}

#[test]
fn test_deserialize_code_execution_call_arguments_missing_code_becomes_unknown() {
    // Arguments path: when arguments object exists but code is missing, treat as Unknown
    // This is the same Evergreen pattern as the direct fields path
    let json = r#"{"type": "code_execution_call", "id": "call_args_no_code", "arguments": {"language": "PYTHON"}}"#;
    let content: InteractionContent = serde_json::from_str(json).expect("Should deserialize");

    // Should be Unknown, not CodeExecutionCall with empty code
    match &content {
        InteractionContent::Unknown { content_type, data } => {
            assert_eq!(content_type, "code_execution_call");
            assert_eq!(data["id"], "call_args_no_code");
            assert_eq!(data["arguments"]["language"], "PYTHON");
        }
        InteractionContent::CodeExecutionCall { code, .. } => {
            panic!(
                "Should NOT be CodeExecutionCall with empty code (got code={:?}). \
                 Arguments path should also treat missing code as Unknown.",
                code
            );
        }
        _ => panic!("Expected Unknown variant, got {:?}", content),
    }

    assert!(content.is_unknown());
}

#[test]
fn test_deserialize_code_execution_call_arguments_valid() {
    // Arguments path: when arguments object has code, should work normally
    let json = r#"{"type": "code_execution_call", "id": "call_valid", "arguments": {"language": "PYTHON", "code": "print('hi')"}}"#;
    let content: InteractionContent = serde_json::from_str(json).expect("Should deserialize");

    match &content {
        InteractionContent::CodeExecutionCall { id, language, code } => {
            assert_eq!(id, "call_valid");
            assert_eq!(*language, CodeExecutionLanguage::Python);
            assert_eq!(code, "print('hi')");
        }
        _ => panic!("Expected CodeExecutionCall variant, got {:?}", content),
    }

    assert!(content.is_code_execution_call());
    assert!(!content.is_unknown());
}

#[test]
fn test_deserialize_code_execution_result() {
    // Test deserialization from old API format (is_error + result)
    let json = r#"{"type": "code_execution_result", "call_id": "call_123", "is_error": false, "result": "42\n"}"#;
    let content: InteractionContent = serde_json::from_str(json).expect("Should deserialize");

    match &content {
        InteractionContent::CodeExecutionResult {
            call_id,
            outcome,
            output,
        } => {
            assert_eq!(call_id, "call_123");
            assert!(outcome.is_success());
            assert_eq!(output, "42\n");
        }
        _ => panic!("Expected CodeExecutionResult variant, got {:?}", content),
    }

    assert!(content.is_code_execution_result());
    assert!(!content.is_unknown());
}

#[test]
fn test_deserialize_code_execution_result_with_outcome() {
    // Test deserialization from new format (outcome + output)
    let json = r#"{"type": "code_execution_result", "call_id": "call_123", "outcome": "OUTCOME_OK", "output": "42\n"}"#;
    let content: InteractionContent = serde_json::from_str(json).expect("Should deserialize");

    match &content {
        InteractionContent::CodeExecutionResult {
            call_id,
            outcome,
            output,
        } => {
            assert_eq!(call_id, "call_123");
            assert_eq!(*outcome, CodeExecutionOutcome::Ok);
            assert_eq!(output, "42\n");
        }
        _ => panic!("Expected CodeExecutionResult variant, got {:?}", content),
    }
}

#[test]
fn test_deserialize_code_execution_result_error() {
    let json = r#"{"type": "code_execution_result", "call_id": "call_456", "is_error": true, "result": "NameError: x not defined"}"#;
    let content: InteractionContent = serde_json::from_str(json).expect("Should deserialize");

    match &content {
        InteractionContent::CodeExecutionResult {
            call_id,
            outcome,
            output,
        } => {
            assert_eq!(call_id, "call_456");
            assert!(outcome.is_error());
            assert!(output.contains("NameError"));
        }
        _ => panic!("Expected CodeExecutionResult variant, got {:?}", content),
    }
}

#[test]
fn test_deserialize_code_execution_result_deadline_exceeded() {
    // Test deserialization of OUTCOME_DEADLINE_EXCEEDED (timeout scenario)
    let json = r#"{"type": "code_execution_result", "call_id": "call_789", "outcome": "OUTCOME_DEADLINE_EXCEEDED", "output": "Execution timed out after 30 seconds"}"#;
    let content: InteractionContent = serde_json::from_str(json).expect("Should deserialize");

    match &content {
        InteractionContent::CodeExecutionResult {
            call_id,
            outcome,
            output,
        } => {
            assert_eq!(call_id, "call_789");
            assert_eq!(*outcome, CodeExecutionOutcome::DeadlineExceeded);
            assert!(outcome.is_error());
            assert!(!outcome.is_success());
            assert!(output.contains("timed out"));
        }
        _ => panic!("Expected CodeExecutionResult variant, got {:?}", content),
    }
}

#[test]
fn test_code_execution_outcome_enum() {
    assert!(CodeExecutionOutcome::Ok.is_success());
    assert!(!CodeExecutionOutcome::Ok.is_error());

    assert!(!CodeExecutionOutcome::Failed.is_success());
    assert!(CodeExecutionOutcome::Failed.is_error());

    assert!(!CodeExecutionOutcome::DeadlineExceeded.is_success());
    assert!(CodeExecutionOutcome::DeadlineExceeded.is_error());

    assert!(!CodeExecutionOutcome::Unspecified.is_success());
    assert!(CodeExecutionOutcome::Unspecified.is_error());
}

#[test]
fn test_deserialize_google_search_call() {
    // Test the actual API format: arguments.queries is an array
    let json = r#"{"type": "google_search_call", "id": "call123", "arguments": {"queries": ["Rust programming", "latest version"]}}"#;
    let content: InteractionContent = serde_json::from_str(json).expect("Should deserialize");

    match &content {
        InteractionContent::GoogleSearchCall { id, queries } => {
            assert_eq!(id, "call123");
            assert_eq!(queries.len(), 2);
            assert_eq!(queries[0], "Rust programming");
            assert_eq!(queries[1], "latest version");
        }
        _ => panic!("Expected GoogleSearchCall variant, got {:?}", content),
    }

    assert!(content.is_google_search_call());
    assert!(!content.is_unknown());
}

#[test]
fn test_deserialize_google_search_result() {
    // Test the actual API format: result is an array of objects with title/url
    let json = r#"{"type": "google_search_result", "call_id": "call123", "result": [{"title": "Rust", "url": "https://rust-lang.org", "renderedContent": "Some content"}]}"#;
    let content: InteractionContent = serde_json::from_str(json).expect("Should deserialize");

    match &content {
        InteractionContent::GoogleSearchResult { call_id, result } => {
            assert_eq!(call_id, "call123");
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].title, "Rust");
            assert_eq!(result[0].url, "https://rust-lang.org");
            assert_eq!(result[0].rendered_content.as_deref(), Some("Some content"));
        }
        _ => panic!("Expected GoogleSearchResult variant, got {:?}", content),
    }

    assert!(content.is_google_search_result());
    assert!(!content.is_unknown());
}

#[test]
fn test_deserialize_url_context_call() {
    let json = r#"{"type": "url_context_call", "url": "https://example.com"}"#;
    let content: InteractionContent = serde_json::from_str(json).expect("Should deserialize");

    match &content {
        InteractionContent::UrlContextCall { url } => {
            assert_eq!(url, "https://example.com");
        }
        _ => panic!("Expected UrlContextCall variant, got {:?}", content),
    }

    assert!(content.is_url_context_call());
    assert!(!content.is_unknown());
}

#[test]
fn test_deserialize_url_context_result() {
    let json = r#"{"type": "url_context_result", "url": "https://example.com", "content": "<html>...</html>"}"#;
    let content: InteractionContent = serde_json::from_str(json).expect("Should deserialize");

    match &content {
        InteractionContent::UrlContextResult { url, content } => {
            assert_eq!(url, "https://example.com");
            assert_eq!(content.as_deref(), Some("<html>...</html>"));
        }
        _ => panic!("Expected UrlContextResult variant, got {:?}", content),
    }

    assert!(content.is_url_context_result());
    assert!(!content.is_unknown());
}

#[test]
fn test_url_context_result_with_none_content() {
    // Test that UrlContextResult with content: None serializes without the content field
    // (the API omits this field when content is not available, e.g., network errors)
    let content = InteractionContent::UrlContextResult {
        url: "https://example.com/blocked".to_string(),
        content: None,
    };

    // Serialize and verify content field is absent
    let json = serde_json::to_string(&content).expect("Serialization should work");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["type"], "url_context_result");
    assert_eq!(value["url"], "https://example.com/blocked");
    // content field should be absent (not null)
    assert!(value.get("content").is_none());

    // Deserialize without content field and verify it works
    let json_without_content =
        r#"{"type": "url_context_result", "url": "https://example.com/timeout"}"#;
    let deserialized: InteractionContent =
        serde_json::from_str(json_without_content).expect("Should deserialize");

    match &deserialized {
        InteractionContent::UrlContextResult { url, content } => {
            assert_eq!(url, "https://example.com/timeout");
            assert_eq!(*content, None);
        }
        _ => panic!("Expected UrlContextResult variant"),
    }
}

#[test]
fn test_serialize_code_execution_call() {
    let content = InteractionContent::CodeExecutionCall {
        id: "call_123".to_string(),
        language: CodeExecutionLanguage::Python,
        code: "print(42)".to_string(),
    };

    let json = serde_json::to_string(&content).expect("Serialization should work");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["type"], "code_execution_call");
    assert_eq!(value["id"], "call_123");
    assert_eq!(value["language"], "PYTHON");
    assert_eq!(value["code"], "print(42)");
}

#[test]
fn test_serialize_code_execution_result() {
    let content = InteractionContent::CodeExecutionResult {
        call_id: "call_123".to_string(),
        outcome: CodeExecutionOutcome::Ok,
        output: "42".to_string(),
    };

    let json = serde_json::to_string(&content).expect("Serialization should work");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["type"], "code_execution_result");
    assert_eq!(value["call_id"], "call_123");
    assert_eq!(value["outcome"], "OUTCOME_OK");
    assert_eq!(value["output"], "42");
}

#[test]
fn test_serialize_code_execution_result_error() {
    let content = InteractionContent::CodeExecutionResult {
        call_id: "call_456".to_string(),
        outcome: CodeExecutionOutcome::Failed,
        output: "NameError: x not defined".to_string(),
    };

    let json = serde_json::to_string(&content).expect("Serialization should work");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["type"], "code_execution_result");
    assert_eq!(value["call_id"], "call_456");
    assert_eq!(value["outcome"], "OUTCOME_FAILED");
    assert!(value["output"].as_str().unwrap().contains("NameError"));
}

#[test]
fn test_roundtrip_built_in_tool_content() {
    // CodeExecutionCall roundtrip
    let original = InteractionContent::CodeExecutionCall {
        id: "call_123".to_string(),
        language: CodeExecutionLanguage::Python,
        code: "print('hello')".to_string(),
    };
    let json = serde_json::to_string(&original).unwrap();
    let restored: InteractionContent = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        restored,
        InteractionContent::CodeExecutionCall { .. }
    ));

    // CodeExecutionResult roundtrip
    let original = InteractionContent::CodeExecutionResult {
        call_id: "call_123".to_string(),
        outcome: CodeExecutionOutcome::Ok,
        output: "hello\n".to_string(),
    };
    let json = serde_json::to_string(&original).unwrap();
    let restored: InteractionContent = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        restored,
        InteractionContent::CodeExecutionResult { .. }
    ));

    // GoogleSearchCall roundtrip
    let original = InteractionContent::GoogleSearchCall {
        id: "call123".to_string(),
        queries: vec!["test query".to_string()],
    };
    let json = serde_json::to_string(&original).unwrap();
    let restored: InteractionContent = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        restored,
        InteractionContent::GoogleSearchCall { .. }
    ));

    // GoogleSearchResult roundtrip
    let original = InteractionContent::GoogleSearchResult {
        call_id: "call123".to_string(),
        result: vec![],
    };
    let json = serde_json::to_string(&original).unwrap();
    let restored: InteractionContent = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        restored,
        InteractionContent::GoogleSearchResult { .. }
    ));

    // UrlContextCall roundtrip
    let original = InteractionContent::UrlContextCall {
        url: "https://example.com".to_string(),
    };
    let json = serde_json::to_string(&original).unwrap();
    let restored: InteractionContent = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        restored,
        InteractionContent::UrlContextCall { .. }
    ));

    // UrlContextResult roundtrip
    let original = InteractionContent::UrlContextResult {
        url: "https://example.com".to_string(),
        content: Some("content".to_string()),
    };
    let json = serde_json::to_string(&original).unwrap();
    let restored: InteractionContent = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        restored,
        InteractionContent::UrlContextResult { .. }
    ));
}

#[test]
fn test_edge_cases_empty_values() {
    // Empty code in CodeExecutionCall
    let content = InteractionContent::CodeExecutionCall {
        id: "call_empty".to_string(),
        language: CodeExecutionLanguage::Python,
        code: "".to_string(),
    };
    let json = serde_json::to_string(&content).unwrap();
    let restored: InteractionContent = serde_json::from_str(&json).unwrap();
    match restored {
        InteractionContent::CodeExecutionCall { id, language, code } => {
            assert_eq!(id, "call_empty");
            assert_eq!(language, CodeExecutionLanguage::Python);
            assert!(code.is_empty());
        }
        _ => panic!("Expected CodeExecutionCall"),
    }

    // Empty results in GoogleSearchResult
    let content = InteractionContent::GoogleSearchResult {
        call_id: "call_empty".to_string(),
        result: vec![],
    };
    let json = serde_json::to_string(&content).unwrap();
    let restored: InteractionContent = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        restored,
        InteractionContent::GoogleSearchResult { .. }
    ));

    // UrlContextResult with None content (failed fetch)
    let content = InteractionContent::UrlContextResult {
        url: "https://blocked.example.com".to_string(),
        content: None,
    };
    let json = serde_json::to_string(&content).unwrap();
    let restored: InteractionContent = serde_json::from_str(&json).unwrap();
    match restored {
        InteractionContent::UrlContextResult { url, content } => {
            assert_eq!(url, "https://blocked.example.com");
            assert!(content.is_none());
        }
        _ => panic!("Expected UrlContextResult"),
    }

    // Empty output string in CodeExecutionResult
    let content = InteractionContent::CodeExecutionResult {
        call_id: "call_no_output".to_string(),
        outcome: CodeExecutionOutcome::Ok,
        output: "".to_string(),
    };
    let json = serde_json::to_string(&content).unwrap();
    let restored: InteractionContent = serde_json::from_str(&json).unwrap();
    match restored {
        InteractionContent::CodeExecutionResult {
            call_id, output, ..
        } => {
            assert_eq!(call_id, "call_no_output");
            assert!(output.is_empty());
        }
        _ => panic!("Expected CodeExecutionResult"),
    }
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

    assert_eq!(response.id.as_deref(), Some("interaction_789"));
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

#[cfg(not(feature = "strict-unknown"))]
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

    assert_eq!(response.id.as_deref(), Some("interaction_789"));
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

// --- Annotation Tests ---

#[test]
fn test_annotation_struct_basics() {
    let annotation = Annotation {
        start_index: 0,
        end_index: 10,
        source: Some("https://example.com".to_string()),
    };

    assert_eq!(annotation.byte_len(), 10);
    assert!(annotation.has_source());

    let no_source = Annotation {
        start_index: 5,
        end_index: 15,
        source: None,
    };
    assert_eq!(no_source.byte_len(), 10);
    assert!(!no_source.has_source());
}

#[test]
fn test_annotation_extract_span() {
    let text = "Hello, world!";
    let annotation = Annotation {
        start_index: 0,
        end_index: 5,
        source: None,
    };
    assert_eq!(annotation.extract_span(text), Some("Hello"));

    let annotation_mid = Annotation {
        start_index: 7,
        end_index: 12,
        source: None,
    };
    assert_eq!(annotation_mid.extract_span(text), Some("world"));

    // Out of bounds
    let out_of_bounds = Annotation {
        start_index: 100,
        end_index: 200,
        source: None,
    };
    assert_eq!(out_of_bounds.extract_span(text), None);
}

#[test]
fn test_annotation_extract_span_utf8() {
    // Test with UTF-8 text - annotations use byte indices
    let text = "Héllo, 世界!"; // "Héllo" = 6 bytes (H=1, é=2, l=1, l=1, o=1), ", " = 2 bytes, "世界" = 6 bytes
    let annotation = Annotation {
        start_index: 0,
        end_index: 6, // "Héllo" in bytes
        source: None,
    };
    assert_eq!(annotation.extract_span(text), Some("Héllo"));

    // Extract Chinese characters
    let world_annotation = Annotation {
        start_index: 8, // After "Héllo, "
        end_index: 14,  // "世界" is 6 bytes
        source: None,
    };
    assert_eq!(world_annotation.extract_span(text), Some("世界"));
}

#[test]
fn test_serialize_text_with_annotations() {
    let annotations = vec![
        Annotation {
            start_index: 0,
            end_index: 5,
            source: Some("https://example.com".to_string()),
        },
        Annotation {
            start_index: 10,
            end_index: 20,
            source: None,
        },
    ];

    let content = InteractionContent::Text {
        text: Some("Hello, world! This is grounded text.".to_string()),
        annotations: Some(annotations),
    };

    let json = serde_json::to_string(&content).expect("Serialization failed");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["type"], "text");
    assert_eq!(value["text"], "Hello, world! This is grounded text.");
    assert!(value["annotations"].is_array());
    assert_eq!(value["annotations"].as_array().unwrap().len(), 2);
    assert_eq!(value["annotations"][0]["start_index"], 0);
    assert_eq!(value["annotations"][0]["end_index"], 5);
    assert_eq!(value["annotations"][0]["source"], "https://example.com");
    assert_eq!(value["annotations"][1]["start_index"], 10);
    assert_eq!(value["annotations"][1]["end_index"], 20);
    assert!(value["annotations"][1].get("source").is_none());
}

#[test]
fn test_serialize_text_with_empty_annotations_omitted() {
    // Empty annotations array should not be serialized
    let content = InteractionContent::Text {
        text: Some("Plain text".to_string()),
        annotations: Some(vec![]),
    };

    let json = serde_json::to_string(&content).expect("Serialization failed");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["type"], "text");
    assert_eq!(value["text"], "Plain text");
    // Empty annotations should not be serialized
    assert!(value.get("annotations").is_none());
}

#[test]
fn test_deserialize_text_with_annotations() {
    let json = r#"{
        "type": "text",
        "text": "This is grounded text.",
        "annotations": [
            {"start_index": 0, "end_index": 4, "source": "https://example.com"},
            {"start_index": 8, "end_index": 16}
        ]
    }"#;

    let content: InteractionContent = serde_json::from_str(json).expect("Deserialization failed");

    match content {
        InteractionContent::Text { text, annotations } => {
            assert_eq!(text, Some("This is grounded text.".to_string()));
            let annots = annotations.expect("Should have annotations");
            assert_eq!(annots.len(), 2);
            assert_eq!(annots[0].start_index, 0);
            assert_eq!(annots[0].end_index, 4);
            assert_eq!(annots[0].source, Some("https://example.com".to_string()));
            assert_eq!(annots[1].start_index, 8);
            assert_eq!(annots[1].end_index, 16);
            assert_eq!(annots[1].source, None);
        }
        _ => panic!("Expected Text variant"),
    }
}

#[test]
fn test_deserialize_text_without_annotations() {
    let json = r#"{"type": "text", "text": "Plain text"}"#;

    let content: InteractionContent = serde_json::from_str(json).expect("Deserialization failed");

    match content {
        InteractionContent::Text { text, annotations } => {
            assert_eq!(text, Some("Plain text".to_string()));
            assert!(annotations.is_none());
        }
        _ => panic!("Expected Text variant"),
    }
}

#[test]
fn test_text_with_annotations_roundtrip() {
    let annotations = vec![Annotation {
        start_index: 5,
        end_index: 15,
        source: Some("https://source.example.com".to_string()),
    }];

    let original = InteractionContent::Text {
        text: Some("Some grounded content here.".to_string()),
        annotations: Some(annotations),
    };

    let json = serde_json::to_string(&original).expect("Serialization failed");
    let deserialized: InteractionContent =
        serde_json::from_str(&json).expect("Deserialization failed");

    // Compare by re-serializing and checking the JSON
    let roundtrip_json = serde_json::to_string(&deserialized).expect("Serialization failed");
    assert_eq!(json, roundtrip_json);

    // Also verify content matches
    match deserialized {
        InteractionContent::Text { text, annotations } => {
            assert_eq!(text, Some("Some grounded content here.".to_string()));
            let annots = annotations.expect("Should have annotations");
            assert_eq!(annots.len(), 1);
            assert_eq!(annots[0].start_index, 5);
            assert_eq!(annots[0].end_index, 15);
            assert_eq!(
                annots[0].source,
                Some("https://source.example.com".to_string())
            );
        }
        _ => panic!("Expected Text variant"),
    }
}

#[test]
fn test_annotations_helper_method() {
    let content_with_annotations = InteractionContent::Text {
        text: Some("Hello".to_string()),
        annotations: Some(vec![Annotation {
            start_index: 0,
            end_index: 5,
            source: None,
        }]),
    };

    assert!(content_with_annotations.annotations().is_some());
    assert_eq!(content_with_annotations.annotations().unwrap().len(), 1);

    let content_without_annotations = InteractionContent::Text {
        text: Some("Hello".to_string()),
        annotations: None,
    };

    assert!(content_without_annotations.annotations().is_none());

    // Non-text content returns None
    let thought = InteractionContent::Thought {
        text: Some("Thinking...".to_string()),
    };
    assert!(thought.annotations().is_none());
}

#[test]
fn test_annotation_extract_span_inverted_indices() {
    // Edge case: start_index > end_index (malformed annotation)
    // The implementation should gracefully return None
    let inverted = Annotation {
        start_index: 10,
        end_index: 5, // start > end - invalid range
        source: None,
    };

    let text = "Hello, world!";
    assert_eq!(
        inverted.extract_span(text),
        None,
        "Inverted indices should return None"
    );

    // byte_len should use saturating_sub to avoid underflow
    assert_eq!(
        inverted.byte_len(),
        0,
        "byte_len of inverted indices should be 0 (saturating_sub)"
    );
}

#[test]
fn test_annotation_extract_span_zero_length() {
    // Edge case: start_index == end_index (zero-length span)
    let zero_len = Annotation {
        start_index: 5,
        end_index: 5,
        source: Some("https://example.com".to_string()),
    };

    let text = "Hello, world!";
    assert_eq!(
        zero_len.extract_span(text),
        Some(""),
        "Zero-length span should return empty string"
    );
    assert_eq!(zero_len.byte_len(), 0);
}

#[test]
fn test_annotation_extract_span_mid_utf8_boundary() {
    // Edge case: indices that land in the middle of a multi-byte character
    // "世" is a 3-byte UTF-8 character (E4 B8 96)
    let text = "Hello, 世界!"; // "世" starts at byte 7, "界" starts at byte 10

    // Try to slice starting in the middle of "世" (byte 8)
    let mid_start = Annotation {
        start_index: 8, // Middle of "世"
        end_index: 13,
        source: None,
    };
    assert_eq!(
        mid_start.extract_span(text),
        None,
        "Slicing from middle of UTF-8 character should return None"
    );

    // Try to slice ending in the middle of "界" (byte 11)
    let mid_end = Annotation {
        start_index: 7, // Start of "世"
        end_index: 11,  // Middle of "界"
        source: None,
    };
    assert_eq!(
        mid_end.extract_span(text),
        None,
        "Slicing to middle of UTF-8 character should return None"
    );

    // Valid slice of "世界" (bytes 7-13)
    let valid_cjk = Annotation {
        start_index: 7,
        end_index: 13,
        source: None,
    };
    assert_eq!(
        valid_cjk.extract_span(text),
        Some("世界"),
        "Valid CJK character slice should work"
    );
}

// --- Resolution Tests ---

#[test]
fn test_resolution_enum_serialization() {
    // Test that Resolution serializes to snake_case
    assert_eq!(serde_json::to_string(&Resolution::Low).unwrap(), "\"low\"");
    assert_eq!(
        serde_json::to_string(&Resolution::Medium).unwrap(),
        "\"medium\""
    );
    assert_eq!(
        serde_json::to_string(&Resolution::High).unwrap(),
        "\"high\""
    );
    assert_eq!(
        serde_json::to_string(&Resolution::UltraHigh).unwrap(),
        "\"ultra_high\""
    );
}

#[test]
fn test_resolution_enum_deserialization() {
    assert_eq!(
        serde_json::from_str::<Resolution>("\"low\"").unwrap(),
        Resolution::Low
    );
    assert_eq!(
        serde_json::from_str::<Resolution>("\"medium\"").unwrap(),
        Resolution::Medium
    );
    assert_eq!(
        serde_json::from_str::<Resolution>("\"high\"").unwrap(),
        Resolution::High
    );
    assert_eq!(
        serde_json::from_str::<Resolution>("\"ultra_high\"").unwrap(),
        Resolution::UltraHigh
    );
}

#[test]
fn test_resolution_default_is_medium() {
    assert_eq!(Resolution::default(), Resolution::Medium);
}

#[test]
fn test_image_with_resolution_serialization() {
    let image = InteractionContent::Image {
        data: Some("base64data".to_string()),
        uri: None,
        mime_type: Some("image/png".to_string()),
        resolution: Some(Resolution::High),
    };

    let json = serde_json::to_string(&image).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["type"], "image");
    assert_eq!(value["data"], "base64data");
    assert_eq!(value["mime_type"], "image/png");
    assert_eq!(value["resolution"], "high");
}

#[test]
fn test_image_with_ultra_high_resolution_serialization() {
    let image = InteractionContent::Image {
        data: None,
        uri: Some("https://example.com/image.png".to_string()),
        mime_type: Some("image/png".to_string()),
        resolution: Some(Resolution::UltraHigh),
    };

    let json = serde_json::to_string(&image).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["type"], "image");
    assert_eq!(value["uri"], "https://example.com/image.png");
    assert_eq!(value["resolution"], "ultra_high");
}

#[test]
fn test_video_with_resolution_serialization() {
    let video = InteractionContent::Video {
        data: Some("videobytes".to_string()),
        uri: None,
        mime_type: Some("video/mp4".to_string()),
        resolution: Some(Resolution::Low),
    };

    let json = serde_json::to_string(&video).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["type"], "video");
    assert_eq!(value["data"], "videobytes");
    assert_eq!(value["mime_type"], "video/mp4");
    assert_eq!(value["resolution"], "low");
}

#[test]
fn test_image_with_resolution_deserialization() {
    let json = r#"{"type": "image", "data": "base64data", "mime_type": "image/png", "resolution": "high"}"#;
    let content: InteractionContent = serde_json::from_str(json).unwrap();

    match content {
        InteractionContent::Image {
            data,
            uri,
            mime_type,
            resolution,
        } => {
            assert_eq!(data, Some("base64data".to_string()));
            assert_eq!(uri, None);
            assert_eq!(mime_type, Some("image/png".to_string()));
            assert_eq!(resolution, Some(Resolution::High));
        }
        _ => panic!("Expected Image variant"),
    }
}

#[test]
fn test_video_with_resolution_deserialization() {
    let json = r#"{"type": "video", "uri": "https://example.com/video.mp4", "mime_type": "video/mp4", "resolution": "ultra_high"}"#;
    let content: InteractionContent = serde_json::from_str(json).unwrap();

    match content {
        InteractionContent::Video {
            data,
            uri,
            mime_type,
            resolution,
        } => {
            assert_eq!(data, None);
            assert_eq!(uri, Some("https://example.com/video.mp4".to_string()));
            assert_eq!(mime_type, Some("video/mp4".to_string()));
            assert_eq!(resolution, Some(Resolution::UltraHigh));
        }
        _ => panic!("Expected Video variant"),
    }
}

#[test]
fn test_image_without_resolution_deserialization() {
    // Verify backward compatibility: images without resolution field should work
    let json = r#"{"type": "image", "data": "base64data", "mime_type": "image/png"}"#;
    let content: InteractionContent = serde_json::from_str(json).unwrap();

    match content {
        InteractionContent::Image { resolution, .. } => {
            assert_eq!(resolution, None);
        }
        _ => panic!("Expected Image variant"),
    }
}

#[test]
fn test_image_with_resolution_roundtrip() {
    let original = InteractionContent::Image {
        data: Some("testdata".to_string()),
        uri: None,
        mime_type: Some("image/jpeg".to_string()),
        resolution: Some(Resolution::Medium),
    };

    let json = serde_json::to_string(&original).unwrap();
    let restored: InteractionContent = serde_json::from_str(&json).unwrap();

    match restored {
        InteractionContent::Image {
            data,
            uri,
            mime_type,
            resolution,
        } => {
            assert_eq!(data, Some("testdata".to_string()));
            assert_eq!(uri, None);
            assert_eq!(mime_type, Some("image/jpeg".to_string()));
            assert_eq!(resolution, Some(Resolution::Medium));
        }
        _ => panic!("Expected Image variant"),
    }
}

#[test]
fn test_video_with_resolution_roundtrip() {
    let original = InteractionContent::Video {
        data: None,
        uri: Some("gs://bucket/video.mp4".to_string()),
        mime_type: Some("video/mp4".to_string()),
        resolution: Some(Resolution::High),
    };

    let json = serde_json::to_string(&original).unwrap();
    let restored: InteractionContent = serde_json::from_str(&json).unwrap();

    match restored {
        InteractionContent::Video {
            data,
            uri,
            mime_type,
            resolution,
        } => {
            assert_eq!(data, None);
            assert_eq!(uri, Some("gs://bucket/video.mp4".to_string()));
            assert_eq!(mime_type, Some("video/mp4".to_string()));
            assert_eq!(resolution, Some(Resolution::High));
        }
        _ => panic!("Expected Video variant"),
    }
}

// --- Resolution Unknown Tests ---

#[test]
fn test_resolution_unknown_deserialization() {
    // Test that unrecognized resolution strings deserialize to Unknown
    let json = r#""super_high""#;
    let resolution: Resolution = serde_json::from_str(json).unwrap();

    assert!(resolution.is_unknown());
    assert_eq!(resolution.unknown_resolution_type(), Some("super_high"));
}

#[test]
fn test_resolution_unknown_roundtrip() {
    // Test that Unknown variant roundtrips correctly
    let unknown = Resolution::Unknown {
        resolution_type: "extreme".to_string(),
        data: serde_json::Value::String("extreme".to_string()),
    };

    let json = serde_json::to_string(&unknown).expect("Serialization failed");
    assert_eq!(json, "\"extreme\"");

    let deserialized: Resolution = serde_json::from_str(&json).unwrap();
    assert!(deserialized.is_unknown());
    assert_eq!(deserialized.unknown_resolution_type(), Some("extreme"));
}

#[test]
fn test_resolution_unknown_helper_methods() {
    let known = Resolution::High;
    assert!(!known.is_unknown());
    assert_eq!(known.unknown_resolution_type(), None);
    assert!(known.unknown_data().is_none());

    let unknown = Resolution::Unknown {
        resolution_type: "future_res".to_string(),
        data: serde_json::json!({"level": "future_res", "extra": true}),
    };
    assert!(unknown.is_unknown());
    assert_eq!(unknown.unknown_resolution_type(), Some("future_res"));
    let data = unknown.unknown_data().unwrap();
    assert_eq!(data.get("extra").unwrap(), true);
}

#[test]
fn test_resolution_unknown_in_image_content() {
    // Test that unknown resolution works within Image content
    let json =
        r#"{"type": "image", "data": "base64", "mime_type": "image/png", "resolution": "auto"}"#;
    let content: InteractionContent = serde_json::from_str(json).unwrap();

    match content {
        InteractionContent::Image { resolution, .. } => {
            let res = resolution.expect("resolution should be present");
            assert!(res.is_unknown());
            assert_eq!(res.unknown_resolution_type(), Some("auto"));
        }
        _ => panic!("Expected Image variant"),
    }
}

#[test]
fn test_resolution_unknown_object_form() {
    // Test that object-form resolution values are handled (future API compatibility)
    // Non-string values get "<non-string: ...>" as the resolution_type
    let json = r#"{"level": "ultra_ultra_high", "tokens": 5000}"#;
    let resolution: Resolution = serde_json::from_str(json).expect("Should deserialize");

    assert!(resolution.is_unknown());
    // Object form gets formatted as "<non-string: ...>" in resolution_type
    assert!(
        resolution
            .unknown_resolution_type()
            .unwrap()
            .starts_with("<non-string:")
    );

    // Verify the full object is preserved in data
    let data = resolution.unknown_data().unwrap();
    assert_eq!(data.get("level").unwrap(), "ultra_ultra_high");
    assert_eq!(data.get("tokens").unwrap(), 5000);
}

// --- File Search Tests ---

#[test]
fn test_deserialize_file_search_result() {
    // Test the actual API format
    let json = r#"{"type": "file_search_result", "callId": "call123", "result": [{"title": "Document.pdf", "text": "Relevant content", "fileSearchStore": "store-1"}]}"#;
    let content: InteractionContent = serde_json::from_str(json).expect("Should deserialize");

    match &content {
        InteractionContent::FileSearchResult { call_id, result } => {
            assert_eq!(call_id, "call123");
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].title, "Document.pdf");
            assert_eq!(result[0].text, "Relevant content");
            assert_eq!(result[0].store, "store-1");
        }
        _ => panic!("Expected FileSearchResult variant, got {:?}", content),
    }

    assert!(content.is_file_search_result());
    assert!(!content.is_unknown());
}

#[test]
fn test_deserialize_file_search_result_multiple_items() {
    let json = r#"{
        "type": "file_search_result",
        "callId": "call456",
        "result": [
            {"title": "First.pdf", "text": "First content", "fileSearchStore": "store-a"},
            {"title": "Second.pdf", "text": "Second content", "fileSearchStore": "store-b"}
        ]
    }"#;
    let content: InteractionContent = serde_json::from_str(json).expect("Should deserialize");

    match &content {
        InteractionContent::FileSearchResult { call_id, result } => {
            assert_eq!(call_id, "call456");
            assert_eq!(result.len(), 2);
            assert_eq!(result[0].title, "First.pdf");
            assert_eq!(result[1].title, "Second.pdf");
        }
        _ => panic!("Expected FileSearchResult variant"),
    }
}

#[test]
fn test_serialize_file_search_result() {
    let content = InteractionContent::FileSearchResult {
        call_id: "call789".to_string(),
        result: vec![FileSearchResultItem {
            title: "Results.pdf".to_string(),
            text: "Found text".to_string(),
            store: "my-store".to_string(),
        }],
    };

    let json = serde_json::to_string(&content).expect("Serialization should work");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["type"], "file_search_result");
    assert_eq!(value["callId"], "call789");
    assert!(value["result"].is_array());
    assert_eq!(value["result"][0]["title"], "Results.pdf");
    assert_eq!(value["result"][0]["text"], "Found text");
    assert_eq!(value["result"][0]["fileSearchStore"], "my-store");
}

#[test]
fn test_file_search_result_roundtrip() {
    let original = InteractionContent::FileSearchResult {
        call_id: "roundtrip_test".to_string(),
        result: vec![
            FileSearchResultItem {
                title: "Doc1.pdf".to_string(),
                text: "Content one".to_string(),
                store: "store-1".to_string(),
            },
            FileSearchResultItem {
                title: "Doc2.pdf".to_string(),
                text: "Content two".to_string(),
                store: "store-2".to_string(),
            },
        ],
    };

    let json = serde_json::to_string(&original).unwrap();
    let restored: InteractionContent = serde_json::from_str(&json).unwrap();

    match restored {
        InteractionContent::FileSearchResult { call_id, result } => {
            assert_eq!(call_id, "roundtrip_test");
            assert_eq!(result.len(), 2);
            assert_eq!(result[0].title, "Doc1.pdf");
            assert_eq!(result[0].text, "Content one");
            assert_eq!(result[1].title, "Doc2.pdf");
        }
        _ => panic!("Expected FileSearchResult variant"),
    }
}

#[test]
fn test_file_search_result_empty_results() {
    // Test empty results array
    let content = InteractionContent::FileSearchResult {
        call_id: "no_results".to_string(),
        result: vec![],
    };

    let json = serde_json::to_string(&content).unwrap();
    let restored: InteractionContent = serde_json::from_str(&json).unwrap();

    match restored {
        InteractionContent::FileSearchResult { call_id, result } => {
            assert_eq!(call_id, "no_results");
            assert!(result.is_empty());
        }
        _ => panic!("Expected FileSearchResult variant"),
    }
}

#[test]
fn test_file_search_result_item_default() {
    let item = FileSearchResultItem::default();
    assert!(item.title.is_empty());
    assert!(item.text.is_empty());
    assert!(item.store.is_empty());
}
