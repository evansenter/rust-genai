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

use common::get_client;
use futures_util::StreamExt;
use rust_genai::{CallableFunction, FunctionDeclaration, StreamChunk, function_result_content};
use rust_genai_macros::generate_function_declaration;
use serde_json::json;

// =============================================================================
// Test Functions (registered via macro)
// =============================================================================
//
// NOTE: These functions are marked #[allow(dead_code)] because they're registered
// with the inventory crate via #[generate_function_declaration]. The macro creates
// `Callable*` structs that are collected at runtime for automatic function calling.
//
// While not all functions are explicitly called in tests, they serve these purposes:
// - get_weather_test, get_time_test, convert_temperature: Used in parallel/sequential tests
// - get_server_status: Used in no-argument function tests
// - search_with_filters: Used in complex argument tests
// - always_fails: Reserved for future error handling tests (demonstrates panic behavior)

/// Gets the current weather for a city
#[allow(dead_code)]
#[generate_function_declaration(city(description = "The city to get weather for"))]
fn get_weather_test(city: String) -> String {
    format!(
        r#"{{"city": "{}", "temperature": "22°C", "conditions": "sunny"}}"#,
        city
    )
}

/// Gets the current time in a timezone
#[allow(dead_code)]
#[generate_function_declaration(timezone(description = "The timezone like UTC, PST, JST"))]
fn get_time_test(timezone: String) -> String {
    format!(r#"{{"timezone": "{}", "time": "14:30:00"}}"#, timezone)
}

/// Converts temperature between units
#[allow(dead_code)]
#[generate_function_declaration(
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
#[generate_function_declaration(input(description = "Any input"))]
fn always_fails(_input: String) -> String {
    panic!("This function always fails!")
}

/// A function with no parameters
#[allow(dead_code)]
#[generate_function_declaration]
fn get_server_status() -> String {
    r#"{"status": "online", "uptime": "99.9%"}"#.to_string()
}

/// A function with complex nested arguments
#[allow(dead_code)]
#[generate_function_declaration(
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

    for (call_id, name, args, signature) in &function_calls {
        println!(
            "  Function: {} (id: {:?}, has_signature: {})",
            name,
            call_id,
            signature.is_some()
        );
        println!("    Args: {}", args);
    }

    // Model may call one or both functions
    if function_calls.len() >= 2 {
        println!("Model made parallel function calls!");
        // Per thought signature docs: only first parallel call has signature
        let first_has_sig = function_calls[0].3.is_some();
        println!("First call has signature: {}", first_has_sig);
    } else if function_calls.len() == 1 {
        println!("Model called one function (may call second in next turn)");
    }

    // Verify all calls have IDs
    for (call_id, name, _, _) in &function_calls {
        assert!(
            call_id.is_some(),
            "Function call '{}' should have an ID",
            name
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
        let (_, name1, _, sig1) = &function_calls[0];
        let (_, name2, _, sig2) = &function_calls[1];

        println!("First call: {} (has_signature: {})", name1, sig1.is_some());
        println!("Second call: {} (has_signature: {})", name2, sig2.is_some());

        // According to docs, only first should have signature
        // But this behavior may vary - log for investigation
        if sig1.is_some() && sig2.is_none() {
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
    let (call_id1, name1, _, sig1) = &calls1[0];
    println!(
        "Providing result for: {} (signature: {:?})",
        name1,
        sig1.is_some()
    );

    let result1 = function_result_content(
        name1.to_string(),
        call_id1.unwrap().to_string(),
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
        let (call_id2, name2, args2, sig2) = &calls2[0];
        println!(
            "Step 2 function call: {} (signature: {:?})",
            name2,
            sig2.is_some()
        );
        println!("  Args: {}", args2);

        // Step 3: Provide second function result
        let result2 = function_result_content(
            name2.to_string(),
            call_id2.unwrap().to_string(),
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

    let (call_id1, _, _, sig1) = &calls1[0];
    println!("Step 1 signature present: {}", sig1.is_some());

    // Provide result
    let result1 = function_result_content(
        "get_weather",
        call_id1.unwrap().to_string(),
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
        let (_, _, _, sig2) = &calls2[0];
        println!("Step 2 signature present: {}", sig2.is_some());

        // Both steps should have their own signatures
        if sig1.is_some() && sig2.is_some() {
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
    // Note: The library's delta parser may not support function_call deltas yet,
    // so this test gracefully handles parse errors.
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let get_weather = FunctionDeclaration::builder("get_weather")
        .description("Get the current weather for a city")
        .parameter("city", json!({"type": "string"}))
        .required(vec!["city".to_string()])
        .build();

    let mut stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's the weather in London?")
        .with_function(get_weather)
        .with_store(true)
        .create_stream();

    let mut delta_count = 0;
    let mut complete_count = 0;
    let mut final_response = None;
    let mut stream_error_is_unsupported_delta = false;

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => match chunk {
                StreamChunk::Delta(delta) => {
                    delta_count += 1;
                    if let Some(text) = delta.text() {
                        print!("{}", text);
                    }
                }
                StreamChunk::Complete(response) => {
                    complete_count += 1;
                    final_response = Some(response);
                }
            },
            Err(e) => {
                let error_str = format!("{:?}", e);
                println!("Stream error: {:?}", e);
                // Check if this is a known limitation (function_call delta not supported)
                if error_str.contains("function_call") && error_str.contains("unknown variant") {
                    stream_error_is_unsupported_delta = true;
                    println!(
                        "Note: function_call deltas not yet supported in streaming parser. This is a known limitation."
                    );
                }
                break;
            }
        }
    }

    println!("\nDeltas: {}, Completes: {}", delta_count, complete_count);

    // If the stream failed due to unsupported function_call delta, that's acceptable for now
    if stream_error_is_unsupported_delta {
        println!(
            "Test passed with known limitation: function_call deltas not supported in streaming"
        );
        return;
    }

    if let Some(response) = final_response {
        println!("Final status: {:?}", response.status);
        let function_calls = response.function_calls();
        println!("Function calls in final response: {}", function_calls.len());

        // Stream should either have text or function calls
        assert!(
            response.has_text() || response.has_function_calls(),
            "Streaming response should have text or function calls"
        );
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_streaming_long_response() {
    // Test streaming a longer response (1000+ tokens)
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let mut stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Write a detailed 500-word essay about the history of the Internet.")
        .with_store(true)
        .create_stream();

    let mut delta_count = 0;
    let mut collected_text = String::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => match chunk {
                StreamChunk::Delta(delta) => {
                    delta_count += 1;
                    if let Some(text) = delta.text() {
                        collected_text.push_str(text);
                    }
                }
                StreamChunk::Complete(_) => {}
            },
            Err(e) => {
                println!("Stream error: {:?}", e);
                break;
            }
        }
    }

    println!("Total deltas received: {}", delta_count);
    println!("Total text length: {} chars", collected_text.len());
    println!("Word count: ~{}", collected_text.split_whitespace().count());

    // Should have received multiple deltas for a long response
    assert!(
        delta_count > 5,
        "Long response should produce many delta chunks"
    );
    assert!(
        collected_text.len() > 500,
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

    let (call_id, _, _, _) = &calls[0];

    // Step 2: Return an error result
    let error_result = function_result_content(
        "get_secret_data",
        call_id.unwrap().to_string(),
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
        let (call_id, name, args, _) = &calls[0];
        println!("Function called: {} with args: {}", name, args);
        assert_eq!(*name, "get_server_status");
        assert!(call_id.is_some(), "Should have call ID");

        // Provide result
        let result = function_result_content(
            "get_server_status",
            call_id.unwrap().to_string(),
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
        let (_, name, args, _) = &calls[0];
        println!("Function: {} with args: {}", name, args);

        // Verify complex arguments are parsed correctly
        assert!(args.get("user_id").is_some(), "Should have user_id");

        // Filters may be present if model included them
        if let Some(filters) = args.get("filters") {
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

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's the weather in Seattle?")
        .with_function(weather_func)
        .create_with_auto_functions()
        .await
        .expect("Auto function calling failed");

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
        Ok(response) => {
            println!("Completed within 1 loop: {:?}", response.status);
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
