//! Property-based tests for serialization roundtrips using proptest.
//!
//! These tests verify that `deserialize(serialize(x)) == x` for all key types,
//! catching edge cases that hand-written tests might miss.

use proptest::prelude::*;

use super::content::{CodeExecutionLanguage, CodeExecutionOutcome, InteractionContent};
use super::metadata::{
    GroundingChunk, GroundingMetadata, UrlContextMetadata, UrlMetadataEntry, UrlRetrievalStatus,
    WebSource,
};
use super::response::{InteractionResponse, InteractionStatus, UsageMetadata};
use super::streaming::StreamChunk;
use crate::models::shared::{FunctionParameters, Tool};

// =============================================================================
// Strategy Generators for Arbitrary Types
// =============================================================================

/// Strategy for generating arbitrary serde_json::Value for function args/results.
/// Limited in depth to avoid overly complex nested structures.
fn arb_json_value() -> impl Strategy<Value = serde_json::Value> {
    // Simple JSON values for function args/results
    prop_oneof![
        Just(serde_json::Value::Null),
        any::<bool>().prop_map(serde_json::Value::Bool),
        any::<i64>().prop_map(|n| serde_json::Value::Number(n.into())),
        ".*".prop_map(serde_json::Value::String),
        // Simple arrays
        prop::collection::vec(
            prop_oneof![
                Just(serde_json::Value::Null),
                any::<bool>().prop_map(serde_json::Value::Bool),
                any::<i64>().prop_map(|n| serde_json::Value::Number(n.into())),
                ".*".prop_map(serde_json::Value::String),
            ],
            0..5
        )
        .prop_map(serde_json::Value::Array),
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

/// Strategy for generating valid identifiers (function names, IDs, etc.)
fn arb_identifier() -> impl Strategy<Value = String> {
    "[a-zA-Z_][a-zA-Z0-9_]{0,30}"
}

/// Strategy for generating text strings up to 500 characters (may be empty).
fn arb_text() -> impl Strategy<Value = String> {
    ".{0,500}"
}

// =============================================================================
// InteractionStatus Strategies
// =============================================================================

fn arb_interaction_status() -> impl Strategy<Value = InteractionStatus> {
    prop_oneof![
        Just(InteractionStatus::Completed),
        Just(InteractionStatus::InProgress),
        Just(InteractionStatus::RequiresAction),
        Just(InteractionStatus::Failed),
        Just(InteractionStatus::Cancelled),
        // Unknown variant with preserved data
        arb_identifier().prop_map(|status_type| InteractionStatus::Unknown {
            status_type: status_type.clone(),
            data: serde_json::Value::String(status_type),
        }),
    ]
}

// =============================================================================
// CodeExecutionLanguage and CodeExecutionOutcome Strategies
// =============================================================================

fn arb_code_execution_language() -> impl Strategy<Value = CodeExecutionLanguage> {
    prop_oneof![
        Just(CodeExecutionLanguage::Python),
        Just(CodeExecutionLanguage::Unspecified),
    ]
}

fn arb_code_execution_outcome() -> impl Strategy<Value = CodeExecutionOutcome> {
    prop_oneof![
        Just(CodeExecutionOutcome::Ok),
        Just(CodeExecutionOutcome::Failed),
        Just(CodeExecutionOutcome::DeadlineExceeded),
        Just(CodeExecutionOutcome::Unspecified),
    ]
}

// =============================================================================
// UsageMetadata Strategy
// =============================================================================

fn arb_usage_metadata() -> impl Strategy<Value = UsageMetadata> {
    (
        proptest::option::of(any::<i32>()),
        proptest::option::of(any::<i32>()),
        proptest::option::of(any::<i32>()),
        proptest::option::of(any::<i32>()),
        proptest::option::of(any::<i32>()),
        proptest::option::of(any::<i32>()),
    )
        .prop_map(
            |(
                total_input_tokens,
                total_output_tokens,
                total_tokens,
                total_cached_tokens,
                total_reasoning_tokens,
                total_tool_use_tokens,
            )| {
                UsageMetadata {
                    total_input_tokens,
                    total_output_tokens,
                    total_tokens,
                    total_cached_tokens,
                    total_reasoning_tokens,
                    total_tool_use_tokens,
                }
            },
        )
}

// =============================================================================
// InteractionContent Strategy
// =============================================================================

fn arb_interaction_content() -> impl Strategy<Value = InteractionContent> {
    prop_oneof![
        // Text content
        proptest::option::of(arb_text()).prop_map(|text| InteractionContent::Text { text }),
        // Thought content
        proptest::option::of(arb_text()).prop_map(|text| InteractionContent::Thought { text }),
        // ThoughtSignature content
        arb_text().prop_map(|signature| InteractionContent::ThoughtSignature { signature }),
        // Image content
        (
            proptest::option::of(arb_text()),
            proptest::option::of(arb_text()),
            proptest::option::of(arb_text())
        )
            .prop_map(|(data, uri, mime_type)| InteractionContent::Image {
                data,
                uri,
                mime_type
            }),
        // Audio content
        (
            proptest::option::of(arb_text()),
            proptest::option::of(arb_text()),
            proptest::option::of(arb_text())
        )
            .prop_map(|(data, uri, mime_type)| InteractionContent::Audio {
                data,
                uri,
                mime_type
            }),
        // Video content
        (
            proptest::option::of(arb_text()),
            proptest::option::of(arb_text()),
            proptest::option::of(arb_text())
        )
            .prop_map(|(data, uri, mime_type)| InteractionContent::Video {
                data,
                uri,
                mime_type
            }),
        // Document content
        (
            proptest::option::of(arb_text()),
            proptest::option::of(arb_text()),
            proptest::option::of(arb_text())
        )
            .prop_map(|(data, uri, mime_type)| InteractionContent::Document {
                data,
                uri,
                mime_type
            }),
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
        // FunctionResult content
        (arb_identifier(), arb_identifier(), arb_json_value()).prop_map(
            |(name, call_id, result)| InteractionContent::FunctionResult {
                name,
                call_id,
                result
            }
        ),
        // CodeExecutionCall content
        (arb_identifier(), arb_code_execution_language(), arb_text()).prop_map(
            |(id, language, code)| InteractionContent::CodeExecutionCall { id, language, code }
        ),
        // CodeExecutionResult content
        (arb_identifier(), arb_code_execution_outcome(), arb_text()).prop_map(
            |(call_id, outcome, output)| InteractionContent::CodeExecutionResult {
                call_id,
                outcome,
                output
            }
        ),
        // GoogleSearchCall content
        arb_text().prop_map(|query| InteractionContent::GoogleSearchCall { query }),
        // GoogleSearchResult content
        arb_json_value().prop_map(|results| InteractionContent::GoogleSearchResult { results }),
        // UrlContextCall content
        arb_text().prop_map(|url| InteractionContent::UrlContextCall { url }),
        // UrlContextResult content
        (arb_text(), proptest::option::of(arb_text()))
            .prop_map(|(url, content)| InteractionContent::UrlContextResult { url, content }),
        // Unknown content (for forward compatibility testing)
        (arb_identifier(), arb_json_value()).prop_map(|(content_type, data)| {
            InteractionContent::Unknown { content_type, data }
        }),
    ]
}

// =============================================================================
// Metadata Strategies
// =============================================================================

fn arb_web_source() -> impl Strategy<Value = WebSource> {
    (arb_text(), arb_text(), arb_text()).prop_map(|(uri, title, domain)| WebSource {
        uri,
        title,
        domain,
    })
}

fn arb_grounding_chunk() -> impl Strategy<Value = GroundingChunk> {
    arb_web_source().prop_map(|web| GroundingChunk { web })
}

fn arb_grounding_metadata() -> impl Strategy<Value = GroundingMetadata> {
    (
        prop::collection::vec(arb_text(), 0..5),
        prop::collection::vec(arb_grounding_chunk(), 0..5),
    )
        .prop_map(|(web_search_queries, grounding_chunks)| GroundingMetadata {
            web_search_queries,
            grounding_chunks,
        })
}

fn arb_url_retrieval_status() -> impl Strategy<Value = UrlRetrievalStatus> {
    prop_oneof![
        Just(UrlRetrievalStatus::UrlRetrievalStatusUnspecified),
        Just(UrlRetrievalStatus::UrlRetrievalStatusSuccess),
        Just(UrlRetrievalStatus::UrlRetrievalStatusUnsafe),
        Just(UrlRetrievalStatus::UrlRetrievalStatusError),
        Just(UrlRetrievalStatus::Unknown),
    ]
}

fn arb_url_metadata_entry() -> impl Strategy<Value = UrlMetadataEntry> {
    (arb_text(), arb_url_retrieval_status()).prop_map(|(retrieved_url, url_retrieval_status)| {
        UrlMetadataEntry {
            retrieved_url,
            url_retrieval_status,
        }
    })
}

fn arb_url_context_metadata() -> impl Strategy<Value = UrlContextMetadata> {
    prop::collection::vec(arb_url_metadata_entry(), 0..5)
        .prop_map(|url_metadata| UrlContextMetadata { url_metadata })
}

// =============================================================================
// Tool Strategy
// =============================================================================

fn arb_function_parameters() -> impl Strategy<Value = FunctionParameters> {
    (
        Just("object".to_string()),
        arb_json_value(),
        prop::collection::vec(arb_identifier(), 0..3),
    )
        .prop_map(|(type_, properties, required)| {
            FunctionParameters::new(type_, properties, required)
        })
}

fn arb_tool() -> impl Strategy<Value = Tool> {
    prop_oneof![
        // Function tool
        (arb_identifier(), arb_text(), arb_function_parameters()).prop_map(
            |(name, description, parameters)| Tool::Function {
                name,
                description,
                parameters
            }
        ),
        // Built-in tools
        Just(Tool::GoogleSearch),
        Just(Tool::CodeExecution),
        Just(Tool::UrlContext),
        // MCP Server
        (arb_identifier(), arb_text()).prop_map(|(name, url)| Tool::McpServer { name, url }),
        // Unknown tool
        (arb_identifier(), arb_json_value())
            .prop_map(|(tool_type, data)| Tool::Unknown { tool_type, data }),
    ]
}

// =============================================================================
// InteractionResponse Strategy
// =============================================================================

fn arb_interaction_response() -> impl Strategy<Value = InteractionResponse> {
    (
        arb_identifier(),                                              // id
        proptest::option::of(arb_identifier()),                        // model
        proptest::option::of(arb_identifier()),                        // agent
        prop::collection::vec(arb_interaction_content(), 0..3),        // input
        prop::collection::vec(arb_interaction_content(), 0..5),        // outputs
        arb_interaction_status(),                                      // status
        proptest::option::of(arb_usage_metadata()),                    // usage
        proptest::option::of(prop::collection::vec(arb_tool(), 0..3)), // tools
        proptest::option::of(arb_grounding_metadata()),                // grounding_metadata
        proptest::option::of(arb_url_context_metadata()),              // url_context_metadata
        proptest::option::of(arb_identifier()),                        // previous_interaction_id
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
                tools,
                grounding_metadata,
                url_context_metadata,
                previous_interaction_id,
            )| {
                InteractionResponse {
                    id,
                    model,
                    agent,
                    input,
                    outputs,
                    status,
                    usage,
                    tools,
                    grounding_metadata,
                    url_context_metadata,
                    previous_interaction_id,
                }
            },
        )
}

// =============================================================================
// StreamChunk Strategy
// =============================================================================

fn arb_stream_chunk() -> impl Strategy<Value = StreamChunk> {
    prop_oneof![
        // Delta variant
        arb_interaction_content().prop_map(StreamChunk::Delta),
        // Complete variant
        arb_interaction_response().prop_map(StreamChunk::Complete),
        // Unknown variant for forward compatibility
        (arb_identifier(), arb_json_value())
            .prop_map(|(chunk_type, data)| StreamChunk::Unknown { chunk_type, data }),
    ]
}

// =============================================================================
// Property Tests
// =============================================================================

proptest! {
    /// Test that UsageMetadata roundtrips correctly through JSON.
    #[test]
    fn usage_metadata_roundtrip(usage in arb_usage_metadata()) {
        let json = serde_json::to_string(&usage).expect("Serialization should succeed");
        let restored: UsageMetadata = serde_json::from_str(&json).expect("Deserialization should succeed");
        prop_assert_eq!(usage, restored);
    }

    /// Test that InteractionStatus roundtrips correctly through JSON.
    #[test]
    fn interaction_status_roundtrip(status in arb_interaction_status()) {
        let json = serde_json::to_string(&status).expect("Serialization should succeed");
        let restored: InteractionStatus = serde_json::from_str(&json).expect("Deserialization should succeed");
        prop_assert_eq!(status, restored);
    }

    /// Test that CodeExecutionLanguage roundtrips correctly through JSON.
    #[test]
    fn code_execution_language_roundtrip(lang in arb_code_execution_language()) {
        let json = serde_json::to_string(&lang).expect("Serialization should succeed");
        let restored: CodeExecutionLanguage = serde_json::from_str(&json).expect("Deserialization should succeed");
        prop_assert_eq!(lang, restored);
    }

    /// Test that CodeExecutionOutcome roundtrips correctly through JSON.
    #[test]
    fn code_execution_outcome_roundtrip(outcome in arb_code_execution_outcome()) {
        let json = serde_json::to_string(&outcome).expect("Serialization should succeed");
        let restored: CodeExecutionOutcome = serde_json::from_str(&json).expect("Deserialization should succeed");
        prop_assert_eq!(outcome, restored);
    }

    /// Test that UrlRetrievalStatus roundtrips correctly through JSON.
    #[test]
    fn url_retrieval_status_roundtrip(status in arb_url_retrieval_status()) {
        let json = serde_json::to_string(&status).expect("Serialization should succeed");
        let restored: UrlRetrievalStatus = serde_json::from_str(&json).expect("Deserialization should succeed");
        prop_assert_eq!(status, restored);
    }

    /// Test that WebSource roundtrips correctly through JSON.
    #[test]
    fn web_source_roundtrip(source in arb_web_source()) {
        let json = serde_json::to_string(&source).expect("Serialization should succeed");
        let restored: WebSource = serde_json::from_str(&json).expect("Deserialization should succeed");
        prop_assert_eq!(source, restored);
    }

    /// Test that GroundingMetadata roundtrips correctly through JSON.
    #[test]
    fn grounding_metadata_roundtrip(metadata in arb_grounding_metadata()) {
        let json = serde_json::to_string(&metadata).expect("Serialization should succeed");
        let restored: GroundingMetadata = serde_json::from_str(&json).expect("Deserialization should succeed");
        prop_assert_eq!(metadata, restored);
    }

    /// Test that UrlContextMetadata roundtrips correctly through JSON.
    #[test]
    fn url_context_metadata_roundtrip(metadata in arb_url_context_metadata()) {
        let json = serde_json::to_string(&metadata).expect("Serialization should succeed");
        let restored: UrlContextMetadata = serde_json::from_str(&json).expect("Deserialization should succeed");
        prop_assert_eq!(metadata, restored);
    }

    /// Test that Tool roundtrips correctly through JSON.
    #[test]
    fn tool_roundtrip(tool in arb_tool()) {
        let json = serde_json::to_string(&tool).expect("Serialization should succeed");
        let restored: Tool = serde_json::from_str(&json).expect("Deserialization should succeed");

        // Tool doesn't derive PartialEq, so we verify roundtrip by comparing JSON strings
        let restored_json = serde_json::to_string(&restored).expect("Re-serialization should succeed");
        prop_assert_eq!(json, restored_json);
    }

    /// Test that InteractionContent roundtrips correctly through JSON.
    ///
    /// Note: This uses structural comparison since InteractionContent doesn't derive PartialEq.
    #[test]
    fn interaction_content_roundtrip(content in arb_interaction_content()) {
        let json = serde_json::to_string(&content).expect("Serialization should succeed");
        let restored: InteractionContent = serde_json::from_str(&json).expect("Deserialization should succeed");

        // Compare by re-serializing since InteractionContent doesn't derive PartialEq
        let restored_json = serde_json::to_string(&restored).expect("Re-serialization should succeed");
        prop_assert_eq!(json, restored_json);
    }

    /// Test that InteractionResponse roundtrips correctly through JSON.
    ///
    /// This is the most comprehensive test, covering the full response structure.
    #[test]
    fn interaction_response_roundtrip(response in arb_interaction_response()) {
        let json = serde_json::to_string(&response).expect("Serialization should succeed");
        let restored: InteractionResponse = serde_json::from_str(&json).expect("Deserialization should succeed");

        // Compare key fields since InteractionResponse doesn't derive PartialEq for all fields
        prop_assert_eq!(&response.id, &restored.id);
        prop_assert_eq!(&response.model, &restored.model);
        prop_assert_eq!(&response.agent, &restored.agent);
        prop_assert_eq!(&response.status, &restored.status);
        prop_assert_eq!(&response.usage, &restored.usage);
        prop_assert_eq!(&response.previous_interaction_id, &restored.previous_interaction_id);
        prop_assert_eq!(response.input.len(), restored.input.len());
        prop_assert_eq!(response.outputs.len(), restored.outputs.len());

        // Verify grounding_metadata if present
        prop_assert_eq!(&response.grounding_metadata, &restored.grounding_metadata);

        // Verify url_context_metadata if present
        prop_assert_eq!(&response.url_context_metadata, &restored.url_context_metadata);

        // Verify the full JSON roundtrip is stable
        let restored_json = serde_json::to_string(&restored).expect("Re-serialization should succeed");
        prop_assert_eq!(json, restored_json);
    }

    /// Test that StreamChunk roundtrips correctly through JSON.
    #[test]
    fn stream_chunk_roundtrip(chunk in arb_stream_chunk()) {
        let json = serde_json::to_string(&chunk).expect("Serialization should succeed");
        let restored: StreamChunk = serde_json::from_str(&json).expect("Deserialization should succeed");

        // StreamChunk doesn't derive PartialEq, so we verify roundtrip by comparing JSON strings
        let restored_json = serde_json::to_string(&restored).expect("Re-serialization should succeed");
        prop_assert_eq!(json, restored_json);
    }
}

// =============================================================================
// Additional Edge Case Tests
// =============================================================================

proptest! {
    /// Test empty strings are handled correctly.
    #[test]
    fn empty_text_content_roundtrip(_unused in Just(())) {
        let content = InteractionContent::Text { text: Some(String::new()) };
        let json = serde_json::to_string(&content).expect("Serialization should succeed");
        let restored: InteractionContent = serde_json::from_str(&json).expect("Deserialization should succeed");
        let restored_json = serde_json::to_string(&restored).expect("Re-serialization should succeed");
        prop_assert_eq!(json, restored_json);
    }

    /// Test None text content is handled correctly.
    #[test]
    fn none_text_content_roundtrip(_unused in Just(())) {
        let content = InteractionContent::Text { text: None };
        let json = serde_json::to_string(&content).expect("Serialization should succeed");
        let restored: InteractionContent = serde_json::from_str(&json).expect("Deserialization should succeed");
        let restored_json = serde_json::to_string(&restored).expect("Re-serialization should succeed");
        prop_assert_eq!(json, restored_json);
    }

    /// Test special characters in strings are handled correctly.
    #[test]
    fn special_chars_in_text(text in ".*[\n\r\t\"\\\\].*") {
        let content = InteractionContent::Text { text: Some(text) };
        let json = serde_json::to_string(&content).expect("Serialization should succeed");
        let restored: InteractionContent = serde_json::from_str(&json).expect("Deserialization should succeed");
        let restored_json = serde_json::to_string(&restored).expect("Re-serialization should succeed");
        prop_assert_eq!(json, restored_json);
    }

    /// Test Unicode in strings is handled correctly.
    #[test]
    fn unicode_in_text(text in ".*[\\u{1F600}-\\u{1F64F}].*") {
        let content = InteractionContent::Text { text: Some(text) };
        let json = serde_json::to_string(&content).expect("Serialization should succeed");
        let restored: InteractionContent = serde_json::from_str(&json).expect("Deserialization should succeed");
        let restored_json = serde_json::to_string(&restored).expect("Re-serialization should succeed");
        prop_assert_eq!(json, restored_json);
    }

    /// Test large token counts don't overflow or cause issues.
    #[test]
    fn large_token_counts(
        input in any::<i32>(),
        output in any::<i32>(),
        total in any::<i32>(),
    ) {
        let usage = UsageMetadata {
            total_input_tokens: Some(input),
            total_output_tokens: Some(output),
            total_tokens: Some(total),
            ..Default::default()
        };
        let json = serde_json::to_string(&usage).expect("Serialization should succeed");
        let restored: UsageMetadata = serde_json::from_str(&json).expect("Deserialization should succeed");
        prop_assert_eq!(usage, restored);
    }
}
