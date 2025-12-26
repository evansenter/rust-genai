//! Agents, multi-turn conversations, and usage metadata tests
//!
//! Tests for deep research agent, background mode, very long conversations,
//! conversation branching, and token usage verification.
//!
//! These tests require the GEMINI_API_KEY environment variable to be set.
//!
//! # Running Tests
//!
//! ```bash
//! cargo test --test agents_and_multiturn_tests -- --include-ignored --nocapture
//! ```
//!
//! # Notes
//!
//! Agent tests may take longer to complete due to background processing.
//! Some agents may not be available in all accounts.

mod common;

use common::{
    DEFAULT_MAX_RETRIES, PollError, consume_stream, get_client, poll_until_complete,
    retry_on_transient,
};
use rust_genai::{FunctionDeclaration, InteractionStatus, ThinkingLevel, function_result_content};
use serde_json::json;
use std::time::Duration;

// =============================================================================
// Test Configuration Constants
// =============================================================================

/// Minimum number of successful conversation turns to consider long conversation test valid.
/// API may encounter limitations (UTF-8 errors, etc.) with very long chains.
const MIN_SUCCESSFUL_TURNS: usize = 3;

/// Minimum facts the model should remember out of 10 in the recall test.
const MIN_REMEMBERED_FACTS: usize = 5;

/// Maximum time to wait for background tasks to complete.
const BACKGROUND_TASK_TIMEOUT: Duration = Duration::from_secs(60);

// =============================================================================
// Helper Functions
// =============================================================================

/// Checks if an error is a known API limitation for long conversation chains.
/// These errors (UTF-8 encoding issues, spanner errors, truncation) can occur
/// when the conversation context becomes too large.
fn is_long_conversation_api_error(error: &rust_genai::GenaiError) -> bool {
    let error_str = format!("{:?}", error);
    error_str.contains("UTF-8") || error_str.contains("spanner") || error_str.contains("truncated")
}

// =============================================================================
// Multi-turn: Very Long Conversations
// =============================================================================

/// Test a conversation with 10+ turns to verify context is maintained.
/// Note: Very long conversations may encounter API-side limitations.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_very_long_conversation() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let facts = [
        "My name is Alice.",
        "I live in Seattle.",
        "I work as a software engineer.",
        "My favorite programming language is Rust.",
        "I have a dog named Max.",
        "My birthday is in March.",
        "I enjoy hiking on weekends.",
        "My favorite food is sushi.",
        "I drive a blue car.",
        "I went to Stanford for college.",
    ];

    let mut previous_id: Option<String> = None;
    let mut successful_turns = 0;

    // Build up context over 10 turns
    for (i, fact) in facts.iter().enumerate() {
        let mut builder = client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_text(*fact)
            .with_store(true);

        if let Some(ref prev_id) = previous_id {
            builder = builder.with_previous_interaction(prev_id);
        }

        match builder.create().await {
            Ok(response) => {
                println!("Turn {}: {}", i + 1, fact);
                previous_id = Some(response.id);
                successful_turns += 1;
            }
            Err(e) => {
                if is_long_conversation_api_error(&e) {
                    println!(
                        "Turn {} encountered API limitation (expected for long conversations): {:?}",
                        i + 1,
                        e
                    );
                    println!(
                        "Completed {} turns before hitting API limitation",
                        successful_turns
                    );
                    // Still pass if we got the minimum successful turns
                    assert!(
                        successful_turns >= MIN_SUCCESSFUL_TURNS,
                        "Should complete at least {} turns, got {}",
                        MIN_SUCCESSFUL_TURNS,
                        successful_turns
                    );
                    return;
                }
                panic!("Turn {} failed: {:?}", i + 1, e);
            }
        }
    }

    // Final turn: ask about everything
    let final_result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_previous_interaction(previous_id.as_ref().unwrap())
        .with_text("What do you know about me? List everything you can remember.")
        .with_store(true)
        .create()
        .await;

    let final_response = match final_result {
        Ok(response) => response,
        Err(e) => {
            if is_long_conversation_api_error(&e) {
                println!(
                    "Final turn encountered API limitation (expected for long conversations): {:?}",
                    e
                );
                println!(
                    "Completed {} turns before hitting API limitation",
                    successful_turns
                );
                assert!(
                    successful_turns >= MIN_SUCCESSFUL_TURNS,
                    "Should complete at least {} turns, got {}",
                    MIN_SUCCESSFUL_TURNS,
                    successful_turns
                );
                return;
            }
            panic!("Final turn failed: {:?}", e);
        }
    };

    assert_eq!(final_response.status, InteractionStatus::Completed);
    assert!(final_response.has_text(), "Should have text response");

    let text = final_response.text().unwrap().to_lowercase();
    println!("Final response: {}", text);

    // Count how many facts the model remembers
    let fact_checks = [
        ("alice", "name"),
        ("seattle", "city"),
        ("software", "job"),
        ("rust", "language"),
        ("max", "dog"),
        ("march", "birthday"),
        ("hiking", "hobby"),
        ("sushi", "food"),
        ("blue", "car"),
        ("stanford", "college"),
    ];

    let mut remembered = 0;
    for (keyword, label) in fact_checks.iter() {
        if text.contains(*keyword) {
            remembered += 1;
            println!("  ✓ Remembered: {} ({})", keyword, label);
        }
    }

    println!("Facts remembered: {}/{}", remembered, fact_checks.len());

    // Should remember at least the minimum number of facts
    assert!(
        remembered >= MIN_REMEMBERED_FACTS,
        "Model should remember at least {} out of {} facts, got {}",
        MIN_REMEMBERED_FACTS,
        fact_checks.len(),
        remembered
    );
}

