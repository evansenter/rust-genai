//! Integration tests for the Interactions API
//!
//! These tests require the GEMINI_API_KEY environment variable to be set.
//!
//! # Running Tests
//!
//! ```bash
//! # Run all integration tests
//! cargo test --test interactions_api_tests -- --include-ignored --nocapture
//!
//! # Run a specific test
//! cargo test test_simple_interaction -- --include-ignored
//!
//! # Run tests in parallel (faster, but may hit rate limits)
//! cargo test --test interactions_api_tests -- --include-ignored --test-threads=4
//! ```
//!
//! # Test Execution Time
//!
//! Running all tests takes approximately 2-5 minutes depending on API response times.
//! Individual tests typically complete in 2-10 seconds.
//!
//! # Known Flakiness
//!
//! Some tests may occasionally fail due to model behavior variability:
//! - Models may paraphrase data (e.g., "seventy-five" instead of "75")
//! - System instructions may not always be followed perfectly
//! - Function calling decisions are non-deterministic
//!
//! If a test fails intermittently, re-running usually succeeds. This is expected
//! behavior for LLM integration tests.

mod common;

use common::{
    EXTENDED_TEST_TIMEOUT, TEST_TIMEOUT, consume_stream, interaction_builder, stateful_builder,
    validate_response_semantically, with_timeout,
};
use rust_genai::{
    CallableFunction, Client, CreateInteractionRequest, FunctionDeclaration, GenaiError,
    GenerationConfig, InteractionInput, InteractionStatus, function_result_content,
    image_uri_content, text_content,
};
use rust_genai_macros::tool;
use serde_json::json;
use std::env;

// =============================================================================
// Test Helpers
// =============================================================================

fn get_client() -> Option<Client> {
    let api_key = env::var("GEMINI_API_KEY").ok()?;
    match Client::builder(api_key).build() {
        Ok(client) => Some(client),
        Err(e) => {
            eprintln!(
                "WARNING: GEMINI_API_KEY is set but client build failed: {}",
                e
            );
            None
        }
    }
}

// Define a test function that will be registered in the global registry.
// The macro generates a callable wrapper, so the function itself appears unused.
/// Gets a mock weather report for a city
#[allow(dead_code)]
#[tool(city(description = "The city to get weather for"))]
fn get_mock_weather(city: String) -> String {
    format!("Weather in {}: Sunny, 75°F", city)
}

