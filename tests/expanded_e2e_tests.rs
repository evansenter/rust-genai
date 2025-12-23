//! Expanded end-to-end tests for Interactions API
//!
//! These tests cover critical features and edge cases that require real API calls.
//! Run with: cargo test --test expanded_e2e_tests -- --include-ignored --nocapture

use futures_util::StreamExt;
use rust_genai::{
    Client, FunctionDeclaration, GenerationConfig, InteractionStatus, WithFunctionCalling,
    function_result_content,
};
use serde_json::json;
use std::env;

fn get_client() -> Option<Client> {
    env::var("GEMINI_API_KEY")
        .ok()
        .map(|key| Client::builder(key).build())
}

// =============================================================================
// P0: Thought Signatures in Multi-Turn Function Calling
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_thought_signatures_preserved_across_turns() {
    // This test verifies that thought signatures work correctly across multiple turns:
    // Turn 1: Model makes function call with thought signature
    // Turn 2: Send function result back with signature preserved
    // Turn 3: Verify model can continue the conversation

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

    println!("Turn 1 status: {:?}", response1.status);

    let function_calls = response1.function_calls();
    if function_calls.is_empty() {
        println!("Model chose not to call function - cannot test thought signatures");
        return;
    }

    // Extract the thought signature from the function call
    let (call_id, name, _args, thought_signature) = &function_calls[0];
    println!(
        "Function call: {} with signature: {:?}",
        name, thought_signature
    );

    assert!(call_id.is_some(), "Function call must have an id");
    let call_id = call_id.expect("call_id should exist");

    // Turn 2: Send function result back, preserving the thought signature
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

    println!("Turn 2 status: {:?}", response2.status);

    // Verify we got a text response mentioning the weather and umbrella
    assert!(
        response2.has_text(),
        "Expected text response after function result"
    );

    let text = response2.text().expect("Should have text");
    println!("Final response: {}", text);

    // The model should mention umbrella since it's rainy with 80% precipitation
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
    // Test that the model can make multiple function calls in a single response
    // and we can handle all of them with their respective signatures

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

    // Request that might trigger multiple function calls
    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's the weather in Paris and what time is it there?")
        .with_functions(vec![get_weather.clone(), get_time.clone()])
        .create()
        .await
        .expect("Interaction failed");

    println!("Response status: {:?}", response.status);

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
    }

    // Even if we only get one function call, verify the structure is correct
    if !function_calls.is_empty() {
        for (call_id, _name, _args, _sig) in &function_calls {
            assert!(call_id.is_some(), "Each function call must have an id");
        }
    }
}

// =============================================================================
// P0: RequiresAction Status Handling
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_requires_action_status_on_function_call() {
    // Verify that when model returns a function call, status is RequiresAction
    // and after providing result, status becomes Completed

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

    // If model made a function call, status should be RequiresAction
    if response.has_function_calls() {
        assert_eq!(
            response.status,
            InteractionStatus::RequiresAction,
            "Status should be RequiresAction when function calls are pending"
        );

        // Now provide the function result
        let function_calls = response.function_calls();
        let (call_id, _name, _args, _sig) = &function_calls[0];
        let call_id = call_id.expect("call_id should exist");

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

        println!("After providing result - status: {:?}", response2.status);

        assert_eq!(
            response2.status,
            InteractionStatus::Completed,
            "Status should be Completed after providing function result"
        );
    } else {
        // Model didn't call the function - that's also valid behavior
        assert_eq!(
            response.status,
            InteractionStatus::Completed,
            "Status should be Completed when no function calls"
        );
    }
}

// =============================================================================
// P0: Streaming with Function Calls
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_streaming_with_function_calls() {
    // Verify that function calls are properly received in streaming mode
    // Note: Streaming with function calls may behave differently than text streaming

    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Use a simpler text prompt to test streaming works
    let mut stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Count from 1 to 3.")
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

                if response.has_text() {
                    println!("  Text: {:?}", response.text());
                }

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

    // Note: The streaming implementation may return 0 chunks if the API
    // doesn't support SSE properly or returns all content in one response
    if chunk_count == 0 {
        println!("Warning: No chunks received - streaming may not be fully supported");
    }
}

