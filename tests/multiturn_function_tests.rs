//! Multi-turn conversation tests with function calling
//!
//! These tests verify the typestate-enforced constraints for multi-turn conversations
//! combined with automatic and manual function calling.
//!
//! Key patterns tested:
//! - FirstTurn with system_instruction -> Chained (auto functions)
//! - FirstTurn with system_instruction -> Chained (manual functions)
//! - Tools must be resent on each turn (not inherited)
//! - System instruction IS inherited via previousInteractionId
//!
//! # Running Tests
//!
//! ```bash
//! cargo test --test multiturn_function_tests -- --include-ignored --nocapture
//! ```

mod common;

use common::{get_client, retry_on_any_error, validate_response_semantically};
use genai_rs::{FunctionDeclaration, InteractionStatus, function_result_content};
use serde_json::json;

// =============================================================================
// Test Constants
// =============================================================================

const SYSTEM_INSTRUCTION: &str = "You are a helpful assistant that uses available tools when appropriate. Always respond concisely.";

// =============================================================================
// Helper: Create test function declarations
// =============================================================================

fn get_weather_function() -> FunctionDeclaration {
    FunctionDeclaration::builder("get_weather")
        .description("Get the current weather for a city")
        .parameter(
            "city",
            json!({"type": "string", "description": "City name"}),
        )
        .required(vec!["city".to_string()])
        .build()
}

fn get_time_function() -> FunctionDeclaration {
    FunctionDeclaration::builder("get_time")
        .description("Get the current time in a timezone")
        .parameter(
            "timezone",
            json!({"type": "string", "description": "Timezone like PST, EST, JST"}),
        )
        .required(vec!["timezone".to_string()])
        .build()
}

// =============================================================================
// Multi-turn + Auto Function Calling Tests
// =============================================================================

/// Test multi-turn conversation with auto function calling.
///
/// Pattern:
/// 1. FirstTurn: Set system instruction + trigger function call
/// 2. Chained: Continue conversation (system instruction inherited, tools resent)
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_multiturn_auto_functions_happy_path() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let functions = vec![get_weather_function()];

    // Turn 1: FirstTurn with system instruction, trigger function call
    println!("--- Turn 1: FirstTurn with system instruction ---");
    let result1 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's the weather in Seattle?")
        .with_functions(functions.clone())
        .with_store_enabled()
        .with_system_instruction(SYSTEM_INSTRUCTION)
        .create_with_auto_functions()
        .await
        .expect("Turn 1 should succeed");

    let response1 = result1.response;
    println!("Turn 1 status: {:?}", response1.status);
    println!("Turn 1 function executions: {}", result1.executions.len());

    // Verify function was called and executed
    assert!(
        result1.executions.iter().any(|e| e.name == "get_weather"),
        "Should have executed get_weather function"
    );

    let turn1_id = response1.id.clone().expect("Turn 1 should have ID");

    // Turn 2: Chained (system instruction inherited, must resend tools)
    println!("\n--- Turn 2: Chained conversation ---");
    let result2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("How about in Tokyo?") // Context from turn 1 should carry over
        .with_functions(functions.clone()) // Must resend tools
        .with_store_enabled()
        .with_previous_interaction(&turn1_id)
        // Note: with_system_instruction() is NOT available here (compile-time enforced)
        .create_with_auto_functions()
        .await
        .expect("Turn 2 should succeed");

    let response2 = result2.response;
    println!("Turn 2 status: {:?}", response2.status);
    println!("Turn 2 function executions: {}", result2.executions.len());

    // Model should understand context and call function again
    assert!(
        result2.executions.iter().any(|e| e.name == "get_weather"),
        "Should have executed get_weather function for Tokyo"
    );

    // Turn 3: Follow-up question (no function call expected)
    // Note: Include context in prompt since previousInteractionId may not preserve
    // function execution results reliably across all scenarios
    println!("\n--- Turn 3: Follow-up without function call ---");
    let turn2_id = response2.id.clone().expect("Turn 2 should have ID");

    let result3 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Based on the weather data you just retrieved (Seattle and Tokyo), which city is warmer?")
        .with_functions(functions) // Still resend tools in case needed
        .with_store_enabled()
        .with_previous_interaction(&turn2_id)
        .create_with_auto_functions()
        .await
        .expect("Turn 3 should succeed");

    let response3 = result3.response;
    println!("Turn 3 status: {:?}", response3.status);
    assert!(response3.has_text(), "Turn 3 should have text response");

    let text = response3.text().unwrap();
    println!("Turn 3 response: {}", text);

    // Model should reference both cities from context - use semantic validation
    let is_valid = validate_response_semantically(
        &client,
        "Previous turns got weather for Seattle (65F) and Tokyo (72F). Asked which city is warmer based on retrieved data.",
        text,
        "Does this response compare the temperatures or identify which city is warmer?",
    )
    .await
    .expect("Semantic validation failed");
    assert!(
        is_valid,
        "Response should reference the weather comparison context"
    );
}