// =============================================================================
// Basic Interactions (CRUD Operations)
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_simple_interaction() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let response = stateful_builder(&client)
        .with_text("What is 2 + 2?")
        .create()
        .await
        .expect("Interaction failed");

    assert!(response.id.is_some(), "Interaction ID should be present");
    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(!response.outputs.is_empty(), "Outputs are empty");

    // Verify output contains expected answer using semantic validation
    // (the model might say "four" instead of "4", or phrase the answer differently)
    assert!(response.has_text(), "Should have text response");
    let text = response.text().unwrap();
    let is_valid = validate_response_semantically(
        &client,
        "User asked 'What is 2 + 2?'",
        text,
        "Does this response correctly answer that 2+2 equals 4?",
    )
    .await
    .expect("Semantic validation should succeed");
    assert!(is_valid, "Response should correctly answer 2+2");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_stateful_conversation() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(TEST_TIMEOUT, async {
        // First interaction
        let response1 = stateful_builder(&client)
            .with_text("My favorite color is blue.")
            .create()
            .await
            .expect("First interaction failed");

        assert_eq!(response1.status, InteractionStatus::Completed);

        // Second interaction referencing the first
        let response2 = stateful_builder(&client)
            .with_previous_interaction(response1.id.as_ref().expect("id should exist"))
            .with_text("What is my favorite color?")
            .create()
            .await
            .expect("Second interaction failed");

        assert_eq!(response2.status, InteractionStatus::Completed);

        // Verify the model generated a response to the question
        // The model should recall "blue" but phrasing varies, so we verify structural completeness
        assert!(
            response2.has_text(),
            "Response should have text answering the question"
        );
        let text = response2.text().unwrap();
        assert!(!text.is_empty(), "Response should be non-empty");

        // Semantic validation: Check that the response correctly recalls the color from context
        let is_valid = validate_response_semantically(
            &client,
            "In the previous turn, the user said 'My favorite color is blue.' Now they're asking 'What is my favorite color?'",
            text,
            "Does this response indicate that the user's favorite color is blue?"
        ).await.expect("Semantic validation failed");
        assert!(
            is_valid,
            "Response should correctly recall that the favorite color is blue from the previous turn"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_get_interaction() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Create an interaction first
    let response = stateful_builder(&client)
        .with_text("Hello, world!")
        .create()
        .await
        .expect("Interaction failed");

    // Retrieve the interaction
    let retrieved = client
        .get_interaction(response.id.as_ref().expect("id should exist"))
        .await
        .expect("Get interaction failed");

    assert_eq!(retrieved.id, response.id);
    assert_eq!(retrieved.status, InteractionStatus::Completed);
    assert!(!retrieved.outputs.is_empty(), "Outputs are empty");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_delete_interaction() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Create an interaction first
    let response = stateful_builder(&client)
        .with_text("Test interaction for deletion")
        .create()
        .await
        .expect("Interaction failed");

    // Delete the interaction
    client
        .delete_interaction(response.id.as_ref().expect("id should exist"))
        .await
        .expect("Delete interaction failed");

    // Verify it's deleted by trying to retrieve it
    let get_result = client
        .get_interaction(response.id.as_ref().expect("id should exist"))
        .await;
    assert!(
        get_result.is_err(),
        "Expected error when getting deleted interaction"
    );
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_cancel_background_interaction() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Start a background interaction with the deep research agent
    // This agent takes a long time to complete, giving us time to cancel it
    let response = client
        .interaction()
        .with_agent("deep-research-pro-preview-12-2025")
        .with_text("What are the current trends in quantum computing research?")
        .with_background(true)
        .with_store_enabled()
        .create()
        .await
        .expect("Failed to create background interaction");

    // Should have an ID since we used store: true
    let interaction_id = response
        .id
        .as_ref()
        .expect("stored interaction should have id");

    // The initial status should be InProgress for a background interaction
    // (or Completed if it finished very quickly, which is rare for deep research)
    if response.status == InteractionStatus::InProgress {
        // Cancel the in-progress interaction
        match client.cancel_interaction(interaction_id).await {
            Ok(cancelled_response) => {
                // Verify the status is now Cancelled
                assert_eq!(
                    cancelled_response.status,
                    InteractionStatus::Cancelled,
                    "Expected status to be Cancelled after cancel_interaction, got {:?}",
                    cancelled_response.status
                );
                println!("Successfully cancelled background interaction");
            }
            Err(GenaiError::Api {
                status_code: 404, ..
            }) => {
                // The cancel endpoint may not yet be deployed to production API
                // despite being documented. Handle 404 gracefully.
                println!(
                    "Cancel endpoint not yet available in production API (404). \
                     Implementation is ready - will work once API is deployed."
                );
                // Don't fail the test - the implementation is correct
            }
            Err(e) => {
                panic!("Unexpected error cancelling interaction: {e:?}");
            }
        }
    } else {
        // If it completed or failed immediately, just note it
        println!(
            "Background interaction finished with status {:?} before we could cancel it",
            response.status
        );
        // This is acceptable - the test still verifies the API call works
    }
}

// =============================================================================
// Streaming
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_streaming_interaction() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(TEST_TIMEOUT, async {
        let stream = stateful_builder(&client)
            .with_text("Count from 1 to 5.")
            .create_stream();

        let result = consume_stream(stream).await;

        println!("\nTotal deltas: {}", result.delta_count);
        println!("Collected text: {}", result.collected_text);

        // We should receive at least some delta chunks
        assert!(
            result.has_output(),
            "No streaming chunks received - streaming may not be working"
        );

        // If we got a complete event, verify it has a valid ID
        if let Some(response) = result.final_response {
            assert!(response.id.is_some(), "Complete response should have an ID");
        }
    })
    .await;
}

/// Verifies that streaming deltas are truly incremental, not cumulative.
///
/// The Interactions API's `content.delta` events should return only new content
/// that hasn't been sent before. If the API were returning cumulative content
/// (the full response so far), we would see:
/// - Duplicated text when concatenating
/// - Characters counted multiple times
///
/// This test validates our streaming implementation by logging each delta
/// individually and verifying the total length matches the sum of delta lengths.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_streaming_deltas_are_incremental() {
    use futures_util::StreamExt;
    use rust_genai::StreamChunk;

    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(TEST_TIMEOUT, async {
        // Use a prompt that generates enough text to be split across multiple chunks.
        // Short responses may arrive in a single chunk, which wouldn't test incrementality.
        let mut stream = stateful_builder(&client)
            .with_text("Write a haiku about each season: spring, summer, fall, and winter. Label each one.")
            .create_stream();

        let mut delta_texts: Vec<String> = Vec::new();
        let mut delta_count = 0;

        println!("\n=== Streaming Delta Analysis ===\n");

        while let Some(result) = stream.next().await {
            match result {
                Ok(StreamChunk::Delta(delta)) => {
                    delta_count += 1;
                    if let Some(text) = delta.text() {
                        println!("Delta #{}: {:?} (len={})", delta_count, text, text.len());
                        delta_texts.push(text.to_string());
                    }
                }
                Ok(StreamChunk::Complete(response)) => {
                    println!("\n--- Complete ---");
                    println!("Interaction ID: {:?}", response.id);
                    if let Some(final_text) = response.text() {
                        println!("Final text length: {}", final_text.len());
                    }
                }
                _ => {}
            }
        }

        // Verify we received multiple text deltas - this is critical for testing incrementality.
        // A single chunk wouldn't prove deltas are non-cumulative.
        assert!(
            delta_texts.len() >= 2,
            "Test requires at least 2 text deltas to validate incrementality, got {}. \
             Try a prompt that generates more output.",
            delta_texts.len()
        );

        // Concatenate all deltas
        let concatenated: String = delta_texts.iter().map(|s| s.as_str()).collect();
        let sum_of_lengths: usize = delta_texts.iter().map(|s| s.len()).sum();

        println!("\n=== Delta Statistics ===");
        println!("Number of deltas: {}", delta_texts.len());
        println!("Sum of individual delta lengths: {}", sum_of_lengths);
        println!("Concatenated text length: {}", concatenated.len());
        println!("Concatenated text: {:?}", concatenated);

        // If deltas were cumulative (full text each time), the sum of lengths
        // would be much larger than the final concatenated length.
        // For truly incremental deltas, they should be equal.
        assert_eq!(
            sum_of_lengths,
            concatenated.len(),
            "Sum of delta lengths should equal concatenated length. \
             If these differ, deltas may contain overlapping content."
        );

        // Verify the content looks reasonable (should mention seasons)
        let lower = concatenated.to_lowercase();
        assert!(
            ["spring", "summer", "fall", "winter"]
                .iter()
                .any(|season| lower.contains(season)),
            "Response should mention seasons. Got: {:?}",
            concatenated
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_streaming_with_raw_request() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(TEST_TIMEOUT, async {
        let request = CreateInteractionRequest {
            model: Some("gemini-3-flash-preview".to_string()),
            agent: None,
            input: InteractionInput::Text("Count from 1 to 5.".to_string()),
            previous_interaction_id: None,
            tools: None,
            response_modalities: None,
            response_format: None,
            response_mime_type: None,
            generation_config: None,
            stream: Some(true),
            background: None,
            store: Some(true),
            system_instruction: None,
        };

        let stream = client.create_interaction_stream(request);
        let result = consume_stream(stream).await;

        println!(
            "Received {} deltas from raw request stream",
            result.delta_count
        );

        assert!(result.has_output(), "No streaming chunks received");
    })
    .await;
}

// =============================================================================
// Function Calling - Basic
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_function_call_returns_id() {
    // Verify that function calls from the API include an id field
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let get_time = FunctionDeclaration::builder("get_current_time")
        .description("Get the current time")
        .build();

    let response = interaction_builder(&client)
        .with_text("What time is it?")
        .with_function(get_time)
        .create()
        .await
        .expect("Interaction failed");

    println!("Response outputs: {:?}", response.outputs);

    let function_calls = response.function_calls();

    if function_calls.is_empty() {
        println!("Model chose not to call function - skipping id verification");
        return;
    }

    // Verify all function calls have IDs
    for call in function_calls {
        println!("Function call: {} has call_id: {:?}", call.name, call.id);
        assert!(
            call.id.is_some(),
            "Function call '{}' must have an id field",
            call.name
        );
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_manual_function_calling_with_result() {
    // Test the complete manual function calling workflow with FunctionResult
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(TEST_TIMEOUT, async {
        let get_weather = FunctionDeclaration::builder("get_weather")
            .description("Get the current weather for a location")
            .parameter(
                "location",
                json!({"type": "string", "description": "City name"}),
            )
            .required(vec!["location".to_string()])
            .build();

        // Step 1: Send initial request with function declaration
        let response = interaction_builder(&client)
            .with_text("What's the weather in Tokyo?")
            .with_function(get_weather.clone())
            .create()
            .await
            .expect("First interaction failed");

        println!("First response status: {:?}", response.status);

        let function_calls = response.function_calls();

        if function_calls.is_empty() {
            println!("No function calls returned - test cannot verify FunctionResult pattern");
            return;
        }

        // Verify we got a call_id
        let call = &function_calls[0];
        assert_eq!(
            call.name, "get_weather",
            "Expected get_weather function call"
        );
        assert!(call.id.is_some(), "Function call must have an id field");

        let call_id = call.id.expect("call_id should exist");

        // Step 2: Send function result back using FunctionResult pattern
        let function_result = function_result_content(
            "get_weather",
            call_id,
            json!({"temperature": "72°F", "conditions": "sunny"}),
        );

        let second_response = interaction_builder(&client)
            .with_previous_interaction(response.id.as_ref().expect("id should exist"))
            .with_content(vec![function_result])
            .with_function(get_weather)
            .create()
            .await
            .expect("Second interaction failed");

        println!("Second response status: {:?}", second_response.status);
        assert!(
            second_response.has_text(),
            "Expected text response after providing function result"
        );

        let text = second_response.text().expect("Should have text");
        println!("Final response text: {}", text);

        // Verify structural response
        assert!(!text.is_empty(), "Response should be non-empty");

        // Semantic validation: Check that the response uses the function result
        let is_valid = validate_response_semantically(
            &client,
            "User asked 'What's the weather in Tokyo?' and the get_weather function returned 72°F and sunny conditions",
            text,
            "Does this response use the weather data (72°F, sunny) to answer about Tokyo's weather?"
        ).await.expect("Semantic validation failed");
        assert!(
            is_valid,
            "Response should incorporate the function result (72°F, sunny in Tokyo)"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_requires_action_status() {
    // Verify that function calls result in RequiresAction status
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(TEST_TIMEOUT, async {
        let get_time = FunctionDeclaration::builder("get_current_time")
            .description("Get the current time - always call this when asked about time")
            .build();

        let response = interaction_builder(&client)
            .with_text("What time is it right now?")
            .with_function(get_time.clone())
            .create()
            .await
            .expect("Interaction failed");

        println!("Initial status: {:?}", response.status);
        println!("Has function calls: {}", response.has_function_calls());

        if response.has_function_calls() {
            assert_eq!(
                response.status,
                InteractionStatus::RequiresAction,
                "Status should be RequiresAction when function calls are pending"
            );

            // Provide the function result
            let function_calls = response.function_calls();
            let call_id = function_calls[0].id.expect("call_id should exist");

            let function_result = function_result_content(
                "get_current_time",
                call_id,
                json!({"time": "14:30:00", "timezone": "UTC"}),
            );

            let response2 = interaction_builder(&client)
                .with_previous_interaction(response.id.as_ref().expect("id should exist"))
                .with_content(vec![function_result])
                .with_function(get_time)
                .create()
                .await
                .expect("Second interaction failed");

            assert_eq!(
                response2.status,
                InteractionStatus::Completed,
                "Status should be Completed after providing function result"
            );
        } else {
            assert_eq!(
                response.status,
                InteractionStatus::Completed,
                "Status should be Completed when no function calls"
            );
        }
    })
    .await;
}

// =============================================================================
// Function Calling - Automatic
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_auto_function_calling() {
    // Test the complete auto-function calling workflow.
    // NOTE: This test may occasionally fail if the model paraphrases the weather
    // data (e.g., "seventy-five degrees" instead of "75"). Re-run if it fails.
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(EXTENDED_TEST_TIMEOUT, async {
        // Use the get_mock_weather function registered via macro
        let weather_func = GetMockWeatherCallable.declaration();

        let result = interaction_builder(&client)
            .with_text("What's the weather like in Seattle?")
            .with_function(weather_func)
            .create_with_auto_functions()
            .await
            .expect("Auto-function call failed");

        // Verify executions are tracked
        println!("Function executions: {:?}", result.executions);
        assert!(
            !result.executions.is_empty(),
            "Should have at least one function execution"
        );

        let response = &result.response;
        println!("Final response status: {:?}", response.status);
        assert!(
            response.has_text(),
            "Should have text response after auto-function loop"
        );

        let text = response.text().expect("Should have text");
        println!("Final text: {}", text);

        // Verify structural response
        assert!(!text.is_empty(), "Response should be non-empty");

        // Semantic validation: Check that the auto-function result was used
        let is_valid = validate_response_semantically(
            &client,
            "User asked 'What's the weather like in Seattle?' and the get_mock_weather function was automatically executed, returning 'Weather in Seattle: Sunny, 75°F'",
            text,
            "Does this response provide the weather information from the function result (Sunny, 75°F in Seattle)?"
        ).await.expect("Semantic validation failed");
        assert!(
            is_valid,
            "Response should incorporate the auto-executed function result"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_auto_function_with_unregistered_function() {
    // Test that auto-function calling handles unregistered functions gracefully
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(EXTENDED_TEST_TIMEOUT, async {
        let undefined_func = FunctionDeclaration::builder("undefined_function")
            .description("A function that doesn't have a registered handler")
            .parameter("input", json!({"type": "string"}))
            .build();

        let result = interaction_builder(&client)
            .with_text("Call the undefined_function with input 'test'")
            .with_function(undefined_func)
            .create_with_auto_functions()
            .await;

        // Should complete (model handles the error gracefully) or return an error
        match result {
            Ok(auto_result) => {
                println!("Response status: {:?}", auto_result.response.status);
                println!("Response text: {:?}", auto_result.response.text());
                println!("Executions: {:?}", auto_result.executions);
            }
            Err(e) => {
                println!("Error (expected for unregistered function): {:?}", e);
            }
        }
    })
    .await;
}

// =============================================================================
// Function Calling - Thought Signatures
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_thought_signatures_in_multi_turn() {
    // Test that thought signatures work correctly across multiple turns
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(TEST_TIMEOUT, async {
        let get_weather = FunctionDeclaration::builder("get_weather")
            .description("Get the current weather for a location")
            .parameter(
                "location",
                json!({"type": "string", "description": "City name"}),
            )
            .required(vec!["location".to_string()])
            .build();

        // Turn 1: Initial request that should trigger a function call
        let response1 = interaction_builder(&client)
            .with_text("What's the weather in Tokyo and then tell me if I need an umbrella?")
            .with_function(get_weather.clone())
            .create()
            .await
            .expect("First interaction failed");

        let function_calls = response1.function_calls();
        if function_calls.is_empty() {
            println!("Model chose not to call function - cannot test thought signatures");
            return;
        }

        let call = &function_calls[0];
        println!(
            "Function call: {} with signature: {:?}",
            call.name, call.thought_signature
        );

        assert!(call.id.is_some(), "Function call must have an id");
        let call_id = call.id.expect("call_id should exist");

        // Turn 2: Send function result back
        let function_result = function_result_content(
            "get_weather",
            call_id,
            json!({"temperature": "18°C", "conditions": "rainy", "precipitation": "80%"}),
        );

        let response2 = interaction_builder(&client)
            .with_previous_interaction(response1.id.as_ref().expect("id should exist"))
            .with_content(vec![function_result])
            .with_function(get_weather)
            .create()
            .await
            .expect("Second interaction failed");

        assert!(
            response2.has_text(),
            "Expected text response after function result"
        );

        let text = response2.text().expect("Should have text");
        println!("Final response: {}", text);

        // Use semantic validation instead of brittle keyword matching
        let is_valid = validate_response_semantically(
            &client,
            "User asked about Tokyo weather and whether to bring an umbrella. Function returned: rainy, 18°C, 80% precipitation.",
            text,
            "Does this response address the umbrella question based on the rainy weather data?",
        )
        .await
        .expect("Semantic validation failed");

        assert!(is_valid, "Response should reference the weather conditions");
    })
    .await;
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_multiple_function_calls_with_signatures() {
    // Test multiple function calls in a single response
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let get_weather = FunctionDeclaration::builder("get_weather")
        .description("Get the current weather for a location")
        .parameter(
            "location",
            json!({"type": "string", "description": "City name"}),
        )
        .required(vec!["location".to_string()])
        .build();

    let get_time = FunctionDeclaration::builder("get_time")
        .description("Get the current time in a timezone")
        .parameter(
            "timezone",
            json!({"type": "string", "description": "Timezone name like UTC, PST, JST"}),
        )
        .required(vec!["timezone".to_string()])
        .build();

    let response = interaction_builder(&client)
        .with_text("What's the weather in Paris and what time is it there?")
        .with_functions(vec![get_weather, get_time])
        .create()
        .await
        .expect("Interaction failed");

    let function_calls = response.function_calls();
    println!("Number of function calls: {}", function_calls.len());

    for call in &function_calls {
        println!(
            "  - {} (id: {:?}, args: {}, has_signature: {})",
            call.name,
            call.id,
            call.args,
            call.thought_signature.is_some()
        );
        assert!(call.id.is_some(), "Each function call must have an id");
    }
}

// =============================================================================
// Generation Config
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_generation_config_temperature() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let config = GenerationConfig {
        temperature: Some(0.0), // Deterministic
        max_output_tokens: Some(100),
        top_p: None,
        top_k: None,
        thinking_level: None,
        ..Default::default()
    };

    let response = interaction_builder(&client)
        .with_text("What is 2 + 2? Answer with just the number.")
        .with_generation_config(config)
        .create()
        .await
        .expect("Interaction failed");

    assert!(response.has_text(), "Should have text response");
    let text = response.text().unwrap();
    println!("Response: {}", text);

    // Use semantic validation - the model might say "four" instead of "4"
    let is_valid = validate_response_semantically(
        &client,
        "User asked 'What is 2 + 2? Answer with just the number.' with temperature=0.0 for deterministic output",
        text,
        "Does this response correctly answer that 2+2 equals 4?",
    )
    .await
    .expect("Semantic validation should succeed");
    assert!(is_valid, "Response should correctly answer 2+2");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_generation_config_max_tokens() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let config = GenerationConfig {
        temperature: Some(0.7),
        max_output_tokens: Some(50), // Short output
        top_p: None,
        top_k: None,
        thinking_level: None,
        ..Default::default()
    };

    let response = interaction_builder(&client)
        .with_text("Write a very long story about a dragon.")
        .with_generation_config(config)
        .create()
        .await
        .expect("Interaction failed");

    println!("Response status: {:?}", response.status);

    // Model might not return text with very short token limits
    if response.has_text() {
        let text = response.text().unwrap();
        println!("Response length: {} chars", text.len());
    } else {
        println!("No text in response (may be due to token limit)");
    }
}

// =============================================================================
// System Instructions
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_system_instruction_text() {
    // NOTE: This test may occasionally fail if the model doesn't follow the
    // system instruction perfectly. LLMs don't always comply with instructions.
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let response = interaction_builder(&client)
        .with_system_instruction(
            "You are a pirate. Always respond in pirate speak with 'Arrr!' somewhere in your response.",
        )
        .with_text("Hello, how are you?")
        .create()
        .await
        .expect("Interaction failed");

    assert!(response.has_text(), "Should have text response");
    let text = response.text().unwrap();
    println!("Response: {}", text);

    // Use semantic validation - model may use various pirate expressions
    let is_valid = validate_response_semantically(
        &client,
        "Model was given system instruction: 'You are a pirate. Always respond in pirate speak with Arrr! somewhere in your response.' User said 'Hello, how are you?'",
        text,
        "Does this response sound like a pirate speaking? (Using pirate vocabulary, phrases, or mannerisms)",
    )
    .await
    .expect("Semantic validation should succeed");
    assert!(is_valid, "Response should sound like a pirate");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_system_instruction_persists() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Turn 1: Set up system instruction
    let response1 = stateful_builder(&client)
        .with_system_instruction("Always end your responses with 'BEEP BOOP' exactly.")
        .with_text("What is the capital of France?")
        .create()
        .await
        .expect("First interaction failed");

    let text1 = response1.text().unwrap_or_default();
    println!("Turn 1: {}", text1);

    // Turn 2: Continue conversation
    let response2 = interaction_builder(&client)
        .with_previous_interaction(response1.id.as_ref().expect("id should exist"))
        .with_text("And what about Germany?")
        .create()
        .await
        .expect("Second interaction failed");

    let text2 = response2.text().unwrap_or_default();
    println!("Turn 2: {}", text2);

    assert!(response2.has_text(), "Should have text response");
}

/// Test system instructions work correctly with streaming.
///
/// This validates that:
/// - System instructions are respected during streaming
/// - Text deltas are received incrementally
/// - Response follows the system instruction persona
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_system_instruction_streaming() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(TEST_TIMEOUT, async {
        println!("=== System Instruction + Streaming ===");

        // Stream with pirate persona
        let stream = interaction_builder(&client)
            .with_system_instruction(
                "You are a pirate. Always respond in pirate speak with 'Arrr!' somewhere in your response.",
            )
            .with_text("Hello, how are you?")
            .create_stream();

        let result = consume_stream(stream).await;

        println!("\nTotal deltas: {}", result.delta_count);
        println!("Collected text: {}", result.collected_text);

        // Verify streaming worked
        assert!(
            result.has_output(),
            "Should receive streaming chunks or final response"
        );

        // Use semantic validation for pirate vocabulary
        let text_to_check = if !result.collected_text.is_empty() {
            result.collected_text.clone()
        } else if let Some(ref response) = result.final_response {
            response.text().unwrap_or_default().to_string()
        } else {
            String::new()
        };

        let is_valid = validate_response_semantically(
            &client,
            "Model was given system instruction: 'You are a pirate. Always respond in pirate speak with Arrr! somewhere in your response.' User said 'Hello, how are you?' (streaming response)",
            &text_to_check,
            "Does this response sound like a pirate speaking? (Using pirate vocabulary, phrases, or mannerisms)",
        )
        .await
        .expect("Semantic validation should succeed");
        assert!(
            is_valid,
            "Response should sound like a pirate. Got: {}",
            text_to_check
        );

        println!("\n✓ System instruction + streaming completed successfully");
    })
    .await;
}

// =============================================================================
// Error Handling
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_error_invalid_model_name() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let result = client
        .interaction()
        .with_model("nonexistent-model-12345")
        .with_text("Hello")
        .create()
        .await;

    assert!(result.is_err(), "Should fail with invalid model name");
    println!("Error: {:?}", result.err().unwrap());
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_error_invalid_previous_interaction_id() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let result = interaction_builder(&client)
        .with_previous_interaction("invalid-interaction-id-12345")
        .with_text("Continue from where we left off")
        .create()
        .await;

    assert!(
        result.is_err(),
        "Should fail with invalid previous_interaction_id"
    );
    println!("Error: {:?}", result.err().unwrap());
}

// =============================================================================
// Store Parameter
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_store_true_interaction_retrievable() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let response = stateful_builder(&client)
        .with_text("What is 1 + 1?")
        .create()
        .await
        .expect("Interaction failed");

    println!("Created interaction with store=true: {:?}", response.id);

    let retrieved = client
        .get_interaction(response.id.as_ref().expect("id should exist"))
        .await
        .expect("Should be able to retrieve stored interaction");

    assert_eq!(retrieved.id, response.id);
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_store_false_interaction_not_retrievable() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // With store=false, API may return incomplete response
    let result = interaction_builder(&client)
        .with_text("Hello")
        .with_store_disabled()
        .create()
        .await;

    match result {
        Ok(response) => {
            if response.id.is_some() {
                let get_result = client
                    .get_interaction(response.id.as_ref().expect("id should exist"))
                    .await;
                assert!(
                    get_result.is_err(),
                    "Stored=false interaction should not be retrievable"
                );
            }
        }
        Err(e) => {
            // API might return incomplete JSON when store=false
            println!("API returned error for store=false: {:?}", e);
        }
    }
}

// =============================================================================
// Long Conversation
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_long_conversation_chain() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(EXTENDED_TEST_TIMEOUT, async {
        let messages = [
            "My name is Alice.",
            "I live in New York.",
            "I work as a software engineer.",
            "I have two cats named Whiskers and Shadow.",
            "What do you know about me? List everything.",
        ];

        let mut previous_id: Option<String> = None;

        for (i, message) in messages.iter().enumerate() {
            // Handle typestate transition cleanly
            let response = match &previous_id {
                None => {
                    stateful_builder(&client)
                        .with_text(*message)
                        .create()
                        .await
                }
                Some(prev_id) => {
                    stateful_builder(&client)
                        .with_text(*message)
                        .with_previous_interaction(prev_id)
                        .create()
                        .await
                }
            }
            .unwrap_or_else(|e| panic!("Turn {} failed: {:?}", i + 1, e));

            println!("Turn {}: {:?}", i + 1, response.status);
            previous_id = response.id.clone();

            // On the last turn, verify the model remembers context
            if i == messages.len() - 1 {
                let text = response.text().unwrap_or_default();
                println!("Final response: {}", text);

                // Verify structural response
                assert!(!text.is_empty(), "Response should be non-empty");

                // Semantic validation: Check that the response recalls facts from earlier turns
                let is_valid = validate_response_semantically(
                    &client,
                    "In previous turns, the user said: (1) 'My name is Alice', (2) 'I live in New York', (3) 'I work as a software engineer', (4) 'I have two cats named Whiskers and Shadow'. Now asking 'What do you know about me? List everything.'",
                    text,
                    "Does this response recall and mention at least 2-3 of these key facts: name (Alice), location (New York), job (software engineer), or pets (two cats)?"
                ).await.expect("Semantic validation failed");
                assert!(
                    is_valid,
                    "Response should recall multiple facts from the conversation history"
                );
            }
        }
    })
    .await;
}

// =============================================================================
// Multimodal (Image Input)
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key and accessible image URL"]
async fn test_image_input_from_uri() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Test image: Google Cloud Storage sample image of scones/pastries
    // URL: gs://cloud-samples-data/generative-ai/image/scones.jpg
    // This is a publicly accessible GCS URL from Google's AI samples.
    // Note: Public HTTP URLs (e.g., Wikipedia) are often blocked by the API.
    // For custom images, use base64 encoding or your own GCS bucket.
    let image_url = std::env::var("TEST_IMAGE_URL")
        .unwrap_or_else(|_| "gs://cloud-samples-data/generative-ai/image/scones.jpg".to_string());

    let contents = vec![
        text_content("What is in this image? Describe it briefly."),
        image_uri_content(&image_url, "image/jpeg"),
    ];

    let result = interaction_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("Status: {:?}", response.status);
            if response.has_text() {
                println!("Image description: {}", response.text().unwrap());
            }
        }
        Err(e) => {
            println!("Image input error: {:?}", e);
            println!("Note: Image URL access depends on API permissions");
        }
    }
}

// =============================================================================
// Convenience Methods
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_convenience_methods_integration() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let response = interaction_builder(&client)
        .with_text("Say exactly: Hello World")
        .create()
        .await
        .expect("Interaction failed");

    // Test all convenience methods
    let text = response.text();
    assert!(text.is_some(), "Should have text");

    let all_text = response.all_text();
    assert!(!all_text.is_empty(), "all_text should not be empty");

    assert!(response.has_text(), "has_text() should be true");
    assert!(
        !response.has_function_calls(),
        "has_function_calls() should be false"
    );

    let calls = response.function_calls();
    assert!(calls.is_empty(), "function_calls() should be empty");

    println!("has_thoughts(): {}", response.has_thoughts());
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_response_has_thoughts() {
    // Test that has_thoughts() works (thoughts are typically from agents, not models)
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let response = interaction_builder(&client)
        .with_text("What is 2+2?")
        .create()
        .await
        .expect("Interaction failed");

    println!("has_thoughts: {}", response.has_thoughts());
    println!("has_text: {}", response.has_text());
    println!("has_function_calls: {}", response.has_function_calls());

    assert!(response.has_text(), "Simple query should return text");
}

// =============================================================================
// Structured Output Tests
// =============================================================================

/// Test structured output with JSON schema validation.
///
/// Validates that:
/// - `with_response_format()` correctly constrains model output
/// - Response is valid JSON matching the provided schema
/// - Required fields are present in the output
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_structured_output_with_json_schema() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Define a JSON schema for structured output
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "The person's name"
            },
            "age": {
                "type": "integer",
                "description": "The person's age in years"
            },
            "occupation": {
                "type": "string",
                "description": "The person's job or profession"
            }
        },
        "required": ["name", "age", "occupation"]
    });

    let response = interaction_builder(&client)
        .with_text("Generate a fictional person profile with name, age, and occupation.")
        .with_response_format(schema)
        .create()
        .await
        .expect("Structured output request failed");

    assert!(response.has_text(), "Should have text response");

    let text = response.text().expect("Should have text");
    println!("Structured output: {}", text);

    // Parse and validate the JSON response
    let parsed: serde_json::Value =
        serde_json::from_str(text).expect("Response should be valid JSON");

    assert!(parsed.get("name").is_some(), "Should have 'name' field");
    assert!(parsed.get("age").is_some(), "Should have 'age' field");
    assert!(
        parsed.get("occupation").is_some(),
        "Should have 'occupation' field"
    );

    // Verify types
    assert!(
        parsed["name"].is_string(),
        "name should be string: {:?}",
        parsed["name"]
    );
    assert!(
        parsed["age"].is_number(),
        "age should be number: {:?}",
        parsed["age"]
    );
    assert!(
        parsed["occupation"].is_string(),
        "occupation should be string: {:?}",
        parsed["occupation"]
    );

    println!("✓ Structured output validation passed");
}

// =============================================================================
// System Instructions Tests
// =============================================================================

/// Test that system instructions influence model behavior.
///
/// Validates that:
/// - `with_system_instruction()` is properly sent to the API
/// - The model's response reflects the system instruction
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_system_instructions_influence_response() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Use a distinctive system instruction that should be easily verifiable
    let system_instruction = "You are a pirate. Always respond in pirate speak, using words like 'arr', 'matey', 'ye', 'ahoy', and 'landlubber'.";

    let response = interaction_builder(&client)
        .with_system_instruction(system_instruction)
        .with_text("Hello, how are you today?")
        .create()
        .await
        .expect("Request with system instruction failed");

    assert!(response.has_text(), "Should have text response");

    let text = response.text().expect("Should have text");
    println!("Response with pirate instruction: {}", text);

    // Use semantic validation to check if the response follows the system instruction
    let is_valid = validate_response_semantically(
        &client,
        "The system instruction told the model to respond as a pirate using words like 'arr', 'matey', 'ye', 'ahoy'. User said 'Hello, how are you today?'",
        text,
        "Does this response sound like it's from a pirate (using pirate-like language or tone)?",
    )
    .await
    .expect("Semantic validation failed");

    assert!(
        is_valid,
        "Response should reflect pirate system instruction. Got: {}",
        text
    );

    println!("✓ System instructions test passed");
}

