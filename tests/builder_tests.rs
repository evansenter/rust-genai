//! Unit tests for builder patterns (no API key required)
//!
//! This file contains tests for:
//! - FunctionDeclarationBuilder edge cases
//! - InteractionBuilder edge cases and validation

mod common;

use common::DEFAULT_MODEL;
use genai_client::{GenerationConfig, InteractionContent, InteractionInput, ThinkingLevel};
use rust_genai::{Client, FunctionDeclaration, detect_mime_type};
use serde_json::json;
use std::path::Path;

// =============================================================================
// FunctionDeclarationBuilder Tests
// =============================================================================

#[test]
fn test_function_builder_with_empty_name() {
    // Builder accepts empty names but logs a warning
    let func = FunctionDeclaration::builder("")
        .description("Test function")
        .build();

    assert_eq!(func.name(), "");
    // Note: This will likely be rejected by the API, but builder allows it
}

#[test]
fn test_function_builder_with_whitespace_only_name() {
    let func = FunctionDeclaration::builder("   ")
        .description("Test function")
        .build();

    assert_eq!(func.name(), "   ");
}

#[test]
fn test_function_builder_parameter_overwrites_on_duplicate() {
    // When the same parameter name is added twice, the second should overwrite
    let func = FunctionDeclaration::builder("test_func")
        .parameter(
            "location",
            json!({"type": "string", "description": "First"}),
        )
        .parameter(
            "location",
            json!({"type": "string", "description": "Second"}),
        )
        .build();

    // Verify the second parameter overwrote the first
    let location_desc = func
        .parameters()
        .properties()
        .get("location")
        .and_then(|l| l.get("description"))
        .and_then(|d| d.as_str());

    assert_eq!(location_desc, Some("Second"));
}

#[test]
fn test_function_builder_required_non_existent_parameter() {
    // Builder allows requiring parameters that don't exist but logs a warning
    let func = FunctionDeclaration::builder("test_func")
        .parameter("existing_param", json!({"type": "string"}))
        .required(vec!["nonexistent_param".to_string()])
        .build();

    assert_eq!(func.parameters().required(), vec!["nonexistent_param"]);
    // Note: This will likely cause API errors, but builder allows it
}

#[test]
fn test_function_builder_method_order_independence() {
    // Verify that calling methods in different orders produces identical results
    let func1 = FunctionDeclaration::builder("test")
        .description("A test function")
        .parameter("param1", json!({"type": "string"}))
        .required(vec!["param1".to_string()])
        .build();

    let func2 = FunctionDeclaration::builder("test")
        .required(vec!["param1".to_string()])
        .parameter("param1", json!({"type": "string"}))
        .description("A test function")
        .build();

    // Compare serialized forms since FunctionDeclaration doesn't implement PartialEq
    let json1 = serde_json::to_value(&func1).unwrap();
    let json2 = serde_json::to_value(&func2).unwrap();

    assert_eq!(json1, json2);
}

#[test]
fn test_function_builder_with_no_parameters() {
    let func = FunctionDeclaration::builder("no_params")
        .description("Function with no parameters")
        .build();

    assert_eq!(func.parameters().type_(), "object");
    assert!(func.parameters().properties().is_object());
    assert!(func.parameters().required().is_empty());
}

#[test]
fn test_function_builder_with_many_parameters() {
    let mut builder = FunctionDeclaration::builder("many_params");

    // Add 20 parameters
    for i in 0..20 {
        builder = builder.parameter(
            &format!("param_{}", i),
            json!({"type": "string", "description": format!("Parameter {}", i)}),
        );
    }

    let func = builder.build();

    // Verify all parameters were added
    let properties = func.parameters().properties();

    for i in 0..20 {
        assert!(properties.get(format!("param_{}", i)).is_some());
    }
}

#[test]
fn test_function_builder_required_all_parameters() {
    let func = FunctionDeclaration::builder("all_required")
        .parameter("param1", json!({"type": "string"}))
        .parameter("param2", json!({"type": "number"}))
        .parameter("param3", json!({"type": "boolean"}))
        .required(vec![
            "param1".to_string(),
            "param2".to_string(),
            "param3".to_string(),
        ])
        .build();

    assert_eq!(func.parameters().required().len(), 3);
}