/// Test that tools are NOT inherited via previousInteractionId.
///
/// If we don't resend tools on a chained turn, the model shouldn't be able
/// to call functions even if they were available on the first turn.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_multiturn_tools_not_inherited() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let functions = vec![get_weather_function()];

    // Turn 1: FirstTurn with functions
    println!("--- Turn 1: FirstTurn with functions ---");
    let result1 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Remember: I'm interested in weather data.")
        .with_functions(functions)
        .with_store_enabled()
        .create()
        .await
        .expect("Turn 1 should succeed");

    let turn1_id = result1.id.clone().expect("Turn 1 should have ID");
    println!("Turn 1 completed: {:?}", result1.status);

    // Turn 2: Chained WITHOUT resending tools
    println!("\n--- Turn 2: Chained WITHOUT tools ---");
    let result2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's the weather in Paris?")
        // NOT resending tools - model should not be able to call functions
        .with_store_enabled()
        .with_previous_interaction(&turn1_id)
        .create()
        .await
        .expect("Turn 2 should succeed");

    println!("Turn 2 status: {:?}", result2.status);

    // Model should NOT have any function calls since we didn't provide tools
    let function_calls = result2.function_calls();
    assert!(
        function_calls.is_empty(),
        "Model should not make function calls when tools not provided (got {} calls)",
        function_calls.len()
    );

    // Model should respond with text instead
    assert!(
        result2.has_text() || result2.status == InteractionStatus::Completed,
        "Model should complete with text response or indicate completion"
    );
}

// =============================================================================
// Multi-turn + Manual Function Calling Tests
// =============================================================================

/// Test multi-turn conversation with manual function calling loop.
///
/// Pattern:
/// 1. FirstTurn: Set system instruction + trigger function call
/// 2. Manual loop: Execute functions and send results
/// 3. Chained: Continue conversation
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_multiturn_manual_functions_happy_path() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let functions = vec![get_weather_function(), get_time_function()];

    // Turn 1: FirstTurn with system instruction
    println!("--- Turn 1: FirstTurn with system instruction ---");
    let response1 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's the weather in London and what time is it there?")
        .with_functions(functions.clone())
        .with_store_enabled()
        .with_system_instruction(SYSTEM_INSTRUCTION)
        .create()
        .await
        .expect("Turn 1 should succeed");

    println!("Turn 1 status: {:?}", response1.status);

    let function_calls = response1.function_calls();
    println!("Turn 1 function calls: {}", function_calls.len());

    // Should have function calls (may be parallel)
    assert!(
        !function_calls.is_empty(),
        "Turn 1 should trigger function calls"
    );

    // Execute functions manually
    let mut results = Vec::new();
    for call in &function_calls {
        let call_id = call.id.expect("Function call should have ID");
        let result = match call.name {
            "get_weather" => {
                json!({"city": "London", "temperature": "15°C", "conditions": "cloudy"})
            }
            "get_time" => json!({"timezone": "GMT", "time": "14:30:00"}),
            _ => json!({"error": "Unknown function"}),
        };
        println!("  Executing {} -> {:?}", call.name, result);
        results.push(function_result_content(
            call.name.to_string(),
            call_id.to_string(),
            result,
        ));
    }

    // Send function results back
    println!("\n--- Sending function results ---");
    let response2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_previous_interaction(response1.id.as_ref().expect("Should have ID"))
        .with_content(results)
        .with_functions(functions.clone()) // Resend tools
        .with_store_enabled()
        .create()
        .await
        .expect("Function result submission should succeed");

    println!("After results status: {:?}", response2.status);
    assert!(
        response2.has_text(),
        "Should have text response after function results"
    );
    println!("Response: {}", response2.text().unwrap());

    // Turn 3: Chained follow-up
    println!("\n--- Turn 3: Chained follow-up ---");
    let response3 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Is it a good time to call someone there?")
        .with_functions(functions) // Resend tools
        .with_store_enabled()
        .with_previous_interaction(response2.id.as_ref().expect("Should have ID"))
        .create()
        .await
        .expect("Turn 3 should succeed");

    println!("Turn 3 status: {:?}", response3.status);

    // Model might answer directly or call get_time again
    if response3.has_text() {
        let text = response3.text().unwrap();
        println!("Turn 3 response: {}", text);
        // Should reference the time context - use semantic validation
        let is_valid = validate_response_semantically(
            &client,
            "Previous turn got time 14:00 in London. Asked 'Is it a good time to call someone there?'",
            text,
            "Does this response address whether it's a good time to call (yes/no) or reference the time?",
        )
        .await
        .expect("Semantic validation failed");
        assert!(
            is_valid,
            "Response should address the calling time question"
        );
    }
}