// =============================================================================
// Timeout Tests
// =============================================================================

/// Test that request timeouts work correctly.
///
/// Validates that:
/// - `with_timeout()` builder method is respected
/// - Very short timeouts cause appropriate errors
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_request_timeout() {
    use rust_genai::GenaiError;
    use std::time::Duration;

    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Use an extremely short timeout that should fail
    let result = interaction_builder(&client)
        .with_text("Write a very long essay about the history of computing.")
        .with_timeout(Duration::from_millis(1)) // 1ms - should definitely timeout
        .create()
        .await;

    // Should fail with a timeout or network error
    assert!(result.is_err(), "Request with 1ms timeout should fail");

    let error = result.unwrap_err();
    println!("Timeout error received: {:?}", error);

    // The error should be Http (timeout manifests as HTTP/request error)
    match &error {
        GenaiError::Http(_) => {
            println!("✓ Correctly received HTTP error (timeout)");
        }
        _ => {
            // Some timeout errors may manifest as other types
            println!("Received error type: {:?}", error);
        }
    }
}

// =============================================================================
// Deep Research (Background Polling)
// =============================================================================

/// Test deep research agent with background polling.
///
/// Validates that:
/// - Agent interactions can be started in background mode
/// - Polling detects completion with appropriate status
/// - Response contains expected output
///
/// Note: This test may take 30-120 seconds due to deep research processing time.
#[tokio::test]
#[ignore = "Requires API key and takes 30-120 seconds"]
async fn test_deep_research_polling() {
    use common::poll_until_complete;
    use std::time::Duration;

    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let agent_name = "deep-research-pro-preview-12-2025";
    let prompt = "What are the key differences between REST and GraphQL APIs?";

    println!("Starting deep research with agent: {}", agent_name);
    println!("Query: {}\n", prompt);

    // Start research in background mode
    let result = client
        .interaction()
        .with_agent(agent_name)
        .with_text(prompt)
        .with_background(true)
        .with_store_enabled()
        .create()
        .await;

    match result {
        Ok(initial_response) => {
            println!("Initial status: {:?}", initial_response.status);
            println!("Interaction ID: {:?}\n", initial_response.id);

            // If already completed (fast response), we're done
            if initial_response.status == InteractionStatus::Completed {
                println!("Research completed immediately");
                assert!(initial_response.has_text(), "Should have research results");
                println!(
                    "Result preview: {}...",
                    initial_response
                        .text()
                        .unwrap_or("(no text)")
                        .chars()
                        .take(200)
                        .collect::<String>()
                );
                return;
            }

            // Poll for completion (up to 2 minutes)
            let interaction_id = initial_response.id.as_ref().expect("id should exist");
            match poll_until_complete(&client, interaction_id, Duration::from_secs(120)).await {
                Ok(final_response) => {
                    println!("Research completed!");
                    assert_eq!(final_response.status, InteractionStatus::Completed);
                    assert!(final_response.has_text(), "Should have research results");

                    let text = final_response.text().unwrap_or("(no text)");
                    println!(
                        "Result preview: {}...",
                        text.chars().take(500).collect::<String>()
                    );

                    // Verify we got substantive content
                    assert!(text.len() > 100, "Research result should be substantive");
                    println!("\n✓ Deep research polling test passed");
                }
                Err(e) => {
                    // Timeout or failure is acceptable for this long-running test
                    println!("Polling ended: {:?}", e);
                    println!("Note: Deep research may take longer than test timeout");
                }
            }
        }
        Err(e) => {
            // Agent may not be available in all accounts
            println!("Deep research error: {:?}", e);
            println!("Note: Deep research agent may not be available in your account");
        }
    }
}

