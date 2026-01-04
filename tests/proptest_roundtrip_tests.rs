//! Property-based tests for serialization roundtrips using proptest.
//!
//! These tests verify that `deserialize(serialize(x)) == x` for key streaming types,
//! catching edge cases that hand-written tests might miss.
//!
//! Note: Since `AutoFunctionResult` is `#[non_exhaustive]`, we can't construct it directly
//! from integration tests. Instead, we deserialize from JSON and verify roundtrip stability.

use chrono::{DateTime, TimeZone, Utc};
use proptest::prelude::*;
use rust_genai::{AutoFunctionResult, AutoFunctionStreamChunk, FunctionExecutionResult};
use std::time::Duration;

// Re-export genai_client types for testing
use genai_client::{
    Annotation, InteractionContent, InteractionResponse, InteractionStatus, ModalityTokens,
    UsageMetadata,
};

// =============================================================================
// Strategy Generators
// =============================================================================

/// Strategy for generating arbitrary serde_json::Value for function args/results.
fn arb_json_value() -> impl Strategy<Value = serde_json::Value> {
    prop_oneof![
        Just(serde_json::Value::Null),
        any::<bool>().prop_map(serde_json::Value::Bool),
        any::<i64>().prop_map(|n| serde_json::Value::Number(n.into())),
        ".*".prop_map(serde_json::Value::String),
        // Simple objects
        prop::collection::hash_map("[a-zA-Z_][a-zA-Z0-9_]*", ".*", 0..5).prop_map(|m| {
            serde_json::Value::Object(
                m.into_iter()
                    .map(|(k, v)| (k, serde_json::Value::String(v)))
                    .collect(),
            )
        }),
    ]
}

/// Strategy for generating valid identifiers
fn arb_identifier() -> impl Strategy<Value = String> {
    "[a-zA-Z_][a-zA-Z0-9_]{0,30}"
}

/// Strategy for generating text
fn arb_text() -> impl Strategy<Value = String> {
    ".{0,500}"
}

/// Strategy for generating DateTime<Utc> values.
/// Uses second precision to ensure reliable roundtrip (avoiding nanosecond precision issues).
fn arb_datetime() -> impl Strategy<Value = DateTime<Utc>> {
    // Generate timestamps between 2020-01-01 and 2030-01-01 (reasonable range)
    (0i64..315_360_000).prop_map(|offset_secs| {
        // Base: 2020-01-01 00:00:00 UTC (timestamp 1577836800)
        Utc.timestamp_opt(1_577_836_800 + offset_secs, 0)
            .single()
            .expect("valid timestamp")
    })
}

// =============================================================================
// FunctionExecutionResult Strategies
// =============================================================================

fn arb_function_execution_result() -> impl Strategy<Value = FunctionExecutionResult> {
    (
        arb_identifier(),
        arb_identifier(),
        arb_json_value(),
        any::<u64>().prop_map(Duration::from_millis),
    )
        .prop_map(|(name, call_id, result, duration)| {
            FunctionExecutionResult::new(name, call_id, result, duration)
        })
}

// =============================================================================
// InteractionStatus Strategy (simplified for use in responses)
// =============================================================================

fn arb_interaction_status() -> impl Strategy<Value = InteractionStatus> {
    prop_oneof![
        Just(InteractionStatus::Completed),
        Just(InteractionStatus::InProgress),
        Just(InteractionStatus::RequiresAction),
        Just(InteractionStatus::Failed),
        Just(InteractionStatus::Cancelled),
    ]
}

// =============================================================================
// ModalityTokens Strategy
// =============================================================================

/// Strategy for generating a single ModalityTokens value.
fn arb_modality_tokens() -> impl Strategy<Value = ModalityTokens> {
    // Use realistic modality names that the API might return
    (
        prop_oneof![
            Just("TEXT".to_string()),
            Just("IMAGE".to_string()),
            Just("AUDIO".to_string()),
            Just("VIDEO".to_string()),
            arb_identifier(), // For forward compatibility with unknown modalities
        ],
        any::<u32>(),
    )
        .prop_map(|(modality, tokens)| ModalityTokens { modality, tokens })
}

/// Strategy for generating an optional Vec of ModalityTokens.
fn arb_modality_tokens_vec() -> impl Strategy<Value = Option<Vec<ModalityTokens>>> {
    proptest::option::of(prop::collection::vec(arb_modality_tokens(), 0..4))
}

// =============================================================================
// UsageMetadata Strategy
// =============================================================================

