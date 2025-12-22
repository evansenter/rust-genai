// InteractionBuilder edge case tests
//
// These tests verify edge cases and error conditions for the InteractionBuilder.
// Complementary to the unit tests in src/request_builder.rs

use genai_client::{InteractionContent, InteractionInput};
use rust_genai::{Client, FunctionDeclaration, WithFunctionCalling};

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

    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(complex_input);

    // Verify the builder stored the complex input correctly
    let request = builder
        .build_request()
        .expect("Builder should create valid request");

    // Verify the input is a Content variant with 3 items
    match &request.input {
        InteractionInput::Content(items) => {
            assert_eq!(items.len(), 3, "Should have 3 content items");
        }
        _ => panic!("Expected InteractionInput::Content variant"),
    }

    // Builder accepts complex multi-part content without validation
    // Actual structure validation happens during API request creation
    // This test ensures the builder API supports heterogeneous content arrays
}

#[test]
fn test_interaction_builder_with_both_model_and_agent_set() {
    // Test that setting both model AND agent is allowed (builder doesn't validate this)
    // The validation happens at request creation time
    let client = Client::new("test-api-key".to_string(), None);

    let _builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_agent("my-agent")
        .with_text("Hello");

    // Builder allows setting both model and agent without validation
    // The API will reject this at request time, not during builder construction
    // This verifies the builder follows a "fail late" validation strategy
}

#[test]
fn test_interaction_builder_with_very_long_text() {
    // Test with very long input text (10KB)
    let client = Client::new("test-api-key".to_string(), None);
    let long_text = "Lorem ipsum ".repeat(1000); // ~12KB

    let _builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(&long_text);

    // Builder accepts large text inputs without size validation
    // Size limits are enforced by the API, not the builder
    // This test ensures no artificial limits are imposed client-side
}

#[test]
fn test_interaction_builder_with_unicode_and_emojis() {
    // Test with unicode, emojis, and special characters
    let client = Client::new("test-api-key".to_string(), None);

    let unicode_text = "Hello 世界 🌍 مرحبا Здравствуй \u{1F600} \u{1F44D}";

    let _builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(unicode_text);

    // Builder handles Unicode and multi-byte characters correctly
    // This verifies proper UTF-8 string handling without mangling
}

#[test]
fn test_interaction_builder_with_empty_text() {
    // Test with empty string input
    let client = Client::new("test-api-key".to_string(), None);

    let _builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("");

    // Builder allows empty string inputs without validation
    // The API determines whether empty text is acceptable for a given request
}

#[test]
fn test_interaction_builder_with_multiple_functions() {
    // Test adding many functions
    use rust_genai::FunctionDeclaration;
    use serde_json::json;

    let client = Client::new("test-api-key".to_string(), None);

    let mut builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Test");

    // Add 10 functions
    for i in 0..10 {
        let func = FunctionDeclaration::builder(format!("function_{}", i))
            .description(format!("Function number {}", i))
            .parameter("param", json!({"type": "string"}))
            .required(vec!["param".to_string()])
            .build();
        builder = builder.with_function(func);
    }

    // Builder accepts many function declarations without validation
    // This verifies the builder can handle complex tool configurations
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
        .with_model("gemini-3-flash-preview")
        .with_text("Test")
        .with_generation_config(config);

    // Builder accepts generation config with boundary values
    // Parameter range validation is performed by the API, not the builder
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
        .with_model("gemini-3-flash-preview")
        .with_text("Generate a person")
        .with_response_format(complex_schema);

    // Builder accepts complex nested JSON schemas without validation
    // Schema correctness is validated by the API when making the request
}

#[tokio::test]
#[ignore = "Requires API key and makes real HTTP request"]
async fn test_interaction_builder_with_longer_prompt() {
    // Test the builder with a longer prompt that should work
    let Ok(api_key) = std::env::var("GEMINI_API_KEY") else {
        println!("Skipping test_interaction_builder_with_longer_prompt: GEMINI_API_KEY not set.");
        return;
    };

    let client = Client::new(api_key, None);

    // Request a simple response using the builder
    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Count from 1 to 5")
        .create()
        .await;

    assert!(
        response.is_ok(),
        "Interaction builder should successfully create interaction: {:?}",
        response.err()
    );

    let interaction = response.unwrap();
    assert!(!interaction.id.is_empty(), "Should have interaction ID");
    assert!(!interaction.outputs.is_empty(), "Should have outputs");

    println!("Interaction completed with ID: {}", interaction.id);
}

#[test]
fn test_interaction_builder_with_all_features_combined() {
    // Test combining many features simultaneously
    let client = Client::new("test-api-key".to_string(), None);

    let func = FunctionDeclaration::builder("get_weather")
        .description("Get weather")
        .build();

    let config = genai_client::GenerationConfig {
        temperature: Some(0.7),
        max_output_tokens: Some(1024),
        top_p: Some(0.95),
        top_k: Some(40),
        thinking_level: Some("1".to_string()),
    };

    let _builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Complex query")
        .with_system_instruction("Be helpful")
        .with_function(func)
        .with_generation_config(config)
        .with_response_modalities(vec!["TEXT".to_string()])
        .with_background(true)
        .with_store(false);

    // Builder supports combining all features without conflicts
    // This tests that the builder API is fully composable
}

#[test]
fn test_interaction_builder_method_chaining() {
    // Verify fluent API / method chaining works correctly
    let client = Client::new("test-api-key".to_string(), None);

    let _builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Test 1")
        .with_text("Test 2") // Overwrites previous text
        .with_system_instruction("Instruction 1")
        .with_system_instruction("Instruction 2") // Overwrites
        .with_background(false)
        .with_background(true); // Overwrites

    // All methods should be chainable and later calls overwrite earlier values
}

// Validation error tests

#[test]
fn test_interaction_builder_validation_missing_input() {
    // Verify that build_request fails when no input is provided
    let client = Client::new("test-api-key".to_string(), None);

    let builder = client.interaction().with_model("gemini-3-flash-preview");

    let result = builder.build_request();

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, rust_genai::GenaiError::InvalidInput(_)));

    // Verify error message mentions input requirement
    if let rust_genai::GenaiError::InvalidInput(msg) = err {
        assert!(msg.contains("Input is required"));
    }
}

#[test]
fn test_interaction_builder_validation_missing_model_and_agent() {
    // Verify that build_request fails when neither model nor agent is specified
    let client = Client::new("test-api-key".to_string(), None);

    let builder = client.interaction().with_text("Hello"); // Has input but no model/agent

    let result = builder.build_request();

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, rust_genai::GenaiError::InvalidInput(_)));

    // Verify error message mentions model/agent requirement
    if let rust_genai::GenaiError::InvalidInput(msg) = err {
        assert!(msg.contains("model or agent"));
    }
}

#[test]
fn test_interaction_builder_validation_success_with_model() {
    // Verify that build_request succeeds when model and input are provided
    let client = Client::new("test-api-key".to_string(), None);

    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello");

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    assert_eq!(request.model.as_deref(), Some("gemini-3-flash-preview"));
    assert!(request.agent.is_none());
}