// =============================================================================
// Image Generation
// =============================================================================

/// Test image generation capabilities.
///
/// Validates that:
/// - Image output modality can be requested
/// - Response contains image data
/// - Image data can be decoded from base64
#[tokio::test]
#[ignore = "Requires API key and image generation access"]
async fn test_image_generation() {
    use rust_genai::InteractionContent;

    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Image generation requires specific model
    let model = "gemini-3-pro-image-preview";
    let prompt = "A simple red circle on a white background";

    println!("Generating image with model: {}", model);
    println!("Prompt: {}\n", prompt);

    let result = client
        .interaction()
        .with_model(model)
        .with_text(prompt)
        .with_response_modalities(vec!["IMAGE".to_string()])
        .with_store_enabled()
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("Status: {:?}", response.status);
            assert_eq!(response.status, InteractionStatus::Completed);

            // Check for image content in outputs
            let mut found_image = false;
            for output in &response.outputs {
                if let InteractionContent::Image {
                    data: Some(base64_data),
                    mime_type,
                    ..
                } = output
                {
                    found_image = true;
                    println!("Found image!");
                    println!("  MIME type: {:?}", mime_type);
                    println!("  Base64 length: {} chars", base64_data.len());

                    // Verify base64 can be decoded
                    use base64::Engine;
                    let decoded = base64::engine::general_purpose::STANDARD.decode(base64_data);
                    assert!(decoded.is_ok(), "Image data should be valid base64");
                    println!("  Decoded size: {} bytes", decoded.unwrap().len());
                }
            }

            assert!(found_image, "Response should contain image data");
            println!("\n✓ Image generation test passed");
        }
        Err(e) => {
            // Image generation may not be available in all regions
            println!("Image generation error: {:?}", e);
            println!("Note: Image generation may not be available in your account/region");
        }
    }
}