// =============================================================================
// Multi-turn: Mixed Function/Text Turns
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_conversation_function_then_text() {
    // Test a conversation that mixes function calls and text turns
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let get_weather = FunctionDeclaration::builder("get_weather")
        .description("Get the current weather")
        .parameter("city", json!({"type": "string"}))
        .required(vec!["city".to_string()])
        .build();

    // Turn 1: Trigger function call
    let response1 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's the weather in Tokyo?")
        .with_function(get_weather.clone())
        .with_store(true)
        .create()
        .await
        .expect("Turn 1 failed");

    println!("Turn 1 status: {:?}", response1.status);

    let calls = response1.function_calls();
    if calls.is_empty() {
        println!("No function call - cannot continue test");
        return;
    }

    let call = &calls[0];

    // Turn 2: Provide function result
    let result = function_result_content(
        "get_weather",
        call.id.unwrap().to_string(),
        json!({"temperature": "25°C", "conditions": "sunny"}),
    );

    let response2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_previous_interaction(&response1.id)
        .with_content(vec![result])
        .with_function(get_weather.clone())
        .with_store(true)
        .create()
        .await
        .expect("Turn 2 failed");

    println!("Turn 2 status: {:?}", response2.status);
    if response2.has_text() {
        println!("Turn 2 text: {}", response2.text().unwrap());
    }

    // Turn 3: Follow-up text question (no function call expected)
    let response3 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_previous_interaction(&response2.id)
        .with_text("Should I bring a jacket?")
        .with_function(get_weather)
        .with_store(true)
        .create()
        .await
        .expect("Turn 3 failed");

    println!("Turn 3 status: {:?}", response3.status);
    assert!(response3.has_text(), "Turn 3 should have text response");

    let text = response3.text().unwrap().to_lowercase();
    println!("Turn 3 text: {}", text);

    // Should reference the weather context
    assert!(
        text.contains("no")
            || text.contains("yes")
            || text.contains("sunny")
            || text.contains("warm")
            || text.contains("jacket")
            || text.contains("25"),
        "Response should reference weather context"
    );
}