fn arb_usage_metadata() -> impl Strategy<Value = UsageMetadata> {
    (
        proptest::option::of(any::<u32>()),
        proptest::option::of(any::<u32>()),
        proptest::option::of(any::<u32>()),
        proptest::option::of(any::<u32>()),
        proptest::option::of(any::<u32>()),
        proptest::option::of(any::<u32>()),
        arb_modality_tokens_vec(),
        arb_modality_tokens_vec(),
        arb_modality_tokens_vec(),
        arb_modality_tokens_vec(),
    )
        .prop_map(
            |(
                total_input_tokens,
                total_output_tokens,
                total_tokens,
                total_cached_tokens,
                total_reasoning_tokens,
                total_tool_use_tokens,
                input_tokens_by_modality,
                output_tokens_by_modality,
                cached_tokens_by_modality,
                tool_use_tokens_by_modality,
            )| {
                UsageMetadata {
                    total_input_tokens,
                    total_output_tokens,
                    total_tokens,
                    total_cached_tokens,
                    total_reasoning_tokens,
                    total_tool_use_tokens,
                    input_tokens_by_modality,
                    output_tokens_by_modality,
                    cached_tokens_by_modality,
                    tool_use_tokens_by_modality,
                }
            },
        )
}

// =============================================================================
// Annotation Strategy
// =============================================================================

fn arb_annotation() -> impl Strategy<Value = Annotation> {
    (0usize..1000, 0usize..1000, proptest::option::of(".{0,100}"))
        .prop_map(|(start, len, source)| Annotation::new(start, start.saturating_add(len), source))
}

// =============================================================================
// InteractionContent Strategy (subset for streaming tests)
// =============================================================================

fn arb_interaction_content() -> impl Strategy<Value = InteractionContent> {
    prop_oneof![
        // Text content with optional annotations
        (
            proptest::option::of(arb_text()),
            proptest::option::of(proptest::collection::vec(arb_annotation(), 0..3))
        )
            .prop_map(|(text, annotations)| InteractionContent::Text { text, annotations }),
        // Thought content
        proptest::option::of(arb_text()).prop_map(|text| InteractionContent::Thought { text }),
        // FunctionCall content
        (
            proptest::option::of(arb_identifier()),
            arb_identifier(),
            arb_json_value(),
            proptest::option::of(arb_text())
        )
            .prop_map(|(id, name, args, thought_signature)| {
                InteractionContent::FunctionCall {
                    id,
                    name,
                    args,
                    thought_signature,
                }
            }),
    ]
}

// =============================================================================
// InteractionResponse Strategy (simplified for streaming tests)
// =============================================================================

fn arb_interaction_response() -> impl Strategy<Value = InteractionResponse> {
    (
        proptest::option::of(arb_identifier()),                 // id
        proptest::option::of(arb_identifier()),                 // model
        proptest::option::of(arb_identifier()),                 // agent
        prop::collection::vec(arb_interaction_content(), 0..3), // input
        prop::collection::vec(arb_interaction_content(), 0..5), // outputs
        arb_interaction_status(),                               // status
        proptest::option::of(arb_usage_metadata()),             // usage
        proptest::option::of(arb_identifier()),                 // previous_interaction_id
        proptest::option::of(arb_datetime()),                   // created
        proptest::option::of(arb_datetime()),                   // updated
    )
        .prop_map(
            |(
                id,
                model,
                agent,
                input,
                outputs,
                status,
                usage,
                previous_interaction_id,
                created,
                updated,
            )| {
                InteractionResponse {
                    id,
                    model,
                    agent,
                    input,
                    outputs,
                    status,
                    usage,
                    tools: None,
                    grounding_metadata: None,
                    url_context_metadata: None,
                    previous_interaction_id,
                    created,
                    updated,
                }
            },
        )
}

// =============================================================================
// AutoFunctionStreamChunk Strategy
// =============================================================================

fn arb_auto_function_stream_chunk() -> impl Strategy<Value = AutoFunctionStreamChunk> {
    prop_oneof![
        // Delta variant
        arb_interaction_content().prop_map(AutoFunctionStreamChunk::Delta),
        // ExecutingFunctions variant
        arb_interaction_response().prop_map(AutoFunctionStreamChunk::ExecutingFunctions),
        // FunctionResults variant
        prop::collection::vec(arb_function_execution_result(), 0..5)
            .prop_map(AutoFunctionStreamChunk::FunctionResults),
        // Complete variant
        arb_interaction_response().prop_map(AutoFunctionStreamChunk::Complete),
        // MaxLoopsReached variant
        arb_interaction_response().prop_map(AutoFunctionStreamChunk::MaxLoopsReached),
        // Unknown variant for forward compatibility
        (arb_identifier(), arb_json_value()).prop_map(|(chunk_type, data)| {
            AutoFunctionStreamChunk::Unknown { chunk_type, data }
        }),
    ]
}