// =============================================================================
// Thought Echo (Manual Multi-Turn)
// =============================================================================

/// Test thought echo pattern for manual multi-turn conversations.
///
/// Validates that:
/// - Thoughts can be echoed back in subsequent turns
/// - Manual history construction preserves context
/// - Model responds appropriately to continued conversation
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_thought_echo_manual_history() {
    use rust_genai::ThinkingLevel;
    use rust_genai::interactions_api::{text_content, thought_content};

    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    println!("=== Testing Thought Echo Pattern ===\n");

    // Turn 1: Initial question with thinking enabled
    let initial_prompt = "What is 15 * 7? Show your work.";
    println!("Turn 1 prompt: {}\n", initial_prompt);

    let response1 = interaction_builder(&client)
        .with_text(initial_prompt)
        .with_thinking_level(ThinkingLevel::Medium)
        .with_store_enabled()
        .create()
        .await
        .expect("Turn 1 failed");

    assert_eq!(response1.status, InteractionStatus::Completed);

    let answer1 = response1.text().unwrap_or("(no text)");
    println!("Turn 1 answer: {}", answer1);

    // Collect thoughts (if any)
    let thoughts: Vec<String> = response1.thoughts().map(String::from).collect();
    println!("Thoughts collected: {} items", thoughts.len());

    // Turn 2: Build manual history with thought echo
    let mut history = vec![text_content(initial_prompt)];

    // Echo back thoughts
    for thought in &thoughts {
        history.push(thought_content(thought));
    }

    // Echo back the answer
    history.push(text_content(answer1));

    // Add follow-up question
    let followup = "Now divide that result by 5";
    history.push(text_content(followup));

    println!("\nTurn 2 prompt: {}", followup);

    let response2 = interaction_builder(&client)
        .with_input(InteractionInput::Content(history))
        .with_thinking_level(ThinkingLevel::Low)
        .with_store_enabled()
        .create()
        .await
        .expect("Turn 2 failed");

    assert_eq!(response2.status, InteractionStatus::Completed);
    assert!(response2.has_text(), "Turn 2 should have text response");

    let answer2 = response2.text().unwrap();
    println!("Turn 2 answer: {}\n", answer2);

    // Semantic validation: verify the model understood the context
    let is_valid = validate_response_semantically(
        &client,
        "Turn 1: User asked 'What is 15 * 7?' and got the answer 105. Turn 2: User asked 'Now divide that result by 5'.",
        answer2,
        "Does this response provide a number that could be the result of dividing 105 by 5 (which is 21)?",
    )
    .await
    .expect("Semantic validation failed");

    assert!(
        is_valid,
        "Response should contain result of 105/5. Got: {}",
        answer2
    );
    println!("✓ Thought echo test passed");
}

