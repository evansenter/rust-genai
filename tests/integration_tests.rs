use futures_util::{StreamExt, pin_mut};
use rust_genai::Client;
use std::env;

// Non-streaming test
#[tokio::test]
async fn test_generate_content_integration() {
    let api_key = match env::var("GEMINI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("Skipping test_generate_content_integration: GEMINI_API_KEY not set.");
            return;
        }
    };
    // Create client
    let client = Client::new(api_key);

    let model = "gemini-1.5-flash-latest";
    let prompt = "What is the capital of France?";

    // Use the client method
    let result = client.generate_content(model, prompt).await;

    // Assert using the specific error type for better messages
    assert!(
        result.is_ok(),
        "generate_content failed: {:?}",
        result.err().unwrap()
    );
    let text = result.unwrap();
    assert!(!text.is_empty(), "Generated text is empty");
    println!("test_generate_content_integration response: {}", text);
    assert!(
        text.to_lowercase().contains("paris"),
        "Response does not contain expected keyword 'paris'"
    );
}

// Streaming test
#[tokio::test]
async fn test_generate_content_stream_integration() {
    let api_key = match env::var("GEMINI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("Skipping test_generate_content_stream_integration: GEMINI_API_KEY not set.");
            return;
        }
    };
    // Create client
    let client = Client::new(api_key);

    let model = "gemini-1.5-flash-latest";
    let prompt = "Why is the sky blue?";

    // Use the client stream method
    let stream = client.generate_content_stream(model, prompt);
    pin_mut!(stream);

    let mut collected_text = String::new();
    let mut chunk_count = 0;
    while let Some(result) = stream.next().await {
        match result {
            // We expect Ok, panic if Err, showing the GenaiError
            Ok(chunk) => {
                chunk_count += 1;
                if let Some(candidate) = chunk.candidates.get(0) {
                    if let Some(part) = candidate.content.parts.get(0) {
                        collected_text.push_str(&part.text);
                    }
                }
            }
            Err(e) => {
                panic!("Stream returned an error: {:?}", e); // Panic shows the specific GenaiError 
            }
        }
    }

    assert!(chunk_count > 0, "Stream did not yield any chunks.");
    assert!(!collected_text.is_empty(), "Collected text is empty.");
    println!(
        "test_generate_content_stream_integration collected text ({} chunks): {}",
        chunk_count, collected_text
    );
    assert!(
        collected_text.to_lowercase().contains("scatter"),
        "Collected text does not contain expected keyword 'scatter'"
    );
}