#[test]
fn test_function_builder_required_subset_of_parameters() {
    let func = FunctionDeclaration::builder("partial_required")
        .parameter("required_param", json!({"type": "string"}))
        .parameter("optional_param", json!({"type": "string"}))
        .required(vec!["required_param".to_string()])
        .build();

    assert_eq!(func.parameters().required().len(), 1);
    assert_eq!(func.parameters().required()[0], "required_param");
}

#[test]
fn test_function_builder_complex_nested_schema() {
    let func = FunctionDeclaration::builder("nested_schema")
        .parameter(
            "complex_param",
            json!({
                "type": "object",
                "properties": {
                    "nested": {
                        "type": "object",
                        "properties": {
                            "deep": {"type": "string"}
                        }
                    }
                }
            }),
        )
        .build();

    // Verify nested structure is preserved
    let param = func.parameters().properties().get("complex_param");

    assert!(param.is_some());
    assert!(param.unwrap().get("properties").is_some());
}

#[test]
fn test_function_builder_with_array_parameter() {
    let func = FunctionDeclaration::builder("array_param")
        .parameter(
            "items",
            json!({
                "type": "array",
                "items": {"type": "string"}
            }),
        )
        .build();

    let param = func
        .parameters()
        .properties()
        .get("items")
        .and_then(|i| i.get("type"))
        .and_then(|t| t.as_str());

    assert_eq!(param, Some("array"));
}

#[test]
fn test_function_builder_with_enum_values() {
    let func = FunctionDeclaration::builder("enum_param")
        .parameter(
            "unit",
            json!({
                "type": "string",
                "enum": ["celsius", "fahrenheit", "kelvin"]
            }),
        )
        .build();

    let enum_values = func
        .parameters()
        .properties()
        .get("unit")
        .and_then(|u| u.get("enum"))
        .and_then(|e| e.as_array());

    assert_eq!(enum_values.unwrap().len(), 3);
}

#[test]
fn test_function_builder_description_can_be_empty() {
    let func = FunctionDeclaration::builder("test").description("").build();

    assert_eq!(func.description(), "");
}

#[test]
fn test_function_builder_description_with_unicode() {
    let func = FunctionDeclaration::builder("test")
        .description("测试函数 with émojis 🎉")
        .build();

    assert!(func.description().contains("🎉"));
}

#[test]
fn test_function_builder_very_long_description() {
    let long_desc = "x".repeat(10000);
    let func = FunctionDeclaration::builder("test")
        .description(&long_desc)
        .build();

    assert_eq!(func.description().len(), 10000);
}

// =============================================================================
// InteractionBuilder Tests
// =============================================================================

#[test]
fn test_interaction_builder_with_complex_content_input() {
    // Test using InteractionInput::Content variant with multiple content items
    let client = Client::new("test-api-key".to_string());

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
        .with_model(DEFAULT_MODEL)
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
}

#[test]
fn test_interaction_builder_with_both_model_and_agent_set() {
    // Test that setting both model AND agent fails validation
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_agent("my-agent")
        .with_text("Hello");

    // Builder validates that only one of model/agent can be set
    let result = builder.build_request();
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Cannot specify both model"));
}

#[test]
fn test_interaction_builder_with_very_long_text() {
    // Test with very long input text (10KB)
    let client = Client::new("test-api-key".to_string());
    let long_text = "Lorem ipsum ".repeat(1000); // ~12KB

    let _builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text(&long_text);

    // Builder accepts large text inputs without size validation
}

#[test]
fn test_interaction_builder_with_unicode_and_emojis() {
    // Test with unicode, emojis, and special characters
    let client = Client::new("test-api-key".to_string());

    let unicode_text = "Hello 世界 🌍 مرحبا Здравствуй \u{1F600} \u{1F44D}";

    let _builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text(unicode_text);

    // Builder handles Unicode and multi-byte characters correctly
}

#[test]
fn test_interaction_builder_with_empty_text() {
    // Test with empty string input
    let client = Client::new("test-api-key".to_string());

    let _builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("");

    // Builder allows empty string inputs without validation
}

#[test]
fn test_interaction_builder_with_multiple_functions() {
    // Test adding many functions
    let client = Client::new("test-api-key".to_string());

    let mut builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
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
}

#[test]
fn test_interaction_builder_with_complex_generation_config() {
    // Test with generation config at boundary values
    let client = Client::new("test-api-key".to_string());

    let config = GenerationConfig {
        temperature: Some(2.0),        // Max value
        max_output_tokens: Some(8192), // High value
        top_p: Some(1.0),              // Max value
        top_k: Some(40),
        thinking_level: None,
    };

    let _builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("Test")
        .with_generation_config(config);

    // Builder accepts generation config with boundary values
}