/// Test error handling in manual function calling loop.
///
/// Simulate a function that returns an error and verify the model
/// handles it gracefully.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_multiturn_manual_functions_error_handling() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let secret_function = FunctionDeclaration::builder("get_secret_data")
        .description("Get secret data that requires special permissions")
        .parameter(
            "key",
            json!({"type": "string", "description": "The secret key"}),
        )
        .required(vec!["key".to_string()])
        .build();

    // Turn 1: Trigger function that will "fail"
    println!("--- Turn 1: Trigger function that will fail ---");
    let response1 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Get the secret data for key 'test123'")
        .with_function(secret_function.clone())
        .with_store_enabled()
        .with_system_instruction(
            "You help users access data. When functions fail, explain the error to the user.",
        )
        .create()
        .await
        .expect("Turn 1 should succeed");

    let function_calls = response1.function_calls();
    assert!(!function_calls.is_empty(), "Should trigger function call");

    // Return error result
    let call = &function_calls[0];
    let error_result = function_result_content(
        "get_secret_data".to_string(),
        call.id.expect("Should have ID").to_string(),
        json!({"error": "Permission denied: insufficient privileges for key 'test123'"}),
    );

    println!("\n--- Sending error result ---");
    let response2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_previous_interaction(response1.id.as_ref().expect("Should have ID"))
        .with_content(vec![error_result])
        .with_function(secret_function)
        .with_store_enabled()
        .create()
        .await
        .expect("Error result submission should succeed");

    println!("After error status: {:?}", response2.status);
    assert!(
        response2.has_text(),
        "Should have text response explaining error"
    );

    let text = response2.text().unwrap();
    println!("Response: {}", text);

    // Model should explain the error to user - use semantic validation to avoid brittle keyword matching
    let is_valid = validate_response_semantically(
        &client,
        "A function call to get_secret_data returned an error: 'Permission denied: insufficient privileges for key test123'. The system instruction says to explain errors to the user.",
        text,
        "Does this response appropriately explain to the user that accessing the secret data failed due to a permissions/privileges issue?",
    )
    .await
    .expect("Semantic validation should succeed");
    assert!(is_valid, "Response should explain the permission error");
}

// =============================================================================
// Edge Cases and Constraints Tests
// =============================================================================

/// Test that system instruction is correctly inherited in chained interactions.
///
/// This test verifies that a system instruction set on the first turn
/// affects behavior on subsequent turns without needing to resend it.
///
/// Note: Uses retry logic because LLM behavior is non-deterministic and the
/// semantic validation may fail intermittently even when the API works correctly.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_system_instruction_inheritance() {
    use std::time::Duration;

    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Retry up to 2 times (3 total attempts) because LLM behavior is non-deterministic
    // and the semantic validation may fail even when the API works correctly
    type BoxError = Box<dyn std::error::Error + Send + Sync>;
    let result: Result<(), BoxError> = retry_on_any_error(2, Duration::from_secs(3), || {
        let client = client.clone();
        async move {
            // Turn 1: Set system instruction to always respond in haiku format
            println!("--- Turn 1: Set haiku system instruction ---");
            let response1 = client
                .interaction()
                .with_model("gemini-3-flash-preview")
                .with_text("Hello!")
                .with_store_enabled()
                .with_system_instruction("You are a haiku poet. Always respond in haiku format (5-7-5 syllables). Never break from this format.")
                .create()
                .await?;

            let turn1_id = response1
                .id
                .clone()
                .ok_or("Turn 1 should have ID")?;
            println!(
                "Turn 1 response: {}",
                response1.text().unwrap_or("(no text)")
            );

            // Turn 2: Chained - system instruction should still be in effect
            println!("\n--- Turn 2: Verify system instruction still active ---");
            let response2 = client
                .interaction()
                .with_model("gemini-3-flash-preview")
                .with_text("Tell me about the ocean.")
                .with_store_enabled()
                .with_previous_interaction(&turn1_id)
                // Note: NOT resending system instruction - it should be inherited
                .create()
                .await?;

            println!(
                "Turn 2 response: {}",
                response2.text().unwrap_or("(no text)")
            );

            // Use semantic validation to verify the system instruction was inherited.
            // We're testing that previousInteractionId carries the system instruction forward,
            // not that the LLM perfectly follows format constraints.
            let text = response2
                .text()
                .ok_or("Turn 2 should have text")?;

            let is_valid = validate_response_semantically(
                &client,
                "The model was given a system instruction to 'always respond in haiku format'. User asked 'Tell me about the ocean.' in a follow-up turn (chained via previousInteractionId, not resending the system instruction).",
                text,
                "Does this response contain a haiku or show evidence of trying to follow a haiku/poetry constraint? (The response may include additional content beyond the haiku - we're checking if the inherited system instruction influenced the response at all.)",
            )
            .await?;

            if is_valid {
                Ok(())
            } else {
                Err("Response did not show evidence of inherited haiku system instruction".into())
            }
        }
    })
    .await;

    match result {
        Ok(()) => println!("\n✓ System instruction inheritance test passed"),
        Err(e) => {
            panic!(
                "System instruction inheritance test failed after retries: {}",
                e
            );
        }
    }
}