// =============================================================================
// AutoFunctionResult Strategy
// =============================================================================

/// Strategy for generating AutoFunctionResult via JSON deserialization.
///
/// Since AutoFunctionResult is #[non_exhaustive], we can't construct it directly
/// from integration tests. Instead, we serialize components to JSON and deserialize.
fn arb_auto_function_result() -> impl Strategy<Value = AutoFunctionResult> {
    (
        arb_interaction_response(),
        prop::collection::vec(arb_function_execution_result(), 0..10),
        any::<bool>(),
    )
        .prop_map(|(response, executions, reached_max_loops)| {
            // Construct via JSON to work around #[non_exhaustive]
            let json = serde_json::json!({
                "response": serde_json::to_value(&response).unwrap(),
                "executions": serde_json::to_value(&executions).unwrap(),
                "reached_max_loops": reached_max_loops,
            });
            serde_json::from_value::<AutoFunctionResult>(json)
                .expect("AutoFunctionResult JSON construction should succeed")
        })
}

// =============================================================================
// Property Tests
// =============================================================================

proptest! {
    /// Test that FunctionExecutionResult roundtrips correctly through JSON.
    #[test]
    fn function_execution_result_roundtrip(result in arb_function_execution_result()) {
        let json = serde_json::to_string(&result).expect("Serialization should succeed");
        let restored: FunctionExecutionResult = serde_json::from_str(&json).expect("Deserialization should succeed");
        prop_assert_eq!(result, restored);
    }

    /// Test that AutoFunctionStreamChunk roundtrips correctly through JSON.
    ///
    /// Note: Since AutoFunctionStreamChunk contains types that don't derive PartialEq,
    /// we verify by re-serializing and comparing JSON strings.
    #[test]
    fn auto_function_stream_chunk_roundtrip(chunk in arb_auto_function_stream_chunk()) {
        let json = serde_json::to_string(&chunk).expect("Serialization should succeed");
        let restored: AutoFunctionStreamChunk = serde_json::from_str(&json).expect("Deserialization should succeed");

        // Compare by re-serializing
        let restored_json = serde_json::to_string(&restored).expect("Re-serialization should succeed");
        prop_assert_eq!(json, restored_json);
    }

    /// Test that AutoFunctionResult roundtrips correctly through JSON.
    ///
    /// Note: We compare by re-serializing since nested types don't all derive PartialEq.
    #[test]
    fn auto_function_result_roundtrip(result in arb_auto_function_result()) {
        let json = serde_json::to_string(&result).expect("Serialization should succeed");
        let restored: AutoFunctionResult = serde_json::from_str(&json).expect("Deserialization should succeed");

        // Compare key fields
        prop_assert_eq!(&result.response.id, &restored.response.id);
        prop_assert_eq!(&result.response.model, &restored.response.model);
        prop_assert_eq!(&result.response.status, &restored.response.status);
        prop_assert_eq!(result.executions.len(), restored.executions.len());
        prop_assert_eq!(result.reached_max_loops, restored.reached_max_loops);

        // Verify executions
        for (orig, rest) in result.executions.iter().zip(restored.executions.iter()) {
            prop_assert_eq!(orig, rest);
        }

        // Compare by re-serializing for full verification
        let restored_json = serde_json::to_string(&restored).expect("Re-serialization should succeed");
        prop_assert_eq!(json, restored_json);
    }

    /// Test Delta variant with various content types.
    #[test]
    fn delta_chunk_roundtrip(content in arb_interaction_content()) {
        let chunk = AutoFunctionStreamChunk::Delta(content);
        let json = serde_json::to_string(&chunk).expect("Serialization should succeed");
        let restored: AutoFunctionStreamChunk = serde_json::from_str(&json).expect("Deserialization should succeed");

        let restored_json = serde_json::to_string(&restored).expect("Re-serialization should succeed");
        prop_assert_eq!(json, restored_json);
    }

    /// Test FunctionResults variant with multiple results.
    #[test]
    fn function_results_chunk_roundtrip(
        results in prop::collection::vec(arb_function_execution_result(), 0..10)
    ) {
        let chunk = AutoFunctionStreamChunk::FunctionResults(results);
        let json = serde_json::to_string(&chunk).expect("Serialization should succeed");
        let restored: AutoFunctionStreamChunk = serde_json::from_str(&json).expect("Deserialization should succeed");

        let restored_json = serde_json::to_string(&restored).expect("Re-serialization should succeed");
        prop_assert_eq!(json, restored_json);
    }

    /// Test various duration values in FunctionExecutionResult.
    #[test]
    fn function_execution_result_duration_roundtrip(millis in any::<u64>()) {
        let result = FunctionExecutionResult::new(
            "test_function",
            "call-123",
            serde_json::json!({"status": "ok"}),
            Duration::from_millis(millis),
        );

        let json = serde_json::to_string(&result).expect("Serialization should succeed");
        let restored: FunctionExecutionResult = serde_json::from_str(&json).expect("Deserialization should succeed");

        // Duration is serialized as milliseconds, so verify the roundtrip is correct
        prop_assert_eq!(result.duration.as_millis(), restored.duration.as_millis());
    }

    /// Test AutoFunctionResult with reached_max_loops = true.
    #[test]
    fn auto_function_result_max_loops_roundtrip(response in arb_interaction_response()) {
        // Construct via JSON to work around #[non_exhaustive]
        let json = serde_json::json!({
            "response": serde_json::to_value(&response).unwrap(),
            "executions": [],
            "reached_max_loops": true,
        });
        let result: AutoFunctionResult = serde_json::from_value(json).expect("Construction should succeed");

        let json_str = serde_json::to_string(&result).expect("Serialization should succeed");
        let restored: AutoFunctionResult = serde_json::from_str(&json_str).expect("Deserialization should succeed");

        prop_assert!(restored.reached_max_loops);
    }

    /// Test AutoFunctionResult with empty executions.
    #[test]
    fn auto_function_result_empty_executions_roundtrip(response in arb_interaction_response()) {
        // Construct via JSON to work around #[non_exhaustive]
        let json = serde_json::json!({
            "response": serde_json::to_value(&response).unwrap(),
            "executions": [],
            "reached_max_loops": false,
        });
        let result: AutoFunctionResult = serde_json::from_value(json).expect("Construction should succeed");

        let json_str = serde_json::to_string(&result).expect("Serialization should succeed");
        let restored: AutoFunctionResult = serde_json::from_str(&json_str).expect("Deserialization should succeed");

        prop_assert!(restored.executions.is_empty());
        prop_assert!(!restored.reached_max_loops);
    }

    /// Test Unknown variant preservation (forward compatibility).
    #[test]
    fn unknown_chunk_preservation(chunk_type in arb_identifier(), data in arb_json_value()) {
        let chunk = AutoFunctionStreamChunk::Unknown {
            chunk_type: chunk_type.clone(),
            data: data.clone(),
        };

        let json = serde_json::to_string(&chunk).expect("Serialization should succeed");
        let restored: AutoFunctionStreamChunk = serde_json::from_str(&json).expect("Deserialization should succeed");

        // Verify the Unknown variant is preserved
        prop_assert!(restored.is_unknown());
        prop_assert_eq!(restored.unknown_chunk_type(), Some(chunk_type.as_str()));
    }
}