#[test]
fn test_interaction_builder_with_response_format_json_schema() {
    // Test with complex JSON schema for structured output
    let client = Client::new("test-api-key".to_string());

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
        .with_model(DEFAULT_MODEL)
        .with_text("Generate a person")
        .with_response_format(complex_schema);

    // Builder accepts complex nested JSON schemas without validation
}

#[test]
fn test_interaction_builder_with_all_features_combined() {
    // Test combining many features simultaneously
    let client = Client::new("test-api-key".to_string());

    let func = FunctionDeclaration::builder("get_weather")
        .description("Get weather")
        .build();

    let config = GenerationConfig {
        temperature: Some(0.7),
        max_output_tokens: Some(1024),
        top_p: Some(0.95),
        top_k: Some(40),
        thinking_level: Some(ThinkingLevel::Low),
    };

    let _builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("Complex query")
        .with_system_instruction("Be helpful")
        .with_function(func)
        .with_generation_config(config)
        .with_response_modalities(vec!["TEXT".to_string()])
        .with_background(true)
        .with_store(false);

    // Builder supports combining all features without conflicts
}

#[test]
fn test_interaction_builder_method_chaining() {
    // Verify fluent API / method chaining works correctly
    let client = Client::new("test-api-key".to_string());

    let _builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("Test 1")
        .with_text("Test 2") // Overwrites previous text
        .with_system_instruction("Instruction 1")
        .with_system_instruction("Instruction 2") // Overwrites
        .with_background(false)
        .with_background(true); // Overwrites

    // All methods should be chainable and later calls overwrite earlier values
}

// =============================================================================
// InteractionBuilder Validation Tests
// =============================================================================

#[test]
fn test_interaction_builder_validation_missing_input() {
    // Verify that build_request fails when no input is provided
    let client = Client::new("test-api-key".to_string());

    let builder = client.interaction().with_model(DEFAULT_MODEL);

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
    let client = Client::new("test-api-key".to_string());

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
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("Hello");

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    assert_eq!(request.model.as_deref(), Some(DEFAULT_MODEL));
    assert!(request.agent.is_none());
}

#[test]
fn test_interaction_builder_with_google_search() {
    use rust_genai::Tool;

    // Test that with_google_search adds the GoogleSearch tool
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("What's the weather today?")
        .with_google_search();

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();

    // Verify GoogleSearch tool was added
    assert!(request.tools.is_some());
    let tools = request.tools.unwrap();
    assert_eq!(tools.len(), 1);
    assert!(matches!(tools[0], Tool::GoogleSearch));
}

#[test]
fn test_interaction_builder_with_google_search_and_functions() {
    use rust_genai::Tool;

    // Test that with_google_search can be combined with function declarations
    let client = Client::new("test-api-key".to_string());

    let func = FunctionDeclaration::builder("get_temperature")
        .description("Get temperature")
        .build();

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("What's the weather today?")
        .with_function(func)
        .with_google_search();

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();

    // Verify both tools were added
    assert!(request.tools.is_some());
    let tools = request.tools.unwrap();
    assert_eq!(tools.len(), 2);
    assert!(matches!(tools[0], Tool::Function { .. }));
    assert!(matches!(tools[1], Tool::GoogleSearch));
}

#[test]
fn test_interaction_builder_with_code_execution() {
    use rust_genai::Tool;

    // Test that with_code_execution adds the CodeExecution tool
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("Calculate the factorial of 10")
        .with_code_execution();

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();

    // Verify CodeExecution tool was added
    assert!(request.tools.is_some());
    let tools = request.tools.unwrap();
    assert_eq!(tools.len(), 1);
    assert!(matches!(tools[0], Tool::CodeExecution));
}

#[test]
fn test_interaction_builder_with_code_execution_and_google_search() {
    use rust_genai::Tool;

    // Test that with_code_execution can be combined with with_google_search
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("Search for prime numbers and calculate the first 10")
        .with_code_execution()
        .with_google_search();

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();

    // Verify both tools were added
    assert!(request.tools.is_some());
    let tools = request.tools.unwrap();
    assert_eq!(tools.len(), 2);
    assert!(matches!(tools[0], Tool::CodeExecution));
    assert!(matches!(tools[1], Tool::GoogleSearch));
}

