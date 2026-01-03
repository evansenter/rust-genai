//! Unit tests for InteractionResponse, UsageMetadata, ContentSummary, InteractionStatus, and helpers.

use super::*;

// --- Response Deserialization ---

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

    assert_eq!(response.id.as_deref(), Some("interaction_123"));
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

// --- UsageMetadata Tests ---

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

// --- Response Helper Tests ---

#[test]
fn test_interaction_response_text() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::Text {
                text: Some("Hello".to_string()),
                annotations: None,
            },
            InteractionContent::Text {
                text: Some("World".to_string()),
                annotations: None,
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
fn test_interaction_response_thoughts() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::Thought {
                text: Some("Let me think about this...".to_string()),
            },
            InteractionContent::Thought {
                text: Some("The answer is 42.".to_string()),
            },
            InteractionContent::Text {
                text: Some("The answer is 42.".to_string()),
                annotations: None,
            },
            // Thought with None text should be filtered out
            InteractionContent::Thought { text: None },
        ],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert!(response.has_thoughts());

    let thoughts: Vec<_> = response.thoughts().collect();
    assert_eq!(thoughts.len(), 2);
    assert_eq!(thoughts[0], "Let me think about this...");
    assert_eq!(thoughts[1], "The answer is 42.");

    // Verify text() still works (only returns Text content)
    assert_eq!(response.text(), Some("The answer is 42."));
}

#[test]
fn test_interaction_response_no_thoughts() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![InteractionContent::Text {
            text: Some("Just text, no thoughts.".to_string()),
            annotations: None,
        }],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert!(!response.has_thoughts());
    let thoughts: Vec<_> = response.thoughts().collect();
    assert!(thoughts.is_empty());
}

#[test]
fn test_interaction_response_function_calls() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
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
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
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
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::Text {
                text: Some("Let me check".to_string()),
                annotations: None,
            },
            InteractionContent::FunctionCall {
                id: Some("call_mixed".to_string()),
                name: "check_status".to_string(),
                args: serde_json::json!({}),
                thought_signature: None,
            },
            InteractionContent::Text {
                text: Some("Done!".to_string()),
                annotations: None,
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
        id: Some("test_id".to_string()),
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

    assert_eq!(response.text(), None);
    assert_eq!(response.all_text(), "");
    assert_eq!(response.function_calls().len(), 0);
    assert!(!response.has_text());
    assert!(!response.has_function_calls());
}

// --- Unknown Response Helper Tests ---

#[test]
fn test_interaction_response_has_unknown() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::Text {
                text: Some("Here's the result:".to_string()),
                annotations: None,
            },
            InteractionContent::Unknown {
                content_type: "code_execution_result".to_string(),
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
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![InteractionContent::Text {
            text: Some("Normal response".to_string()),
            annotations: None,
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

// --- ContentSummary Tests ---

#[test]
fn test_content_summary() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::Text {
                text: Some("Text 1".to_string()),
                annotations: None,
            },
            InteractionContent::Text {
                text: Some("Text 2".to_string()),
                annotations: None,
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
                content_type: "type_a".to_string(),
                data: serde_json::json!({"type": "type_a"}),
            },
            InteractionContent::Unknown {
                content_type: "type_b".to_string(),
                data: serde_json::json!({"type": "type_b"}),
            },
            InteractionContent::Unknown {
                content_type: "type_a".to_string(), // Duplicate type
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
        id: Some("test_id".to_string()),
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

#[test]
fn test_content_summary_with_built_in_tools() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::CodeExecutionCall {
                id: "call_1".to_string(),
                language: CodeExecutionLanguage::Python,
                code: "print(1)".to_string(),
            },
            InteractionContent::CodeExecutionCall {
                id: "call_2".to_string(),
                language: CodeExecutionLanguage::Python,
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

// --- Built-in Tool Helper Tests ---

#[test]
fn test_interaction_response_code_execution_helpers() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::Text {
                text: Some("Here's the code:".to_string()),
                annotations: None,
            },
            InteractionContent::CodeExecutionCall {
                id: "call_123".to_string(),
                language: CodeExecutionLanguage::Python,
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
    assert_eq!(code_blocks[0].id, "call_123");
    assert_eq!(code_blocks[0].language, CodeExecutionLanguage::Python);
    assert_eq!(code_blocks[0].code, "print(42)");

    // Test code_execution_results helper
    let results = response.code_execution_results();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].call_id, "call_123");
    assert_eq!(results[0].outcome, CodeExecutionOutcome::Ok);
    assert_eq!(results[0].output, "42\n");

    // Test successful_code_output helper
    assert_eq!(response.successful_code_output(), Some("42\n"));
}

#[test]
fn test_interaction_response_google_search_helpers() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::GoogleSearchResult {
                results: serde_json::json!({"items": [{"title": "Test"}]}),
            },
            InteractionContent::Text {
                text: Some("Based on search results...".to_string()),
                annotations: None,
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
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
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
    assert_eq!(url_results[0].url, "https://example.com");
    assert_eq!(url_results[0].content, Some("Example content"));
}

// --- URL Context Metadata Tests ---

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

// --- Function Result Helpers ---

#[test]
fn test_interaction_response_function_results() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::FunctionResult {
                name: "get_weather".to_string(),
                call_id: "call_001".to_string(),
                result: serde_json::json!({"temp": 72, "unit": "F"}),
            },
            InteractionContent::FunctionResult {
                name: "get_time".to_string(),
                call_id: "call_002".to_string(),
                result: serde_json::json!({"time": "14:30", "zone": "UTC"}),
            },
            InteractionContent::Text {
                text: Some("Here are the results".to_string()),
                annotations: None,
            },
        ],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert!(response.has_function_results());
    assert!(response.has_text());

    let results = response.function_results();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].name, "get_weather");
    assert_eq!(results[0].call_id, "call_001");
    assert_eq!(results[0].result["temp"], 72);
    assert_eq!(results[1].name, "get_time");
    assert_eq!(results[1].call_id, "call_002");
}