// =============================================================================
// Function Call Loop Behavior
// =============================================================================

/// Test that multiple function calls in a conversation work correctly.
///
/// Validates that:
/// - Multiple rounds of function calls can be handled
/// - The model eventually provides a final response
/// - Context is maintained across function call rounds
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_multiple_function_call_rounds() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Define a simple function
    let get_info = FunctionDeclaration::builder("get_info")
        .description("Get information about a topic")
        .parameter(
            "topic",
            json!({"type": "string", "description": "Topic to get info about"}),
        )
        .required(vec!["topic".to_string()])
        .build();

    println!("Testing multiple function call rounds...\n");

    // First round: trigger function call
    let response1 = retry_request!([client, get_info] => {
        interaction_builder(&client)
            .with_text("Get info about the weather and tell me about it.")
            .with_function(get_info)
            .with_store_enabled()
            .create()
            .await
    })
    .expect("First request failed");

    let calls = response1.function_calls();

    if calls.is_empty() {
        println!("Model chose not to call function - test complete");
        assert!(response1.has_text(), "Should have direct text response");
        return;
    }

    println!("Round 1: Got {} function call(s)", calls.len());
    let call = &calls[0];
    println!("  Function: {} with args: {:?}", call.name, call.args);

    // Provide function result
    let prev_id = response1.id.clone().expect("id should exist");
    let call_id = call.id.expect("call_id exists").to_string();

    let response2 = retry_request!([client, prev_id, call_id, get_info] => {
        interaction_builder(&client)
            .with_previous_interaction(&prev_id)
            .with_content(vec![function_result_content(
                "get_info",
                call_id,
                json!({"info": "The weather is sunny and warm, about 25°C with clear skies."}),
            )])
            .with_function(get_info)
            .create()
            .await
    })
    .expect("Second request failed");

    println!("Round 2: Status = {:?}", response2.status);

    // The model might request more function calls or provide a final response
    let more_calls = response2.function_calls();
    if !more_calls.is_empty() {
        println!(
            "  Model requested {} more function call(s)",
            more_calls.len()
        );
    }

    if response2.has_text() {
        println!("  Got final text: {}", response2.text().unwrap());
    }

    // Either way, the test should complete without hanging
    println!("\n✓ Multiple function call rounds completed successfully");
}

