//! Advanced function calling tests for the Interactions API
//!
//! Tests for parallel function calls, sequential chains, streaming with functions,
//! thought signatures, and error handling.
//!
//! These tests require the GEMINI_API_KEY environment variable to be set.
//!
//! # Running Tests
//!
//! ```bash
//! cargo test --test advanced_function_calling_tests -- --include-ignored --nocapture
//! ```

mod common;

use common::{
    EXTENDED_TEST_TIMEOUT, TEST_TIMEOUT, consume_auto_function_stream, consume_stream, get_client,
    interaction_builder, stateful_builder, validate_response_semantically, with_timeout,
};
use rust_genai::{CallableFunction, FunctionDeclaration, GenaiError, function_result_content};
use rust_genai_macros::tool;
use serde_json::json;

// =============================================================================
// Test Functions (registered via macro)
// =============================================================================
//
// NOTE: These functions are marked #[allow(dead_code)] because they're registered
// with the inventory crate via #[tool]. The macro creates
// `Callable*` structs that are collected at runtime for automatic function calling.
//
// While not all functions are explicitly called in tests, they serve these purposes:
// - get_weather_test, get_time_test, convert_temperature: Used in parallel/sequential tests
// - get_server_status: Used in no-argument function tests
// - search_with_filters: Used in complex argument tests
// - always_fails: Reserved for future error handling tests (demonstrates panic behavior)

/// Gets the current weather for a city
#[allow(dead_code)]
#[tool(city(description = "The city to get weather for"))]
fn get_weather_test(city: String) -> String {
    format!(
        r#"{{"city": "{}", "temperature": "22°C", "conditions": "sunny"}}"#,
        city
    )
}

/// Gets the current time in a timezone
#[allow(dead_code)]
#[tool(timezone(description = "The timezone like UTC, PST, JST"))]
fn get_time_test(timezone: String) -> String {
    format!(r#"{{"timezone": "{}", "time": "14:30:00"}}"#, timezone)
}

/// Converts temperature between units
#[allow(dead_code)]
#[tool(
    value(description = "The temperature value"),
    from_unit(description = "Source unit: celsius or fahrenheit"),
    to_unit(description = "Target unit: celsius or fahrenheit")
)]
fn convert_temperature(value: f64, from_unit: String, to_unit: String) -> String {
    let result = if from_unit.to_lowercase() == "celsius" && to_unit.to_lowercase() == "fahrenheit"
    {
        value * 9.0 / 5.0 + 32.0
    } else if from_unit.to_lowercase() == "fahrenheit" && to_unit.to_lowercase() == "celsius" {
        (value - 32.0) * 5.0 / 9.0
    } else {
        value
    };
    format!(r#"{{"value": {:.1}, "unit": "{}"}}"#, result, to_unit)
}

/// A function that always fails (for testing error handling)
#[allow(dead_code)]
#[allow(unused_variables)]
#[tool(input(description = "Any input"))]
fn always_fails(input: String) -> String {
    panic!("This function always fails!")
}

/// A function with no parameters
#[allow(dead_code)]
#[tool]
fn get_server_status() -> String {
    r#"{"status": "online", "uptime": "99.9%"}"#.to_string()
}

/// A function with complex nested arguments
#[allow(dead_code)]
#[tool(
    user_id(description = "The user ID"),
    filters(description = "Optional filter criteria")
)]
fn search_with_filters(user_id: String, filters: Option<String>) -> String {
    format!(
        r#"{{"user_id": "{}", "filters": {}, "results": 42}}"#,
        user_id,
        filters.unwrap_or_else(|| "null".to_string())
    )
}