#[test]
fn test_interaction_response_no_function_results() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![InteractionContent::Text {
            text: Some("Just text".to_string()),
            annotations: None,
        }],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert!(!response.has_function_results());
    assert!(response.function_results().is_empty());
}

// --- Google Search Helpers ---

#[test]
fn test_interaction_response_google_search_call_helpers() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::GoogleSearchCall {
                query: "Rust programming language".to_string(),
            },
            InteractionContent::GoogleSearchCall {
                query: "async await Rust".to_string(),
            },
            InteractionContent::Text {
                text: Some("Search results...".to_string()),
                annotations: None,
            },
        ],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert!(response.has_google_search_calls());

    // Test google_search_call() - returns first one
    assert_eq!(
        response.google_search_call(),
        Some("Rust programming language")
    );

    // Test google_search_calls() - returns all
    let queries = response.google_search_calls();
    assert_eq!(queries.len(), 2);
    assert_eq!(queries[0], "Rust programming language");
    assert_eq!(queries[1], "async await Rust");
}

#[test]
fn test_interaction_response_no_google_search_calls() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![InteractionContent::Text {
            text: Some("No search".to_string()),
            annotations: None,
        }],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert!(!response.has_google_search_calls());
    assert!(response.google_search_call().is_none());
    assert!(response.google_search_calls().is_empty());
}

// --- URL Context Helpers ---

#[test]
fn test_interaction_response_url_context_call_helpers() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::UrlContextCall {
                url: "https://docs.rs".to_string(),
            },
            InteractionContent::UrlContextCall {
                url: "https://rust-lang.org".to_string(),
            },
        ],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert!(response.has_url_context_calls());

    // Test url_context_call() - returns first one
    assert_eq!(response.url_context_call(), Some("https://docs.rs"));

    // Test url_context_calls() - returns all
    let urls = response.url_context_calls();
    assert_eq!(urls.len(), 2);
    assert_eq!(urls[0], "https://docs.rs");
    assert_eq!(urls[1], "https://rust-lang.org");
}

#[test]
fn test_interaction_response_no_url_context_calls() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
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

    assert!(!response.has_url_context_calls());
    assert!(response.url_context_call().is_none());
    assert!(response.url_context_calls().is_empty());
}

// --- Code Execution Helpers ---

#[test]
fn test_interaction_response_code_execution_call_singular() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::CodeExecutionCall {
                id: "call_first".to_string(),
                language: CodeExecutionLanguage::Python,
                code: "print('first')".to_string(),
            },
            InteractionContent::CodeExecutionCall {
                id: "call_second".to_string(),
                language: CodeExecutionLanguage::Python,
                code: "print('second')".to_string(),
            },
        ],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    // Test code_execution_call() - returns first one
    let call = response.code_execution_call();
    assert!(call.is_some());
    let call = call.unwrap();
    assert_eq!(call.id, "call_first");
    assert_eq!(call.language, CodeExecutionLanguage::Python);
    assert_eq!(call.code, "print('first')");
}

#[test]
fn test_interaction_response_no_code_execution_call() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![InteractionContent::Text {
            text: Some("No code".to_string()),
            annotations: None,
        }],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert!(response.code_execution_call().is_none());
}

// --- Metadata Helpers ---

