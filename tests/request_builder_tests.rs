use rust_genai::{Client, FunctionDeclaration, WithFunctionCalling};
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

    let function1 = FunctionDeclaration::builder("test_func1")
        .description("Test function 1")
        .parameter("param1", json!({"type": "string"}))
        .build();

    let function2 = FunctionDeclaration::builder("test_func2")
        .description("Test function 2")
        .build();

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
    let complex_function = FunctionDeclaration::builder("complex_func")
        .description("A function with various parameter types")
        .parameter(
            "string_param",
            json!({"type": "string", "description": "A string parameter"}),
        )
        .parameter(
            "number_param",
            json!({"type": "number", "description": "A number parameter"}),
        )
        .parameter(
            "boolean_param",
            json!({"type": "boolean", "description": "A boolean parameter"}),
        )
        .parameter(
            "array_param",
            json!({
                "type": "array",
                "items": {"type": "string"},
                "description": "An array parameter"
            }),
        )
        .parameter(
            "enum_param",
            json!({
                "type": "string",
                "enum": ["option1", "option2", "option3"],
                "description": "An enum parameter"
            }),
        )
        .required(vec!["string_param".to_string()])
        .build();

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
    let _builder = client.with_model("gemini-3-flash-preview").with_prompt("");

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