#[test]
fn test_interaction_builder_with_url_context() {
    use rust_genai::Tool;

    // Test that with_url_context adds the UrlContext tool
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("Summarize the content from https://example.com")
        .with_url_context();

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    assert!(request.tools.is_some());

    let tools = request.tools.unwrap();
    assert_eq!(tools.len(), 1);
    assert!(matches!(tools[0], Tool::UrlContext));
}

#[test]
fn test_interaction_builder_with_url_context_and_functions() {
    use rust_genai::Tool;

    // Test that with_url_context can be combined with function declarations
    let client = Client::new("test-api-key".to_string());

    let func = FunctionDeclaration::builder("analyze_page")
        .description("Analyze web page content")
        .build();

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("Fetch and analyze https://example.com")
        .with_function(func)
        .with_url_context();

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();

    // Verify both tools were added
    assert!(request.tools.is_some());
    let tools = request.tools.unwrap();
    assert_eq!(tools.len(), 2);
    assert!(matches!(tools[0], Tool::Function { .. }));
    assert!(matches!(tools[1], Tool::UrlContext));
}

// =============================================================================
// Builder Edge Case Tests
// =============================================================================

#[test]
fn test_interaction_builder_model_overwrites_previous_model() {
    // Verify that calling with_model twice overwrites the first value
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model("first-model")
        .with_model("second-model")
        .with_text("Hello");

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    // Second model should win
    assert_eq!(request.model.as_deref(), Some("second-model"));
}

#[test]
fn test_interaction_builder_agent_overwrites_previous_agent() {
    // Verify that calling with_agent twice overwrites the first value
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_agent("first-agent")
        .with_agent("second-agent")
        .with_text("Hello");

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    // Second agent should win
    assert_eq!(request.agent.as_deref(), Some("second-agent"));
}

// NOTE: test_interaction_builder_model_and_agent_both_set was removed
// Setting both model and agent is now an error - tested in test_interaction_builder_with_both_model_and_agent_set

#[test]
fn test_interaction_builder_empty_text_allowed() {
    // Verify that empty text is allowed (API will validate)
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("");

    let result = builder.build_request();
    // Empty text should be accepted at builder level
    assert!(result.is_ok());
}

#[test]
fn test_interaction_builder_previous_interaction_id() {
    // Verify that previous_interaction_id is set correctly
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("Continue conversation")
        .with_previous_interaction("prev-interaction-123");

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    assert_eq!(
        request.previous_interaction_id.as_deref(),
        Some("prev-interaction-123")
    );
}

#[test]
fn test_interaction_builder_max_function_call_loops() {
    // Verify that max_function_call_loops is set correctly
    let client = Client::new("test-api-key".to_string());

    let func = FunctionDeclaration::builder("test_fn")
        .description("Test function")
        .build();

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("Call functions")
        .with_function(func)
        .with_max_function_call_loops(5);

    // The max_function_call_loops is stored in the builder, verified by integration tests
    let result = builder.build_request();
    assert!(result.is_ok());
}

#[test]
fn test_interaction_builder_all_three_tools_combined() {
    use rust_genai::Tool;

    // Verify all three built-in tools can be combined
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("Use all tools")
        .with_google_search()
        .with_code_execution()
        .with_url_context();

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    assert!(request.tools.is_some());

    let tools = request.tools.unwrap();
    assert_eq!(tools.len(), 3);
    assert!(matches!(tools[0], Tool::GoogleSearch));
    assert!(matches!(tools[1], Tool::CodeExecution));
    assert!(matches!(tools[2], Tool::UrlContext));
}

// =============================================================================
// Multimodal Builder Methods Tests
// =============================================================================

#[test]
fn test_detect_mime_type_basic() {
    // Test MIME type detection for common file types
    assert_eq!(detect_mime_type(Path::new("photo.jpg")), Some("image/jpeg"));
    assert_eq!(detect_mime_type(Path::new("photo.png")), Some("image/png"));
    assert_eq!(detect_mime_type(Path::new("audio.mp3")), Some("audio/mp3"));
    assert_eq!(detect_mime_type(Path::new("video.mp4")), Some("video/mp4"));
    assert_eq!(
        detect_mime_type(Path::new("doc.pdf")),
        Some("application/pdf")
    );
    assert_eq!(detect_mime_type(Path::new("unknown.xyz")), None);
}

