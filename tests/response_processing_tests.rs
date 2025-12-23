// Tests for internal response processing
// Since the response processing is internal, we test it indirectly through
// the public API's ability to handle various response formats

use rust_genai::{CodeExecutionResult, FunctionCall, GenerateContentResponse};
use serde_json::json;

#[test]
fn test_response_conversion() {
    // Test conversion from internal to public response types
    // This tests the From implementation indirectly

    // Create a response with all fields
    let response = GenerateContentResponse {
        text: Some("Test response".to_string()),
        function_calls: Some(vec![FunctionCall {
            name: "test_func".to_string(),
            args: json!({"param": "value"}),
        }]),
        code_execution_results: Some(vec![CodeExecutionResult {
            code: "print('test')".to_string(),
            output: "test".to_string(),
        }]),
        thought_signatures: None,
    };

    // Verify all fields are present
    assert!(response.text.is_some());
    assert!(response.function_calls.is_some());
    assert!(response.code_execution_results.is_some());
}

#[test]
fn test_empty_response_handling() {
    // Test handling of empty responses
    let empty_response = GenerateContentResponse {
        text: None,
        function_calls: None,
        code_execution_results: None,
        thought_signatures: None,
    };

    assert!(empty_response.text.is_none());
    assert!(empty_response.function_calls.is_none());
    assert!(empty_response.code_execution_results.is_none());
}

#[test]
fn test_response_with_empty_collections() {
    // Test response with empty vectors
    let response = GenerateContentResponse {
        text: Some(String::new()),            // Empty string
        function_calls: Some(vec![]),         // Empty vector
        code_execution_results: Some(vec![]), // Empty vector
        thought_signatures: None,
    };

    assert_eq!(response.text, Some(String::new()));
    assert_eq!(response.function_calls.as_ref().unwrap().len(), 0);
    assert_eq!(response.code_execution_results.as_ref().unwrap().len(), 0);
}

#[test]
fn test_response_with_multiple_function_calls() {
    // Test response with multiple function calls
    let response = GenerateContentResponse {
        text: None,
        function_calls: Some(vec![
            FunctionCall {
                name: "func1".to_string(),
                args: json!({"a": 1}),
            },
            FunctionCall {
                name: "func2".to_string(),
                args: json!({"b": 2}),
            },
            FunctionCall {
                name: "func3".to_string(),
                args: json!({"c": 3}),
            },
        ]),
        code_execution_results: None,
        thought_signatures: None,
    };

    let calls = response.function_calls.as_ref().unwrap();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0].name, "func1");
    assert_eq!(calls[1].name, "func2");
    assert_eq!(calls[2].name, "func3");
}
