// InteractionBuilder edge case tests
//
// These tests verify edge cases and error conditions for the InteractionBuilder.
// Complementary to the unit tests in src/request_builder.rs

use genai_client::{InteractionContent, InteractionInput};
use rust_genai::Client;

#[test]
fn test_interaction_builder_with_complex_content_input() {
    // Test using InteractionInput::Content variant with multiple content items
    let client = Client::new("test-api-key".to_string(), None);

    let complex_input = InteractionInput::Content(vec![
        InteractionContent::Text {
            text: Some("First message".to_string()),
        },
        InteractionContent::Text {
            text: Some("Second message".to_string()),
        },
        InteractionContent::Thought {
            text: Some("Internal reasoning".to_string()),
        },
    ]);

    let _builder = client
        .interaction()
        .with_model("gemini-pro")
        .with_input(complex_input);

    // Verify the builder can be created with complex input
    // The actual request would be built when .create() or .create_stream() is called
    // This test verifies the API accepts complex content structures
    // If this compiles and runs, the test passes
}

#[test]
fn test_interaction_builder_with_both_model_and_agent_set() {
    // Test that setting both model AND agent is allowed (builder doesn't validate this)
    // The validation happens at request creation time
    let client = Client::new("test-api-key".to_string(), None);

    let _builder = client
        .interaction()
        .with_model("gemini-pro")
        .with_agent("my-agent")
        .with_text("Hello");

    // The builder allows setting both, but the API request will likely fail
    // This tests that the builder API is permissive
    // If this compiles and runs, the test passes
}

#[test]
fn test_interaction_builder_with_very_long_text() {
    // Test with very long input text (10KB)
    let client = Client::new("test-api-key".to_string(), None);
    let long_text = "Lorem ipsum ".repeat(1000); // ~12KB

    let _builder = client
        .interaction()
        .with_model("gemini-pro")
        .with_text(&long_text);

    // Verify builder accepts large text inputs
    // If this compiles and runs, the test passes
}

#[test]
fn test_interaction_builder_with_unicode_and_emojis() {
    // Test with unicode, emojis, and special characters
    let client = Client::new("test-api-key".to_string(), None);

    let unicode_text = "Hello 世界 🌍 مرحبا Здравствуй \u{1F600} \u{1F44D}";

    let _builder = client
        .interaction()
        .with_model("gemini-pro")
        .with_text(unicode_text);

    // If this compiles and runs, the test passes
}

#[test]
fn test_interaction_builder_with_empty_text() {
    // Test with empty string input
    let client = Client::new("test-api-key".to_string(), None);

    let _builder = client.interaction().with_model("gemini-pro").with_text("");

    // Empty text is allowed by the builder
    // If this compiles and runs, the test passes
}

#[test]
fn test_interaction_builder_with_multiple_functions() {
    // Test adding many functions
    use rust_genai::FunctionDeclaration;
    use serde_json::json;

    let client = Client::new("test-api-key".to_string(), None);

    let mut builder = client
        .interaction()
        .with_model("gemini-pro")
        .with_text("Test");

    // Add 10 functions
    for i in 0..10 {
        let func = FunctionDeclaration {
            name: format!("function_{}", i),
            description: format!("Function number {}", i),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "param": {"type": "string"}
                }
            })),
            required: vec!["param".to_string()],
        };
        builder = builder.with_function(func);
    }

    // If this compiles and runs, the test passes
}

#[test]
fn test_interaction_builder_with_complex_generation_config() {
    // Test with generation config at boundary values
    let client = Client::new("test-api-key".to_string(), None);

    let config = genai_client::GenerationConfig {
        temperature: Some(2.0),        // Max value
        max_output_tokens: Some(8192), // High value
        top_p: Some(1.0),              // Max value
        top_k: Some(40),
        thinking_level: None,
    };

    let _builder = client
        .interaction()
        .with_model("gemini-pro")
        .with_text("Test")
        .with_generation_config(config);

    // If this compiles and runs, the test passes
}

#[test]
fn test_interaction_builder_with_response_format_json_schema() {
    // Test with complex JSON schema for structured output
    use serde_json::json;

    let client = Client::new("test-api-key".to_string(), None);

    let complex_schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "number"},
            "address": {
                "type": "object",
                "properties": {
                    "street": {"type": "string"},
                    "city": {"type": "string"},
                    "zipcode": {"type": "string"}
                }
            },
            "hobbies": {
                "type": "array",
                "items": {"type": "string"}
            }
        },
        "required": ["name", "age"]
    });

    let _builder = client
        .interaction()
        .with_model("gemini-pro")
        .with_text("Generate a person")
        .with_response_format(complex_schema);

    // If this compiles and runs, the test passes
}

#[tokio::test]
#[ignore = "Requires API key and makes real HTTP request"]
async fn test_interaction_builder_stream_with_large_response() {
    // Test streaming with a request that generates large output
    // This tests stream error propagation and handling

    let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_else(|_| "test-key".to_string());
    let client = Client::new(api_key, None);

    // Request a very long response
    let mut stream = client
        .interaction()
        .with_model("gemini-2.0-flash-exp")
        .with_text("Write a 2000 word essay about artificial intelligence")
        .create_stream();

    use futures_util::StreamExt;

    let mut chunk_count = 0;
    while let Some(result) = stream.next().await {
        match result {
            Ok(_response) => {
                chunk_count += 1;
            }
            Err(e) => {
                // Stream error occurred
                eprintln!("Stream error: {}", e);
                break;
            }
        }
    }

    // Verify we received multiple chunks
    assert!(chunk_count > 0, "Should receive at least one chunk");
}

#[test]
fn test_interaction_builder_with_all_features_combined() {
    // Test combining many features simultaneously
    use rust_genai::FunctionDeclaration;
    use serde_json::json;

    let client = Client::new("test-api-key".to_string(), None);

    let func = FunctionDeclaration {
        name: "get_weather".to_string(),
        description: "Get weather".to_string(),
        parameters: Some(json!({"type": "object"})),
        required: vec![],
    };

    let config = genai_client::GenerationConfig {
        temperature: Some(0.7),
        max_output_tokens: Some(1024),
        top_p: Some(0.95),
        top_k: Some(40),
        thinking_level: Some("1".to_string()),
    };

    let _builder = client
        .interaction()
        .with_model("gemini-pro")
        .with_text("Complex query")
        .with_system_instruction("Be helpful")
        .with_function(func)
        .with_generation_config(config)
        .with_response_modalities(vec!["TEXT".to_string()])
        .with_background(true)
        .with_store(false);

    // If this compiles and runs, the test passes
}

#[test]
fn test_interaction_builder_method_chaining() {
    // Verify fluent API / method chaining works correctly
    let client = Client::new("test-api-key".to_string(), None);

    let _builder = client
        .interaction()
        .with_model("gemini-pro")
        .with_text("Test 1")
        .with_text("Test 2") // Overwrites previous text
        .with_system_instruction("Instruction 1")
        .with_system_instruction("Instruction 2") // Overwrites
        .with_background(false)
        .with_background(true); // Overwrites

    // All methods should be chainable
    // If this compiles and runs, the test passes
}
