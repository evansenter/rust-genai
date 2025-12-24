//! API Canary Tests
//!
//! These tests act as early-warning "canaries" to detect when the Gemini API
//! starts returning content types that the library doesn't recognize.
//!
//! When these tests fail, it indicates:
//! 1. Google has added new content types to the API
//! 2. The library should be updated to add proper support
//!
//! The Unknown variant ensures the library doesn't break, but we want to know
//! when new types appear so we can add first-class support.

mod common;

use common::get_client;
use futures_util::StreamExt;
use rust_genai::interactions_api::text_input;

/// Helper to check a response for unknown content types and panic with details if found
fn assert_no_unknown_content(response: &rust_genai::InteractionResponse, context: &str) {
    if response.has_unknown() {
        let summary = response.content_summary();
        panic!(
            "API returned unknown content types in {context}!\n\
             Unknown types: {:?}\n\
             Full summary: {summary}\n\n\
             Action required: Add support for these content types in \
             genai-client/src/models/interactions.rs",
            summary.unknown_types
        );
    }
}

/// Canary test for basic text interaction
///
/// Tests the simplest API call pattern to detect any new content types
/// in basic text responses.
#[tokio::test]
#[ignore] // Requires GEMINI_API_KEY
async fn canary_basic_text_interaction() {
    let client = get_client().expect("GEMINI_API_KEY must be set");

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(text_input("Say 'hello' and nothing else."))
        .create()
        .await
        .expect("API call should succeed");

    assert_no_unknown_content(&response, "basic text interaction");
}

/// Canary test for streaming interaction
///
/// Tests streaming responses to detect any new delta content types.
#[tokio::test]
#[ignore] // Requires GEMINI_API_KEY
async fn canary_streaming_interaction() {
    let client = get_client().expect("GEMINI_API_KEY must be set");

    let mut stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(text_input("Count from 1 to 3."))
        .create_stream();

    let mut unknown_types_found = Vec::new();

    while let Some(result) = stream.next().await {
        let chunk = result.expect("Stream chunk should be valid");
        match chunk {
            rust_genai::StreamChunk::Delta(content) => {
                if let rust_genai::InteractionContent::Unknown { type_name, .. } = &content
                    && !unknown_types_found.contains(type_name)
                {
                    unknown_types_found.push(type_name.clone());
                }
            }
            rust_genai::StreamChunk::Complete(response) => {
                assert_no_unknown_content(&response, "streaming complete response");
            }
        }
    }

    if !unknown_types_found.is_empty() {
        panic!(
            "API returned unknown content types in streaming deltas!\n\
             Unknown types: {:?}\n\n\
             Action required: Add support for these content types in \
             genai-client/src/models/interactions.rs",
            unknown_types_found
        );
    }
}

/// Canary test for function calling interaction
///
/// Tests function calling responses to detect any new content types
/// in function call/result handling.
#[tokio::test]
#[ignore] // Requires GEMINI_API_KEY
async fn canary_function_calling_interaction() {
    use rust_genai::FunctionDeclaration;
    use serde_json::json;

    let client = get_client().expect("GEMINI_API_KEY must be set");

    let get_time = FunctionDeclaration::builder("get_current_time")
        .description("Get the current time")
        .build();

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(text_input("What time is it?"))
        .with_functions(vec![get_time])
        .create()
        .await
        .expect("API call should succeed");

    assert_no_unknown_content(&response, "function calling interaction");

    // If there's a function call, respond to it and check that response too
    if !response.function_calls().is_empty() {
        // function_calls() returns (id, name, args, thought_signature)
        let (id, name, _args, _thought_sig) = &response.function_calls()[0];

        use rust_genai::interactions_api::{
            build_interaction_input, function_call_content, function_result_content, text_content,
        };

        // Build conversation history with the function call and result
        let call_id = id.unwrap_or("call_1");
        let history = build_interaction_input(vec![
            text_content("What time is it?"),
            function_call_content(*name, json!({})),
            function_result_content(*name, call_id, json!({"time": "12:00 PM"})),
        ]);

        let followup = client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_input(history)
            .create()
            .await
            .expect("Follow-up API call should succeed");

        assert_no_unknown_content(&followup, "function calling follow-up");
    }
}

/// Canary test for code execution tool
///
/// Tests the built-in code execution tool to detect any new content types.
#[tokio::test]
#[ignore] // Requires GEMINI_API_KEY
async fn canary_code_execution_interaction() {
    use rust_genai::Tool;

    let client = get_client().expect("GEMINI_API_KEY must be set");

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(text_input("Use code execution to calculate 2 + 2"))
        .with_tools(vec![Tool::CodeExecution])
        .create()
        .await
        .expect("API call should succeed");

    assert_no_unknown_content(&response, "code execution interaction");
}

/// Canary test for multimodal interaction
///
/// Tests image input to detect any new content types in multimodal responses.
#[tokio::test]
#[ignore] // Requires GEMINI_API_KEY
async fn canary_multimodal_interaction() {
    use rust_genai::interactions_api::{build_interaction_input, image_data_content, text_content};

    let client = get_client().expect("GEMINI_API_KEY must be set");

    // Use a tiny 1x1 red PNG
    let input = build_interaction_input(vec![
        text_content("What color is this image?"),
        image_data_content(common::TINY_RED_PNG_BASE64, "image/png"),
    ]);

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(input)
        .create()
        .await
        .expect("API call should succeed");

    assert_no_unknown_content(&response, "multimodal interaction");
}

/// Canary test for thinking/reasoning models
///
/// Tests models with extended thinking to detect any new thought-related content types.
#[tokio::test]
#[ignore] // Requires GEMINI_API_KEY
async fn canary_thinking_model_interaction() {
    use rust_genai::GenerationConfig;

    let client = get_client().expect("GEMINI_API_KEY must be set");

    // Use generation config with thinking level enabled
    let config = GenerationConfig {
        thinking_level: Some("medium".to_string()),
        ..Default::default()
    };

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(text_input("What is 15 * 23?"))
        .with_generation_config(config)
        .create()
        .await
        .expect("API call should succeed");

    assert_no_unknown_content(&response, "thinking model interaction");
}
