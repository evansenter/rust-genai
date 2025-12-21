// Tests for debug mode functionality
use rust_genai::{ApiVersion, Client};
use std::env;

#[tokio::test]
#[ignore = "Makes real API calls - may hit rate limits"]
async fn test_debug_mode_output() {
    let Ok(api_key) = env::var("GEMINI_API_KEY") else {
        println!("Skipping test_debug_mode_output: GEMINI_API_KEY not set.");
        return;
    };

    // Create client with debug mode enabled and V1Alpha API version
    let client = Client::builder(api_key)
        .api_version(ApiVersion::V1Alpha)
        .debug()
        .build();

    // Make a simple request
    let result = client
        .with_model("gemini-3-flash-preview")
        .with_prompt("Say hello")
        .generate()
        .await;

    // The request should succeed
    assert!(result.is_ok(), "Request failed: {:?}", result.err());

    // Note: We can't easily capture stdout in tests, but we can verify
    // that debug mode doesn't break functionality
}

#[tokio::test]
#[ignore = "Makes real API calls - may hit rate limits"]
async fn test_debug_mode_streaming() {
    use futures_util::StreamExt;

    let Ok(api_key) = env::var("GEMINI_API_KEY") else {
        println!("Skipping test_debug_mode_streaming: GEMINI_API_KEY not set.");
        return;
    };

    // Create client with debug mode enabled and V1Alpha API version
    let client = Client::builder(api_key)
        .api_version(ApiVersion::V1Alpha)
        .debug()
        .build();

    // Make a streaming request
    let stream_result = client
        .with_model("gemini-3-flash-preview")
        .with_prompt("Count to 3")
        .generate_stream();

    assert!(
        stream_result.is_ok(),
        "Failed to create stream: {:?}",
        stream_result.err()
    );
    let mut stream = Box::pin(stream_result.unwrap());

    let mut chunk_count = 0;
    while let Some(result) = stream.next().await {
        assert!(result.is_ok(), "Stream chunk failed: {:?}", result.err());
        chunk_count += 1;
    }

    assert!(chunk_count > 0, "Expected at least one chunk");
}

#[test]
fn test_debug_mode_builder() {
    let api_key = "test-key".to_string();

    // Test that debug mode can be toggled
    let client_no_debug = Client::builder(api_key.clone()).build();
    let client_with_debug = Client::builder(api_key).debug().build();

    // Both clients should be created successfully
    // We can't check the internal debug flag, but we verify no panic
    let _builder1 = client_no_debug.with_model("test");
    let _builder2 = client_with_debug.with_model("test");
}