#[test]
fn test_interaction_response_google_search_metadata_helpers() {
    use crate::models::interactions::GroundingMetadata;

    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![InteractionContent::Text {
            text: Some("Response grounded with search".to_string()),
            annotations: None,
        }],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: Some(GroundingMetadata {
            web_search_queries: vec!["Rust language".to_string()],
            grounding_chunks: vec![],
        }),
        url_context_metadata: None,
    };

    assert!(response.has_google_search_metadata());

    let metadata = response.google_search_metadata();
    assert!(metadata.is_some());
    let metadata = metadata.unwrap();
    assert_eq!(metadata.web_search_queries, vec!["Rust language"]);
}

#[test]
fn test_interaction_response_no_google_search_metadata() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
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

    assert!(!response.has_google_search_metadata());
    assert!(response.google_search_metadata().is_none());
}

#[test]
fn test_interaction_response_url_context_metadata_helpers() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: Some(UrlContextMetadata {
            url_metadata: vec![UrlMetadataEntry {
                retrieved_url: "https://example.com".to_string(),
                url_retrieval_status: UrlRetrievalStatus::UrlRetrievalStatusSuccess,
            }],
        }),
    };

    assert!(response.has_url_context_metadata());

    let metadata = response.url_context_metadata();
    assert!(metadata.is_some());
    let metadata = metadata.unwrap();
    assert_eq!(metadata.url_metadata.len(), 1);
    assert_eq!(
        metadata.url_metadata[0].retrieved_url,
        "https://example.com"
    );
    assert_eq!(
        metadata.url_metadata[0].url_retrieval_status,
        UrlRetrievalStatus::UrlRetrievalStatusSuccess
    );
}

#[test]
fn test_interaction_response_no_url_context_metadata() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
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

    assert!(!response.has_url_context_metadata());
    assert!(response.url_context_metadata().is_none());
}

#[test]
fn test_interaction_response_code_execution_calls_plural() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::CodeExecutionCall {
                id: "call_1".to_string(),
                language: CodeExecutionLanguage::Python,
                code: "print('first')".to_string(),
            },
            InteractionContent::CodeExecutionCall {
                id: "call_2".to_string(),
                language: CodeExecutionLanguage::Python,
                code: "print('second')".to_string(),
            },
            InteractionContent::Text {
                text: Some("Results".to_string()),
                annotations: None,
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

    let calls = response.code_execution_calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].id, "call_1");
    assert_eq!(calls[0].language, CodeExecutionLanguage::Python);
    assert_eq!(calls[0].code, "print('first')");
    assert_eq!(calls[1].id, "call_2");
    assert_eq!(calls[1].language, CodeExecutionLanguage::Python);
    assert_eq!(calls[1].code, "print('second')");
}

#[test]
fn test_interaction_response_code_execution_results() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::CodeExecutionResult {
                call_id: "call_1".to_string(),
                outcome: CodeExecutionOutcome::Ok,
                output: "first output".to_string(),
            },
            InteractionContent::CodeExecutionResult {
                call_id: "call_2".to_string(),
                outcome: CodeExecutionOutcome::Failed,
                output: "error message".to_string(),
            },
        ],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert!(response.has_code_execution_results());

    let results = response.code_execution_results();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].call_id, "call_1");
    assert_eq!(results[0].outcome, CodeExecutionOutcome::Ok);
    assert_eq!(results[0].output, "first output");
    assert_eq!(results[1].call_id, "call_2");
    assert_eq!(results[1].outcome, CodeExecutionOutcome::Failed);
    assert_eq!(results[1].output, "error message");

    // Test successful_code_output - should return first successful output
    let success = response.successful_code_output();
    assert_eq!(success, Some("first output"));
}

#[test]
fn test_interaction_response_no_code_execution_results() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![InteractionContent::Text {
            text: Some("No code".to_string()),
            annotations: None,
        }],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert!(!response.has_code_execution_results());
    assert!(response.code_execution_results().is_empty());
    assert!(response.successful_code_output().is_none());
}

#[test]
fn test_interaction_response_google_search_results() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::GoogleSearchResult {
                results: serde_json::json!({"items": [{"title": "Rust Lang"}]}),
            },
            InteractionContent::GoogleSearchResult {
                results: serde_json::json!({"items": [{"title": "Cargo"}]}),
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

    let results = response.google_search_results();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0]["items"][0]["title"], "Rust Lang");
    assert_eq!(results[1]["items"][0]["title"], "Cargo");
}

#[test]
fn test_interaction_response_no_google_search_results() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
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

    assert!(!response.has_google_search_results());
    assert!(response.google_search_results().is_empty());
}

