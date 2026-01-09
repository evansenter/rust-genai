//! Multi-turn conversation and usage metadata tests
//!
//! Tests for very long conversations, mixed function/text turns, conversation branching,
//! and token usage verification.
//!
//! These tests require the GEMINI_API_KEY environment variable to be set.
//!
//! # Running Tests
//!
//! ```bash
//! cargo test --test multiturn_tests -- --include-ignored --nocapture
//! ```

mod common;

use common::{get_client, stateful_builder};
use genai_rs::{FunctionDeclaration, InteractionStatus, function_result_content};
use serde_json::json;

// =============================================================================
// Test Configuration Constants
// =============================================================================

/// Minimum number of successful conversation turns to consider long conversation test valid.
/// API may encounter limitations (UTF-8 errors, etc.) with very long chains.
const MIN_SUCCESSFUL_TURNS: usize = 3;

/// Minimum facts the model should remember out of 10 in the recall test.
const MIN_REMEMBERED_FACTS: usize = 5;

// =============================================================================
// Helper Functions
// =============================================================================

/// Checks if an error is a known API limitation for long conversation chains.
/// These errors (UTF-8 encoding issues, spanner errors, truncation) can occur
/// when the conversation context becomes too large.
fn is_long_conversation_api_error(error: &genai_rs::GenaiError) -> bool {
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
        // Use a helper closure to handle the typestate transition cleanly
        let result = match &previous_id {
            None => {
                // First turn: FirstTurn builder
                stateful_builder(&client).with_text(*fact).create().await
            }
            Some(prev_id) => {
                // Subsequent turns: Chained builder
                stateful_builder(&client)
                    .with_text(*fact)
                    .with_previous_interaction(prev_id)
                    .create()
                    .await
            }
        };

        match result {
            Ok(response) => {
                println!("Turn {}: {}", i + 1, fact);
                previous_id = response.id;
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
    let final_result = stateful_builder(&client)
        .with_previous_interaction(previous_id.as_ref().unwrap())
        .with_text("What do you know about me? List everything you can remember.")
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
    let response1 = stateful_builder(&client)
        .with_text("What's the weather in Tokyo?")
        .with_function(get_weather.clone())
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

    let response2 = stateful_builder(&client)
        .with_previous_interaction(response1.id.as_ref().expect("id should exist"))
        .with_content(vec![result])
        .with_function(get_weather.clone())
        .create()
        .await
        .expect("Turn 2 failed");

    println!("Turn 2 status: {:?}", response2.status);
    if response2.has_text() {
        println!("Turn 2 text: {}", response2.text().unwrap());
    }

    // Turn 3: Follow-up text question (no function call expected)
    let response3 = stateful_builder(&client)
        .with_previous_interaction(response2.id.as_ref().expect("id should exist"))
        .with_text("Should I bring a jacket?")
        .with_function(get_weather)
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
    // NOTE: This test uses retry_request! to handle intermittent Spanner UTF-8
    // errors from the Google backend. See issue #60 for details.
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Build initial context
    // Each call is wrapped with retry logic to handle transient Spanner errors
    let response1 = retry_request!([client] => {
        stateful_builder(&client)
            .with_text("My favorite color is red.")
            .create()
            .await
    })
    .expect("Turn 1 failed");

    let prev_id = response1.id.clone().expect("id should exist");
    let response2 = retry_request!([client, prev_id] => {
        stateful_builder(&client)
            .with_previous_interaction(&prev_id)
            .with_text("My favorite number is 7.")
            .create()
            .await
    })
    .expect("Turn 2 failed");

    let prev_id = response2.id.clone().expect("id should exist");
    let response3 = retry_request!([client, prev_id] => {
        stateful_builder(&client)
            .with_previous_interaction(&prev_id)
            .with_text("My favorite animal is a cat.")
            .create()
            .await
    })
    .expect("Turn 3 failed");

    // Branch from turn 2 (before the cat fact)
    let prev_id = response2.id.clone().expect("id should exist");
    let branch_response = retry_request!([client, prev_id] => {
        stateful_builder(&client)
            .with_previous_interaction(&prev_id) // Branch from turn 2
            .with_text("What do you know about my favorites so far?")
            .create()
            .await
    })
    .expect("Branch failed");

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
    let prev_id = response3.id.clone().expect("id should exist");
    let continue_response = retry_request!([client, prev_id] => {
        stateful_builder(&client)
            .with_previous_interaction(&prev_id)
            .with_text("And what's my favorite animal?")
            .create()
            .await
    })
    .expect("Continue failed");

    let continue_text = continue_response.text().unwrap().to_lowercase();
    println!("Continue response (from turn 3): {}", continue_text);
    assert!(
        continue_text.contains("cat"),
        "Continue should remember the cat from turn 3"
    );
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

    let response = stateful_builder(&client)
        .with_text("What is the capital of France? Answer briefly.")
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
    let short_response = stateful_builder(&client)
        .with_text("Say 'hello'")
        .create()
        .await
        .expect("Short interaction failed");

    // Longer response
    let long_response = stateful_builder(&client)
        .with_text("Write a 100-word paragraph about space exploration.")
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
// Explicit Turn Array Tests (Issue #271)
// =============================================================================

use common::interaction_builder;
use genai_rs::Turn;

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_explicit_turns_basic() {
    // Test basic multi-turn conversation using explicit Turn array
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let turns = vec![
        Turn::user("What is 2+2?"),
        Turn::model("2+2 equals 4."),
        Turn::user("And what's that times 3?"),
    ];

    let response = interaction_builder(&client)
        .with_turns(turns)
        .create()
        .await
        .expect("Request failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");

    let text = response.text().unwrap().to_lowercase();
    println!("Response: {}", text);

    // Model should compute 4 * 3 = 12
    assert!(
        text.contains("12") || text.contains("twelve"),
        "Response should contain the answer 12"
    );
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_conversation_builder_fluent_api() {
    // Test the ConversationBuilder fluent API
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .conversation()
        .user("My name is Alice and I love hiking.")
        .model("Nice to meet you, Alice! Hiking is a wonderful outdoor activity.")
        .user("What's my name and what do I enjoy doing?")
        .done()
        .create()
        .await
        .expect("Request failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");

    let text = response.text().unwrap().to_lowercase();
    println!("Response: {}", text);

    // Model should remember both facts
    assert!(text.contains("alice"), "Response should mention Alice");
    assert!(
        text.contains("hik"),
        "Response should mention hiking (or hiking-related word)"
    );
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_explicit_turns_context_preservation() {
    // Test that context is properly preserved across explicit turns
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let turns = vec![
        Turn::user("I'm planning a trip to Tokyo next month."),
        Turn::model(
            "Tokyo is an amazing destination! It offers incredible food, \
             ancient temples, modern technology, and beautiful cherry blossoms. \
             What aspects of the city are you most excited to explore?",
        ),
        Turn::user("I love trying local cuisine."),
        Turn::model(
            "Great choice! Tokyo has some of the best food in the world. \
             Try ramen in a local shop, fresh sushi at Tsukiji Outer Market, \
             and don't miss Japanese convenience store food - it's surprisingly excellent!",
        ),
        Turn::user("Where am I going and what do I enjoy?"),
    ];

    let response = interaction_builder(&client)
        .with_turns(turns)
        .create()
        .await
        .expect("Request failed");

    assert_eq!(response.status, InteractionStatus::Completed);

    let text = response.text().unwrap().to_lowercase();
    println!("Response: {}", text);

    // Should remember both destination and interest
    assert!(
        text.contains("tokyo") || text.contains("japan"),
        "Response should mention Tokyo/Japan"
    );
    assert!(
        text.contains("food") || text.contains("cuisine") || text.contains("eat"),
        "Response should mention food/cuisine"
    );
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_explicit_turns_single_user_message() {
    // Test that a single user turn works (equivalent to with_text)
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let turns = vec![Turn::user(
        "What is the capital of France? Reply in one word.",
    )];

    let response = interaction_builder(&client)
        .with_turns(turns)
        .create()
        .await
        .expect("Request failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");

    let text = response.text().unwrap().to_lowercase();
    println!("Response: {}", text);

    assert!(text.contains("paris"), "Response should contain Paris");
}
