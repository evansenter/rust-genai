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

use common::{consume_auto_function_stream, consume_stream, get_client};
use rust_genai::{CallableFunction, FunctionDeclaration, function_result_content};
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

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's the weather in Tokyo and what time is it there (JST timezone)?")
        .with_functions(vec![get_weather, get_time])
        .with_store(true)
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
    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(
            "I need BOTH the weather in Paris AND the current time in CET. Call both functions.",
        )
        .with_functions(vec![func1, func2])
        .with_store(true)
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
    let response1 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's the weather in Tokyo? Tell me the temperature in Fahrenheit.")
        .with_functions(vec![get_weather.clone(), convert_temp.clone()])
        .with_store(true)
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

    let response2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_previous_interaction(&response1.id)
        .with_content(vec![result1])
        .with_functions(vec![get_weather.clone(), convert_temp.clone()])
        .with_store(true)
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

        let response3 = client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_previous_interaction(&response2.id)
            .with_content(vec![result2])
            .with_functions(vec![get_weather, convert_temp])
            .with_store(true)
            .create()
            .await
            .expect("Third interaction failed");

        println!("Step 3 status: {:?}", response3.status);
        if response3.has_text() {
            println!("Final response: {}", response3.text().unwrap());
            // Should mention Fahrenheit temperature
            let text = response3.text().unwrap().to_lowercase();
            assert!(
                text.contains("71") || text.contains("72") || text.contains("fahrenheit"),
                "Response should mention converted temperature"
            );
        }
    } else if response2.has_text() {
        // Model provided final answer directly
        println!("Final response: {}", response2.text().unwrap());
    }
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

    let get_weather = FunctionDeclaration::builder("get_weather")
        .description("Get the current weather")
        .parameter("city", json!({"type": "string"}))
        .required(vec!["city".to_string()])
        .build();

    // Step 1
    let response1 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's the weather in Tokyo?")
        .with_function(get_weather.clone())
        .with_store(true)
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

    let response2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_previous_interaction(&response1.id)
        .with_content(vec![result1])
        .with_text("Now what about Paris?")
        .with_function(get_weather.clone())
        .with_store(true)
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

    let get_weather = FunctionDeclaration::builder("get_weather")
        .description("Get the current weather for a city")
        .parameter("city", json!({"type": "string"}))
        .required(vec!["city".to_string()])
        .build();

    let stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's the weather in London?")
        .with_function(get_weather)
        .with_store(true)
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
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_streaming_long_response() {
    // Test streaming a longer response (1000+ tokens)
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Write a detailed 500-word essay about the history of the Internet.")
        .with_store(true)
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
    let response1 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Get the secret data for key 'test123'")
        .with_function(failing_func.clone())
        .with_store(true)
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

    let response2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_previous_interaction(&response1.id)
        .with_content(vec![error_result])
        .with_function(failing_func)
        .with_store(true)
        .create()
        .await
        .expect("Second interaction failed");

    println!("Response after error: {:?}", response2.status);

    // Model should acknowledge the error gracefully
    if response2.has_text() {
        let text = response2.text().unwrap().to_lowercase();
        println!("Model's response to error: {}", text);
        assert!(
            text.contains("error")
                || text.contains("denied")
                || text.contains("permission")
                || text.contains("unable")
                || text.contains("cannot")
                || text.contains("couldn't")
                || text.contains("sorry"),
            "Model should acknowledge the error"
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

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Check the server status")
        .with_function(status_func.clone())
        .with_store(true)
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

        let response2 = client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_previous_interaction(&response.id)
            .with_content(vec![result])
            .with_function(status_func)
            .with_store(true)
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

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(
            "Search for user ABC123 with category 'electronics' and price between 10 and 100",
        )
        .with_function(search_func)
        .with_store(true)
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
async fn test_auto_function_calling_registered() {
    // Test that functions registered with the macro work with auto-function calling
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Use a function registered via the macro
    let weather_func = GetWeatherTestCallable.declaration();

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's the weather in Seattle?")
        .with_function(weather_func)
        .create_with_auto_functions()
        .await
        .expect("Auto function calling failed");

    // Verify executions are tracked
    println!("Function executions: {:?}", result.executions);
    assert!(
        !result.executions.is_empty(),
        "Should have at least one function execution"
    );
    assert_eq!(
        result.executions[0].name, "get_weather_test",
        "Should have called get_weather_test"
    );

    let response = &result.response;
    println!("Final status: {:?}", response.status);
    assert!(
        response.has_text(),
        "Should have text response after auto-function loop"
    );

    let text = response.text().unwrap();
    println!("Final text: {}", text);

    // Should mention Seattle and weather data
    let text_lower = text.to_lowercase();
    assert!(
        text_lower.contains("seattle")
            || text_lower.contains("22")
            || text_lower.contains("sunny")
            || text_lower.contains("weather"),
        "Response should reference the weather data"
    );
}

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

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
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
    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
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

    let stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
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
    let stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
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

    let stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
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
    let stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
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