#[test]
fn test_interaction_response_url_context_results() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::UrlContextResult {
                url: "https://docs.rs".to_string(),
                content: Some("<html>docs content</html>".to_string()),
            },
            InteractionContent::UrlContextResult {
                url: "https://crates.io".to_string(),
                content: Some("<html>crates content</html>".to_string()),
            },
            InteractionContent::UrlContextResult {
                url: "https://blocked.com".to_string(),
                content: None, // Failed fetch
            },
        ],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert!(response.has_url_context_results());

    let results = response.url_context_results();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].url, "https://docs.rs");
    assert_eq!(results[0].content, Some("<html>docs content</html>"));
    assert_eq!(results[1].url, "https://crates.io");
    assert_eq!(results[1].content, Some("<html>crates content</html>"));
    assert_eq!(results[2].url, "https://blocked.com");
    assert_eq!(results[2].content, None); // Failed fetch has no content
}

#[test]
fn test_interaction_response_no_url_context_results() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
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

    assert!(!response.has_url_context_results());
    assert!(response.url_context_results().is_empty());
}

/// Comprehensive roundtrip test for InteractionResponse with all content types.
///
/// This test verifies that complex responses with multiple content types,
/// function calls, thoughts, and metadata can be serialized and deserialized
/// without data loss. This is critical for save/resume semantics.
#[test]
fn test_interaction_response_complex_roundtrip() {
    // Build a response with many different content types
    let response = InteractionResponse {
        id: Some("complex-interaction-xyz".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![
            InteractionContent::Text {
                text: Some("Analyze this and call the weather function".to_string()),
                annotations: None,
            },
            InteractionContent::Image {
                mime_type: Some("image/png".to_string()),
                data: Some("base64encodeddata".to_string()),
                uri: None,
            },
        ],
        outputs: vec![
            // Thought with signature (thinking models)
            InteractionContent::Thought {
                text: Some("Let me think about this request...".to_string()),
            },
            InteractionContent::ThoughtSignature {
                signature: "thought-sig-abc123".to_string(),
            },
            // Function call
            InteractionContent::FunctionCall {
                id: Some("call-func-001".to_string()),
                name: "get_weather".to_string(),
                args: serde_json::json!({"city": "Tokyo", "units": "celsius"}),
                thought_signature: Some("thought-sig-for-call".to_string()),
            },
            // Function result
            InteractionContent::FunctionResult {
                call_id: "call-func-001".to_string(),
                name: "get_weather".to_string(),
                result: serde_json::json!({"temp": 22, "conditions": "sunny"}),
            },
            // Code execution
            InteractionContent::CodeExecutionCall {
                id: "code-exec-001".to_string(),
                language: CodeExecutionLanguage::Python,
                code: "print(2 + 2)".to_string(),
            },
            InteractionContent::CodeExecutionResult {
                call_id: "code-exec-001".to_string(),
                outcome: CodeExecutionOutcome::Ok,
                output: "4".to_string(),
            },
            // Google search
            InteractionContent::GoogleSearchCall {
                query: "weather in Tokyo".to_string(),
            },
            InteractionContent::GoogleSearchResult {
                results: serde_json::json!({"query": "weather tokyo", "results": []}),
            },
            // URL context
            InteractionContent::UrlContextCall {
                url: "https://example.com".to_string(),
            },
            InteractionContent::UrlContextResult {
                url: "https://example.com".to_string(),
                content: Some("<html>Example content</html>".to_string()),
            },
            // Final text response
            InteractionContent::Text {
                text: Some("The weather in Tokyo is 22°C and sunny.".to_string()),
                annotations: None,
            },
        ],
        status: InteractionStatus::Completed,
        usage: Some(UsageMetadata {
            total_input_tokens: Some(150),
            total_output_tokens: Some(200),
            total_tokens: Some(350),
            total_cached_tokens: Some(50),
            total_reasoning_tokens: Some(30),
            total_tool_use_tokens: Some(20),
            ..Default::default()
        }),
        tools: Some(vec![crate::Tool::GoogleSearch, crate::Tool::CodeExecution]),
        previous_interaction_id: Some("previous-interaction-abc".to_string()),
        grounding_metadata: Some(GroundingMetadata {
            grounding_chunks: vec![GroundingChunk {
                web: WebSource {
                    title: "Weather Report".to_string(),
                    uri: "https://weather.example.com".to_string(),
                    domain: "weather.example.com".to_string(),
                },
            }],
            web_search_queries: vec!["tokyo weather".to_string()],
        }),
        url_context_metadata: Some(UrlContextMetadata {
            url_metadata: vec![UrlMetadataEntry {
                retrieved_url: "https://example.com".to_string(),
                url_retrieval_status: UrlRetrievalStatus::UrlRetrievalStatusSuccess,
            }],
        }),
    };

    // Serialize to JSON
    let json_str = serde_json::to_string(&response).expect("Serialization should succeed");

    // Verify key data is present in serialized JSON
    assert!(
        json_str.contains("complex-interaction-xyz"),
        "Should contain ID"
    );
    assert!(
        json_str.contains("gemini-3-flash-preview"),
        "Should contain model"
    );
    assert!(
        json_str.contains("get_weather"),
        "Should contain function name"
    );
    assert!(
        json_str.contains("call-func-001"),
        "Should contain function call ID"
    );
    assert!(json_str.contains("Tokyo"), "Should contain city");
    assert!(
        json_str.contains("thought-sig-abc123"),
        "Should contain thought signature"
    );
    assert!(json_str.contains("print(2 + 2)"), "Should contain code");
    assert!(
        json_str.contains("weather.example.com"),
        "Should contain grounding URI"
    );
    assert!(
        json_str.contains("previous-interaction-abc"),
        "Should contain previous ID"
    );

    // Deserialize back
    let deserialized: InteractionResponse =
        serde_json::from_str(&json_str).expect("Deserialization should succeed");

    // Verify top-level fields
    assert_eq!(deserialized.id.as_deref(), Some("complex-interaction-xyz"));
    assert_eq!(
        deserialized.model,
        Some("gemini-3-flash-preview".to_string())
    );
    assert_eq!(deserialized.status, InteractionStatus::Completed);
    assert_eq!(
        deserialized.previous_interaction_id,
        Some("previous-interaction-abc".to_string())
    );

    // Verify input
    assert_eq!(deserialized.input.len(), 2);

    // Verify outputs have correct count
    assert_eq!(deserialized.outputs.len(), 11);

    // Verify function calls are accessible
    let function_calls = deserialized.function_calls();
    assert_eq!(function_calls.len(), 1);
    assert_eq!(function_calls[0].name, "get_weather");
    assert_eq!(function_calls[0].id, Some("call-func-001"));
    assert_eq!(function_calls[0].args["city"], "Tokyo");

    // Verify code execution results
    let code_results = deserialized.code_execution_results();
    assert_eq!(code_results.len(), 1);
    assert_eq!(code_results[0].outcome, CodeExecutionOutcome::Ok);
    assert_eq!(code_results[0].output, "4");

    // Verify URL context results
    let url_results = deserialized.url_context_results();
    assert_eq!(url_results.len(), 1);
    assert_eq!(url_results[0].url, "https://example.com");

    // Verify usage metadata
    let usage = deserialized.usage.expect("Should have usage");
    assert_eq!(usage.total_input_tokens, Some(150));
    assert_eq!(usage.total_output_tokens, Some(200));
    assert_eq!(usage.total_tokens, Some(350));
    assert_eq!(usage.total_cached_tokens, Some(50));
    assert_eq!(usage.total_reasoning_tokens, Some(30));
    assert_eq!(usage.total_tool_use_tokens, Some(20));

    // Verify grounding metadata
    let grounding = deserialized
        .grounding_metadata
        .expect("Should have grounding");
    assert_eq!(grounding.grounding_chunks.len(), 1);
    assert_eq!(grounding.grounding_chunks[0].web.title, "Weather Report");
    assert_eq!(
        grounding.grounding_chunks[0].web.uri,
        "https://weather.example.com"
    );
    assert_eq!(
        grounding.web_search_queries,
        vec!["tokyo weather".to_string()]
    );

    // Verify URL context metadata
    let url_meta = deserialized
        .url_context_metadata
        .expect("Should have URL metadata");
    assert_eq!(url_meta.url_metadata.len(), 1);
    assert_eq!(
        url_meta.url_metadata[0].retrieved_url,
        "https://example.com"
    );
    assert_eq!(
        url_meta.url_metadata[0].url_retrieval_status,
        UrlRetrievalStatus::UrlRetrievalStatusSuccess
    );

    // Verify tools
    let tools = deserialized.tools.expect("Should have tools");
    assert_eq!(tools.len(), 2);
    assert!(matches!(tools[0], crate::Tool::GoogleSearch));
    assert!(matches!(tools[1], crate::Tool::CodeExecution));
}

// --- InteractionStatus Unknown Variant Tests ---

#[test]
fn test_interaction_status_unknown_deserialize() {
    // Simulate a new API status that this library doesn't know about
    let json = r#""future_pending_state""#;
    let status: InteractionStatus = serde_json::from_str(json).expect("Should deserialize");

    assert!(status.is_unknown());
    assert_eq!(status.unknown_status_type(), Some("future_pending_state"));
    assert!(status.unknown_data().is_some());
}

#[test]
fn test_interaction_status_unknown_roundtrip() {
    // Deserialize unknown status
    let json = r#""new_background_processing""#;
    let status: InteractionStatus = serde_json::from_str(json).expect("Should deserialize");

    assert!(status.is_unknown());

    // Serialize back
    let reserialized = serde_json::to_string(&status).expect("Should serialize");
    assert_eq!(reserialized, r#""new_background_processing""#);

    // Deserialize again to verify roundtrip
    let status2: InteractionStatus =
        serde_json::from_str(&reserialized).expect("Should deserialize again");
    assert!(status2.is_unknown());
    assert_eq!(
        status2.unknown_status_type(),
        Some("new_background_processing")
    );
}

#[test]
fn test_interaction_status_known_types_not_unknown() {
    // Verify known types don't trigger Unknown
    let completed: InteractionStatus =
        serde_json::from_str(r#""completed""#).expect("Should deserialize");
    assert!(!completed.is_unknown());
    assert_eq!(completed.unknown_status_type(), None);
    assert_eq!(completed.unknown_data(), None);

    let in_progress: InteractionStatus =
        serde_json::from_str(r#""in_progress""#).expect("Should deserialize");
    assert!(!in_progress.is_unknown());

    let failed: InteractionStatus =
        serde_json::from_str(r#""failed""#).expect("Should deserialize");
    assert!(!failed.is_unknown());

    let requires_action: InteractionStatus =
        serde_json::from_str(r#""requires_action""#).expect("Should deserialize");
    assert!(!requires_action.is_unknown());
}

#[test]
fn test_interaction_status_non_string_handled() {
    // Edge case: API returns non-string (shouldn't happen but code handles it)
    let json = r#"42"#;
    let status: InteractionStatus = serde_json::from_str(json).expect("Should deserialize");

    assert!(status.is_unknown());
    // The status_type should indicate it was non-string
    let status_type = status
        .unknown_status_type()
        .expect("Should have status_type");
    assert!(status_type.contains("non-string"));

    // The data should preserve the original value
    let data = status.unknown_data().expect("Should have data");
    assert_eq!(*data, serde_json::json!(42));
}

// --- Optional ID Tests (Issue #210) ---

#[test]
fn test_interaction_response_deserialize_without_id() {
    // When store=false, the API response does not include an id field.
    // This test verifies that we can deserialize such responses correctly.
    let json = r#"{
        "model": "gemini-3-flash-preview",
        "input": [{"type": "text", "text": "Hello"}],
        "outputs": [{"type": "text", "text": "Hi there!"}],
        "status": "completed"
    }"#;

    let response: InteractionResponse =
        serde_json::from_str(json).expect("Deserialization should succeed without id");

    assert!(response.id.is_none(), "ID should be None when not present");
    assert_eq!(response.model, Some("gemini-3-flash-preview".to_string()));
    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(!response.outputs.is_empty());
}

#[test]
fn test_interaction_response_deserialize_with_id() {
    // When store=true (or by default), the API response includes an id field.
    // This test verifies that we can still deserialize such responses correctly.
    let json = r#"{
        "id": "interaction-abc123",
        "model": "gemini-3-flash-preview",
        "input": [{"type": "text", "text": "Hello"}],
        "outputs": [{"type": "text", "text": "Hi there!"}],
        "status": "completed"
    }"#;

    let response: InteractionResponse =
        serde_json::from_str(json).expect("Deserialization should succeed with id");

    assert_eq!(
        response.id.as_deref(),
        Some("interaction-abc123"),
        "ID should be present when included"
    );
    assert_eq!(response.model, Some("gemini-3-flash-preview".to_string()));
    assert_eq!(response.status, InteractionStatus::Completed);
}

#[test]
fn test_interaction_response_serialize_without_id() {
    // When id is None, it should not be serialized into the JSON output.
    // This uses skip_serializing_if to avoid "id": null in the output.
    let response = InteractionResponse {
        id: None,
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![InteractionContent::Text {
            text: Some("Hello".to_string()),
            annotations: None,
        }],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        grounding_metadata: None,
        url_context_metadata: None,
        previous_interaction_id: None,
    };

    let json = serde_json::to_string(&response).expect("Serialization should succeed");

    assert!(
        !json.contains(r#""id""#),
        "JSON should not contain id field when None: {}",
        json
    );
}

#[test]
fn test_interaction_response_roundtrip_without_id() {
    // Verify roundtrip serialization works correctly when id is None.
    let original = InteractionResponse {
        id: None,
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![InteractionContent::Text {
            text: Some("Test response".to_string()),
            annotations: None,
        }],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        grounding_metadata: None,
        url_context_metadata: None,
        previous_interaction_id: None,
    };

    let json = serde_json::to_string(&original).expect("Serialization should succeed");
    let restored: InteractionResponse =
        serde_json::from_str(&json).expect("Deserialization should succeed");

    assert_eq!(restored.id, original.id);
    assert_eq!(restored.model, original.model);
    assert_eq!(restored.status, original.status);
}

// --- OwnedFunctionCallInfo Tests ---

#[test]
fn test_function_call_info_to_owned() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![InteractionContent::FunctionCall {
            id: Some("call_123".to_string()),
            name: "get_weather".to_string(),
            args: serde_json::json!({"city": "Tokyo", "units": "celsius"}),
            thought_signature: Some("sig_abc".to_string()),
        }],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    let calls = response.function_calls();
    assert_eq!(calls.len(), 1);

    // Convert to owned
    let owned = calls[0].to_owned();

    // Verify all fields are correctly converted
    assert_eq!(owned.id, Some("call_123".to_string()));
    assert_eq!(owned.name, "get_weather");
    assert_eq!(owned.args["city"], "Tokyo");
    assert_eq!(owned.args["units"], "celsius");
    assert_eq!(owned.thought_signature, Some("sig_abc".to_string()));
}

#[test]
fn test_function_call_info_to_owned_none_fields() {
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![InteractionContent::FunctionCall {
            id: None,
            name: "simple_function".to_string(),
            args: serde_json::json!({}),
            thought_signature: None,
        }],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    let calls = response.function_calls();
    let owned = calls[0].to_owned();

    assert_eq!(owned.id, None);
    assert_eq!(owned.name, "simple_function");
    assert_eq!(owned.args, serde_json::json!({}));
    assert_eq!(owned.thought_signature, None);
}

#[test]
fn test_owned_function_call_info_outlives_response() {
    // Demonstrate the main use case: owned call can outlive the response
    let owned_calls: Vec<OwnedFunctionCallInfo> = {
        let response = InteractionResponse {
            id: Some("test_id".to_string()),
            model: Some("gemini-3-flash-preview".to_string()),
            agent: None,
            input: vec![],
            outputs: vec![
                InteractionContent::FunctionCall {
                    id: Some("call_1".to_string()),
                    name: "func_a".to_string(),
                    args: serde_json::json!({"x": 1}),
                    thought_signature: None,
                },
                InteractionContent::FunctionCall {
                    id: Some("call_2".to_string()),
                    name: "func_b".to_string(),
                    args: serde_json::json!({"y": 2}),
                    thought_signature: None,
                },
            ],
            status: InteractionStatus::RequiresAction,
            usage: None,
            tools: None,
            previous_interaction_id: None,
            grounding_metadata: None,
            url_context_metadata: None,
        };

        // Convert to owned before response goes out of scope
        response
            .function_calls()
            .into_iter()
            .map(|call| call.to_owned())
            .collect()
    }; // response is dropped here

    // owned_calls is still valid and usable
    assert_eq!(owned_calls.len(), 2);
    assert_eq!(owned_calls[0].name, "func_a");
    assert_eq!(owned_calls[0].args["x"], 1);
    assert_eq!(owned_calls[1].name, "func_b");
    assert_eq!(owned_calls[1].args["y"], 2);
}

#[test]
fn test_owned_function_call_info_serialization_roundtrip() {
    let owned = OwnedFunctionCallInfo {
        id: Some("call_xyz".to_string()),
        name: "my_function".to_string(),
        args: serde_json::json!({"key": "value", "number": 42}),
        thought_signature: Some("thought_sig".to_string()),
    };

    // Serialize to JSON
    let json = serde_json::to_string(&owned).expect("Serialization should succeed");

    // Verify JSON contains expected data
    assert!(json.contains("call_xyz"));
    assert!(json.contains("my_function"));
    assert!(json.contains("thought_sig"));

    // Deserialize back
    let restored: OwnedFunctionCallInfo =
        serde_json::from_str(&json).expect("Deserialization should succeed");

    assert_eq!(restored.id, owned.id);
    assert_eq!(restored.name, owned.name);
    assert_eq!(restored.args, owned.args);
    assert_eq!(restored.thought_signature, owned.thought_signature);
}

#[test]
fn test_owned_function_call_info_clone() {
    let owned = OwnedFunctionCallInfo {
        id: Some("call_id".to_string()),
        name: "cloneable".to_string(),
        args: serde_json::json!({"data": [1, 2, 3]}),
        thought_signature: None,
    };

    let cloned = owned.clone();

    assert_eq!(cloned.id, owned.id);
    assert_eq!(cloned.name, owned.name);
    assert_eq!(cloned.args, owned.args);
    assert_eq!(cloned.thought_signature, owned.thought_signature);
}

#[test]
fn test_owned_function_call_info_equality() {
    let owned1 = OwnedFunctionCallInfo {
        id: Some("same_id".to_string()),
        name: "same_name".to_string(),
        args: serde_json::json!({"same": true}),
        thought_signature: Some("same_sig".to_string()),
    };

    let owned2 = OwnedFunctionCallInfo {
        id: Some("same_id".to_string()),
        name: "same_name".to_string(),
        args: serde_json::json!({"same": true}),
        thought_signature: Some("same_sig".to_string()),
    };

    let different = OwnedFunctionCallInfo {
        id: Some("different_id".to_string()),
        name: "same_name".to_string(),
        args: serde_json::json!({"same": true}),
        thought_signature: Some("same_sig".to_string()),
    };

    assert_eq!(owned1, owned2);
    assert_ne!(owned1, different);
}

// --- Annotation Helper Tests ---

#[test]
fn test_interaction_response_has_annotations() {
    // Response with annotations
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![InteractionContent::Text {
            text: Some("According to the source, climate change is accelerating.".to_string()),
            annotations: Some(vec![Annotation {
                start_index: 19,
                end_index: 25,
                source: Some("https://climate.gov".to_string()),
            }]),
        }],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert!(response.has_annotations());
}

#[test]
fn test_interaction_response_no_annotations() {
    // Response without annotations
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![InteractionContent::Text {
            text: Some("Plain text without citations.".to_string()),
            annotations: None,
        }],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert!(!response.has_annotations());
}

#[test]
fn test_interaction_response_empty_annotations_not_counted() {
    // Response with empty annotations array (should not count as having annotations)
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![InteractionContent::Text {
            text: Some("Text with empty annotations.".to_string()),
            annotations: Some(vec![]), // Empty array
        }],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    assert!(!response.has_annotations());
}

#[test]
fn test_interaction_response_all_annotations() {
    // Response with multiple text outputs, each with annotations
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::Text {
                text: Some("First claim from source A.".to_string()),
                annotations: Some(vec![Annotation {
                    start_index: 0,
                    end_index: 11,
                    source: Some("https://source-a.com".to_string()),
                }]),
            },
            InteractionContent::Thought {
                text: Some("Thinking about sources...".to_string()),
            },
            InteractionContent::Text {
                text: Some("Second and third claims.".to_string()),
                annotations: Some(vec![
                    Annotation {
                        start_index: 0,
                        end_index: 6,
                        source: Some("https://source-b.com".to_string()),
                    },
                    Annotation {
                        start_index: 11,
                        end_index: 16,
                        source: Some("https://source-c.com".to_string()),
                    },
                ]),
            },
            InteractionContent::Text {
                text: Some("Text without annotations.".to_string()),
                annotations: None,
            },
        ],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    let annotations = response.all_annotations();

    // Should collect all 3 annotations from the two Text outputs with annotations
    assert_eq!(annotations.len(), 3);
    assert_eq!(
        annotations[0].source.as_deref(),
        Some("https://source-a.com")
    );
    assert_eq!(
        annotations[1].source.as_deref(),
        Some("https://source-b.com")
    );
    assert_eq!(
        annotations[2].source.as_deref(),
        Some("https://source-c.com")
    );
}

#[test]
fn test_interaction_response_all_annotations_empty() {
    // Response with no annotations at all
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::Text {
                text: Some("No annotations here.".to_string()),
                annotations: None,
            },
            InteractionContent::FunctionCall {
                id: Some("call_1".to_string()),
                name: "test".to_string(),
                args: serde_json::json!({}),
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

    let annotations = response.all_annotations();
    assert!(annotations.is_empty());
}

#[test]
fn test_interaction_response_all_annotations_skips_non_text() {
    // Verify all_annotations only looks at Text content, not other types
    let response = InteractionResponse {
        id: Some("test_id".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![
            InteractionContent::Image {
                data: Some("base64".to_string()),
                uri: None,
                mime_type: Some("image/png".to_string()),
            },
            InteractionContent::CodeExecutionResult {
                call_id: "call_1".to_string(),
                outcome: CodeExecutionOutcome::Ok,
                output: "result".to_string(),
            },
            InteractionContent::Text {
                text: Some("Only text has annotations.".to_string()),
                annotations: Some(vec![Annotation {
                    start_index: 0,
                    end_index: 4,
                    source: Some("https://example.com".to_string()),
                }]),
            },
        ],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        previous_interaction_id: None,
        grounding_metadata: None,
        url_context_metadata: None,
    };

    let annotations = response.all_annotations();

    // Should only find the one annotation from the Text content
    assert_eq!(annotations.len(), 1);
    assert_eq!(
        annotations[0].source.as_deref(),
        Some("https://example.com")
    );
}
