use futures_util::{StreamExt, pin_mut};
use rust_genai::Client;
use std::env;

// Note: These integration tests make real API calls and may hit rate limits
// on free tier API keys (10 requests per minute). To run these tests:
// 1. Use a paid API key with higher rate limits, or
// 2. Run tests individually with delays between them, or
// 3. Run with: cargo test --test integration_tests -- --ignored

// Non-streaming test with system instruction
#[tokio::test]
#[ignore = "Makes real API calls - may hit rate limits"]
async fn test_generate_content_with_system_instruction() {
    let Ok(api_key) = env::var("GEMINI_API_KEY") else {
        println!("Skipping test_generate_content_with_system_instruction: GEMINI_API_KEY not set.");
        return;
    };
    let client = Client::builder(api_key).build();
    let model_name = "gemini-3-flash-preview";
    let prompt = "What is the capital of France?";
    let system_instruction = "You are a helpful geography expert.";

    let result = client
        .with_model(model_name)
        .with_prompt(prompt)
        .with_system_instruction(system_instruction)
        .generate()
        .await;

    assert!(
        result.is_ok(),
        "generate_content failed: {:?}",
        result.err()
    );
    let response = result.unwrap();
    assert!(
        response.text.as_deref().is_some_and(|s| !s.is_empty()),
        "Generated text is empty"
    );
    println!(
        "test_generate_content_with_system_instruction response: {}",
        response.text.as_deref().unwrap_or_default()
    );
    assert!(
        response
            .text
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains("paris"),
        "Response does not contain expected keyword 'paris'"
    );
}

// Non-streaming test without system instruction
#[tokio::test]
#[ignore = "Makes real API calls - may hit rate limits"]
async fn test_generate_content_without_system_instruction() {
    let Ok(api_key) = env::var("GEMINI_API_KEY") else {
        println!(
            "Skipping test_generate_content_without_system_instruction: GEMINI_API_KEY not set."
        );
        return;
    };
    let client = Client::builder(api_key).build();
    let model_name = "gemini-3-flash-preview";
    let prompt = "What is the capital of Germany?";

    let result = client
        .with_model(model_name)
        .with_prompt(prompt)
        .generate()
        .await;

    assert!(
        result.is_ok(),
        "generate_content failed: {:?}",
        result.err()
    );
    let response = result.unwrap();
    assert!(
        response.text.as_deref().is_some_and(|s| !s.is_empty()),
        "Generated text is empty"
    );
    println!(
        "test_generate_content_without_system_instruction response: {}",
        response.text.as_deref().unwrap_or_default()
    );
    assert!(
        response
            .text
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains("berlin"),
        "Response does not contain expected keyword 'berlin'"
    );
}

// Streaming test with system instruction
#[tokio::test]
#[ignore = "Makes real API calls - may hit rate limits"]
async fn test_generate_content_stream_with_system_instruction() {
    let Ok(api_key) = env::var("GEMINI_API_KEY") else {
        println!(
            "Skipping test_generate_content_stream_with_system_instruction: GEMINI_API_KEY not set."
        );
        return;
    };
    let client = Client::builder(api_key).build();
    let model_name = "gemini-3-flash-preview";
    let prompt = "Why is grass green?";
    let system_instruction = "Explain simply.";

    let stream_result = client
        .with_model(model_name)
        .with_prompt(prompt)
        .with_system_instruction(system_instruction)
        .generate_stream();

    let stream = match stream_result {
        Ok(s) => s,
        Err(e) => panic!("Failed to create stream: {e:?}"),
    };
    pin_mut!(stream);
    let mut collected_text = String::new();
    let mut chunk_count = 0;
    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                chunk_count += 1;
                if let Some(text) = &chunk.text {
                    collected_text.push_str(text);
                }
            }
            Err(e) => panic!("Stream returned an error: {e:?}"),
        }
    }
    assert!(chunk_count > 0, "Stream did not yield any chunks.");
    assert!(!collected_text.is_empty(), "Collected text is empty.");
    println!(
        "test_generate_content_stream_with_system_instruction collected text ({chunk_count} chunks): {collected_text}"
    );
    assert!(
        collected_text.to_lowercase().contains("chlorophyll"),
        "Collected text does not contain expected keyword 'chlorophyll'"
    );
}

// Streaming test without system instruction
#[tokio::test]
#[ignore = "Makes real API calls - may hit rate limits"]
async fn test_generate_content_stream_without_system_instruction() {
    let Ok(api_key) = env::var("GEMINI_API_KEY") else {
        println!(
            "Skipping test_generate_content_stream_without_system_instruction: GEMINI_API_KEY not set."
        );
        return;
    };
    let client = Client::builder(api_key).build();
    let model_name = "gemini-3-flash-preview";
    let prompt = "Tell a short joke.";

    let stream_result = client
        .with_model(model_name)
        .with_prompt(prompt)
        .generate_stream();
    let stream = match stream_result {
        Ok(s) => s,
        Err(e) => panic!("Failed to create stream: {e:?}"),
    };
    pin_mut!(stream);
    let mut collected_text = String::new();
    let mut chunk_count = 0;
    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                chunk_count += 1;
                if let Some(text) = &chunk.text {
                    collected_text.push_str(text);
                }
            }
            Err(e) => panic!("Stream returned an error: {e:?}"),
        }
    }
    assert!(chunk_count > 0, "Stream did not yield any chunks.");
    assert!(!collected_text.is_empty(), "Collected text is empty.");
    println!(
        "test_generate_content_stream_without_system_instruction collected text ({chunk_count} chunks): {collected_text}"
    );
}
