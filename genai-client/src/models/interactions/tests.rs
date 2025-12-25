//! Unit tests for the interactions module.

use super::*;

#[test]
fn test_serialize_create_interaction_request_with_model() {
    let request = CreateInteractionRequest {
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: InteractionInput::Text("Hello, world!".to_string()),
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

    let json = serde_json::to_string(&request).expect("Serialization failed");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["model"], "gemini-3-flash-preview");
    assert_eq!(value["input"], "Hello, world!");
    assert!(value.get("agent").is_none());
}

#[test]
fn test_serialize_interaction_content() {
    let content = InteractionContent::Text {
        text: Some("Hello".to_string()),
    };

    let json = serde_json::to_string(&content).expect("Serialization failed");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["type"], "text");
    assert_eq!(value["text"], "Hello");
}

#[test]
fn test_deserialize_interaction_response_completed() {
    let response_json = r#"{
        "id": "interaction_123",
        "model": "gemini-3-flash-preview",
        "input": [{"type": "text", "text": "Hello"}],
        "outputs": [{"type": "text", "text": "Hi there!"}],
        "status": "completed",
        "usage": {
            "total_input_tokens": 5,
            "total_output_tokens": 10,
            "total_tokens": 15
        }
    }"#;

    let response: InteractionResponse =
        serde_json::from_str(response_json).expect("Deserialization failed");

    assert_eq!(response.id, "interaction_123");
    assert_eq!(response.model.as_deref(), Some("gemini-3-flash-preview"));
    assert_eq!(response.status, InteractionStatus::Completed);
    assert_eq!(response.input.len(), 1);
    assert_eq!(response.outputs.len(), 1);
    assert!(response.usage.is_some());
    let usage = response.usage.unwrap();
    assert_eq!(usage.total_input_tokens, Some(5));
    assert_eq!(usage.total_output_tokens, Some(10));
    assert_eq!(usage.total_tokens, Some(15));
}

#[test]
fn test_deserialize_usage_metadata_partial() {
    // Test that partial usage responses deserialize correctly with #[serde(default)]
    let partial_json = r#"{"total_tokens": 42}"#;
    let usage: UsageMetadata = serde_json::from_str(partial_json).expect("Deserialization failed");

    assert_eq!(usage.total_tokens, Some(42));
    assert_eq!(usage.total_input_tokens, None);
    assert_eq!(usage.total_output_tokens, None);
    assert_eq!(usage.total_cached_tokens, None);
    assert_eq!(usage.total_reasoning_tokens, None);
    assert_eq!(usage.total_tool_use_tokens, None);
}

#[test]
fn test_deserialize_usage_metadata_empty() {
    // Test that empty usage object deserializes to defaults
    let empty_json = r#"{}"#;
    let usage: UsageMetadata = serde_json::from_str(empty_json).expect("Deserialization failed");

    assert_eq!(usage.total_tokens, None);
    assert_eq!(usage.total_input_tokens, None);
    assert_eq!(usage.total_output_tokens, None);
}

#[test]
fn test_usage_metadata_has_data() {
    // Empty usage has no data
    let empty = UsageMetadata::default();
    assert!(!empty.has_data());

    // Usage with only total_tokens
    let with_total = UsageMetadata {
        total_tokens: Some(100),
        ..Default::default()
    };
    assert!(with_total.has_data());

    // Usage with only cached tokens
    let with_cached = UsageMetadata {
        total_cached_tokens: Some(50),
        ..Default::default()
    };
    assert!(with_cached.has_data());
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
fn test_generation_config_serialization() {
    let config = GenerationConfig {
        temperature: Some(0.7),
        max_output_tokens: Some(500),
        top_p: Some(0.9),
        top_k: Some(40),
        thinking_level: Some("medium".to_string()),
    };

    let json = serde_json::to_string(&config).expect("Serialization failed");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["temperature"], 0.7);
    assert_eq!(value["maxOutputTokens"], 500);
    assert_eq!(value["thinkingLevel"], "medium");
}