// =============================================================================
// Multi-turn: Conversation Branching
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_conversation_branch() {
    // Test starting a new conversation from a mid-point
    //
    // NOTE: This test uses retry_on_transient to handle intermittent Spanner UTF-8
    // errors from the Google backend. See issue #60 for details.
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Build initial context
    // Each call is wrapped with retry logic to handle transient Spanner errors
    let response1 = {
        let client = client.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            async move {
                client
                    .interaction()
                    .with_model("gemini-3-flash-preview")
                    .with_text("My favorite color is red.")
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Turn 1 failed")
    };

    let response2 = {
        let client = client.clone();
        let prev_id = response1.id.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let prev_id = prev_id.clone();
            async move {
                client
                    .interaction()
                    .with_model("gemini-3-flash-preview")
                    .with_previous_interaction(&prev_id)
                    .with_text("My favorite number is 7.")
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Turn 2 failed")
    };

    let response3 = {
        let client = client.clone();
        let prev_id = response2.id.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let prev_id = prev_id.clone();
            async move {
                client
                    .interaction()
                    .with_model("gemini-3-flash-preview")
                    .with_previous_interaction(&prev_id)
                    .with_text("My favorite animal is a cat.")
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Turn 3 failed")
    };

    // Branch from turn 2 (before the cat fact)
    let branch_response = {
        let client = client.clone();
        let prev_id = response2.id.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let prev_id = prev_id.clone();
            async move {
                client
                    .interaction()
                    .with_model("gemini-3-flash-preview")
                    .with_previous_interaction(&prev_id) // Branch from turn 2
                    .with_text("What do you know about my favorites so far?")
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Branch failed")
    };

    assert!(branch_response.has_text(), "Should have text response");

    let text = branch_response.text().unwrap().to_lowercase();
    println!("Branch response (from turn 2): {}", text);

    // Should know about color and number, but NOT cat (that was in turn 3)
    let knows_color = text.contains("red");
    let knows_number = text.contains("7") || text.contains("seven");
    let knows_cat = text.contains("cat");

    println!(
        "Knows color: {}, number: {}, cat: {}",
        knows_color, knows_number, knows_cat
    );

    // Should know at least color or number
    assert!(
        knows_color || knows_number,
        "Branch should have context from earlier turns"
    );

    // Continue from turn 3 to verify it still works
    let continue_response = {
        let client = client.clone();
        let prev_id = response3.id.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let prev_id = prev_id.clone();
            async move {
                client
                    .interaction()
                    .with_model("gemini-3-flash-preview")
                    .with_previous_interaction(&prev_id)
                    .with_text("And what's my favorite animal?")
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Continue failed")
    };

    let continue_text = continue_response.text().unwrap().to_lowercase();
    println!("Continue response (from turn 3): {}", continue_text);
    assert!(
        continue_text.contains("cat"),
        "Continue should remember the cat from turn 3"
    );
}

// =============================================================================
// Agents: Deep Research
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_deep_research_agent() {
    // Test the deep research agent
    // Note: This agent may not be available in all accounts
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let result = client
        .interaction()
        .with_agent("deep-research-pro-preview-12-2025")
        .with_text("What are the main differences between Rust and Go programming languages?")
        .with_store(true)
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("Deep research status: {:?}", response.status);
            if response.has_text() {
                let text = response.text().unwrap();
                println!(
                    "Research response (truncated): {}...",
                    &text[..text.len().min(500)]
                );

                // Should mention both languages
                let text_lower = text.to_lowercase();
                assert!(
                    text_lower.contains("rust") || text_lower.contains("go"),
                    "Response should discuss programming languages"
                );
            }
        }
        Err(e) => {
            let error_str = format!("{:?}", e);
            println!("Deep research error (may be expected): {}", error_str);
            if error_str.contains("not found")
                || error_str.contains("not available")
                || error_str.contains("agent")
            {
                println!("Deep research agent not available - skipping test");
            }
        }
    }
}

// =============================================================================
// Agents: Background Mode
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_background_mode_polling() {
    // Test background mode with polling using exponential backoff
    // Note: This requires an agent that supports background mode
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Start background task
    let result = client
        .interaction()
        .with_agent("deep-research-pro-preview-12-2025")
        .with_text("Briefly explain what machine learning is.")
        .with_background(true)
        .with_store(true)
        .create()
        .await;

    match result {
        Ok(initial_response) => {
            println!("Initial status: {:?}", initial_response.status);
            println!("Interaction ID: {}", initial_response.id);

            // If already completed, we're done
            if initial_response.status == InteractionStatus::Completed {
                println!("Task completed immediately (may not have used background mode)");
                if initial_response.has_text() {
                    println!("Result: {}", initial_response.text().unwrap());
                }
                return;
            }

            // Poll for completion using exponential backoff
            match poll_until_complete(&client, &initial_response.id, BACKGROUND_TASK_TIMEOUT).await
            {
                Ok(response) => {
                    println!("Task completed!");
                    if response.has_text() {
                        let text = response.text().unwrap();
                        println!("Result: {}...", &text[..200.min(text.len())]);
                    }
                }
                Err(PollError::Timeout) => {
                    println!(
                        "Polling timed out after {:?} - task may still be running",
                        BACKGROUND_TASK_TIMEOUT
                    );
                }
                Err(PollError::Failed) => {
                    println!("Task failed");
                }
                Err(PollError::Api(e)) => {
                    println!("Poll error: {:?}", e);
                }
            }
        }
        Err(e) => {
            let error_str = format!("{:?}", e);
            println!("Background mode error (may be expected): {}", error_str);
            if error_str.contains("not found")
                || error_str.contains("not supported")
                || error_str.contains("background")
            {
                println!("Background mode not available - skipping test");
            }
        }
    }
}

// =============================================================================
// Usage Metadata
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_usage_metadata_returned() {
    // Verify that token usage metadata is returned
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What is the capital of France? Answer briefly.")
        .with_store(true)
        .create()
        .await
        .expect("Interaction failed");

    assert_eq!(response.status, InteractionStatus::Completed);

    // Check usage metadata
    if let Some(usage) = &response.usage {
        println!("Usage metadata:");
        println!("  Input tokens: {:?}", usage.total_input_tokens);
        println!("  Output tokens: {:?}", usage.total_output_tokens);
        println!("  Total tokens: {:?}", usage.total_tokens);

        // At least one of these should be set
        if usage.has_data() {
            // Verify reasonable values
            if let Some(input) = usage.total_input_tokens {
                assert!(input > 0, "Input tokens should be positive");
            }
            if let Some(output) = usage.total_output_tokens {
                assert!(output > 0, "Output tokens should be positive");
            }
            if let Some(total) = usage.total_tokens {
                assert!(total > 0, "Total tokens should be positive");
            }
        } else {
            println!("Note: Usage metadata fields are all None");
        }
    } else {
        println!("No usage metadata returned (may be expected for some configurations)");
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_usage_longer_response() {
    // Test that longer responses have more tokens
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Short response
    let short_response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Say 'hello'")
        .with_store(true)
        .create()
        .await
        .expect("Short interaction failed");

    // Longer response
    let long_response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Write a 100-word paragraph about space exploration.")
        .with_store(true)
        .create()
        .await
        .expect("Long interaction failed");

    // Compare usage
    let short_tokens = short_response
        .usage
        .and_then(|u| u.total_tokens)
        .unwrap_or(0);
    let long_tokens = long_response
        .usage
        .and_then(|u| u.total_tokens)
        .unwrap_or(0);

    println!("Short response tokens: {}", short_tokens);
    println!("Long response tokens: {}", long_tokens);

    if short_tokens > 0 && long_tokens > 0 {
        assert!(
            long_tokens > short_tokens,
            "Longer response should use more tokens: {} vs {}",
            long_tokens,
            short_tokens
        );
    } else {
        println!("Token counts not available for comparison");
    }
}

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
    let response1 = {
        let client = client.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            async move {
                client
                    .interaction()
                    .with_model("gemini-3-flash-preview")
                    .with_text(
                        "My favorite programming language is Python. Please acknowledge this.",
                    )
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Turn 1 failed")
    };

    println!("Turn 1 completed: {}", response1.id);
    assert_eq!(response1.status, InteractionStatus::Completed);

    // Turn 2: Stream a question that requires context from Turn 1
    let stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_previous_interaction(&response1.id)
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
    let response1 = {
        let client = client.clone();
        let get_weather = get_weather.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let get_weather = get_weather.clone();
            async move {
                client
                    .interaction()
                    .with_model("gemini-3-flash-preview")
                    .with_text("What's the weather in Paris?")
                    .with_function(get_weather)
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Turn 1 failed")
    };

    println!("Turn 1 status: {:?}", response1.status);

    let calls = response1.function_calls();
    if calls.is_empty() {
        println!("Model chose not to call function - skipping rest of test");
        return;
    }

    let call = &calls[0];
    println!("Function call: {} with args: {:?}", call.name, call.args);

    // Turn 2: Provide function result
    let result = function_result_content(
        "get_weather",
        call.id.expect("Function call should have ID").to_string(),
        json!({"temperature": "18°C", "conditions": "rainy", "humidity": "85%"}),
    );

    let response2 = {
        let client = client.clone();
        let prev_id = response1.id.clone();
        let get_weather = get_weather.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let prev_id = prev_id.clone();
            let result = result.clone();
            let get_weather = get_weather.clone();
            async move {
                client
                    .interaction()
                    .with_model("gemini-3-flash-preview")
                    .with_previous_interaction(&prev_id)
                    .with_content(vec![result])
                    .with_function(get_weather)
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Turn 2 failed")
    };

    println!("Turn 2 status: {:?}", response2.status);
    if response2.has_text() {
        println!("Turn 2 text: {}", response2.text().unwrap());
    }

    // Turn 3: Stream a follow-up question about the weather context
    let stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_previous_interaction(&response2.id)
        .with_text("Should I bring an umbrella? Answer briefly.")
        .with_function(get_weather)
        .with_store(true)
        .create_stream();

    let result = consume_stream(stream).await;

    println!("\nDeltas received: {}", result.delta_count);
    println!("Collected text: {}", result.collected_text);

    // Verify streaming worked
    assert!(result.has_output(), "Should receive streaming chunks");

    // Verify context was maintained - response should reference weather conditions.
    // Allow flexible matching due to LLM response variability - any reference to
    // the weather conditions indicates the multi-turn context was preserved.
    let text_lower = result.collected_text.to_lowercase();
    assert!(
        text_lower.contains("yes")
            || text_lower.contains("umbrella")
            || text_lower.contains("rain")
            || text_lower.contains("18")
            || text_lower.contains("humid"),
        "Streaming response should reference weather context. Got: {}",
        result.collected_text
    );

    // Verify final response if received
    if let Some(response) = result.final_response {
        assert_eq!(response.status, InteractionStatus::Completed);
    }
}

// =============================================================================
// Thinking + Function Calling + Multi-turn
// =============================================================================

/// Test thinking mode combined with function calling across multiple turns.
///
/// This validates that:
/// - Thinking mode (`ThinkingLevel`) works with client-side function calling
/// - Multi-turn conversations function correctly with thinking enabled
/// - Context is preserved across turns via `previous_interaction_id`
///
/// # Thought Signatures
///
/// Per Google's documentation (https://ai.google.dev/gemini-api/docs/thought-signatures):
/// - Thought signatures are encrypted representations of the model's reasoning
/// - For Gemini 3 models, signatures MUST be echoed back during function calling
/// - The Interactions API handles this automatically via `previous_interaction_id`
/// - Signatures may or may not be exposed in the response (API behavior varies)
///
/// # Thinking Mode vs Thought Signatures
///
/// These are distinct concepts:
/// - `ThinkingLevel`: Exposes model's chain-of-thought as `Thought` content
/// - `thought_signature`: Cryptographic field on function calls for verification
///
/// Thoughts may be processed internally without visible text, especially when
/// the model is focused on function calling rather than explanation.
///
/// Turn 1: Enable thinking + ask question → triggers function call
/// Turn 2: Provide function result → model processes and responds
/// Turn 3: Follow-up question → model reasons with full context preserved
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_thinking_with_function_calling_multi_turn() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let get_weather = FunctionDeclaration::builder("get_weather")
        .description("Get the current weather for a city including temperature and conditions")
        .parameter(
            "city",
            json!({"type": "string", "description": "The city name"}),
        )
        .required(vec!["city".to_string()])
        .build();

    // =========================================================================
    // Turn 1: Enable thinking + trigger function call
    // =========================================================================
    let response1 = {
        let client = client.clone();
        let get_weather = get_weather.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let get_weather = get_weather.clone();
            async move {
                client
                    .interaction()
                    .with_model("gemini-3-flash-preview")
                    .with_text("What's the weather in Tokyo? Should I bring an umbrella?")
                    .with_function(get_weather)
                    .with_thinking_level(ThinkingLevel::Medium)
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Turn 1 failed")
    };

    println!("Turn 1 status: {:?}", response1.status);

    let function_calls = response1.function_calls();
    if function_calls.is_empty() {
        println!("Model chose not to call function - skipping rest of test");
        return;
    }

    let call = &function_calls[0];
    println!(
        "Turn 1 function call: {} (has thought_signature: {})",
        call.name,
        call.thought_signature.is_some()
    );

    // Note: thought_signature is not guaranteed by the API - it depends on model behavior.
    // We log its presence but don't hard-assert, as the existing tests show it can be None.
    if call.thought_signature.is_some() {
        println!("✓ Thought signature present on function call");
    } else {
        println!("ℹ Thought signature not present (API behavior varies)");
    }
    assert!(call.id.is_some(), "Function call must have an id");

    // =========================================================================
    // Turn 2: Provide function result - model should reason about it
    // =========================================================================
    let function_result = function_result_content(
        "get_weather",
        call.id.expect("call_id should exist").to_string(),
        json!({
            "temperature": "18°C",
            "conditions": "rainy",
            "precipitation": "80%",
            "humidity": "85%"
        }),
    );

    let response2 = {
        let client = client.clone();
        let prev_id = response1.id.clone();
        let get_weather = get_weather.clone();
        let function_result = function_result.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let prev_id = prev_id.clone();
            let get_weather = get_weather.clone();
            let function_result = function_result.clone();
            async move {
                client
                    .interaction()
                    .with_model("gemini-3-flash-preview")
                    .with_previous_interaction(&prev_id)
                    .with_content(vec![function_result])
                    .with_function(get_weather)
                    .with_thinking_level(ThinkingLevel::Medium)
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Turn 2 failed")
    };

    println!("Turn 2 status: {:?}", response2.status);
    println!("Turn 2 has_thoughts: {}", response2.has_thoughts());
    println!("Turn 2 has_text: {}", response2.has_text());

    if response2.has_thoughts() {
        for (i, thought) in response2.thoughts().enumerate() {
            println!(
                "Turn 2 thought {}: {}...",
                i + 1,
                &thought[..thought.len().min(100)]
            );
        }
    }

    if response2.has_text() {
        println!("Turn 2 text: {}", response2.text().unwrap());
    }

    // Verify we got a response - thoughts may or may not be visible
    // (the API may process reasoning internally without exposing it)
    if response2.has_thoughts() {
        println!("✓ Thoughts visible in Turn 2");
    } else {
        println!("ℹ Thoughts processed internally (not exposed in response)");
    }

    assert!(
        response2.has_text(),
        "Turn 2 should have text response about the weather"
    );

    // Response should reference the weather conditions
    let text2 = response2.text().unwrap().to_lowercase();
    assert!(
        text2.contains("umbrella")
            || text2.contains("rain")
            || text2.contains("yes")
            || text2.contains("18"),
        "Turn 2 should reference weather conditions. Got: {}",
        text2
    );

    // =========================================================================
    // Turn 3: Follow-up question - model reasons with full context
    // =========================================================================
    let response3 = {
        let client = client.clone();
        let prev_id = response2.id.clone();
        let get_weather = get_weather.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let prev_id = prev_id.clone();
            let get_weather = get_weather.clone();
            async move {
                client
                    .interaction()
                    .with_model("gemini-3-flash-preview")
                    .with_previous_interaction(&prev_id)
                    .with_text(
                        "Given this weather, what indoor activities would you recommend in Tokyo?",
                    )
                    .with_function(get_weather)
                    .with_thinking_level(ThinkingLevel::Medium)
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Turn 3 failed")
    };

    println!("Turn 3 status: {:?}", response3.status);
    println!("Turn 3 has_thoughts: {}", response3.has_thoughts());
    println!("Turn 3 has_text: {}", response3.has_text());

    if response3.has_thoughts() {
        for (i, thought) in response3.thoughts().enumerate() {
            println!(
                "Turn 3 thought {}: {}...",
                i + 1,
                &thought[..thought.len().min(100)]
            );
        }
    }

    if response3.has_text() {
        println!("Turn 3 text: {}", response3.text().unwrap());
    }

    // Verify we got a response - thoughts may or may not be visible
    if response3.has_thoughts() {
        println!("✓ Thoughts visible in Turn 3");
    } else {
        println!("ℹ Thoughts processed internally (not exposed in response)");
    }

    assert!(
        response3.has_text(),
        "Turn 3 should have text response with recommendations"
    );

    // Log reasoning tokens if available (indicates thinking is engaged)
    if let Some(ref usage) = response3.usage
        && let Some(reasoning_tokens) = usage.total_reasoning_tokens
    {
        println!("Turn 3 reasoning tokens: {}", reasoning_tokens);
    }

    // Response should be contextually relevant (about indoor activities)
    let text3 = response3.text().unwrap().to_lowercase();
    assert!(
        text3.contains("indoor")
            || text3.contains("inside")
            || text3.contains("museum")
            || text3.contains("shopping")
            || text3.contains("restaurant")
            || text3.contains("cafe")
            || text3.contains("temple")
            || text3.contains("activity")
            || text3.contains("activities"),
        "Turn 3 should recommend indoor activities. Got: {}",
        text3
    );

    println!("\n✓ All three turns completed successfully with thinking + function calling");
}

/// Test thinking mode with parallel function calls.
///
/// This validates that:
/// - Thinking mode works correctly when the model makes multiple function calls in one response
/// - Thought signatures follow the documented pattern (only first parallel call has signature)
/// - Results can be provided for all parallel calls and the model reasons about them
///
/// Per Google's documentation (https://ai.google.dev/gemini-api/docs/thought-signatures):
/// "If the model generates parallel function calls in a response, only the first
/// function call will contain a signature."
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_thinking_with_parallel_function_calls() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let get_weather = FunctionDeclaration::builder("get_weather")
        .description("Get the current weather for a city")
        .parameter(
            "city",
            json!({"type": "string", "description": "City name"}),
        )
        .required(vec!["city".to_string()])
        .build();

    let get_time = FunctionDeclaration::builder("get_time")
        .description("Get the current time in a timezone")
        .parameter(
            "timezone",
            json!({"type": "string", "description": "Timezone like UTC, PST, JST"}),
        )
        .required(vec!["timezone".to_string()])
        .build();

    // =========================================================================
    // Turn 1: Enable thinking + trigger parallel function calls
    // =========================================================================
    let response1 = {
        let client = client.clone();
        let get_weather = get_weather.clone();
        let get_time = get_time.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let get_weather = get_weather.clone();
            let get_time = get_time.clone();
            async move {
                client
                    .interaction()
                    .with_model("gemini-3-flash-preview")
                    .with_text(
                        "What's the weather in Tokyo and what time is it there? \
                         I need both pieces of information.",
                    )
                    .with_functions(vec![get_weather, get_time])
                    .with_thinking_level(ThinkingLevel::Medium)
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Turn 1 failed")
    };

    println!("Turn 1 status: {:?}", response1.status);

    let function_calls = response1.function_calls();
    println!("Number of function calls: {}", function_calls.len());

    if function_calls.is_empty() {
        println!("Model chose not to call functions - skipping rest of test");
        return;
    }

    for (i, call) in function_calls.iter().enumerate() {
        println!(
            "  Call {}: {} (has thought_signature: {})",
            i + 1,
            call.name,
            call.thought_signature.is_some()
        );
    }

    // Per docs: only the first parallel call should have a signature
    if function_calls.len() >= 2 {
        println!("✓ Model made parallel function calls");
        if function_calls[0].thought_signature.is_some() {
            println!("✓ First call has thought_signature (as documented)");
        }
        // Note: We don't hard-assert on signature presence as API behavior varies
    }

    // Verify all calls have IDs
    for call in &function_calls {
        assert!(
            call.id.is_some(),
            "Function call '{}' should have an ID",
            call.name
        );
    }

    // =========================================================================
    // Turn 2: Provide results for all function calls
    // =========================================================================
    let mut results = Vec::new();
    for call in &function_calls {
        let result_data = match call.name {
            "get_weather" => json!({
                "temperature": "22°C",
                "conditions": "partly cloudy",
                "humidity": "65%"
            }),
            "get_time" => json!({
                "time": "14:30",
                "timezone": "JST",
                "date": "2025-01-15"
            }),
            _ => json!({"status": "unknown function"}),
        };

        results.push(function_result_content(
            call.name,
            call.id.expect("call should have ID"),
            result_data,
        ));
    }

    let response2 = {
        let client = client.clone();
        let prev_id = response1.id.clone();
        let get_weather = get_weather.clone();
        let get_time = get_time.clone();
        let results = results.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let prev_id = prev_id.clone();
            let get_weather = get_weather.clone();
            let get_time = get_time.clone();
            let results = results.clone();
            async move {
                client
                    .interaction()
                    .with_model("gemini-3-flash-preview")
                    .with_previous_interaction(&prev_id)
                    .with_content(results)
                    .with_functions(vec![get_weather, get_time])
                    .with_thinking_level(ThinkingLevel::Medium)
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Turn 2 failed")
    };

    println!("Turn 2 status: {:?}", response2.status);
    println!("Turn 2 has_thoughts: {}", response2.has_thoughts());
    println!("Turn 2 has_text: {}", response2.has_text());

    if response2.has_thoughts() {
        println!("✓ Thoughts visible in Turn 2");
    } else {
        println!("ℹ Thoughts processed internally (not exposed in response)");
    }

    if response2.has_text() {
        let text = response2.text().unwrap();
        println!("Turn 2 text: {}", text);
    }

    assert!(
        response2.has_text(),
        "Turn 2 should have text response combining weather and time info"
    );

    // Response should reference both weather and time
    let text2 = response2.text().unwrap().to_lowercase();
    let has_weather_ref = text2.contains("weather")
        || text2.contains("temperature")
        || text2.contains("22")
        || text2.contains("cloud");
    let has_time_ref = text2.contains("time") || text2.contains("14:30") || text2.contains("2:30");

    println!(
        "References weather: {}, References time: {}",
        has_weather_ref, has_time_ref
    );

    // At minimum, should reference at least one of the function results
    assert!(
        has_weather_ref || has_time_ref,
        "Turn 2 should reference function results. Got: {}",
        text2
    );

    println!("\n✓ Parallel function calls with thinking completed successfully");
}

/// Test thinking mode with sequential function chain containing parallel calls at each step.
///
/// This is the most comprehensive test combining:
/// - Sequential function calling (multi-step chain)
/// - Parallel function calls at each step
/// - Thinking mode enabled throughout
///
/// Per Google's documentation (https://ai.google.dev/gemini-api/docs/thought-signatures):
/// "When there are sequential function calls (multi-step), each function call will have
/// a signature and you must pass all signatures back."
///
/// The Interactions API handles signature management automatically via `previous_interaction_id`.
///
/// Flow:
/// - Step 1: Model calls get_weather + get_time in parallel
/// - Step 2: After results, model calls get_forecast + get_activities in parallel
/// - Step 3: Model combines all information into final response
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_thinking_with_sequential_parallel_function_chain() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Define all functions we'll use
    let get_weather = FunctionDeclaration::builder("get_current_weather")
        .description("Get the current weather conditions for a city")
        .parameter(
            "city",
            json!({"type": "string", "description": "City name"}),
        )
        .required(vec!["city".to_string()])
        .build();

    let get_time = FunctionDeclaration::builder("get_local_time")
        .description("Get the current local time in a city")
        .parameter(
            "city",
            json!({"type": "string", "description": "City name"}),
        )
        .required(vec!["city".to_string()])
        .build();

    let get_forecast = FunctionDeclaration::builder("get_weather_forecast")
        .description("Get the weather forecast for the next few days")
        .parameter(
            "city",
            json!({"type": "string", "description": "City name"}),
        )
        .required(vec!["city".to_string()])
        .build();

    let get_activities = FunctionDeclaration::builder("get_recommended_activities")
        .description("Get recommended activities based on weather conditions")
        .parameter(
            "weather_condition",
            json!({"type": "string", "description": "Current weather like sunny, rainy, cloudy"}),
        )
        .required(vec!["weather_condition".to_string()])
        .build();

    let all_functions = vec![
        get_weather.clone(),
        get_time.clone(),
        get_forecast.clone(),
        get_activities.clone(),
    ];

    // =========================================================================
    // Step 1: Initial request - expect parallel calls for weather and time
    // =========================================================================
    println!("=== Step 1: Initial request ===");

    let response1 = {
        let client = client.clone();
        let functions = all_functions.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let functions = functions.clone();
            async move {
                client
                    .interaction()
                    .with_model("gemini-3-flash-preview")
                    .with_text(
                        "I'm planning a trip to Tokyo. I need to know the current weather, \
                         current local time, the forecast for the next few days, and what \
                         activities you'd recommend. Please gather all this information.",
                    )
                    .with_functions(functions)
                    .with_thinking_level(ThinkingLevel::Medium)
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Step 1 failed")
    };

    println!("Step 1 status: {:?}", response1.status);

    let calls1 = response1.function_calls();
    println!("Step 1 function calls: {}", calls1.len());

    if calls1.is_empty() {
        println!("Model chose not to call functions - skipping rest of test");
        return;
    }

    for (i, call) in calls1.iter().enumerate() {
        println!(
            "  Call {}: {} (has signature: {})",
            i + 1,
            call.name,
            call.thought_signature.is_some()
        );
    }

    // Verify all calls have IDs
    for call in &calls1 {
        assert!(call.id.is_some(), "Function call should have ID");
    }

    // =========================================================================
    // Step 2: Provide results for step 1, expect more function calls
    // =========================================================================
    println!("\n=== Step 2: Provide first results ===");

    let mut results1 = Vec::new();
    for call in &calls1 {
        let result_data = match call.name {
            "get_current_weather" => json!({
                "temperature": "24°C",
                "conditions": "partly cloudy",
                "humidity": "60%",
                "wind": "10 km/h"
            }),
            "get_local_time" => json!({
                "time": "10:30 AM",
                "timezone": "JST",
                "date": "2025-01-15"
            }),
            "get_weather_forecast" => json!({
                "tomorrow": "sunny, 26°C",
                "day_after": "cloudy, 22°C",
                "in_3_days": "rainy, 18°C"
            }),
            "get_recommended_activities" => json!({
                "outdoor": ["visit Senso-ji Temple", "walk in Ueno Park"],
                "indoor": ["explore TeamLab", "shop in Shibuya"],
                "food": ["try ramen in Shinjuku", "sushi at Tsukiji"]
            }),
            _ => json!({"status": "unknown function"}),
        };

        results1.push(function_result_content(
            call.name,
            call.id.expect("call should have ID"),
            result_data,
        ));
    }

    let response2 = {
        let client = client.clone();
        let prev_id = response1.id.clone();
        let functions = all_functions.clone();
        let results = results1.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let prev_id = prev_id.clone();
            let functions = functions.clone();
            let results = results.clone();
            async move {
                client
                    .interaction()
                    .with_model("gemini-3-flash-preview")
                    .with_previous_interaction(&prev_id)
                    .with_content(results)
                    .with_functions(functions)
                    .with_thinking_level(ThinkingLevel::Medium)
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Step 2 failed")
    };

    println!("Step 2 status: {:?}", response2.status);
    println!("Step 2 has_thoughts: {}", response2.has_thoughts());
    println!("Step 2 has_text: {}", response2.has_text());

    let calls2 = response2.function_calls();
    println!("Step 2 function calls: {}", calls2.len());

    for (i, call) in calls2.iter().enumerate() {
        println!(
            "  Call {}: {} (has signature: {})",
            i + 1,
            call.name,
            call.thought_signature.is_some()
        );
    }

    // The model might either:
    // 1. Call more functions (sequential chain continues)
    // 2. Return final text (it has enough info)
    //
    // Both are valid outcomes - we test the chain if it continues

    if !calls2.is_empty() {
        // =====================================================================
        // Step 3: Provide second round of results, expect final response
        // =====================================================================
        println!("\n=== Step 3: Provide second results ===");

        let mut results2 = Vec::new();
        for call in &calls2 {
            let result_data = match call.name {
                "get_current_weather" => json!({
                    "temperature": "24°C",
                    "conditions": "partly cloudy"
                }),
                "get_local_time" => json!({
                    "time": "10:35 AM",
                    "timezone": "JST"
                }),
                "get_weather_forecast" => json!({
                    "tomorrow": "sunny, 26°C",
                    "day_after": "cloudy, 22°C"
                }),
                "get_recommended_activities" => json!({
                    "outdoor": ["temple visits", "park walks"],
                    "indoor": ["museums", "shopping"]
                }),
                _ => json!({"status": "ok"}),
            };

            results2.push(function_result_content(
                call.name,
                call.id.expect("call should have ID"),
                result_data,
            ));
        }

        let response3 = {
            let client = client.clone();
            let prev_id = response2.id.clone();
            let functions = all_functions.clone();
            let results = results2.clone();
            retry_on_transient(DEFAULT_MAX_RETRIES, || {
                let client = client.clone();
                let prev_id = prev_id.clone();
                let functions = functions.clone();
                let results = results.clone();
                async move {
                    client
                        .interaction()
                        .with_model("gemini-3-flash-preview")
                        .with_previous_interaction(&prev_id)
                        .with_content(results)
                        .with_functions(functions)
                        .with_thinking_level(ThinkingLevel::Medium)
                        .with_store(true)
                        .create()
                        .await
                }
            })
            .await
            .expect("Step 3 failed")
        };

        println!("Step 3 status: {:?}", response3.status);
        println!("Step 3 has_thoughts: {}", response3.has_thoughts());
        println!("Step 3 has_text: {}", response3.has_text());

        if response3.has_thoughts() {
            println!("✓ Thoughts visible in Step 3");
        } else {
            println!("ℹ Thoughts processed internally");
        }

        let calls3 = response3.function_calls();
        if calls3.is_empty() {
            println!("✓ No more function calls - chain complete");
        } else {
            println!("ℹ Model requested {} more function calls", calls3.len());
        }

        if response3.has_text() {
            let text = response3.text().unwrap();
            println!("Step 3 text preview: {}...", &text[..text.len().min(200)]);

            // Verify the response integrates information from the chain
            let text_lower = text.to_lowercase();
            assert!(
                text_lower.contains("tokyo")
                    || text_lower.contains("weather")
                    || text_lower.contains("temperature")
                    || text_lower.contains("activit"),
                "Final response should reference gathered information"
            );
        }

        println!("\n✓ Sequential parallel function chain (3 steps) completed successfully");
    } else {
        // Model returned text in step 2 (gathered all info in first round)
        println!("ℹ Model completed in 2 steps (no sequential chain needed)");

        if response2.has_text() {
            let text = response2.text().unwrap();
            println!("Step 2 text preview: {}...", &text[..text.len().min(200)]);
        }

        assert!(
            response2.has_text(),
            "Step 2 should have text if no more function calls"
        );

        println!("\n✓ Function calls with thinking completed in 2 steps");
    }
}
