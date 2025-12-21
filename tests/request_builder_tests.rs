use rust_genai::{Client, FunctionDeclaration};
use serde_json::json;

#[test]
fn test_request_builder_basic() {
    let api_key = "test-key".to_string();
    let client = Client::builder(api_key).build();

    // Test basic builder
    let _builder = client.with_model("gemini-3-flash-preview");

    // Test chaining methods
    let _builder = client
        .with_model("gemini-3-flash-preview")
        .with_prompt("Hello");

    let _builder = client
        .with_model("gemini-3-flash-preview")
        .with_prompt("Hello")
        .with_system_instruction("Be helpful");
}

#[test]
fn test_request_builder_with_functions() {
    let api_key = "test-key".to_string();
    let client = Client::builder(api_key).build();

    let function1 = FunctionDeclaration {
        name: "test_func1".to_string(),
        description: "Test function 1".to_string(),
        parameters: Some(json!({
            "type": "object",
            "properties": {
                "param1": {"type": "string"}
            }
        })),
        required: vec![],
    };

    let function2 = FunctionDeclaration {
        name: "test_func2".to_string(),
        description: "Test function 2".to_string(),
        parameters: None,
        required: vec![],
    };

    // Test single function
    let _builder = client
        .with_model("gemini-3-flash-preview")
        .with_prompt("Test")
        .with_function(function1.clone());

    // Test multiple functions
    let _builder = client
        .with_model("gemini-3-flash-preview")
        .with_prompt("Test")
        .with_function(function1)
        .with_function(function2);
}

#[test]
fn test_request_builder_function_variants() {
    let api_key = "test-key".to_string();
    let client = Client::builder(api_key).build();

    // Test function with all parameter types
    let complex_function = FunctionDeclaration {
        name: "complex_func".to_string(),
        description: "A function with various parameter types".to_string(),
        parameters: Some(json!({
            "type": "object",
            "properties": {
                "string_param": {"type": "string", "description": "A string parameter"},
                "number_param": {"type": "number", "description": "A number parameter"},
                "boolean_param": {"type": "boolean", "description": "A boolean parameter"},
                "array_param": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "An array parameter"
                },
                "enum_param": {
                    "type": "string",
                    "enum": ["option1", "option2", "option3"],
                    "description": "An enum parameter"
                }
            },
            "required": ["string_param"]
        })),
        required: vec!["string_param".to_string()],
    };

    let _builder = client
        .with_model("gemini-3-flash-preview")
        .with_prompt("Test complex function")
        .with_function(complex_function);
}

#[test]
fn test_request_builder_edge_cases() {
    let api_key = "test-key".to_string();
    let client = Client::builder(api_key).build();

    // Test empty prompt
    let _builder = client
        .with_model("gemini-3-flash-preview")
        .with_prompt("");

    // Test very long prompt
    let long_prompt = "x".repeat(10_000);
    let _builder = client
        .with_model("gemini-3-flash-preview")
        .with_prompt(&long_prompt);

    // Test special characters in prompt
    let _builder = client
        .with_model("gemini-3-flash-preview")
        .with_prompt("Test with special chars: 你好 �� \n\t\"quotes\"");

    // Test empty system instruction
    let _builder = client
        .with_model("gemini-3-flash-preview")
        .with_prompt("Hello")
        .with_system_instruction("");
}