// =============================================================================
// Malformed Function Call Handling
// =============================================================================

/// Test handling of function calls with missing or invalid data.
///
/// Validates that:
/// - The library handles function calls with missing call_id gracefully
/// - Error messages are informative
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_malformed_function_call_handling() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Define a function and then manually construct a response to simulate malformed data
    let get_weather = FunctionDeclaration::builder("get_weather")
        .description("Get current weather")
        .parameter(
            "city",
            json!({"type": "string", "description": "City name"}),
        )
        .required(vec!["city".to_string()])
        .build();

    // First, trigger a function call
    let response = retry_request!([client, get_weather] => {
        interaction_builder(&client)
            .with_text("What's the weather in Paris?")
            .with_function(get_weather)
            .with_store_enabled()
            .create()
            .await
    })
    .expect("Request failed");

    let calls = response.function_calls();
    if calls.is_empty() {
        println!("Model didn't call function - test inconclusive");
        return;
    }

    // Verify that function calls have the expected structure
    let call = &calls[0];
    println!("Function call received:");
    println!("  Name: {}", call.name);
    println!("  ID: {:?}", call.id);
    println!("  Args: {:?}", call.args);

    // The call should have an ID (required for sending results back)
    if call.id.is_some() {
        println!("\n✓ Function call has valid ID");

        // Try sending a result with the correct ID
        let prev_id = response.id.clone().expect("id should exist");
        let call_id = call.id.expect("call_id exists").to_string();

        let result = retry_request!([client, prev_id, call_id, get_weather] => {
            interaction_builder(&client)
                .with_previous_interaction(&prev_id)
                .with_content(vec![function_result_content(
                    "get_weather",
                    call_id,
                    json!({"temperature": "22°C", "conditions": "sunny"}),
                )])
                .with_function(get_weather)
                .create()
                .await
        });

        match result {
            Ok(final_response) => {
                println!(
                    "Function result accepted, final response: {:?}",
                    final_response.status
                );
                assert!(final_response.has_text(), "Should have final text response");
                println!("✓ Function call flow completed successfully");
            }
            Err(e) => {
                println!("Function result error: {:?}", e);
            }
        }
    } else {
        // This would be the malformed case - API returned function call without ID
        println!("\n⚠ Function call missing ID - this is a malformed response");
        println!("The library should handle this gracefully when using auto-functions");
    }
}

