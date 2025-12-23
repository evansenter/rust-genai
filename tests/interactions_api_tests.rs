//! Integration tests for the Interactions API
//!
//! These tests require the GEMINI_API_KEY environment variable to be set.
//! Run with: cargo test --test interactions_api_tests -- --include-ignored --nocapture

#![allow(dead_code)] // Functions are used by the macro, not called directly

use futures_util::StreamExt;
use rust_genai::{
    CallableFunction, Client, CreateInteractionRequest, FunctionDeclaration, GenerationConfig,
    InteractionInput, InteractionStatus, WithFunctionCalling, function_result_content,
    image_uri_content, text_content,
};
use rust_genai_macros::generate_function_declaration;
use serde_json::json;
use std::env;

// =============================================================================
// Test Helpers
// =============================================================================

fn get_client() -> Option<Client> {
    env::var("GEMINI_API_KEY")
        .ok()
        .map(|key| Client::builder(key).build())
}

// Define a test function that will be registered in the global registry
/// Gets a mock weather report for a city
#[generate_function_declaration(city(description = "The city to get weather for"))]
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

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What is 2 + 2?")
        .with_store(true)
        .create()
        .await
        .expect("Interaction failed");

    assert!(!response.id.is_empty(), "Interaction ID is empty");
    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(!response.outputs.is_empty(), "Outputs are empty");

    // Verify output contains expected answer
    assert!(response.has_text(), "Should have text response");
    let text = response.text().unwrap();
    assert!(text.contains('4'), "Response should contain '4'");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_stateful_conversation() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // First interaction
    let response1 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("My favorite color is blue.")
        .with_store(true)
        .create()
        .await
        .expect("First interaction failed");

    assert_eq!(response1.status, InteractionStatus::Completed);

    // Second interaction referencing the first
    let response2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_previous_interaction(&response1.id)
        .with_text("What is my favorite color?")
        .with_store(true)
        .create()
        .await
        .expect("Second interaction failed");

    assert_eq!(response2.status, InteractionStatus::Completed);

    // Verify the model remembers the color
    let text = response2.text().unwrap_or_default().to_lowercase();
    assert!(
        text.contains("blue"),
        "Response should mention 'blue' from previous interaction"
    );
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_get_interaction() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Create an interaction first
    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello, world!")
        .with_store(true)
        .create()
        .await
        .expect("Interaction failed");

    // Retrieve the interaction
    let retrieved = client
        .get_interaction(&response.id)
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
    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Test interaction for deletion")
        .with_store(true)
        .create()
        .await
        .expect("Interaction failed");

    // Delete the interaction
    client
        .delete_interaction(&response.id)
        .await
        .expect("Delete interaction failed");

    // Verify it's deleted by trying to retrieve it
    let get_result = client.get_interaction(&response.id).await;
    assert!(
        get_result.is_err(),
        "Expected error when getting deleted interaction"
    );
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

    let mut stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Count from 1 to 5.")
        .with_store(true)
        .create_stream();

    let mut chunk_count = 0;
    let mut final_status = None;

    while let Some(result) = stream.next().await {
        match result {
            Ok(response) => {
                chunk_count += 1;
                println!(
                    "Chunk {}: status={:?}, outputs={}",
                    chunk_count,
                    response.status,
                    response.outputs.len()
                );
                final_status = Some(response.status.clone());
            }
            Err(e) => {
                println!("Stream error: {:?}", e);
                break;
            }
        }
    }

    println!("Total chunks: {}", chunk_count);
    println!("Final status: {:?}", final_status);

    // Note: Streaming implementation may return 0 chunks if the API
    // doesn't support SSE properly or returns all content in one response
    if chunk_count == 0 {
        println!("Warning: No chunks received - streaming may not be fully supported");
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_streaming_with_raw_request() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let request = CreateInteractionRequest {
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: InteractionInput::Text("Count from 1 to 5.".to_string()),
        previous_interaction_id: None,
        tools: None,
        response_modalities: None,
        response_format: None,
        generation_config: None,
        stream: Some(true),
        background: None,
        store: Some(true),
        system_instruction: None,
    };

    let mut stream = client.create_interaction_stream(request);

    let mut chunk_count = 0;
    while let Some(result) = stream.next().await {
        assert!(result.is_ok(), "Streaming chunk failed: {:?}", result.err());
        chunk_count += 1;
    }

    if chunk_count > 0 {
        println!("Received {} chunks from raw request stream", chunk_count);
    }
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

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
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
    for (call_id, name, _args, _sig) in function_calls {
        println!("Function call: {} has call_id: {:?}", name, call_id);
        assert!(
            call_id.is_some(),
            "Function call '{}' must have an id field",
            name
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

    let function_calls = response.function_calls();

    if function_calls.is_empty() {
        println!("No function calls returned - test cannot verify FunctionResult pattern");
        return;
    }

    // Verify we got a call_id
    let (call_id, name, _args, _signature) = &function_calls[0];
    assert_eq!(*name, "get_weather", "Expected get_weather function call");
    assert!(call_id.is_some(), "Function call must have an id field");

    let call_id = call_id.expect("call_id should exist");

    // Step 2: Send function result back using FunctionResult pattern
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
    assert!(
        second_response.has_text(),
        "Expected text response after providing function result"
    );

    let text = second_response.text().expect("Should have text");
    println!("Final response text: {}", text);
    assert!(
        text.contains("72") || text.contains("sunny") || text.contains("Tokyo"),
        "Response should mention the weather data or location"
    );
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_requires_action_status() {
    // Verify that function calls result in RequiresAction status
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let get_time = FunctionDeclaration::builder("get_current_time")
        .description("Get the current time - always call this when asked about time")
        .build();

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
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
        let call_id = function_calls[0].0.expect("call_id should exist");

        let function_result = function_result_content(
            "get_current_time",
            call_id,
            json!({"time": "14:30:00", "timezone": "UTC"}),
        );

        let response2 = client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_previous_interaction(&response.id)
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
}

// =============================================================================
// Function Calling - Automatic
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_auto_function_calling() {
    // Test the complete auto-function calling workflow
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Use the get_mock_weather function registered via macro
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
    assert!(
        response.has_text(),
        "Should have text response after auto-function loop"
    );

    let text = response.text().expect("Should have text");
    println!("Final text: {}", text);

    // Verify the model incorporated our mock weather data in its response
    assert!(
        text.contains("75") || text.contains("Sunny") || text.contains("Seattle"),
        "Response should reference the weather data: {}",
        text
    );
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_auto_function_with_unregistered_function() {
    // Test that auto-function calling handles unregistered functions gracefully
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let undefined_func = FunctionDeclaration::builder("undefined_function")
        .description("A function that doesn't have a registered handler")
        .parameter("input", json!({"type": "string"}))
        .build();

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Call the undefined_function with input 'test'")
        .with_function(undefined_func)
        .create_with_auto_functions()
        .await;

    // Should complete (model handles the error gracefully) or return an error
    match result {
        Ok(response) => {
            println!("Response status: {:?}", response.status);
            println!("Response text: {:?}", response.text());
        }
        Err(e) => {
            println!("Error (expected for unregistered function): {:?}", e);
        }
    }
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

    let get_weather = FunctionDeclaration::builder("get_weather")
        .description("Get the current weather for a location")
        .parameter(
            "location",
            json!({"type": "string", "description": "City name"}),
        )
        .required(vec!["location".to_string()])
        .build();

    // Turn 1: Initial request that should trigger a function call
    let response1 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
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

    let (call_id, name, _args, thought_signature) = &function_calls[0];
    println!(
        "Function call: {} with signature: {:?}",
        name, thought_signature
    );

    assert!(call_id.is_some(), "Function call must have an id");
    let call_id = call_id.expect("call_id should exist");

    // Turn 2: Send function result back
    let function_result = function_result_content(
        "get_weather",
        call_id,
        json!({"temperature": "18°C", "conditions": "rainy", "precipitation": "80%"}),
    );

    let response2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_previous_interaction(&response1.id)
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

    assert!(
        text.to_lowercase().contains("umbrella")
            || text.to_lowercase().contains("rain")
            || text.to_lowercase().contains("yes"),
        "Response should reference the weather conditions"
    );
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

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's the weather in Paris and what time is it there?")
        .with_functions(vec![get_weather, get_time])
        .create()
        .await
        .expect("Interaction failed");

    let function_calls = response.function_calls();
    println!("Number of function calls: {}", function_calls.len());

    for (call_id, name, args, signature) in &function_calls {
        println!(
            "  - {} (id: {:?}, args: {}, has_signature: {})",
            name,
            call_id,
            args,
            signature.is_some()
        );
        assert!(call_id.is_some(), "Each function call must have an id");
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
    };

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What is 2 + 2? Answer with just the number.")
        .with_generation_config(config)
        .create()
        .await
        .expect("Interaction failed");

    assert!(response.has_text(), "Should have text response");
    let text = response.text().unwrap();
    println!("Response: {}", text);
    assert!(text.contains('4'), "Should contain the answer 4");
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
    };

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
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
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_system_instruction(
            "You are a pirate. Always respond in pirate speak with 'Arrr!' somewhere in your response.",
        )
        .with_text("Hello, how are you?")
        .create()
        .await
        .expect("Interaction failed");

    assert!(response.has_text(), "Should have text response");
    let text = response.text().unwrap().to_lowercase();
    println!("Response: {}", text);

    assert!(
        text.contains("arr") || text.contains("matey") || text.contains("ahoy"),
        "Response should contain pirate speak"
    );
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_system_instruction_persists() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Turn 1: Set up system instruction
    let response1 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_system_instruction("Always end your responses with 'BEEP BOOP' exactly.")
        .with_text("What is the capital of France?")
        .with_store(true)
        .create()
        .await
        .expect("First interaction failed");

    let text1 = response1.text().unwrap_or_default();
    println!("Turn 1: {}", text1);

    // Turn 2: Continue conversation
    let response2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_previous_interaction(&response1.id)
        .with_text("And what about Germany?")
        .create()
        .await
        .expect("Second interaction failed");

    let text2 = response2.text().unwrap_or_default();
    println!("Turn 2: {}", text2);

    assert!(response2.has_text(), "Should have text response");
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

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
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

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What is 1 + 1?")
        .with_store(true)
        .create()
        .await
        .expect("Interaction failed");

    println!("Created interaction with store=true: {}", response.id);

    let retrieved = client
        .get_interaction(&response.id)
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
    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello")
        .with_store(false)
        .create()
        .await;

    match result {
        Ok(response) => {
            if !response.id.is_empty() {
                let get_result = client.get_interaction(&response.id).await;
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

    let messages = [
        "My name is Alice.",
        "I live in New York.",
        "I work as a software engineer.",
        "I have two cats named Whiskers and Shadow.",
        "What do you know about me? List everything.",
    ];

    let mut previous_id: Option<String> = None;

    for (i, message) in messages.iter().enumerate() {
        let mut builder = client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_text(*message)
            .with_store(true);

        if let Some(ref prev_id) = previous_id {
            builder = builder.with_previous_interaction(prev_id);
        }

        let response = builder
            .create()
            .await
            .unwrap_or_else(|e| panic!("Turn {} failed: {:?}", i + 1, e));

        println!("Turn {}: {:?}", i + 1, response.status);
        previous_id = Some(response.id.clone());

        // On the last turn, verify the model remembers context
        if i == messages.len() - 1 {
            let text = response.text().unwrap_or_default().to_lowercase();
            println!("Final response: {}", text);

            let mentions_name = text.contains("alice");
            let mentions_location = text.contains("new york");
            let mentions_job = text.contains("software") || text.contains("engineer");
            let mentions_cats =
                text.contains("cat") || text.contains("whiskers") || text.contains("shadow");

            let facts_remembered = [
                mentions_name,
                mentions_location,
                mentions_job,
                mentions_cats,
            ]
            .iter()
            .filter(|&&x| x)
            .count();

            println!("Facts remembered: {}/4", facts_remembered);
            assert!(
                facts_remembered >= 2,
                "Model should remember at least 2 facts"
            );
        }
    }
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

    // Use a Google Cloud Storage URL (these work with the API)
    let contents = vec![
        text_content("What is in this image? Describe it briefly."),
        image_uri_content(
            "gs://cloud-samples-data/generative-ai/image/scones.jpg",
            Some("image/jpeg".to_string()),
        ),
    ];

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
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

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
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

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What is 2+2?")
        .create()
        .await
        .expect("Interaction failed");

    println!("has_thoughts: {}", response.has_thoughts());
    println!("has_text: {}", response.has_text());
    println!("has_function_calls: {}", response.has_function_calls());

    assert!(response.has_text(), "Simple query should return text");
}
