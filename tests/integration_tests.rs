use futures_util::{StreamExt, pin_mut};
use rust_genai::Client;
use std::env;

// Non-streaming test with system instruction
#[tokio::test]
async fn test_generate_content_with_system_instruction() {
    let api_key = match env::var("GEMINI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!(
                "Skipping test_generate_content_with_system_instruction: GEMINI_API_KEY not set."
            );
            return;
        }
    };
    // Create client
    let client = Client::new(api_key);

    let model = "gemini-1.5-flash-latest";
    let prompt = "What is the capital of France?";
    let system_instruction = "You are a helpful geography expert.";

    // Use the client method with builder pattern
    let result = client
        .with_model(model)
        .with_prompt(prompt)
        .with_system_instruction(system_instruction)
        .generate()
        .await;

    assert!(
        result.is_ok(),
        "generate_content failed: {:?}",
        result.err().unwrap()
    );

    // Result is now Ok(GenerateContentResponse)
    let response = result.unwrap();
    assert!(!response.text.is_empty(), "Generated text is empty");
    println!(
        "test_generate_content_with_system_instruction response: {}",
        response.text
    );
    assert!(
        response.text.to_lowercase().contains("paris"),
        "Response does not contain expected keyword 'paris'"
    );
}

// Non-streaming test without system instruction
#[tokio::test]
async fn test_generate_content_without_system_instruction() {
    let api_key = match env::var("GEMINI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!(
                "Skipping test_generate_content_without_system_instruction: GEMINI_API_KEY not set."
            );
            return;
        }
    };
    // Create client
    let client = Client::new(api_key);

    let model = "gemini-1.5-flash-latest";
    let prompt = "What is the capital of France?";

    // Use the client method without system instruction
    let result = client
        .with_model(model)
        .with_prompt(prompt)
        .generate()
        .await;

    assert!(
        result.is_ok(),
        "generate_content failed: {:?}",
        result.err().unwrap()
    );

    // Result is now Ok(GenerateContentResponse)
    let response = result.unwrap();
    assert!(!response.text.is_empty(), "Generated text is empty");
    println!(
        "test_generate_content_without_system_instruction response: {}",
        response.text
    );
    assert!(
        response.text.to_lowercase().contains("paris"),
        "Response does not contain expected keyword 'paris'"
    );
}

// Streaming test with system instruction
#[tokio::test]
async fn test_generate_content_stream_with_system_instruction() {
    let api_key = match env::var("GEMINI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!(
                "Skipping test_generate_content_stream_with_system_instruction: GEMINI_API_KEY not set."
            );
            return;
        }
    };
    // Create client
    let client = Client::new(api_key);

    let model = "gemini-1.5-flash-latest";
    let prompt = "Why is the sky blue?";
    let system_instruction =
        "You are a helpful science teacher who explains concepts in simple terms.";

    // Use the client stream method with builder pattern
    let stream = client
        .with_model(model)
        .with_prompt(prompt)
        .with_system_instruction(system_instruction)
        .stream();
    pin_mut!(stream);

    let mut collected_text = String::new();
    let mut chunk_count = 0;
    while let Some(result) = stream.next().await {
        match result {
            // Stream yields Ok(GenerateContentResponse)
            Ok(chunk) => {
                chunk_count += 1;
                // Access the .text field directly
                collected_text.push_str(&chunk.text);
            }
            Err(e) => {
                panic!("Stream returned an error: {:?}", e);
            }
        }
    }

    assert!(chunk_count > 0, "Stream did not yield any chunks.");
    assert!(!collected_text.is_empty(), "Collected text is empty.");
    println!(
        "test_generate_content_stream_with_system_instruction collected text ({} chunks): {}",
        chunk_count, collected_text
    );
    assert!(
        collected_text.to_lowercase().contains("scatter"),
        "Collected text does not contain expected keyword 'scatter'"
    );
}

// Streaming test without system instruction
#[tokio::test]
async fn test_generate_content_stream_without_system_instruction() {
    let api_key = match env::var("GEMINI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!(
                "Skipping test_generate_content_stream_without_system_instruction: GEMINI_API_KEY not set."
            );
            return;
        }
    };
    // Create client
    let client = Client::new(api_key);

    let model = "gemini-1.5-flash-latest";
    let prompt = "Why is the sky blue?";

    // Use the client stream method without system instruction
    let stream = client.with_model(model).with_prompt(prompt).stream();
    pin_mut!(stream);

    let mut collected_text = String::new();
    let mut chunk_count = 0;
    while let Some(result) = stream.next().await {
        match result {
            // Stream yields Ok(GenerateContentResponse)
            Ok(chunk) => {
                chunk_count += 1;
                // Access the .text field directly
                collected_text.push_str(&chunk.text);
            }
            Err(e) => {
                panic!("Stream returned an error: {:?}", e);
            }
        }
    }

    assert!(chunk_count > 0, "Stream did not yield any chunks.");
    assert!(!collected_text.is_empty(), "Collected text is empty.");
    println!(
        "test_generate_content_stream_without_system_instruction collected text ({} chunks): {}",
        chunk_count, collected_text
    );
    assert!(
        collected_text.to_lowercase().contains("scatter"),
        "Collected text does not contain expected keyword 'scatter'"
    );
}