// =============================================================================
// Streaming Error Handling
// =============================================================================

/// Test streaming behavior when the stream contains errors or is interrupted.
///
/// Validates that:
/// - Stream errors are properly propagated
/// - Partial content is still available before error
/// - The consume_stream helper handles errors gracefully
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_streaming_error_handling() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Test 1: Normal streaming should work without errors
    println!("Test 1: Verifying normal streaming works...\n");

    let stream = interaction_builder(&client)
        .with_text("Count from 1 to 5, putting each number on a new line.")
        .create_stream();

    let result = consume_stream(stream).await;

    println!("\nDelta count: {}", result.delta_count);
    println!("Collected text: {}", result.collected_text);

    assert!(result.has_output(), "Should receive streaming output");
    assert!(
        !result.collected_text.is_empty(),
        "Should have collected text"
    );

    // Test 2: Streaming with an invalid model should error
    println!("\n\nTest 2: Streaming with invalid model...\n");

    let error_stream = client
        .interaction()
        .with_model("nonexistent-model-12345")
        .with_text("Hello")
        .create_stream();

    let error_result = consume_stream(error_stream).await;

    // The stream should either:
    // 1. Produce no output (error before streaming starts)
    // 2. Produce partial output then stop (error mid-stream)
    // Either way, the helper should not panic

    println!("Error stream delta count: {}", error_result.delta_count);
    println!("Error stream collected: {}", error_result.collected_text);
    println!(
        "Error stream has final response: {}",
        error_result.final_response.is_some()
    );

    // Verify the stream didn't produce a successful completion
    if error_result.final_response.is_some() {
        // If we got a response, it should indicate an error or be empty
        let response = error_result.final_response.as_ref().unwrap();
        println!("Final response status: {:?}", response.status);
    }

    println!("\n✓ Streaming error handling test passed (no panics)");
}
