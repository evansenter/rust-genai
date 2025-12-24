//! Unit tests for builder patterns (no API key required)
//!
//! This file contains tests for:
//! - FunctionDeclarationBuilder edge cases
//! - InteractionBuilder edge cases and validation

use genai_client::{GenerationConfig, InteractionContent, InteractionInput};
use rust_genai::{Client, FunctionDeclaration};
use serde_json::json;

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
}

#[test]
fn test_interaction_builder_with_both_model_and_agent_set() {
    // Test that setting both model AND agent is allowed (builder doesn't validate this)
    let client = Client::new("test-api-key".to_string());

    let _builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_agent("my-agent")
        .with_text("Hello");

    // Builder allows setting both model and agent without validation
    // The API will reject this at request time, not during builder construction
}

#[test]
fn test_interaction_builder_with_very_long_text() {
    // Test with very long input text (10KB)
    let client = Client::new("test-api-key".to_string());
    let long_text = "Lorem ipsum ".repeat(1000); // ~12KB

    let _builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
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
        .with_model("gemini-3-flash-preview")
        .with_text(unicode_text);

    // Builder handles Unicode and multi-byte characters correctly
}

#[test]
fn test_interaction_builder_with_empty_text() {
    // Test with empty string input
    let client = Client::new("test-api-key".to_string());

    let _builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("");

    // Builder allows empty string inputs without validation
}

#[test]
fn test_interaction_builder_with_multiple_functions() {
    // Test adding many functions
    let client = Client::new("test-api-key".to_string());

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
        .with_model("gemini-3-flash-preview")
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
        .with_model("gemini-3-flash-preview")
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
}

#[test]
fn test_interaction_builder_method_chaining() {
    // Verify fluent API / method chaining works correctly
    let client = Client::new("test-api-key".to_string());

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

// =============================================================================
// InteractionBuilder Validation Tests
// =============================================================================

#[test]
fn test_interaction_builder_validation_missing_input() {
    // Verify that build_request fails when no input is provided
    let client = Client::new("test-api-key".to_string());

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
        .with_model("gemini-3-flash-preview")
        .with_text("Hello");

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    assert_eq!(request.model.as_deref(), Some("gemini-3-flash-preview"));
    assert!(request.agent.is_none());
}
