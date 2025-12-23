#![allow(dead_code)] // Functions are used by the macro, not called directly

//! Integration tests for function calling with the Interactions API
//! These tests require GEMINI_API_KEY environment variable to be set

use rust_genai::{CallableFunction, Client, FunctionDeclaration, WithFunctionCalling};
use rust_genai_macros::generate_function_declaration;
use serde_json::json;
use std::env;

// Define a test function that will be registered in the global registry
/// Gets a mock weather report for a city
#[generate_function_declaration(city(description = "The city to get weather for"))]
fn get_mock_weather(city: String) -> String {
    format!("Weather in {}: Sunny, 75°F", city)
}

#[tokio::test]
#[ignore] // Requires API key
async fn test_manual_function_calling_with_function_result() {
    // This test verifies that the new FunctionResult pattern works with the real API
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");
    let client = Client::builder(api_key).build();

    // Define a simple function
    let get_weather = FunctionDeclaration::builder("get_weather")
        .description("Get the current weather for a location")
        .parameter(
            "location",
            json!({"type": "string", "description": "City name"}),
        )
        .required(vec!["location".to_string()])
        .build();

    // Step 1: Send initial request with function declaration
    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's the weather in Tokyo?")
        .with_function(get_weather.clone())
        .create()
        .await
        .expect("First interaction failed");

    println!("First response status: {:?}", response.status);
    println!("First response outputs: {:?}", response.outputs);

    // Extract function calls
    let function_calls = response.function_calls();

    if function_calls.is_empty() {
        println!("No function calls returned - test cannot verify FunctionResult pattern");
        println!("Response text: {:?}", response.text());
        // This is not necessarily a failure - the model might choose not to call the function
        // But we'll skip the rest of the test
        return;
    }

    // Verify we got a call_id
    let (call_id, name, args, _signature) = &function_calls[0];
    println!(
        "Function call: name={}, call_id={:?}, args={:?}",
        name, call_id, args
    );

    assert_eq!(*name, "get_weather", "Expected get_weather function call");
    assert!(
        call_id.is_some(),
        "CRITICAL: Function call must have an id field"
    );

    let call_id = call_id.expect("call_id should exist");

    // Step 2: Send function result back using new FunctionResult pattern
    use rust_genai::function_result_content;

    let function_result = function_result_content(
        "get_weather",
        call_id,
        json!({"temperature": "72°F", "conditions": "sunny"}),
    );

    let second_response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_previous_interaction(&response.id)
        .with_content(vec![function_result])
        .with_function(get_weather)
        .create()
        .await
        .expect("Second interaction failed");

    println!("Second response status: {:?}", second_response.status);
    println!("Second response: {:?}", second_response.text());

    // Verify we got a text response (not another function call)
    assert!(
        second_response.has_text(),
        "Expected text response after providing function result"
    );

    let text = second_response.text().expect("Should have text");
    println!("Final response text: {}", text);

    // Verify the response mentions the weather data we provided
    assert!(
        text.contains("72") || text.contains("sunny") || text.contains("Tokyo"),
        "Response should mention the weather data or location"
    );
}

#[tokio::test]
#[ignore] // Requires API key
async fn test_simple_text_interaction() {
    // This test verifies basic interaction without function calling
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");
    let client = Client::builder(api_key).build();

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello, how are you?")
        .create()
        .await
        .expect("Interaction failed");

    println!("Response status: {:?}", response.status);
    assert!(response.has_text(), "Should have text response");
    println!("Response: {}", response.text().unwrap_or("No text"));
}

#[tokio::test]
#[ignore] // Requires API key
async fn test_function_call_has_id_field() {
    // This test specifically verifies that the API returns function calls with id fields
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");
    let client = Client::builder(api_key).build();

    let get_current_time = FunctionDeclaration::builder("get_current_time")
        .description("Get the current time")
        .build();

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What time is it?")
        .with_function(get_current_time)
        .create()
        .await
        .expect("Interaction failed");

    println!("Response outputs: {:?}", response.outputs);

    // Check if we got function calls
    let function_calls = response.function_calls();

    if function_calls.is_empty() {
        println!("Model chose not to call function - skipping id verification");
        return;
    }

    // Verify all function calls have IDs
    for (call_id, name, _args, _sig) in function_calls {
        println!("Function call: {} has call_id: {:?}", name, call_id);
        assert!(
            call_id.is_some(),
            "Function call '{}' must have an id field as per API spec",
            name
        );
    }
}

#[tokio::test]
#[ignore] // Requires API key
async fn test_full_auto_function_loop() {
    // This test verifies the complete auto-function calling workflow:
    // 1. Function registered via macro
    // 2. Model calls the function
    // 3. Auto-functions executes it
    // 4. Result sent back and final response generated

    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");
    let client = Client::builder(api_key).build();

    // Use the get_mock_weather function registered at the top of this file
    // via #[generate_function_declaration]
    let weather_func = GetMockWeatherCallable.declaration();

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's the weather like in Seattle?")
        .with_function(weather_func)
        .create_with_auto_functions()
        .await
        .expect("Auto-function call failed");

    println!("Final response status: {:?}", response.status);
    println!("Final response: {:?}", response.text());

    // The response should contain text mentioning the weather from our mock function
    assert!(
        response.has_text(),
        "Should have text response after auto-function loop"
    );

    let text = response.text().expect("Should have text");
    println!("Final text: {}", text);

    // Verify the model incorporated our mock weather data in its response
    // Our mock returns "Weather in {city}: Sunny, 75°F"
    assert!(
        text.contains("75") || text.contains("Sunny") || text.contains("Seattle"),
        "Response should reference the weather data: {}",
        text
    );
}