/// Test streaming multi-turn with auto functions.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_multiturn_streaming_auto_functions() {
    use futures_util::StreamExt;
    use genai_rs::AutoFunctionStreamChunk;

    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let functions = vec![get_weather_function()];

    // Turn 1: FirstTurn streaming with auto functions
    println!("--- Turn 1: Streaming with auto functions ---");
    let mut stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's the weather in Miami?")
        .with_functions(functions.clone())
        .with_store_enabled()
        .with_system_instruction(SYSTEM_INSTRUCTION)
        .create_stream_with_auto_functions();

    let mut final_response = None;
    let mut function_count = 0;
    let mut delta_count = 0;

    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => match event.chunk {
                AutoFunctionStreamChunk::Delta(_) => {
                    delta_count += 1;
                }
                AutoFunctionStreamChunk::FunctionResults(execs) => {
                    for exec in &execs {
                        println!("  Function executed: {}", exec.name);
                    }
                    function_count += execs.len();
                }
                AutoFunctionStreamChunk::Complete(response) => {
                    final_response = Some(response);
                }
                _ => {} // Unknown future variants
            },
            Err(e) => panic!("Stream error: {:?}", e),
        }
    }

    println!(
        "Turn 1: {} deltas, {} functions",
        delta_count, function_count
    );

    let response1 = final_response.expect("Should have final response");
    assert!(
        function_count > 0,
        "Should have executed at least one function"
    );

    // Turn 2: Chained streaming
    println!("\n--- Turn 2: Chained streaming ---");
    let mut stream2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Compare that to New York.")
        .with_functions(functions)
        .with_store_enabled()
        .with_previous_interaction(response1.id.as_ref().expect("Should have ID"))
        .create_stream_with_auto_functions();

    let mut turn2_deltas = 0;
    let mut turn2_functions = 0;

    while let Some(result) = stream2.next().await {
        match result {
            Ok(event) => match event.chunk {
                AutoFunctionStreamChunk::Delta(_) => {
                    turn2_deltas += 1;
                }
                AutoFunctionStreamChunk::FunctionResults(execs) => {
                    for exec in &execs {
                        println!("  Turn 2 function: {}", exec.name);
                    }
                    turn2_functions += execs.len();
                }
                AutoFunctionStreamChunk::Complete(response) => {
                    println!("Turn 2 complete: {:?}", response.status);
                    if response.has_text() {
                        println!("Response: {}", response.text().unwrap());
                    }
                }
                _ => {}
            },
            Err(e) => panic!("Stream error: {:?}", e),
        }
    }

    println!(
        "Turn 2: {} deltas, {} functions",
        turn2_deltas, turn2_functions
    );

    // Should have called function for New York
    assert!(
        turn2_functions > 0,
        "Should have executed function for comparison"
    );
}

// =============================================================================
// Typestate Compile-Time Constraint Tests
// =============================================================================

// NOTE: The following constraints are enforced at COMPILE TIME:
//
// 1. After with_previous_interaction():
//    - with_system_instruction() is NOT available
//    - with_store_disabled() is NOT available
//
// 2. After with_store_disabled():
//    - with_previous_interaction() is NOT available
//    - with_background(true) is NOT available
//    - create_with_auto_functions() is NOT available
//
// These cannot be tested at runtime because the code won't compile.
// See tests/ui_tests.rs for compile-fail tests that verify these constraints.

/// This test exists to document what CAN'T be tested at runtime.
/// The typestate pattern prevents these invalid combinations at compile time.
#[test]
fn test_typestate_constraints_documented() {
    // The following code would NOT compile:
    //
    // ```
    // // Error: with_system_instruction not available on Chained
    // client.interaction()
    //     .with_previous_interaction("id")
    //     .with_system_instruction("..."); // Compile error!
    // ```
    //
    // ```
    // // Error: create_with_auto_functions not available on StoreDisabled
    // client.interaction()
    //     .with_store_disabled()
    //     .create_with_auto_functions(); // Compile error!
    // ```
    //
    // These are verified by compile-fail tests in tests/ui_tests.rs

    println!("Typestate constraints are enforced at compile time.");
    println!("See tests/ui_tests.rs for compile-fail verification.");
}