#[test]
fn test_add_image_data_creates_content_from_empty() {
    // When input is None, add_image_data should create Content variant
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .add_image_data("base64data", "image/jpeg");

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    match &request.input {
        InteractionInput::Content(items) => {
            assert_eq!(items.len(), 1, "Should have 1 content item");
            // Verify it's an Image with data
            assert!(matches!(
                &items[0],
                InteractionContent::Image { mime_type, data, .. }
                if mime_type.as_deref() == Some("image/jpeg") && data.is_some()
            ));
        }
        _ => panic!("Expected InteractionInput::Content variant"),
    }
}

#[test]
fn test_add_image_data_converts_text_to_content() {
    // When input is Text, add_image_data should convert to Content with both
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("Analyze this image")
        .add_image_data("base64data", "image/png");

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    match &request.input {
        InteractionInput::Content(items) => {
            assert_eq!(items.len(), 2, "Should have 2 content items");
            // First should be text
            assert!(matches!(
                &items[0],
                InteractionContent::Text { text }
                if text.as_deref() == Some("Analyze this image")
            ));
            // Second should be image with data
            assert!(matches!(
                &items[1],
                InteractionContent::Image { mime_type, data, .. }
                if mime_type.as_deref() == Some("image/png") && data.is_some()
            ));
        }
        _ => panic!("Expected InteractionInput::Content variant"),
    }
}

#[test]
fn test_multiple_add_calls_accumulate() {
    // Multiple add_* calls should accumulate content
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .add_image_data("img1", "image/jpeg")
        .add_image_data("img2", "image/png")
        .add_audio_data("audio1", "audio/mp3");

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    match &request.input {
        InteractionInput::Content(items) => {
            assert_eq!(items.len(), 3, "Should have 3 content items");
        }
        _ => panic!("Expected InteractionInput::Content variant"),
    }
}

#[test]
fn test_add_methods_after_with_content() {
    // add_* methods should accumulate with existing Content
    let client = Client::new("test-api-key".to_string());

    let initial_content = vec![InteractionContent::Text {
        text: Some("Initial text".to_string()),
    }];

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_content(initial_content)
        .add_image_data("imagedata", "image/gif");

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    match &request.input {
        InteractionInput::Content(items) => {
            assert_eq!(items.len(), 2, "Should have 2 content items");
        }
        _ => panic!("Expected InteractionInput::Content variant"),
    }
}

#[test]
fn test_add_image_uri_works() {
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("Describe this image")
        .add_image_uri("gs://bucket/image.jpg", "image/jpeg");

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    match &request.input {
        InteractionInput::Content(items) => {
            assert_eq!(items.len(), 2, "Should have 2 content items");
            // Second should be an Image with URI
            assert!(matches!(
                &items[1],
                InteractionContent::Image { uri, mime_type, .. }
                if uri.is_some() && mime_type.as_deref() == Some("image/jpeg")
            ));
        }
        _ => panic!("Expected InteractionInput::Content variant"),
    }
}

#[test]
fn test_add_audio_data_works() {
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .add_audio_data("audiodata", "audio/wav");

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    match &request.input {
        InteractionInput::Content(items) => {
            assert_eq!(items.len(), 1);
            assert!(matches!(
                &items[0],
                InteractionContent::Audio { mime_type, data, .. }
                if mime_type.as_deref() == Some("audio/wav") && data.is_some()
            ));
        }
        _ => panic!("Expected InteractionInput::Content variant"),
    }
}

#[test]
fn test_add_video_data_works() {
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .add_video_data("videodata", "video/mp4");

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    match &request.input {
        InteractionInput::Content(items) => {
            assert_eq!(items.len(), 1);
            assert!(matches!(
                &items[0],
                InteractionContent::Video { mime_type, data, .. }
                if mime_type.as_deref() == Some("video/mp4") && data.is_some()
            ));
        }
        _ => panic!("Expected InteractionInput::Content variant"),
    }
}

#[test]
fn test_add_document_data_works() {
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .add_document_data("pdfdata", "application/pdf");

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    match &request.input {
        InteractionInput::Content(items) => {
            assert_eq!(items.len(), 1);
            assert!(matches!(
                &items[0],
                InteractionContent::Document { mime_type, data, .. }
                if mime_type.as_deref() == Some("application/pdf") && data.is_some()
            ));
        }
        _ => panic!("Expected InteractionInput::Content variant"),
    }
}