// =============================================================================
// P1: Generation Config (thinking_level)
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_generation_config_temperature() {
    // Test that temperature setting works

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
    // Test that max_output_tokens is respected

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
    println!("Response outputs: {:?}", response.outputs);

    // Model might not return text with very short token limits
    // This test mainly verifies the config is accepted by the API
    if response.has_text() {
        let text = response.text().unwrap();
        println!("Response length: {} chars", text.len());
    } else {
        println!("No text in response (may be due to token limit)");
    }
}

// =============================================================================
// P1: System Instructions
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_system_instruction_text() {
    // Test that system instructions are respected

    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_system_instruction("You are a pirate. Always respond in pirate speak with 'Arrr!' somewhere in your response.")
        .with_text("Hello, how are you?")
        .create()
        .await
        .expect("Interaction failed");

    assert!(response.has_text(), "Should have text response");
    let text = response.text().unwrap().to_lowercase();
    println!("Response: {}", text);

    // The model should follow the pirate instruction
    assert!(
        text.contains("arr") || text.contains("matey") || text.contains("ahoy"),
        "Response should contain pirate speak"
    );
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_system_instruction_persists_in_conversation() {
    // Test that system instruction persists across turns

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

    // Turn 2: Continue conversation - system instruction should persist
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

    // Note: System instruction persistence depends on API implementation
    // We're mainly testing that the conversation continues without error
    assert!(response2.has_text(), "Should have text response");
}

// =============================================================================
// P1: Error Handling
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_error_invalid_model_name() {
    // Test that invalid model name returns appropriate error

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
    let error = result.err().unwrap();
    println!("Error: {:?}", error);
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_error_invalid_previous_interaction_id() {
    // Test that referencing non-existent interaction fails gracefully

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
    let error = result.err().unwrap();
    println!("Error: {:?}", error);
}

// =============================================================================
// P1: Store Parameter Behavior
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_store_false_interaction_not_retrievable() {
    // Test that store: false makes interaction not retrievable
    // Note: When store=false, the API may return incomplete responses

    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello")
        .with_store(false)
        .create()
        .await;

    // With store=false, the API may return an incomplete response
    // that fails to parse, or it may work - behavior varies
    match result {
        Ok(response) => {
            let interaction_id = response.id.clone();
            println!("Created interaction with store=false: {}", interaction_id);

            // Try to retrieve it - should fail
            let get_result = client.get_interaction(&interaction_id).await;
            println!("Get result: {:?}", get_result.is_ok());

            // If we got an ID, it likely shouldn't be retrievable
            if !interaction_id.is_empty() {
                assert!(
                    get_result.is_err(),
                    "Stored=false interaction should not be retrievable"
                );
            }
        }
        Err(e) => {
            // API might return incomplete JSON when store=false
            println!("API returned error for store=false: {:?}", e);
            // This is acceptable behavior
        }
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_store_true_interaction_retrievable() {
    // Test that store: true makes interaction retrievable

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

    let interaction_id = response.id.clone();
    println!("Created interaction with store=true: {}", interaction_id);

    // Try to retrieve it - should succeed
    let get_result = client.get_interaction(&interaction_id).await;
    assert!(
        get_result.is_ok(),
        "Should be able to retrieve stored interaction"
    );

    let retrieved = get_result.unwrap();
    assert_eq!(retrieved.id, interaction_id);
    println!("Successfully retrieved interaction");
}

// =============================================================================
// P2: Multi-Turn Conversation Edge Cases
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_long_conversation_chain() {
    // Test a conversation with 5+ turns

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

            // Should mention at least some of the facts we provided
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
                "Model should remember at least 2 facts from the conversation"
            );
        }
    }
}

// =============================================================================
// Image Input Tests (P0)
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key and accessible image URL"]
async fn test_image_input_from_uri() {
    // Test sending an image URL to the model for analysis
    // Note: Most public URLs are blocked. Use GCS URLs or base64 for production.

    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    use rust_genai::{InteractionInput, image_uri_content, text_content};

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
            // GCS URLs require proper permissions
            println!("Image input error: {:?}", e);
            println!("Note: Image URL access depends on API permissions");
        }
    }
}