#[test]
fn test_interaction_response_text() {
    let response = InteractionResponse {
        id: "test_id".to_string(),
        model: Some("gemini-3-flash".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::Text {
                text: Some("Hello".to_string()),
            },
            InteractionContent::Text {
                text: Some("World".to_string()),
            },
        ],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert_eq!(response.text(), Some("Hello"));
    assert_eq!(response.all_text(), "HelloWorld");
    assert!(response.has_text());
    assert!(!response.has_function_calls());
}

#[test]
fn test_interaction_response_function_calls() {
    let response = InteractionResponse {
        id: "test_id".to_string(),
        model: Some("gemini-3-flash".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::FunctionCall {
                id: Some("call_001".to_string()),
                name: "get_weather".to_string(),
                args: serde_json::json!({"location": "Paris"}),
                thought_signature: Some("sig123".to_string()),
            },
            InteractionContent::FunctionCall {
                id: Some("call_002".to_string()),
                name: "get_time".to_string(),
                args: serde_json::json!({"timezone": "UTC"}),
                thought_signature: None,
            },
        ],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    let calls = response.function_calls();
    assert_eq!(calls.len(), 2);
    // FunctionCallInfo struct fields
    assert_eq!(calls[0].id, Some("call_001"));
    assert_eq!(calls[0].name, "get_weather");
    assert_eq!(calls[0].args["location"], "Paris");
    assert_eq!(calls[0].thought_signature, Some("sig123"));
    assert_eq!(calls[1].id, Some("call_002"));
    assert_eq!(calls[1].name, "get_time");
    assert_eq!(calls[1].thought_signature, None);
    assert!(response.has_function_calls());
    assert!(!response.has_text());
}

#[test]
fn test_function_call_missing_id() {
    // Test that function calls with missing id are correctly captured as None.
    // This scenario should not normally occur (API contract requires call_id),
    // but if it does, the auto-function loop will return an error.
    let response = InteractionResponse {
        id: "test_id".to_string(),
        model: Some("gemini-3-flash".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![InteractionContent::FunctionCall {
            id: None, // Missing call_id - should be captured correctly
            name: "get_weather".to_string(),
            args: serde_json::json!({"location": "Tokyo"}),
            thought_signature: None,
        }],
        status: InteractionStatus::RequiresAction,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    let calls = response.function_calls();
    assert_eq!(calls.len(), 1);
    // Verify that missing id is correctly captured as None
    assert_eq!(calls[0].id, None);
    assert_eq!(calls[0].name, "get_weather");
    assert_eq!(calls[0].args["location"], "Tokyo");

    // The auto-function loop in request_builder.rs will return an error
    // when it encounters a function call with None id, since call_id is
    // required to send function results back to the API.
}

#[test]
fn test_interaction_response_mixed_content() {
    let response = InteractionResponse {
        id: "test_id".to_string(),
        model: Some("gemini-3-flash".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::Text {
                text: Some("Let me check".to_string()),
            },
            InteractionContent::FunctionCall {
                id: Some("call_mixed".to_string()),
                name: "check_status".to_string(),
                args: serde_json::json!({}),
                thought_signature: None,
            },
            InteractionContent::Text {
                text: Some("Done!".to_string()),
            },
        ],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert_eq!(response.text(), Some("Let me check"));
    assert_eq!(response.all_text(), "Let me checkDone!");
    assert_eq!(response.function_calls().len(), 1);
    assert!(response.has_text());
    assert!(response.has_function_calls());
}

#[test]
fn test_interaction_response_empty_outputs() {
    let response = InteractionResponse {
        id: "test_id".to_string(),
        model: Some("gemini-3-flash".to_string()),
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

    assert_eq!(response.text(), None);
    assert_eq!(response.all_text(), "");
    assert_eq!(response.function_calls().len(), 0);
    assert!(!response.has_text());
    assert!(!response.has_function_calls());
}

// --- Streaming Event Tests ---

#[test]
fn test_deserialize_streaming_text_content() {
    // Streaming deltas now use InteractionContent directly
    let delta_json = r#"{"type": "text", "text": "Hello world"}"#;
    let delta: InteractionContent =
        serde_json::from_str(delta_json).expect("Deserialization failed");

    match &delta {
        InteractionContent::Text { text } => {
            assert_eq!(text.as_deref(), Some("Hello world"));
        }
        _ => panic!("Expected Text content"),
    }

    assert!(delta.is_text());
    assert!(!delta.is_thought());
    assert_eq!(delta.text(), Some("Hello world"));
}

#[test]
fn test_deserialize_streaming_thought_content() {
    let delta_json = r#"{"type": "thought", "text": "I'm thinking..."}"#;
    let delta: InteractionContent =
        serde_json::from_str(delta_json).expect("Deserialization failed");

    match &delta {
        InteractionContent::Thought { text } => {
            assert_eq!(text.as_deref(), Some("I'm thinking..."));
        }
        _ => panic!("Expected Thought content"),
    }

    assert!(!delta.is_text());
    assert!(delta.is_thought());
    // text() returns None for thoughts (only returns text for Text variant)
    assert_eq!(delta.text(), None);
}

#[test]
fn test_deserialize_streaming_function_call() {
    // Function calls can now be streamed - this was issue #27
    let delta_json =
        r#"{"type": "function_call", "name": "get_weather", "arguments": {"city": "Paris"}}"#;
    let delta: InteractionContent =
        serde_json::from_str(delta_json).expect("Deserialization failed");

    match &delta {
        InteractionContent::FunctionCall { name, args, .. } => {
            assert_eq!(name, "get_weather");
            assert_eq!(args["city"], "Paris");
        }
        _ => panic!("Expected FunctionCall content"),
    }

    assert!(delta.is_function_call());
    assert!(!delta.is_unknown()); // function_call is now a KNOWN type!
}

#[test]
fn test_deserialize_streaming_thought_signature() {
    let delta_json = r#"{"type": "thought_signature", "signature": "abc123"}"#;
    let delta: InteractionContent =
        serde_json::from_str(delta_json).expect("Deserialization failed");

    match &delta {
        InteractionContent::ThoughtSignature { signature } => {
            assert_eq!(signature, "abc123");
        }
        _ => panic!("Expected ThoughtSignature content"),
    }

    assert!(delta.is_thought_signature());
}

#[test]
fn test_deserialize_content_delta_event() {
    let event_json = r#"{
        "event_type": "content.delta",
        "interaction_id": "test_123",
        "delta": {"type": "text", "text": "Hello"}
    }"#;

    let event: InteractionStreamEvent =
        serde_json::from_str(event_json).expect("Deserialization failed");

    assert_eq!(event.event_type, "content.delta");
    assert_eq!(event.interaction_id.as_deref(), Some("test_123"));
    assert!(event.delta.is_some());
    assert!(event.interaction.is_none());

    let delta = event.delta.unwrap();
    assert!(delta.is_text());
    assert_eq!(delta.text(), Some("Hello"));
}

#[test]
fn test_deserialize_interaction_complete_event() {
    let event_json = r#"{
        "event_type": "interaction.complete",
        "interaction": {
            "id": "interaction_456",
            "model": "gemini-3-flash-preview",
            "input": [{"type": "text", "text": "Count to 3"}],
            "outputs": [{"type": "text", "text": "1, 2, 3"}],
            "status": "completed"
        }
    }"#;

    let event: InteractionStreamEvent =
        serde_json::from_str(event_json).expect("Deserialization failed");

    assert_eq!(event.event_type, "interaction.complete");
    assert!(event.interaction.is_some());
    assert!(event.delta.is_none());

    let interaction = event.interaction.unwrap();
    assert_eq!(interaction.id, "interaction_456");
    assert_eq!(interaction.text(), Some("1, 2, 3"));
}

#[test]
fn test_content_empty_text_returns_none() {
    let content = InteractionContent::Text {
        text: Some(String::new()),
    };
    assert_eq!(content.text(), None);

    let content_none = InteractionContent::Text { text: None };
    assert_eq!(content_none.text(), None);
}

// --- Unknown Variant Tests ---

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

#[test]
fn test_interaction_response_has_unknown() {
    let response = InteractionResponse {
        id: "test_id".to_string(),
        model: Some("gemini-3-flash".to_string()),
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
        model: Some("gemini-3-flash".to_string()),
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

#[test]
fn test_content_summary() {
    let response = InteractionResponse {
        id: "test_id".to_string(),
        model: Some("gemini-3-flash".to_string()),
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
        model: Some("gemini-3-flash".to_string()),
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

// --- Built-in Tool Content Tests ---

#[test]
fn test_deserialize_code_execution_call() {
    // Test deserialization from the API format (arguments object)
    let json = r#"{"type": "code_execution_call", "id": "call_123", "arguments": {"code": "print(42)", "language": "python"}}"#;
    let content: InteractionContent = serde_json::from_str(json).expect("Should deserialize");

    match &content {
        InteractionContent::CodeExecutionCall { id, language, code } => {
            assert_eq!(id, "call_123");
            assert_eq!(language, "python");
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
            assert_eq!(language, "PYTHON");
            assert_eq!(code, "print(42)");
        }
        _ => panic!("Expected CodeExecutionCall variant, got {:?}", content),
    }
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
    let json = r#"{"type": "google_search_call", "query": "Rust programming"}"#;
    let content: InteractionContent = serde_json::from_str(json).expect("Should deserialize");

    match &content {
        InteractionContent::GoogleSearchCall { query } => {
            assert_eq!(query, "Rust programming");
        }
        _ => panic!("Expected GoogleSearchCall variant, got {:?}", content),
    }

    assert!(content.is_google_search_call());
    assert!(!content.is_unknown());
}

#[test]
fn test_deserialize_google_search_result() {
    let json = r#"{"type": "google_search_result", "results": {"items": [{"title": "Rust", "url": "https://rust-lang.org"}]}}"#;
    let content: InteractionContent = serde_json::from_str(json).expect("Should deserialize");

    match &content {
        InteractionContent::GoogleSearchResult { results } => {
            assert!(results["items"].is_array());
            assert_eq!(results["items"][0]["title"], "Rust");
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
        language: "PYTHON".to_string(),
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
        language: "PYTHON".to_string(),
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
        query: "test query".to_string(),
    };
    let json = serde_json::to_string(&original).unwrap();
    let restored: InteractionContent = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        restored,
        InteractionContent::GoogleSearchCall { .. }
    ));

    // GoogleSearchResult roundtrip
    let original = InteractionContent::GoogleSearchResult {
        results: serde_json::json!({"items": []}),
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
        language: "PYTHON".to_string(),
        code: "".to_string(),
    };
    let json = serde_json::to_string(&content).unwrap();
    let restored: InteractionContent = serde_json::from_str(&json).unwrap();
    match restored {
        InteractionContent::CodeExecutionCall { id, language, code } => {
            assert_eq!(id, "call_empty");
            assert_eq!(language, "PYTHON");
            assert!(code.is_empty());
        }
        _ => panic!("Expected CodeExecutionCall"),
    }

    // Empty results in GoogleSearchResult
    let content = InteractionContent::GoogleSearchResult {
        results: serde_json::json!({}),
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
fn test_interaction_response_code_execution_helpers() {
    let response = InteractionResponse {
        id: "test_id".to_string(),
        model: Some("gemini-3-flash".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::Text {
                text: Some("Here's the code:".to_string()),
            },
            InteractionContent::CodeExecutionCall {
                id: "call_123".to_string(),
                language: "PYTHON".to_string(),
                code: "print(42)".to_string(),
            },
            InteractionContent::CodeExecutionResult {
                call_id: "call_123".to_string(),
                outcome: CodeExecutionOutcome::Ok,
                output: "42\n".to_string(),
            },
        ],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert!(response.has_code_execution_calls());
    assert!(response.has_code_execution_results());
    assert!(!response.has_unknown());

    // Test code_execution_calls helper
    let code_blocks = response.code_execution_calls();
    assert_eq!(code_blocks.len(), 1);
    assert_eq!(code_blocks[0].0, "PYTHON");
    assert_eq!(code_blocks[0].1, "print(42)");

    // Test code_execution_results helper
    let results = response.code_execution_results();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, CodeExecutionOutcome::Ok);
    assert_eq!(results[0].1, "42\n");

    // Test successful_code_output helper
    assert_eq!(response.successful_code_output(), Some("42\n"));
}

#[test]
fn test_interaction_response_google_search_helpers() {
    let response = InteractionResponse {
        id: "test_id".to_string(),
        model: Some("gemini-3-flash".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::GoogleSearchResult {
                results: serde_json::json!({"items": [{"title": "Test"}]}),
            },
            InteractionContent::Text {
                text: Some("Based on search results...".to_string()),
            },
        ],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert!(response.has_google_search_results());

    let search_results = response.google_search_results();
    assert_eq!(search_results.len(), 1);
    assert_eq!(search_results[0]["items"][0]["title"], "Test");
}

#[test]
fn test_interaction_response_url_context_helpers() {
    let response = InteractionResponse {
        id: "test_id".to_string(),
        model: Some("gemini-3-flash".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![InteractionContent::UrlContextResult {
            url: "https://example.com".to_string(),
            content: Some("Example content".to_string()),
        }],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert!(response.has_url_context_results());

    let url_results = response.url_context_results();
    assert_eq!(url_results.len(), 1);
    assert_eq!(
        url_results[0],
        ("https://example.com", Some("Example content"))
    );
}

#[test]
fn test_content_summary_with_built_in_tools() {
    let response = InteractionResponse {
        id: "test_id".to_string(),
        model: Some("gemini-3-flash".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::CodeExecutionCall {
                id: "call_1".to_string(),
                language: "PYTHON".to_string(),
                code: "print(1)".to_string(),
            },
            InteractionContent::CodeExecutionCall {
                id: "call_2".to_string(),
                language: "PYTHON".to_string(),
                code: "print(2)".to_string(),
            },
            InteractionContent::CodeExecutionResult {
                call_id: "call_1".to_string(),
                outcome: CodeExecutionOutcome::Ok,
                output: "1\n2\n".to_string(),
            },
            InteractionContent::GoogleSearchCall {
                query: "test".to_string(),
            },
            InteractionContent::GoogleSearchResult {
                results: serde_json::json!({}),
            },
            InteractionContent::UrlContextCall {
                url: "https://example.com".to_string(),
            },
            InteractionContent::UrlContextResult {
                url: "https://example.com".to_string(),
                content: None,
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

    assert_eq!(summary.code_execution_call_count, 2);
    assert_eq!(summary.code_execution_result_count, 1);
    assert_eq!(summary.google_search_call_count, 1);
    assert_eq!(summary.google_search_result_count, 1);
    assert_eq!(summary.url_context_call_count, 1);
    assert_eq!(summary.url_context_result_count, 1);
    assert_eq!(summary.unknown_count, 0);
}

#[test]
fn test_deserialize_url_context_metadata() {
    // Test full deserialization with all statuses
    let json = r#"{
        "urlMetadata": [
            {
                "retrievedUrl": "https://example.com",
                "urlRetrievalStatus": "URL_RETRIEVAL_STATUS_SUCCESS"
            },
            {
                "retrievedUrl": "https://blocked.com",
                "urlRetrievalStatus": "URL_RETRIEVAL_STATUS_UNSAFE"
            },
            {
                "retrievedUrl": "https://failed.com",
                "urlRetrievalStatus": "URL_RETRIEVAL_STATUS_ERROR"
            }
        ]
    }"#;

    let metadata: UrlContextMetadata = serde_json::from_str(json).expect("Failed to deserialize");

    assert_eq!(metadata.url_metadata.len(), 3);

    assert_eq!(
        metadata.url_metadata[0].retrieved_url,
        "https://example.com"
    );
    assert_eq!(
        metadata.url_metadata[0].url_retrieval_status,
        UrlRetrievalStatus::UrlRetrievalStatusSuccess
    );

    assert_eq!(
        metadata.url_metadata[1].retrieved_url,
        "https://blocked.com"
    );
    assert_eq!(
        metadata.url_metadata[1].url_retrieval_status,
        UrlRetrievalStatus::UrlRetrievalStatusUnsafe
    );

    assert_eq!(metadata.url_metadata[2].retrieved_url, "https://failed.com");
    assert_eq!(
        metadata.url_metadata[2].url_retrieval_status,
        UrlRetrievalStatus::UrlRetrievalStatusError
    );
}

#[test]
fn test_deserialize_url_context_metadata_empty() {
    // Test empty url_metadata array
    let json = r#"{"urlMetadata": []}"#;
    let metadata: UrlContextMetadata = serde_json::from_str(json).expect("Failed to deserialize");
    assert!(metadata.url_metadata.is_empty());
}

#[test]
fn test_deserialize_url_context_metadata_missing_field() {
    // Test missing urlMetadata field (should default to empty vec)
    let json = r#"{}"#;
    let metadata: UrlContextMetadata = serde_json::from_str(json).expect("Failed to deserialize");
    assert!(metadata.url_metadata.is_empty());
}

#[test]
fn test_url_retrieval_status_serialization_roundtrip() {
    // Test all enum variants roundtrip correctly
    let statuses = vec![
        UrlRetrievalStatus::UrlRetrievalStatusUnspecified,
        UrlRetrievalStatus::UrlRetrievalStatusSuccess,
        UrlRetrievalStatus::UrlRetrievalStatusUnsafe,
        UrlRetrievalStatus::UrlRetrievalStatusError,
    ];

    for status in statuses {
        let serialized = serde_json::to_string(&status).expect("Failed to serialize");
        let deserialized: UrlRetrievalStatus =
            serde_json::from_str(&serialized).expect("Failed to deserialize");
        assert_eq!(status, deserialized);
    }
}
