//! Streaming multi-turn conversation tests
//!
//! Tests for streaming responses in multi-turn conversations, including
//! basic streaming and streaming with function calling.
//!
//! These tests require the GEMINI_API_KEY environment variable to be set.
//!
//! # Running Tests
//!
//! ```bash
//! cargo test --test streaming_multiturn_tests -- --include-ignored --nocapture
//! ```

mod common;

use common::{consume_stream, get_client, stateful_builder, validate_response_semantically};
use rust_genai::{FunctionDeclaration, InteractionStatus, function_result_content};
use serde_json::json;

// =============================================================================
// Multi-turn: Streaming
// =============================================================================

/// Test that streaming works correctly in a multi-turn conversation.
/// Turn 1 establishes context, Turn 2 uses streaming to verify recall.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_streaming_multi_turn_basic() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Turn 1: Establish a fact (non-streaming)
    let response1 = retry_request!([client] => {
        stateful_builder(&client)
            .with_text(
                "My favorite programming language is Python. Please acknowledge this.",
            )
            .with_store(true)
            .create()
            .await
    })
    .expect("Turn 1 failed");

    println!("Turn 1 completed: {:?}", response1.id);
    assert_eq!(response1.status, InteractionStatus::Completed);

    // Turn 2: Stream a question that requires context from Turn 1
    let stream = stateful_builder(&client)
        .with_previous_interaction(response1.id.as_ref().expect("id should exist"))
        .with_text("What is my favorite programming language? Answer in one word.")
        .with_store(true)
        .create_stream();

    let result = consume_stream(stream).await;

    println!("\nDeltas received: {}", result.delta_count);
    println!("Collected text: {}", result.collected_text);

    // Verify streaming worked
    assert!(result.has_output(), "Should receive streaming chunks");

    // Verify context was maintained - response should mention Python
    let text_lower = result.collected_text.to_lowercase();
    assert!(
        text_lower.contains("python"),
        "Streaming response should recall the fact from Turn 1. Got: {}",
        result.collected_text
    );

    // Verify final response if received
    if let Some(response) = result.final_response {
        assert_eq!(response.status, InteractionStatus::Completed);
    }
}

/// Test streaming in a multi-turn conversation with function calling.
/// Turn 1: Trigger function call
/// Turn 2: Provide function result
/// Turn 3: Stream a follow-up question
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_streaming_multi_turn_function_calling() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let get_weather = FunctionDeclaration::builder("get_weather")
        .description("Get the current weather for a city")
        .parameter(
            "city",
            json!({"type": "string", "description": "The city name"}),
        )
        .required(vec!["city".to_string()])
        .build();

    // Turn 1: Trigger function call
    let response1 = retry_request!([client, get_weather] => {
        stateful_builder(&client)
            .with_text("What's the weather in Paris?")
            .with_function(get_weather)
            .with_store(true)
            .create()
            .await
    })
    .expect("Turn 1 failed");

    println!("Turn 1 status: {:?}", response1.status);

    let calls = response1.function_calls();
    if calls.is_empty() {
        println!("Model chose not to call function - skipping rest of test");
        return;
    }

    let call = &calls[0];
    println!("Function call: {} with args: {:?}", call.name, call.args);

    // Turn 2: Provide function result
    let function_result = function_result_content(
        "get_weather",
        call.id.expect("Function call should have ID").to_string(),
        json!({"temperature": "18°C", "conditions": "rainy", "humidity": "85%"}),
    );

    let prev_id = response1.id.clone().expect("id should exist");
    let response2 = retry_request!([client, prev_id, function_result, get_weather] => {
        stateful_builder(&client)
            .with_previous_interaction(&prev_id)
            .with_content(vec![function_result])
            .with_function(get_weather)
            .with_store(true)
            .create()
            .await
    })
    .expect("Turn 2 failed");

    println!("Turn 2 status: {:?}", response2.status);
    if response2.has_text() {
        println!("Turn 2 text: {}", response2.text().unwrap());
    }

    // Turn 3: Stream a follow-up question about the weather context
    let stream = stateful_builder(&client)
        .with_previous_interaction(response2.id.as_ref().expect("id should exist"))
        .with_text("Should I bring an umbrella? Answer briefly.")
        .with_function(get_weather)
        .with_store(true)
        .create_stream();

    let result = consume_stream(stream).await;

    println!("\nDeltas received: {}", result.delta_count);
    println!("Collected text: {}", result.collected_text);

    // Verify streaming worked
    assert!(result.has_output(), "Should receive streaming chunks");

    // Verify context was maintained using semantic validation
    // The model should reference the weather conditions from Turn 1 (rainy, 18°C)
    let is_valid = validate_response_semantically(
        &client,
        "Turn 1 established weather in Tokyo: rainy, 18°C, high humidity. User asked 'Should I bring an umbrella?' in Turn 2.",
        &result.collected_text,
        "Does this response address whether to bring an umbrella based on the rainy weather?",
    )
    .await
    .expect("Semantic validation failed");

    assert!(
        is_valid,
        "Streaming response should reference weather context. Got: {}",
        result.collected_text
    );

    // Verify final response if received
    if let Some(response) = result.final_response {
        assert_eq!(response.status, InteractionStatus::Completed);
    }
}
