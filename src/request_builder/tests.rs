//! Unit tests for InteractionBuilder.

use super::*;
use crate::{Client, FunctionDeclaration};
use genai_client::Tool;
use serde_json::json;

fn create_test_client() -> Client {
    Client::builder("test-api-key".to_string()).build()
}

#[test]
fn test_function_declaration_builder() {
    let func_decl = FunctionDeclaration::builder("my_func")
        .description("Does something")
        .parameter("arg1", json!({"type": "string"}))
        .required(vec!["arg1".to_string()])
        .build();

    assert_eq!(func_decl.name(), "my_func");
    assert_eq!(func_decl.description(), "Does something");
    assert_eq!(func_decl.parameters().type_(), "object");
    assert_eq!(
        func_decl
            .parameters()
            .properties()
            .get("arg1")
            .unwrap()
            .get("type")
            .unwrap()
            .as_str(),
        Some("string")
    );
    assert_eq!(func_decl.parameters().required(), vec!["arg1".to_string()]);
}

#[test]
fn test_function_declaration_into_tool() {
    let func_decl = FunctionDeclaration::builder("test")
        .description("Test function")
        .build();

    let tool = func_decl.into_tool();
    match tool {
        Tool::Function { name, .. } => {
            assert_eq!(name, "test");
        }
        _ => panic!("Expected Tool::Function variant"),
    }
}

// --- InteractionBuilder Tests ---

#[test]
fn test_interaction_builder_with_model() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello");

    assert_eq!(builder.model.as_deref(), Some("gemini-3-flash-preview"));
    assert!(builder.agent.is_none());
    assert!(matches!(
        builder.input,
        Some(genai_client::InteractionInput::Text(_))
    ));
}

#[test]
fn test_interaction_builder_with_agent() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_agent("deep-research-pro")
        .with_text("Research topic");

    assert!(builder.model.is_none());
    assert_eq!(builder.agent.as_deref(), Some("deep-research-pro"));
}

#[test]
fn test_interaction_builder_with_previous_interaction() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Follow-up question")
        .with_previous_interaction("interaction_123");

    assert_eq!(
        builder.previous_interaction_id.as_deref(),
        Some("interaction_123")
    );
}

#[test]
fn test_interaction_builder_with_system_instruction() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello")
        .with_system_instruction("You are a helpful assistant");

    assert!(matches!(
        builder.system_instruction,
        Some(genai_client::InteractionInput::Text(_))
    ));
}

#[test]
fn test_interaction_builder_with_generation_config() {
    let client = create_test_client();
    let config = genai_client::GenerationConfig {
        temperature: Some(0.7),
        max_output_tokens: Some(1000),
        top_p: Some(0.9),
        top_k: Some(40),
        thinking_level: Some(ThinkingLevel::Medium),
    };

    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello")
        .with_generation_config(config.clone());

    assert!(builder.generation_config.is_some());
    assert_eq!(
        builder.generation_config.as_ref().unwrap().temperature,
        Some(0.7)
    );
}

#[test]
fn test_interaction_builder_with_function() {
    let client = create_test_client();
    let func = FunctionDeclaration::builder("test_func")
        .description("Test function")
        .parameter("location", json!({"type": "string"}))
        .required(vec!["location".to_string()])
        .build();

    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Call a function")
        .with_function(func);

    assert!(builder.tools.is_some());
    assert_eq!(builder.tools.as_ref().unwrap().len(), 1);
}

#[test]
fn test_interaction_builder_with_background() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_agent("deep-research-pro")
        .with_text("Long running task")
        .with_background(true);

    assert_eq!(builder.background, Some(true));
}

#[test]
fn test_interaction_builder_with_store() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Temporary interaction")
        .with_store(false);

    assert_eq!(builder.store, Some(false));
}

#[test]
fn test_interaction_builder_build_request_success() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello");

    let result = builder.build_request();
    assert!(result.is_ok());

    let request = result.unwrap();
    assert_eq!(request.model.as_deref(), Some("gemini-3-flash-preview"));
    assert!(matches!(
        request.input,
        genai_client::InteractionInput::Text(_)
    ));
}

#[test]
fn test_interaction_builder_build_request_missing_input() {
    let client = create_test_client();
    let builder = client.interaction().with_model("gemini-3-flash-preview");

    let result = builder.build_request();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        crate::GenaiError::InvalidInput(_)
    ));
}

#[test]
fn test_interaction_builder_build_request_missing_model_and_agent() {
    let client = create_test_client();
    let builder = client.interaction().with_text("Hello");

    let result = builder.build_request();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        crate::GenaiError::InvalidInput(_)
    ));
}

#[test]
fn test_interaction_builder_with_response_modalities() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Generate an image")
        .with_response_modalities(vec!["IMAGE".to_string()]);

    assert_eq!(
        builder.response_modalities.as_ref().unwrap(),
        &vec!["IMAGE".to_string()]
    );
}

#[test]
fn test_interaction_builder_with_max_function_call_loops() {
    let client = create_test_client();

    // Test default value
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Test");
    assert_eq!(
        builder.max_function_call_loops,
        super::auto_functions::DEFAULT_MAX_FUNCTION_CALL_LOOPS
    );

    // Test custom value
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Test")
        .with_max_function_call_loops(10);
    assert_eq!(builder.max_function_call_loops, 10);

    // Test setting to minimum (1)
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Test")
        .with_max_function_call_loops(1);
    assert_eq!(builder.max_function_call_loops, 1);
}
