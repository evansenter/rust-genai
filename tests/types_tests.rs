use rust_genai::{
    CodeExecutionResult, FunctionCall, FunctionDeclaration, FunctionParameters,
    GenerateContentResponse,
};
use serde_json::json;

#[test]
fn test_function_declaration_to_tool() {
    // Test basic function declaration
    let func_decl = FunctionDeclaration {
        name: "test_function".to_string(),
        description: "A test function".to_string(),
        parameters: FunctionParameters {
            type_: "object".to_string(),
            properties: json!({
                "param1": {"type": "string"},
                "param2": {"type": "number"}
            }),
            required: vec!["param1".to_string()],
        },
    };

    let tool = func_decl.into_tool();

    // Verify tool structure
    assert!(tool.function_declarations.is_some());
    let declarations = tool.function_declarations.unwrap();
    assert_eq!(declarations.len(), 1);

    let internal_decl = &declarations[0];
    assert_eq!(internal_decl.name, "test_function");
    assert_eq!(internal_decl.description, "A test function");

    // Verify parameters were properly converted
    assert_eq!(internal_decl.parameters.type_, "object");
    assert!(internal_decl.parameters.properties.is_object());
    assert_eq!(internal_decl.parameters.required, vec!["param1"]);
}

#[test]
fn test_function_declaration_no_parameters() {
    let func_decl = FunctionDeclaration {
        name: "no_params".to_string(),
        description: "Function with no parameters".to_string(),
        parameters: FunctionParameters {
            type_: "object".to_string(),
            properties: json!({}),
            required: vec![],
        },
    };

    let tool = func_decl.into_tool();

    let declarations = tool.function_declarations.unwrap();
    let internal_decl = &declarations[0];

    // Should still have object type and empty properties
    assert_eq!(internal_decl.parameters.type_, "object");
    assert!(internal_decl.parameters.properties.is_object());
    assert!(internal_decl.parameters.required.is_empty());
}

#[test]
fn test_function_declaration_with_required_params() {
    // Test function declaration with required parameters
    let func_decl = FunctionDeclaration {
        name: "test_func".to_string(),
        description: "Test".to_string(),
        parameters: FunctionParameters {
            type_: "object".to_string(),
            properties: json!({
                "a": {"type": "string"},
                "b": {"type": "string"}
            }),
            required: vec!["a".to_string(), "b".to_string()],
        },
    };

    let tool = func_decl.into_tool();
    let internal_decl = &tool.function_declarations.unwrap()[0];

    // Should preserve the required array
    assert_eq!(internal_decl.parameters.required, vec!["a", "b"]);
}

#[test]
fn test_function_call() {
    let fc = FunctionCall {
        name: "test_func".to_string(),
        args: json!({"arg1": "value1", "arg2": 42}),
    };

    assert_eq!(fc.name, "test_func");
    assert_eq!(fc.args["arg1"], "value1");
    assert_eq!(fc.args["arg2"], 42);

    // Test equality
    let fc2 = FunctionCall {
        name: "test_func".to_string(),
        args: json!({"arg1": "value1", "arg2": 42}),
    };
    assert_eq!(fc, fc2);
}

#[test]
fn test_code_execution_result() {
    let result = CodeExecutionResult {
        code: "print('Hello, World!')".to_string(),
        output: "Hello, World!".to_string(),
    };

    assert_eq!(result.code, "print('Hello, World!')");
    assert_eq!(result.output, "Hello, World!");

    // Test default
    let default_result = CodeExecutionResult::default();
    assert_eq!(default_result.code, "");
    assert_eq!(default_result.output, "");
}

#[test]
fn test_generate_content_response() {
    // Test text-only response
    let text_response = GenerateContentResponse {
        text: Some("Hello from AI".to_string()),
        function_calls: None,
        code_execution_results: None,
    };
    assert_eq!(text_response.text, Some("Hello from AI".to_string()));
    assert!(text_response.function_calls.is_none());
    assert!(text_response.code_execution_results.is_none());

    // Test function call response
    let fc = FunctionCall {
        name: "get_weather".to_string(),
        args: json!({"location": "Paris"}),
    };
    let fc_response = GenerateContentResponse {
        text: None,
        function_calls: Some(vec![fc]),
        code_execution_results: None,
    };
    assert!(fc_response.text.is_none());
    assert_eq!(fc_response.function_calls.as_ref().unwrap().len(), 1);

    // Test code execution response
    let code_result = CodeExecutionResult {
        code: "1 + 1".to_string(),
        output: "2".to_string(),
    };
    let code_response = GenerateContentResponse {
        text: Some("The result is:".to_string()),
        function_calls: None,
        code_execution_results: Some(vec![code_result]),
    };
    assert!(code_response.text.is_some());
    assert!(code_response.function_calls.is_none());
    assert_eq!(
        code_response.code_execution_results.as_ref().unwrap().len(),
        1
    );

    // Test mixed response
    let mixed_response = GenerateContentResponse {
        text: Some("Processing...".to_string()),
        function_calls: Some(vec![
            FunctionCall {
                name: "func1".to_string(),
                args: json!({}),
            },
            FunctionCall {
                name: "func2".to_string(),
                args: json!({"x": 1}),
            },
        ]),
        code_execution_results: Some(vec![CodeExecutionResult {
            code: "test".to_string(),
            output: "output".to_string(),
        }]),
    };
    assert!(mixed_response.text.is_some());
    assert_eq!(mixed_response.function_calls.as_ref().unwrap().len(), 2);
    assert_eq!(
        mixed_response
            .code_execution_results
            .as_ref()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn test_function_declaration_serialization() {
    let func_decl = FunctionDeclaration {
        name: "serialize_test".to_string(),
        description: "Test serialization".to_string(),
        parameters: FunctionParameters {
            type_: "object".to_string(),
            properties: json!({
                "test": {"type": "string"}
            }),
            required: vec!["test".to_string()],
        },
    };

    // Test that it can be serialized and deserialized
    let serialized = serde_json::to_string(&func_decl).unwrap();
    let deserialized: FunctionDeclaration = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized.name, func_decl.name);
    assert_eq!(deserialized.description, func_decl.description);
    assert_eq!(deserialized.parameters.type_, func_decl.parameters.type_);
    assert_eq!(
        deserialized.parameters.properties,
        func_decl.parameters.properties
    );
    assert_eq!(
        deserialized.parameters.required,
        func_decl.parameters.required
    );
}
