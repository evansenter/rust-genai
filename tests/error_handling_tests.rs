// Tests for error handling scenarios
use rust_genai::{Client, GenaiError};
use std::env;

#[tokio::test]
async fn test_invalid_api_key() {
    let client = Client::builder("invalid-api-key".to_string()).build();

    let result = client
        .with_model("gemini-2.5-flash-preview-05-20")
        .with_prompt("Hello")
        .generate()
        .await;

    assert!(result.is_err(), "Expected error with invalid API key");
    match result.err().unwrap() {
        GenaiError::Http(_) | GenaiError::Api(_) => {
            // Expected error types
        }
        other => panic!("Unexpected error type: {other:?}"),
    }
}

#[tokio::test]
#[ignore = "Makes real API calls - may hit rate limits"]
async fn test_invalid_model_name() {
    let Ok(api_key) = env::var("GEMINI_API_KEY") else {
        println!("Skipping test_invalid_model_name: GEMINI_API_KEY not set.");
        return;
    };

    let client = Client::builder(api_key).build();

    let result = client
        .with_model("non-existent-model-12345")
        .with_prompt("Hello")
        .generate()
        .await;

    assert!(result.is_err(), "Expected error with invalid model name");
}

#[test]
fn test_genai_error_display() {
    // Test error display implementations
    let parse_error = GenaiError::Parse("test parse error".to_string());
    assert_eq!(
        parse_error.to_string(),
        "SSE parsing error: test parse error"
    );

    let api_error = GenaiError::Api("API error message".to_string());
    assert_eq!(
        api_error.to_string(),
        "API Error returned by Google: API error message"
    );

    let internal_error = GenaiError::Internal("internal error".to_string());
    assert_eq!(
        internal_error.to_string(),
        "Internal client error: internal error"
    );
}

#[test]
fn test_error_conversion_from_json() {
    // Test JSON error conversion
    let invalid_json = "{invalid json";
    let json_result: Result<serde_json::Value, _> = serde_json::from_str(invalid_json);
    assert!(json_result.is_err());

    let genai_error: GenaiError = json_result.unwrap_err().into();
    match genai_error {
        GenaiError::Json(_) => {
            // Expected
        }
        _ => panic!("Expected JSON error"),
    }
}

#[tokio::test]
#[ignore = "Makes real API calls - may hit rate limits"]
async fn test_empty_prompt_handling() {
    let Ok(api_key) = env::var("GEMINI_API_KEY") else {
        println!("Skipping test_empty_prompt_handling: GEMINI_API_KEY not set.");
        return;
    };

    let client = Client::builder(api_key).build();

    // Empty prompt should still work
    let result = client
        .with_model("gemini-pro")
        .with_prompt("")
        .generate()
        .await;

    // The API might accept empty prompts or return an error
    // We just verify it doesn't panic
    let _ = result;
}

#[tokio::test]
async fn test_streaming_error_handling() {
    use futures_util::StreamExt;

    let client = Client::builder("invalid-api-key".to_string()).build();

    let stream_result = client
        .with_model("gemini-2.5-flash-preview-05-20")
        .with_prompt("Hello")
        .generate_stream();

    // With an invalid API key, we might get an error immediately or in the stream
    let Ok(stream) = stream_result else {
        // Got an error immediately, which is expected with invalid API key
        return;
    };

    let mut stream = Box::pin(stream);

    // Should get an error in the stream
    let mut got_error = false;
    while let Some(result) = stream.next().await {
        if result.is_err() {
            got_error = true;
            break;
        }
    }

    assert!(got_error, "Expected error in stream with invalid API key");
}