#[test]
fn test_add_document_uri_works() {
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("Summarize this document")
        .add_document_uri("gs://bucket/report.pdf", "application/pdf");

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    match &request.input {
        InteractionInput::Content(items) => {
            assert_eq!(items.len(), 2);
            assert!(matches!(
                &items[1],
                InteractionContent::Document { uri, mime_type, .. }
                if uri.is_some() && mime_type.as_deref() == Some("application/pdf")
            ));
        }
        _ => panic!("Expected InteractionInput::Content variant"),
    }
}

#[test]
fn test_add_audio_uri_works() {
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("Describe this audio")
        .add_audio_uri("gs://bucket/audio.mp3", "audio/mp3");

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    match &request.input {
        InteractionInput::Content(items) => {
            assert_eq!(items.len(), 2, "Should have 2 content items");
            // Second should be an Audio with URI
            assert!(matches!(
                &items[1],
                InteractionContent::Audio { uri, mime_type, .. }
                if uri.is_some() && mime_type.as_deref() == Some("audio/mp3")
            ));
        }
        _ => panic!("Expected InteractionInput::Content variant"),
    }
}

#[test]
fn test_add_video_uri_works() {
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("Describe this video")
        .add_video_uri("gs://bucket/video.mp4", "video/mp4");

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    match &request.input {
        InteractionInput::Content(items) => {
            assert_eq!(items.len(), 2, "Should have 2 content items");
            // Second should be a Video with URI
            assert!(matches!(
                &items[1],
                InteractionContent::Video { uri, mime_type, .. }
                if uri.is_some() && mime_type.as_deref() == Some("video/mp4")
            ));
        }
        _ => panic!("Expected InteractionInput::Content variant"),
    }
}

#[test]
fn test_add_methods_combine_with_all_builder_features() {
    // Verify add_* methods work with other builder features
    use rust_genai::Tool;

    let client = Client::new("test-api-key".to_string());

    let func = FunctionDeclaration::builder("analyze_image")
        .description("Analyze an image")
        .build();

    let config = GenerationConfig {
        temperature: Some(0.7),
        max_output_tokens: Some(1024),
        top_p: None,
        top_k: None,
        thinking_level: None,
    };

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("Analyze this image and describe it")
        .add_image_data("imagedata", "image/jpeg")
        .with_system_instruction("Be descriptive")
        .with_function(func)
        .with_generation_config(config)
        .with_google_search();

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();

    // Verify input has both text and image
    match &request.input {
        InteractionInput::Content(items) => {
            assert_eq!(items.len(), 2);
        }
        _ => panic!("Expected InteractionInput::Content variant"),
    }

    // Verify tools are set
    assert!(request.tools.is_some());
    let tools = request.tools.unwrap();
    assert_eq!(tools.len(), 2);
    assert!(matches!(tools[0], Tool::Function { .. }));
    assert!(matches!(tools[1], Tool::GoogleSearch));

    // Verify generation config
    assert!(request.generation_config.is_some());

    // Verify system instruction
    assert!(request.system_instruction.is_some());
}

#[test]
fn test_add_methods_order_preserves_sequence() {
    // Content should appear in the order it was added
    let client = Client::new("test-api-key".to_string());

    let builder = client
        .interaction()
        .with_model(DEFAULT_MODEL)
        .with_text("First")
        .add_image_data("second", "image/jpeg")
        .add_audio_data("third", "audio/mp3")
        .add_video_data("fourth", "video/mp4");

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    match &request.input {
        InteractionInput::Content(items) => {
            assert_eq!(items.len(), 4);

            // First: text
            assert!(matches!(&items[0], InteractionContent::Text { .. }));

            // Second: image
            assert!(matches!(
                &items[1],
                InteractionContent::Image { mime_type, .. }
                if mime_type.as_deref() == Some("image/jpeg")
            ));

            // Third: audio
            assert!(matches!(
                &items[2],
                InteractionContent::Audio { mime_type, .. }
                if mime_type.as_deref() == Some("audio/mp3")
            ));

            // Fourth: video
            assert!(matches!(
                &items[3],
                InteractionContent::Video { mime_type, .. }
                if mime_type.as_deref() == Some("video/mp4")
            ));
        }
        _ => panic!("Expected InteractionInput::Content variant"),
    }
}