// =============================================================================
// Edge Case Tests
// =============================================================================

proptest! {
    /// Test that valid identifier patterns in function names roundtrip correctly.
    #[test]
    fn valid_identifier_function_name_roundtrip(name in "[a-zA-Z_][a-zA-Z0-9_]*") {
        let result = FunctionExecutionResult::new(
            name,
            "call-1",
            serde_json::json!(null),
            Duration::from_millis(1),
        );

        let json = serde_json::to_string(&result).expect("Serialization should succeed");
        let restored: FunctionExecutionResult = serde_json::from_str(&json).expect("Deserialization should succeed");
        prop_assert_eq!(result, restored);
    }

    /// Test that very large JSON values in results are handled correctly.
    #[test]
    fn large_result_value(
        count in 0usize..100,
    ) {
        // Create a moderately large nested object
        let mut obj = serde_json::Map::new();
        for i in 0..count {
            obj.insert(format!("key_{}", i), serde_json::Value::String(format!("value_{}", i)));
        }
        let result_value = serde_json::Value::Object(obj);

        let result = FunctionExecutionResult::new(
            "big_result_function",
            "call-big",
            result_value,
            Duration::from_millis(500),
        );

        let json = serde_json::to_string(&result).expect("Serialization should succeed");
        let restored: FunctionExecutionResult = serde_json::from_str(&json).expect("Deserialization should succeed");
        prop_assert_eq!(result, restored);
    }
}