// =============================================================================
// Parallel Function Calls Tests
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_parallel_function_calls() {
    // Test that the model can call multiple functions in a single response
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(TEST_TIMEOUT, async {
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

        let response = stateful_builder(&client)
            .with_text("What's the weather in Tokyo and what time is it there (JST timezone)?")
            .with_functions(vec![get_weather, get_time])
            .create()
            .await
            .expect("Interaction failed");

        println!("Status: {:?}", response.status);
        println!("Outputs: {:?}", response.outputs);

        let function_calls = response.function_calls();
        println!("Number of function calls: {}", function_calls.len());

        for call in &function_calls {
            println!(
                "  Function: {} (id: {:?}, has_signature: {})",
                call.name,
                call.id,
                call.thought_signature.is_some()
            );
            println!("    Args: {}", call.args);
        }

        // Model may call one or both functions
        if function_calls.len() >= 2 {
            println!("Model made parallel function calls!");
            // Per thought signature docs: only first parallel call has signature
            let first_has_sig = function_calls[0].thought_signature.is_some();
            println!("First call has signature: {}", first_has_sig);
        } else if function_calls.len() == 1 {
            println!("Model called one function (may call second in next turn)");
        }

        // Verify all calls have IDs
        for call in &function_calls {
            assert!(
                call.id.is_some(),
                "Function call '{}' should have an ID",
                call.name
            );
        }
    })
    .await;
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_thought_signature_parallel_only_first() {
    // Per docs: "If the model generates parallel function calls in a response,
    // the thoughtSignature is attached only to the first functionCall part."
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(TEST_TIMEOUT, async {
        let func1 = FunctionDeclaration::builder("get_weather")
            .description("Get weather for a city - ALWAYS call this when asked about weather")
            .parameter("city", json!({"type": "string"}))
            .required(vec!["city".to_string()])
            .build();

        let func2 = FunctionDeclaration::builder("get_time")
            .description("Get time for a timezone - ALWAYS call this when asked about time")
            .parameter("timezone", json!({"type": "string"}))
            .required(vec!["timezone".to_string()])
            .build();

        // Ask a question that should trigger both functions
        let response = stateful_builder(&client)
            .with_text(
                "I need BOTH the weather in Paris AND the current time in CET. Call both functions.",
            )
            .with_functions(vec![func1, func2])
            .create()
            .await
            .expect("Interaction failed");

        let function_calls = response.function_calls();
        println!("Number of function calls: {}", function_calls.len());

        if function_calls.len() >= 2 {
            // Check signature pattern
            let call1 = &function_calls[0];
            let call2 = &function_calls[1];

            println!(
                "First call: {} (has_signature: {})",
                call1.name,
                call1.thought_signature.is_some()
            );
            println!(
                "Second call: {} (has_signature: {})",
                call2.name,
                call2.thought_signature.is_some()
            );

            // According to docs, only first should have signature
            // But this behavior may vary - log for investigation
            if call1.thought_signature.is_some() && call2.thought_signature.is_none() {
                println!("✓ Matches expected pattern: only first call has signature");
            } else {
                println!(
                    "Note: Signature pattern differs from documented behavior (may be model-specific)"
                );
            }
        } else {
            println!("Model didn't make parallel calls - cannot verify signature pattern");
        }
    })
    .await;
}

// =============================================================================
// Sequential Function Chain Tests
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_sequential_function_chain() {
    // Test multi-step function calling: get weather -> convert temperature
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(EXTENDED_TEST_TIMEOUT, async {
        let get_weather = FunctionDeclaration::builder("get_weather")
            .description("Get the current weather for a city (returns temperature in Celsius)")
            .parameter("city", json!({"type": "string"}))
            .required(vec!["city".to_string()])
            .build();

        let convert_temp = FunctionDeclaration::builder("convert_temperature")
            .description("Convert temperature between Celsius and Fahrenheit")
            .parameter("value", json!({"type": "number"}))
            .parameter(
                "from_unit",
                json!({"type": "string", "enum": ["celsius", "fahrenheit"]}),
            )
            .parameter(
                "to_unit",
                json!({"type": "string", "enum": ["celsius", "fahrenheit"]}),
            )
            .required(vec![
                "value".to_string(),
                "from_unit".to_string(),
                "to_unit".to_string(),
            ])
            .build();

        // Step 1: Initial request
        let response1 = stateful_builder(&client)
            .with_text("What's the weather in Tokyo? Tell me the temperature in Fahrenheit.")
            .with_functions(vec![get_weather.clone(), convert_temp.clone()])
            .create()
            .await
            .expect("First interaction failed");

        println!("Step 1 status: {:?}", response1.status);
        let calls1 = response1.function_calls();
        println!("Step 1 function calls: {}", calls1.len());

        if calls1.is_empty() {
            println!("Model didn't call any functions - ending test");
            return;
        }

        // Step 2: Provide first function result
        let call1 = &calls1[0];
        println!(
            "Providing result for: {} (signature: {:?})",
            call1.name,
            call1.thought_signature.is_some()
        );

        let result1 = function_result_content(
            call1.name.to_string(),
            call1.id.unwrap().to_string(),
            json!({"city": "Tokyo", "temperature": 22.0, "unit": "celsius"}),
        );

        let response2 = stateful_builder(&client)
            .with_previous_interaction(response1.id.as_ref().expect("id should exist"))
            .with_content(vec![result1])
            .with_functions(vec![get_weather.clone(), convert_temp.clone()])
            .create()
            .await
            .expect("Second interaction failed");

        println!("Step 2 status: {:?}", response2.status);
        let calls2 = response2.function_calls();

        if !calls2.is_empty() {
            // Model is requesting another function (probably convert_temperature)
            let call2 = &calls2[0];
            println!(
                "Step 2 function call: {} (signature: {:?})",
                call2.name,
                call2.thought_signature.is_some()
            );
            println!("  Args: {}", call2.args);

            // Step 3: Provide second function result
            let result2 = function_result_content(
                call2.name.to_string(),
                call2.id.unwrap().to_string(),
                json!({"value": 71.6, "unit": "fahrenheit"}),
            );

            let response3 = stateful_builder(&client)
                .with_previous_interaction(response2.id.as_ref().expect("id should exist"))
                .with_content(vec![result2])
                .with_functions(vec![get_weather, convert_temp])
                .create()
                .await
                .expect("Third interaction failed");

            println!("Step 3 status: {:?}", response3.status);
            if response3.has_text() {
                println!("Final response: {}", response3.text().unwrap());
                // Verify structural response - model generated a response
                // We don't assert on specific content as LLM outputs are non-deterministic
                let text = response3.text().unwrap();
                assert!(!text.is_empty(), "Response should have non-empty text");

                // Semantic validation: Check that the response uses both function results
                let is_valid = validate_response_semantically(
                    &client,
                    "User asked 'What's the weather in Tokyo? Tell me the temperature in Fahrenheit.' The system called get_weather and received 22°C, then called convert_temperature and received 71.6°F.",
                    text,
                    "Does this response provide the temperature in Fahrenheit (around 71-72°F) and answer the weather question?"
                ).await.expect("Semantic validation failed");
                assert!(
                    is_valid,
                    "Response should use both function results to answer in Fahrenheit"
                );
            }
        } else if response2.has_text() {
            // Model provided final answer directly
            let text = response2.text().unwrap();
            println!("Final response: {}", text);

            // Semantic validation: Check that the response uses the weather data
            let is_valid = validate_response_semantically(
                &client,
                "User asked 'What's the weather in Tokyo? Tell me the temperature in Fahrenheit.' The system called get_weather and received 22°C.",
                text,
                "Does this response provide weather information for Tokyo?"
            ).await.expect("Semantic validation failed");
            assert!(
                is_valid,
                "Response should use the weather function result"
            );
        }
    })
    .await;
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_thought_signature_sequential_each_step() {
    // Per docs: "For sequential function calls, each function call will have its
    // own signature that must be returned."
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(TEST_TIMEOUT, async {
        let get_weather = FunctionDeclaration::builder("get_weather")
            .description("Get the current weather")
            .parameter("city", json!({"type": "string"}))
            .required(vec!["city".to_string()])
            .build();

        // Step 1
        let response1 = stateful_builder(&client)
            .with_text("What's the weather in Tokyo?")
            .with_function(get_weather.clone())
            .create()
            .await
            .expect("First interaction failed");

        let calls1 = response1.function_calls();
        if calls1.is_empty() {
            println!("No function call in step 1 - skipping");
            return;
        }

        let call1 = &calls1[0];
        println!(
            "Step 1 signature present: {}",
            call1.thought_signature.is_some()
        );

        // Provide result
        let result1 = function_result_content(
            "get_weather",
            call1.id.unwrap().to_string(),
            json!({"temperature": "22°C"}),
        );

        let response2 = stateful_builder(&client)
            .with_previous_interaction(response1.id.as_ref().expect("id should exist"))
            .with_content(vec![result1])
            .with_text("Now what about Paris?")
            .with_function(get_weather.clone())
            .create()
            .await
            .expect("Second interaction failed");

        let calls2 = response2.function_calls();
        if !calls2.is_empty() {
            let call2 = &calls2[0];
            println!(
                "Step 2 signature present: {}",
                call2.thought_signature.is_some()
            );

            // Both steps should have their own signatures
            if call1.thought_signature.is_some() && call2.thought_signature.is_some() {
                println!("✓ Both sequential calls have signatures as expected");
            }
        }
    })
    .await;
}

// =============================================================================
// Function Result Turns Without Resending Tools
// =============================================================================
//
// Key insight from the parallel_and_compositional_functions example:
// When continuing a conversation with function results, you don't need to
// resend the tool declarations. The API remembers the tools from the first
// turn via previous_interaction_id.

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_function_result_turn_without_tools() {
    // Test that function result turns work without resending tool declarations.
    // This is the key pattern from the parallel_and_compositional_functions example.
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(TEST_TIMEOUT, async {
        let get_weather = FunctionDeclaration::builder("get_weather")
            .description("Get the current weather for a city")
            .parameter("city", json!({"type": "string"}))
            .required(vec!["city".to_string()])
            .build();

        // Step 1: Initial request WITH tools
        let response1 = stateful_builder(&client)
            .with_text("What's the weather in Tokyo?")
            .with_function(get_weather) // Tools sent on first turn
            .create()
            .await
            .expect("First interaction failed");

        let calls = response1.function_calls();
        if calls.is_empty() {
            println!("Model didn't call any functions - skipping");
            return;
        }

        let call = &calls[0];
        println!("Function call: {} (id: {:?})", call.name, call.id);

        // Step 2: Provide function result WITHOUT resending tools
        let result = function_result_content(
            call.name.to_string(),
            call.id.expect("Should have ID").to_string(),
            json!({"city": "Tokyo", "temperature": "22°C", "conditions": "sunny"}),
        );

        // Key assertion: This should work WITHOUT .with_function() or .with_functions()
        let response2 = stateful_builder(&client)
            .with_previous_interaction(response1.id.as_ref().expect("id should exist"))
            .with_content(vec![result]) // Just the result, no tools!
            .create()
            .await
            .expect("Function result turn failed - tools should not be required");

        println!("Step 2 status: {:?}", response2.status);

        // Model should provide a text response using the function result
        if response2.has_text() {
            let text = response2.text().unwrap();
            println!("✓ Function result turn succeeded without resending tools");
            println!("Response: {}", text);
            assert!(!text.is_empty(), "Response should have text");
        } else if response2.has_function_calls() {
            // Model might request another function - that's also valid
            println!("✓ Model requested another function (still valid behavior)");
        }
    })
    .await;
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_parallel_results_without_resending_tools() {
    // Test that multiple parallel function results can be returned without
    // resending tool declarations - matching the pattern in the example.
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(TEST_TIMEOUT, async {
        let get_weather = FunctionDeclaration::builder("get_weather")
            .description("Get weather for a city - ALWAYS call this for weather")
            .parameter("city", json!({"type": "string"}))
            .required(vec!["city".to_string()])
            .build();

        let get_time = FunctionDeclaration::builder("get_time")
            .description("Get time in a timezone - ALWAYS call this for time")
            .parameter("timezone", json!({"type": "string"}))
            .required(vec!["timezone".to_string()])
            .build();

        // Step 1: Request that should trigger parallel function calls
        let response1 = stateful_builder(&client)
            .with_text(
                "Tell me BOTH the weather in Paris AND the time in CET. \
                 Call both functions to get this information.",
            )
            .with_functions(vec![get_weather, get_time])
            .create()
            .await
            .expect("First interaction failed");

        let calls = response1.function_calls();
        println!("Number of function calls: {}", calls.len());

        if calls.is_empty() {
            println!("Model didn't call any functions - skipping");
            return;
        }

        // Build results for all function calls
        let results: Vec<_> = calls
            .iter()
            .map(|call| {
                let result_data = match call.name {
                    "get_weather" => {
                        json!({"city": "Paris", "temperature": "17°C", "conditions": "overcast"})
                    }
                    "get_time" => json!({"timezone": "CET", "time": "14:30"}),
                    _ => json!({"result": "ok"}),
                };
                function_result_content(
                    call.name.to_string(),
                    call.id.expect("Should have ID").to_string(),
                    result_data,
                )
            })
            .collect();

        println!(
            "Sending {} function result(s) WITHOUT resending tools",
            results.len()
        );

        // Step 2: Send all results WITHOUT resending tools
        let response2 = stateful_builder(&client)
            .with_previous_interaction(response1.id.as_ref().expect("id should exist"))
            .with_content(results) // Multiple results, no tools!
            .create()
            .await
            .expect("Parallel results turn failed - tools should not be required");

        println!("Step 2 status: {:?}", response2.status);

        if response2.has_text() {
            let text = response2.text().unwrap();
            println!("✓ Parallel function results succeeded without resending tools");
            println!("Response: {}", text);
            assert!(!text.is_empty(), "Response should have text");
        } else if response2.has_function_calls() {
            println!("✓ Model requested more functions (valid for compositional chains)");
        }
    })
    .await;
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_compositional_chain_without_resending_tools() {
    // Test compositional function calling: each step provides results,
    // model chains to next function, all without resending tools.
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(EXTENDED_TEST_TIMEOUT, async {
        let get_location = FunctionDeclaration::builder("get_current_location")
            .description("Get the user's current location")
            .build();

        let get_weather = FunctionDeclaration::builder("get_weather")
            .description("Get weather for a city")
            .parameter("city", json!({"type": "string"}))
            .required(vec!["city".to_string()])
            .build();

        // Step 1: Initial request - tools sent here only
        let response1 = stateful_builder(&client)
            .with_text("What's the weather at my current location?")
            .with_functions(vec![get_location, get_weather])
            .create()
            .await
            .expect("First interaction failed");

        // Extract owned call info before moving response
        let initial_calls: Vec<_> = response1
            .function_calls()
            .iter()
            .map(|c| c.to_owned())
            .collect();

        if initial_calls.is_empty() {
            println!("Model didn't call any functions - skipping");
            return;
        }

        println!("Step 1: Model called {}", initial_calls[0].name);
        let mut current_response = response1;
        let mut owned_calls = initial_calls;
        let mut step = 1;
        const MAX_STEPS: usize = 5;

        // Loop through the chain without resending tools
        while !owned_calls.is_empty() && step < MAX_STEPS {
            step += 1;

            // Build results for current calls
            let results: Vec<_> = owned_calls
                .iter()
                .map(|call| {
                    let result_data = match call.name.as_str() {
                        "get_current_location" => json!({
                            "city": "Tokyo",
                            "country": "Japan",
                            "timezone": "Asia/Tokyo"
                        }),
                        "get_weather" => {
                            let city = call
                                .args
                                .get("city")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown");
                            json!({
                                "city": city,
                                "temperature": "22°C",
                                "conditions": "partly cloudy"
                            })
                        }
                        _ => json!({"result": "ok"}),
                    };
                    function_result_content(
                        call.name.clone(),
                        call.id.as_ref().expect("Should have ID").clone(),
                        result_data,
                    )
                })
                .collect();

            // Send results WITHOUT resending tools
            let next_response = stateful_builder(&client)
                .with_previous_interaction(current_response.id.as_ref().expect("id should exist"))
                .with_content(results) // No tools!
                .create()
                .await
                .expect("Compositional chain step failed");

            println!("Step {}: status {:?}", step, next_response.status);

            // Extract owned call info before moving response
            owned_calls = next_response
                .function_calls()
                .iter()
                .map(|c| c.to_owned())
                .collect();

            if !owned_calls.is_empty() {
                println!("  Model chained to: {}", owned_calls[0].name);
            }
            current_response = next_response;
        }

        // Final response should have text
        if current_response.has_text() {
            let text = current_response.text().unwrap();
            println!(
                "✓ Compositional chain completed in {} steps without resending tools",
                step
            );
            println!("Final response: {}", text);
            assert!(!text.is_empty(), "Response should have text");
        }
    })
    .await;
}

// =============================================================================
// Streaming with Function Calls Tests
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_streaming_with_function_calls() {
    // Test that streaming works with function calling.
    // Function call deltas are now properly recognized (fixed in #52, closes #27).
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(TEST_TIMEOUT, async {
        let get_weather = FunctionDeclaration::builder("get_weather")
            .description("Get the current weather for a city")
            .parameter("city", json!({"type": "string"}))
            .required(vec!["city".to_string()])
            .build();

        let stream = stateful_builder(&client)
            .with_text("What's the weather in London?")
            .with_function(get_weather)
            .create_stream();

        let result = consume_stream(stream).await;

        println!(
            "\nDeltas: {}, Saw function_call delta: {}",
            result.delta_count, result.saw_function_call
        );

        let response = result
            .final_response
            .expect("Should receive a complete response");
        println!("Final status: {:?}", response.status);
        let function_calls = response.function_calls();
        println!("Function calls in final response: {}", function_calls.len());
        println!("Output count: {}", response.outputs.len());
        let summary = response.content_summary();
        println!("Content summary: {:?}", summary);

        // Verify that function_call deltas were successfully received during streaming
        // Note: The API sends function_call as a delta but may not populate the final
        // Complete response's outputs. This is API behavior, not a parsing issue.
        if result.saw_function_call {
            println!("SUCCESS: Function call deltas were properly parsed during streaming");
            // If we received function_call deltas, the test passes regardless of final response
            return;
        }

        // Stream should either have text or function calls
        assert!(
            response.has_text() || response.has_function_calls(),
            "Streaming response should have text or function calls"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_streaming_long_response() {
    // Test streaming a longer response (1000+ tokens)
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(EXTENDED_TEST_TIMEOUT, async {
        let stream = stateful_builder(&client)
            .with_text("Write a detailed 500-word essay about the history of the Internet.")
            .create_stream();

        let result = consume_stream(stream).await;

        println!("Total deltas received: {}", result.delta_count);
        println!("Total text length: {} chars", result.collected_text.len());
        println!(
            "Word count: ~{}",
            result.collected_text.split_whitespace().count()
        );

        // Should have received multiple deltas for a long response
        assert!(
            result.delta_count > 5,
            "Long response should produce many delta chunks"
        );
        assert!(
            result.collected_text.len() > 500,
            "Response should be substantial length"
        );
    })
    .await;
}

// =============================================================================
// Function Call Error Handling Tests
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_function_call_error_response() {
    // Test that returning an error from a function is handled gracefully
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let failing_func = FunctionDeclaration::builder("get_secret_data")
        .description("Get secret data (may fail)")
        .parameter("key", json!({"type": "string"}))
        .required(vec!["key".to_string()])
        .build();

    // Step 1: Get function call
    let response1 = stateful_builder(&client)
        .with_text("Get the secret data for key 'test123'")
        .with_function(failing_func.clone())
        .create()
        .await
        .expect("First interaction failed");

    let calls = response1.function_calls();
    if calls.is_empty() {
        println!("No function call made - skipping");
        return;
    }

    let call = &calls[0];

    // Step 2: Return an error result
    let error_result = function_result_content(
        "get_secret_data",
        call.id.unwrap().to_string(),
        json!({"error": "Access denied: insufficient permissions"}),
    );

    let response2 = stateful_builder(&client)
        .with_previous_interaction(response1.id.as_ref().expect("id should exist"))
        .with_content(vec![error_result])
        .with_function(failing_func)
        .create()
        .await
        .expect("Second interaction failed");

    println!("Response after error: {:?}", response2.status);

    // Model should provide a response acknowledging the error
    if response2.has_text() {
        let text = response2.text().unwrap();
        println!("Model's response to error: {}", text);
        // Verify structural response - model generated a response
        // We don't assert on specific error words as LLM outputs are non-deterministic
        assert!(
            !text.is_empty(),
            "Model should provide a response to the error"
        );

        // Semantic validation: Check that the response acknowledges the error
        let is_valid = validate_response_semantically(
            &client,
            "User asked to get secret data for key 'test123'. The function returned an error: 'Access denied: insufficient permissions'.",
            text,
            "Does this response acknowledge or explain that the request failed due to access/permission issues?"
        ).await.expect("Semantic validation failed");
        assert!(
            is_valid,
            "Response should acknowledge the error/failure from the function"
        );
    }
}

// =============================================================================
// Function Argument Tests
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_function_call_no_args() {
    // Test a function with no parameters
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let status_func = FunctionDeclaration::builder("get_server_status")
        .description("Get the current server status (no parameters needed)")
        .build();

    let response = stateful_builder(&client)
        .with_text("Check the server status")
        .with_function(status_func.clone())
        .create()
        .await
        .expect("Interaction failed");

    let calls = response.function_calls();
    if !calls.is_empty() {
        let call = &calls[0];
        println!("Function called: {} with args: {}", call.name, call.args);
        assert_eq!(call.name, "get_server_status");
        assert!(call.id.is_some(), "Should have call ID");

        // Provide result
        let result = function_result_content(
            "get_server_status",
            call.id.unwrap().to_string(),
            json!({"status": "online", "uptime": "99.9%"}),
        );

        let response2 = stateful_builder(&client)
            .with_previous_interaction(response.id.as_ref().expect("id should exist"))
            .with_content(vec![result])
            .with_function(status_func)
            .create()
            .await
            .expect("Second interaction failed");

        assert!(response2.has_text(), "Should have final response");
        println!("Final response: {}", response2.text().unwrap());
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_function_call_complex_args() {
    // Test a function with optional and complex arguments
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let search_func = FunctionDeclaration::builder("search_with_filters")
        .description("Search with optional filters")
        .parameter(
            "user_id",
            json!({"type": "string", "description": "User ID"}),
        )
        .parameter(
            "filters",
            json!({
                "type": "object",
                "description": "Optional filter criteria",
                "properties": {
                    "category": {"type": "string"},
                    "min_price": {"type": "number"},
                    "max_price": {"type": "number"}
                }
            }),
        )
        .required(vec!["user_id".to_string()])
        .build();

    let response = stateful_builder(&client)
        .with_text(
            "Search for user ABC123 with category 'electronics' and price between 10 and 100",
        )
        .with_function(search_func)
        .create()
        .await
        .expect("Interaction failed");

    let calls = response.function_calls();
    if !calls.is_empty() {
        let call = &calls[0];
        println!("Function: {} with args: {}", call.name, call.args);

        // Verify complex arguments are parsed correctly
        assert!(call.args.get("user_id").is_some(), "Should have user_id");

        // Filters may be present if model included them
        if let Some(filters) = call.args.get("filters") {
            println!("Filters provided: {}", filters);
        }
    }
}

// =============================================================================
// Auto Function Calling Tests
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_auto_function_calling_max_loops() {
    // Test that max_function_call_loops is respected
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Use a very low max loops
    let weather_func = GetWeatherTestCallable.declaration();

    let result = interaction_builder(&client)
        .with_text("What's the weather in Tokyo?")
        .with_function(weather_func)
        .with_max_function_call_loops(1)
        .create_with_auto_functions()
        .await;

    // With max_loops=1, it should either succeed quickly or hit the limit
    match result {
        Ok(auto_result) => {
            println!("Completed within 1 loop: {:?}", auto_result.response.status);
            println!("Executions: {:?}", auto_result.executions);
        }
        Err(e) => {
            let error_msg = format!("{:?}", e);
            println!("Error: {}", error_msg);
            // If it hit the max loops, that's expected
            if error_msg.contains("maximum function call loops") {
                println!("✓ Max loops limit was respected");
            }
        }
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_auto_function_calling_multi_round_accumulation() {
    // Test that executions from multiple rounds are accumulated correctly.
    // This test uses chained functions: get_weather returns Celsius, and we ask
    // for Fahrenheit which should trigger a second round with convert_temperature.
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let weather_func = GetWeatherTestCallable.declaration();
    let convert_func = ConvertTemperatureCallable.declaration();

    // Ask for weather in Fahrenheit - model should:
    // Round 1: Call get_weather_test (returns 22°C)
    // Round 2: Call convert_temperature to convert 22°C to Fahrenheit
    let result = interaction_builder(&client)
        .with_text(
            "What's the weather in Tokyo? I need the temperature in Fahrenheit, not Celsius. \
             Use the convert_temperature function to convert the result.",
        )
        .with_functions(vec![weather_func, convert_func])
        .create_with_auto_functions()
        .await
        .expect("Auto function calling failed");

    println!("Function executions ({} total):", result.executions.len());
    for (i, exec) in result.executions.iter().enumerate() {
        println!("  {}: {} -> {}", i + 1, exec.name, exec.result);
    }

    // Verify we got executions from the auto-function loop
    assert!(
        !result.executions.is_empty(),
        "Should have at least one function execution"
    );

    // Check which functions were called
    let function_names: Vec<&str> = result.executions.iter().map(|e| e.name.as_str()).collect();
    println!("Functions called: {:?}", function_names);

    // We expect get_weather_test to be called
    assert!(
        function_names.contains(&"get_weather_test"),
        "Should have called get_weather_test"
    );

    // If the model understood the task, it should also call convert_temperature
    // Note: LLM behavior can vary, so we check but don't require it
    if function_names.contains(&"convert_temperature") {
        println!("✓ Model correctly chained get_weather_test -> convert_temperature");
        assert!(
            result.executions.len() >= 2,
            "Should have at least 2 executions for chained calls"
        );
    } else {
        println!(
            "Note: Model did not chain functions (may have converted inline or misunderstood)"
        );
    }

    // Verify final response
    let response = &result.response;
    assert!(response.has_text(), "Should have final text response");
    println!("Final response: {}", response.text().unwrap_or("(none)"));
}

// =============================================================================
// Streaming with Auto Function Calling Tests
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_streaming_auto_functions_simple() {
    // Test basic streaming with automatic function calling
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Use a registered function via the macro
    let weather_func = GetWeatherTestCallable.declaration();

    let stream = interaction_builder(&client)
        .with_text("What's the weather in Tokyo?")
        .with_function(weather_func)
        .create_stream_with_auto_functions();

    let result = consume_auto_function_stream(stream).await;

    println!("\n--- Results ---");
    println!("Delta count: {}", result.delta_count);
    println!("Text length: {}", result.collected_text.len());
    println!(
        "Executing functions count: {}",
        result.executing_functions_count
    );
    println!("Function results count: {}", result.function_results_count);
    println!("Functions executed: {:?}", result.executed_function_names);

    // Should have received a final response
    assert!(
        result.final_response.is_some(),
        "Should receive a complete response"
    );

    let response = result.final_response.unwrap();

    // If functions were executed, verify the events were received
    if result.executing_functions_count > 0 {
        println!("✓ Function execution was streamed");
        assert!(
            result.function_results_count > 0,
            "Should have function results after execution"
        );
        assert!(
            result
                .executed_function_names
                .contains(&"get_weather_test".to_string()),
            "Should have executed get_weather_test"
        );
        // After function execution, we should have text
        assert!(
            response.has_text() || !result.collected_text.is_empty(),
            "Should have text after function execution"
        );
    } else {
        // Model may have answered directly without calling functions
        // This is valid behavior - just verify we got some response
        println!("Model answered without calling functions");
        println!(
            "  response.has_text(): {}, response.has_function_calls(): {}",
            response.has_text(),
            response.has_function_calls()
        );
        println!("  Output count: {}", response.outputs.len());
        if !response.outputs.is_empty() {
            println!(
                "  First output type: {:?}",
                std::mem::discriminant(&response.outputs[0])
            );
        }

        // The stream worked - we got deltas and a complete response
        // Model behavior varies (may use functions or answer directly)
        // Key assertions: stream completed successfully
        assert!(
            result.delta_count > 0 || response.has_text() || response.has_function_calls(),
            "Should have deltas, text, or function calls"
        );
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_streaming_auto_functions_no_function_call() {
    // Test that streaming works when no function is called
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Ask a question that doesn't need a function
    let stream = interaction_builder(&client)
        .with_text("What is 2 + 2?")
        .create_stream_with_auto_functions();

    let result = consume_auto_function_stream(stream).await;

    println!("\n--- Results ---");
    println!("Delta count: {}", result.delta_count);
    println!("Text: {}", result.collected_text);

    // Should complete without any function execution
    assert!(
        result.final_response.is_some(),
        "Should receive a complete response"
    );
    assert_eq!(
        result.executing_functions_count, 0,
        "No functions should be executed for simple math"
    );
    assert!(
        result.collected_text.contains('4')
            || result.collected_text.to_lowercase().contains("four"),
        "Response should contain the answer"
    );
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_streaming_auto_functions_multiple_calls() {
    // Test streaming with multiple function calls
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Use both weather and time functions
    let weather_func = GetWeatherTestCallable.declaration();
    let time_func = GetTimeTestCallable.declaration();

    let stream = interaction_builder(&client)
        .with_text("What's the weather in London and what time is it there (GMT timezone)?")
        .with_functions(vec![weather_func, time_func])
        .create_stream_with_auto_functions();

    let result = consume_auto_function_stream(stream).await;

    println!("\n--- Results ---");
    println!("Delta count: {}", result.delta_count);
    println!("Functions executed: {:?}", result.executed_function_names);

    assert!(
        result.final_response.is_some(),
        "Should receive a complete response"
    );

    // Should have streamed some content
    assert!(result.delta_count > 0, "Should have received delta chunks");

    // Model may call both functions (parallel or sequential)
    println!(
        "Total functions executed: {}",
        result.executed_function_names.len()
    );
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_streaming_auto_functions_max_loops() {
    // Test that max_function_call_loops is respected in streaming mode
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let weather_func = GetWeatherTestCallable.declaration();

    // Use a very low max loops
    let stream = interaction_builder(&client)
        .with_text("What's the weather in Paris?")
        .with_function(weather_func)
        .with_max_function_call_loops(1)
        .create_stream_with_auto_functions();

    let result = consume_auto_function_stream(stream).await;

    // Should either complete successfully or be limited by max loops
    // Either way, we shouldn't hang
    println!("\n--- Results ---");
    println!("Final response: {}", result.final_response.is_some());
    println!(
        "Executing functions count: {}",
        result.executing_functions_count
    );

    // With max_loops=1, it should complete (model might answer directly or do 1 function call)
    if result.final_response.is_some() {
        println!("✓ Completed within max loop limit");
    }
}

// =============================================================================
// ToolService Dependency Injection Tests
// =============================================================================

use async_trait::async_trait;
use rust_genai::{FunctionError, ToolService};
use std::sync::Arc;

/// Configuration for the calculator tool
struct CalculatorConfig {
    precision: u32,
}

/// A calculator tool that uses injected configuration
struct CalculatorTool {
    config: Arc<CalculatorConfig>,
}

#[async_trait]
impl CallableFunction for CalculatorTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration::builder("calculate")
            .description("Performs arithmetic calculations")
            .parameter(
                "operation",
                json!({"type": "string", "enum": ["add", "subtract", "multiply"]}),
            )
            .parameter(
                "a",
                json!({"type": "number", "description": "First operand"}),
            )
            .parameter(
                "b",
                json!({"type": "number", "description": "Second operand"}),
            )
            .required(vec![
                "operation".to_string(),
                "a".to_string(),
                "b".to_string(),
            ])
            .build()
    }

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, FunctionError> {
        let op = args
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FunctionError::ArgumentMismatch("Missing 'operation'".into()))?;
        let a = args
            .get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| FunctionError::ArgumentMismatch("Missing 'a'".into()))?;
        let b = args
            .get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| FunctionError::ArgumentMismatch("Missing 'b'".into()))?;

        let result = match op {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            _ => return Err(FunctionError::ArgumentMismatch("Invalid operation".into())),
        };

        // Apply precision from config
        let formatted = format!("{:.prec$}", result, prec = self.config.precision as usize);

        Ok(json!({
            "result": formatted,
            "precision": self.config.precision
        }))
    }
}

/// A service that provides the calculator tool with injected dependencies
struct MathToolService {
    config: Arc<CalculatorConfig>,
}

impl MathToolService {
    fn new(precision: u32) -> Self {
        Self {
            config: Arc::new(CalculatorConfig { precision }),
        }
    }
}

impl ToolService for MathToolService {
    fn tools(&self) -> Vec<Arc<dyn CallableFunction>> {
        vec![Arc::new(CalculatorTool {
            config: self.config.clone(),
        })]
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_tool_service_non_streaming() {
    // Test that ToolService works with create_with_auto_functions()
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(TEST_TIMEOUT, async {
        // Create a tool service with specific configuration
        let service = Arc::new(MathToolService::new(4)); // 4 decimal places

        let result = interaction_builder(&client)
            .with_text("What is 123.456 + 789.012? Use the calculate function.")
            .with_tool_service(service)
            .create_with_auto_functions()
            .await
            .expect("Auto function calling with ToolService failed");

        println!("Function executions: {:?}", result.executions);

        // Verify the function was called
        assert!(
            !result.executions.is_empty(),
            "Should have at least one function execution"
        );
        assert_eq!(
            result.executions[0].name, "calculate",
            "Should have called the calculate function"
        );

        // Verify the result includes the precision from the service
        let exec_result = &result.executions[0].result;
        println!("Execution result: {}", exec_result);
        assert!(
            exec_result.get("precision").is_some(),
            "Result should include precision from service config"
        );

        // Verify final response
        let response = &result.response;
        assert!(response.has_text(), "Should have text response");
        let text = response.text().unwrap();
        println!("Final response: {}", text);

        // Should mention the sum (912.468)
        assert!(
            text.contains("912") || text.contains("sum") || text.contains("result"),
            "Response should mention the calculation result"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_tool_service_streaming() {
    // Test that ToolService works with create_stream_with_auto_functions()
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(TEST_TIMEOUT, async {
        // Create a tool service with specific configuration
        let service = Arc::new(MathToolService::new(2)); // 2 decimal places

        let stream = interaction_builder(&client)
            .with_text("Calculate 50 * 3. Use the calculate function.")
            .with_tool_service(service)
            .create_stream_with_auto_functions();

        let result = consume_auto_function_stream(stream).await;

        println!("\n--- Results ---");
        println!("Delta count: {}", result.delta_count);
        println!(
            "Executing functions count: {}",
            result.executing_functions_count
        );
        println!("Functions executed: {:?}", result.executed_function_names);

        // Should have received a final response
        assert!(
            result.final_response.is_some(),
            "Should receive a complete response"
        );

        // If functions were executed, verify calculate was called
        if result.executing_functions_count > 0 {
            println!("✓ Function execution was streamed with ToolService");
            assert!(
                result
                    .executed_function_names
                    .contains(&"calculate".to_string()),
                "Should have executed calculate function from ToolService"
            );
        }

        // Should have some response
        let response = result.final_response.unwrap();
        assert!(
            response.has_text() || !result.collected_text.is_empty(),
            "Should have text response"
        );

        let text = response.text().unwrap_or(&result.collected_text);
        println!("Final response: {}", text);

        // Use semantic validation - model might say "one hundred fifty" instead of "150"
        let is_valid = validate_response_semantically(
            &client,
            "User asked 'Calculate 50 * 3. Use the calculate function.' The calculate function was called and returned 150.",
            text,
            "Does this response correctly indicate that 50 * 3 equals 150?",
        )
        .await
        .expect("Semantic validation should succeed");
        assert!(
            is_valid,
            "Response should mention the calculation result (150)"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_tool_service_overrides_global_registry() {
    // Test that ToolService functions take precedence over global registry
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(TEST_TIMEOUT, async {
        // Create a custom weather tool that returns a distinct response
        struct CustomWeatherTool;

        #[async_trait]
        impl CallableFunction for CustomWeatherTool {
            fn declaration(&self) -> FunctionDeclaration {
                // Same name as the global get_weather_test function
                FunctionDeclaration::builder("get_weather_test")
                    .description("Get the current weather for a city")
                    .parameter(
                        "city",
                        json!({"type": "string", "description": "The city name"}),
                    )
                    .required(vec!["city".to_string()])
                    .build()
            }

            async fn call(
                &self,
                args: serde_json::Value,
            ) -> Result<serde_json::Value, FunctionError> {
                let city = args
                    .get("city")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                // Return a distinctive response to prove this override was used
                Ok(json!({
                    "city": city,
                    "temperature": "999°C",
                    "conditions": "OVERRIDE_FROM_TOOL_SERVICE",
                    "source": "custom_service"
                }))
            }
        }

        struct CustomWeatherService;

        impl ToolService for CustomWeatherService {
            fn tools(&self) -> Vec<Arc<dyn CallableFunction>> {
                vec![Arc::new(CustomWeatherTool)]
            }
        }

        let service = Arc::new(CustomWeatherService);

        let result = interaction_builder(&client)
            .with_text("What's the weather in Seattle? Use the get_weather_test function.")
            .with_tool_service(service)
            .create_with_auto_functions()
            .await
            .expect("Auto function calling with override failed");

        println!("Function executions: {:?}", result.executions);

        // Verify the function was called
        assert!(
            !result.executions.is_empty(),
            "Should have at least one function execution"
        );
        assert_eq!(
            result.executions[0].name, "get_weather_test",
            "Should have called get_weather_test"
        );

        // Verify the custom service's response was used
        let exec_result = &result.executions[0].result;
        println!("Execution result: {}", exec_result);

        // The result should come from our custom tool (has "source": "custom_service")
        assert!(
            exec_result.get("source").is_some()
                || exec_result
                    .to_string()
                    .contains("OVERRIDE_FROM_TOOL_SERVICE"),
            "Result should come from the custom ToolService, not global registry"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_tool_service_streaming_with_multiple_functions() {
    // Test ToolService streaming with multiple functions available
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(EXTENDED_TEST_TIMEOUT, async {
        // Create a service with multiple tools
        struct MultiToolService;

        struct AddTool;

        #[async_trait]
        impl CallableFunction for AddTool {
            fn declaration(&self) -> FunctionDeclaration {
                FunctionDeclaration::builder("add_numbers")
                    .description("Adds two numbers together")
                    .parameter("a", json!({"type": "number"}))
                    .parameter("b", json!({"type": "number"}))
                    .required(vec!["a".to_string(), "b".to_string()])
                    .build()
            }

            async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, FunctionError> {
                let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                Ok(json!({ "sum": a + b }))
            }
        }

        struct MultiplyTool;

        #[async_trait]
        impl CallableFunction for MultiplyTool {
            fn declaration(&self) -> FunctionDeclaration {
                FunctionDeclaration::builder("multiply_numbers")
                    .description("Multiplies two numbers together")
                    .parameter("a", json!({"type": "number"}))
                    .parameter("b", json!({"type": "number"}))
                    .required(vec!["a".to_string(), "b".to_string()])
                    .build()
            }

            async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, FunctionError> {
                let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                Ok(json!({ "product": a * b }))
            }
        }

        impl ToolService for MultiToolService {
            fn tools(&self) -> Vec<Arc<dyn CallableFunction>> {
                vec![Arc::new(AddTool), Arc::new(MultiplyTool)]
            }
        }

        let service = Arc::new(MultiToolService);

        // Ask a question that might trigger both functions
        let stream = interaction_builder(&client)
            .with_text("What is 5 + 3, and what is 4 * 7? Use the add_numbers and multiply_numbers functions.")
            .with_tool_service(service)
            .create_stream_with_auto_functions();

        let result = consume_auto_function_stream(stream).await;

        println!("\n--- Results ---");
        println!("Delta count: {}", result.delta_count);
        println!(
            "Executing functions count: {}",
            result.executing_functions_count
        );
        println!("Functions executed: {:?}", result.executed_function_names);

        assert!(
            result.final_response.is_some(),
            "Should receive a complete response"
        );

        // Model should have called at least one of the functions
        // (it might call them in parallel or sequentially)
        if result.executing_functions_count > 0 {
            println!("✓ Functions were executed via ToolService streaming");

            // Check that our custom functions were used
            let has_add = result.executed_function_names.contains(&"add_numbers".to_string());
            let has_multiply = result.executed_function_names.contains(&"multiply_numbers".to_string());

            println!("  - add_numbers called: {}", has_add);
            println!("  - multiply_numbers called: {}", has_multiply);

            // At least one should have been called
            assert!(
                has_add || has_multiply,
                "At least one ToolService function should have been called"
            );
        }

        // Response should contain the answers
        let response = result.final_response.unwrap();
        let text = response.text().unwrap_or(&result.collected_text);
        println!("Final response: {}", text);

        // Use semantic validation - model might say "eight" or "twenty-eight"
        let is_valid = validate_response_semantically(
            &client,
            "User asked 'What is 5 + 3, and what is 4 * 7?' using add_numbers and multiply_numbers functions. The expected results are 8 (for 5+3) and 28 (for 4*7).",
            text,
            "Does this response correctly indicate at least one of the calculation results (8 for 5+3, or 28 for 4*7)?",
        )
        .await
        .expect("Semantic validation should succeed");
        assert!(is_valid, "Response should contain calculation results");
    })
    .await;
}

// =============================================================================
// Timeout Behavior Tests
// =============================================================================

/// Tests that `create_with_auto_functions` returns a timeout error when given
/// an impossibly short timeout.
#[tokio::test]
#[ignore = "requires GEMINI_API_KEY"]
async fn test_auto_functions_timeout_returns_error() {
    use std::time::Duration;

    with_timeout(TEST_TIMEOUT, async {
        let client = get_client().expect("GEMINI_API_KEY required");

        // Use an impossibly short timeout - 1ms is far too short for any API call
        let result = interaction_builder(&client)
            .with_text("What is 2 + 2?")
            .with_timeout(Duration::from_millis(1))
            .create_with_auto_functions()
            .await;

        // Should return a timeout error
        assert!(
            matches!(result, Err(GenaiError::Timeout(_))),
            "Expected GenaiError::Timeout, got: {:?}",
            result
        );

        println!("✓ create_with_auto_functions correctly returns timeout error");
    })
    .await;
}

/// Tests that `create_stream_with_auto_functions` returns a timeout error when given
/// an impossibly short timeout.
#[tokio::test]
#[ignore = "requires GEMINI_API_KEY"]
async fn test_auto_functions_stream_timeout_returns_error() {
    use futures_util::StreamExt;
    use std::time::Duration;

    with_timeout(TEST_TIMEOUT, async {
        let client = get_client().expect("GEMINI_API_KEY required");

        // Use an impossibly short timeout - 1ms is far too short for any API call
        let mut stream = interaction_builder(&client)
            .with_text("What is 2 + 2?")
            .with_timeout(Duration::from_millis(1))
            .create_stream_with_auto_functions();

        // Consume the stream - should get a timeout error
        let mut got_timeout = false;
        while let Some(result) = stream.next().await {
            match result {
                Ok(_) => continue,
                Err(GenaiError::Timeout(_)) => {
                    got_timeout = true;
                    break;
                }
                Err(e) => panic!("Expected GenaiError::Timeout, got: {:?}", e),
            }
        }

        assert!(got_timeout, "Stream should have yielded a timeout error");
        println!("✓ create_stream_with_auto_functions correctly returns timeout error");
    })
    .await;
}

/// Tests that timeout works correctly when functions are registered.
/// This validates the documented behavior that on timeout, previous function
/// executions are preserved on the API side but an error is returned.
#[tokio::test]
#[ignore = "requires GEMINI_API_KEY"]
async fn test_auto_functions_timeout_with_registered_functions() {
    use std::time::Duration;

    with_timeout(TEST_TIMEOUT, async {
        let client = get_client().expect("GEMINI_API_KEY required");

        // Use an impossibly short timeout with functions registered
        // The functions are registered via #[tool] macro (get_weather_test, etc.)
        let result = interaction_builder(&client)
            .with_text("What's the weather in Tokyo and what time is it in JST?")
            .with_timeout(Duration::from_millis(1))
            .create_with_auto_functions()
            .await;

        // Should return a timeout error even with functions registered
        assert!(
            matches!(result, Err(GenaiError::Timeout(_))),
            "Expected GenaiError::Timeout with registered functions, got: {:?}",
            result
        );

        println!("✓ Timeout correctly fires with registered functions");
        println!("  Note: Any function calls that completed before timeout are");
        println!("  preserved on the API side via previous_interaction_id chain");
    })
    .await;
}

// =============================================================================
// Stateless Function Calling Tests
// =============================================================================
//
// Tests for multi-turn function calling with `store: false` (stateless mode).
// In stateless mode, conversation history must be manually maintained.

/// Tests multi-turn function calling with stateless mode (store: false).
/// This validates the pattern documented in the MULTI_TURN_FUNCTION_CALLING.md guide.
#[tokio::test]
#[ignore = "requires GEMINI_API_KEY"]
async fn test_stateless_function_calling_multi_turn() {
    use rust_genai::InteractionInput;
    use rust_genai::interactions_api::{
        function_call_content, function_result_content, text_content,
    };

    with_timeout(EXTENDED_TEST_TIMEOUT, async {
        let client = get_client().expect("GEMINI_API_KEY required");

        let functions = vec![
            FunctionDeclaration::builder("get_weather")
                .description("Get the current weather for a city")
                .parameter(
                    "city",
                    json!({"type": "string", "description": "City name"}),
                )
                .required(vec!["city".to_string()])
                .build(),
            FunctionDeclaration::builder("get_time")
                .description("Get the current time in a timezone")
                .parameter(
                    "timezone",
                    json!({"type": "string", "description": "Timezone"}),
                )
                .required(vec!["timezone".to_string()])
                .build(),
        ];

        // Build conversation history manually
        let mut history: Vec<rust_genai::InteractionContent> = vec![];

        // Turn 1: User asks a question
        history.push(text_content("What's the weather in Tokyo?"));

        let response1 = client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_input(InteractionInput::Content(history.clone()))
            .with_functions(functions.clone())
            .with_store_disabled()
            .create()
            .await
            .expect("First turn failed");

        // Handle function calls if any
        let calls = response1.function_calls();
        assert!(
            !calls.is_empty(),
            "Expected function call for weather query"
        );

        for call in &calls {
            let call_id = call.id.expect("Function call should have ID");
            // Add function call to history
            history.push(function_call_content(call.name, call.args.clone()));
            // Execute and add result
            let result = json!({"city": "Tokyo", "temperature": "22°C", "conditions": "sunny"});
            history.push(function_result_content(call.name, call_id, result));
        }

        // Turn 2: Send function results (stateless - must send full history)
        let response2 = client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_input(InteractionInput::Content(history.clone()))
            .with_functions(functions.clone())
            .with_store_disabled()
            .create()
            .await
            .expect("Function result turn failed");

        // Should have a text response
        let text = response2.text();
        assert!(
            text.is_some(),
            "Expected text response after function result"
        );
        println!(
            "✓ Stateless function call turn 1 complete: {}",
            text.unwrap()
        );

        // Add model response to history
        if let Some(t) = text {
            history.push(text_content(t));
        }

        // Turn 3: Follow-up question (tests context preservation)
        history.push(text_content("What about the time there?"));

        let response3 = client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_input(InteractionInput::Content(history.clone()))
            .with_functions(functions.clone())
            .with_store_disabled()
            .create()
            .await
            .expect("Follow-up turn failed");

        // Should trigger another function call for time
        let calls3 = response3.function_calls();
        if !calls3.is_empty() {
            for call in &calls3 {
                let call_id = call.id.expect("Function call should have ID");
                history.push(function_call_content(call.name, call.args.clone()));
                let result = json!({"timezone": "JST", "time": "14:30"});
                history.push(function_result_content(call.name, call_id, result));
            }

            // Turn 4: Final response
            let response4 = client
                .interaction()
                .with_model("gemini-3-flash-preview")
                .with_input(InteractionInput::Content(history.clone()))
                .with_functions(functions)
                .with_store_disabled()
                .create()
                .await
                .expect("Final turn failed");

            assert!(response4.text().is_some(), "Expected final text response");
            println!("✓ Stateless multi-turn function calling complete");
        } else {
            // Model might have answered directly if it inferred from context
            assert!(
                response3.text().is_some(),
                "Expected either function call or text response"
            );
            println!("✓ Stateless multi-turn complete (model answered from context)");
        }
    })
    .await;
}

// =============================================================================
// Parallel Function Call Edge Cases
// =============================================================================

/// Tests that parallel function results can be returned in any order.
/// The API should accept results regardless of the order they're sent back.
#[tokio::test]
#[ignore = "requires GEMINI_API_KEY"]
async fn test_parallel_function_result_order_independence() {
    with_timeout(TEST_TIMEOUT, async {
        let client = get_client().expect("GEMINI_API_KEY required");

        let functions = vec![
            FunctionDeclaration::builder("get_weather")
                .description("Get weather for a city")
                .parameter("city", json!({"type": "string"}))
                .required(vec!["city".to_string()])
                .build(),
            FunctionDeclaration::builder("get_time")
                .description("Get time in timezone")
                .parameter("timezone", json!({"type": "string"}))
                .required(vec!["timezone".to_string()])
                .build(),
        ];

        // Prompt that should trigger parallel calls
        let response1 = stateful_builder(&client)
            .with_text("What's the weather in Tokyo and what time is it in JST?")
            .with_functions(functions)
            .create()
            .await
            .expect("Initial request failed");

        let calls: Vec<_> = response1
            .function_calls()
            .iter()
            .map(|c| {
                (
                    c.name.to_string(),
                    c.id.map(|s| s.to_string()),
                    c.args.clone(),
                )
            })
            .collect();

        if calls.len() >= 2 {
            println!("Got {} parallel function calls", calls.len());

            // Return results in REVERSE order to test order independence
            let mut results = Vec::new();
            for (name, id, _args) in calls.iter().rev() {
                let result = match name.as_str() {
                    "get_weather" => json!({"city": "Tokyo", "temp": "22°C"}),
                    "get_time" => json!({"timezone": "JST", "time": "14:30"}),
                    _ => json!({"result": "unknown"}),
                };
                results.push(function_result_content(
                    name,
                    id.as_ref().expect("call should have ID"),
                    result,
                ));
            }

            // Send results in reverse order
            let response2 = client
                .interaction()
                .with_model("gemini-3-flash-preview")
                .with_previous_interaction(response1.id.as_ref().expect("id required"))
                .with_content(results)
                .create()
                .await
                .expect("Function result turn failed - order might matter?");

            assert!(
                response2.text().is_some(),
                "Expected text response after reversed results"
            );
            println!("✓ Parallel results accepted in reverse order");
        } else {
            println!("⚠ Model didn't make parallel calls, skipping order test");
            // Still valid - just means model chose sequential approach
        }
    })
    .await;
}

/// Tests behavior when one function in a parallel batch returns an error.
/// The model should handle partial failures gracefully.
#[tokio::test]
#[ignore = "requires GEMINI_API_KEY"]
async fn test_parallel_function_partial_failure() {
    with_timeout(TEST_TIMEOUT, async {
        let client = get_client().expect("GEMINI_API_KEY required");

        let functions = vec![
            FunctionDeclaration::builder("get_weather")
                .description("Get weather for a city")
                .parameter("city", json!({"type": "string"}))
                .required(vec!["city".to_string()])
                .build(),
            FunctionDeclaration::builder("get_stock_price")
                .description("Get stock price (may fail)")
                .parameter("symbol", json!({"type": "string"}))
                .required(vec!["symbol".to_string()])
                .build(),
        ];

        // Prompt designed to trigger parallel calls
        let response1 = stateful_builder(&client)
            .with_text("What's the weather in Tokyo and what's the stock price of INVALID_STOCK?")
            .with_functions(functions)
            .create()
            .await
            .expect("Initial request failed");

        let calls: Vec<_> = response1
            .function_calls()
            .iter()
            .map(|c| {
                (
                    c.name.to_string(),
                    c.id.map(|s| s.to_string()),
                    c.args.clone(),
                )
            })
            .collect();

        if !calls.is_empty() {
            let mut results = Vec::new();
            for (name, id, _args) in &calls {
                let result = match name.as_str() {
                    "get_weather" => {
                        json!({"city": "Tokyo", "temp": "22°C", "conditions": "sunny"})
                    }
                    "get_stock_price" => {
                        // Return an error response
                        json!({"error": "Stock symbol not found", "code": "NOT_FOUND"})
                    }
                    _ => json!({"result": "ok"}),
                };
                results.push(function_result_content(
                    name,
                    id.as_ref().expect("call should have ID"),
                    result,
                ));
            }

            let response2 = client
                .interaction()
                .with_model("gemini-3-flash-preview")
                .with_previous_interaction(response1.id.as_ref().expect("id required"))
                .with_content(results)
                .create()
                .await
                .expect("Function result turn failed");

            let text = response2.text();
            assert!(
                text.is_some(),
                "Expected text response with partial results"
            );

            // Model should acknowledge both the success and the error
            println!("✓ Partial failure handled gracefully");
            println!("  Response: {}", text.unwrap());
        } else {
            println!("⚠ No function calls made, test inconclusive");
        }
    })
    .await;
}
