// Additional edge case tests for content_api module
//
// These tests complement the existing content_api_tests.rs by adding
// more edge cases and boundary conditions.

use rust_genai::content_api::*;
use serde_json::json;

#[test]
fn test_user_text_with_newlines_and_tabs() {
    // Test multiline text with various whitespace
    let text = "Line 1\nLine 2\r\nLine 3\tTabbed\t\tDouble tab".to_string();
    let content = user_text(text.clone());

    assert_eq!(content.parts.len(), 1);
    assert_eq!(content.parts[0].text, Some(text));
    assert_eq!(content.role, Some("user".to_string()));
}

#[test]
fn test_model_text_with_code_blocks() {
    // Test model response containing code
    let text = r#"Here's some code:
```rust
fn main() {
    println!("Hello, world!");
}
```
That's how you do it!"#.to_string();

    let content = model_text(text.clone());
    assert_eq!(content.parts[0].text, Some(text));
    assert_eq!(content.role, Some("model".to_string()));
}

#[test]
fn test_model_function_call_with_nested_objects() {
    // Test function call with deeply nested arguments
    let args = json!({
        "user": {
            "name": "Alice",
            "address": {
                "street": "123 Main St",
                "city": "Springfield",
                "coordinates": {
                    "lat": 42.123,
                    "lon": -71.456
                }
            },
            "preferences": {
                "notifications": {
                    "email": true,
                    "sms": false
                }
            }
        }
    });

    let content = model_function_call("update_user".to_string(), args.clone());

    assert_eq!(content.parts.len(), 1);
    assert!(content.parts[0].function_call.is_some());
    let fc = content.parts[0].function_call.as_ref().unwrap();
    assert_eq!(fc.name, "update_user");
    assert_eq!(fc.args, args);
}

#[test]
fn test_model_function_calls_request_empty_list() {
    // Test with empty function calls list
    let content = model_function_calls_request(vec![]);

    assert_eq!(content.parts.len(), 0);
    assert_eq!(content.role, Some("model".to_string()));
}

#[test]
fn test_model_function_calls_request_multiple() {
    // Test with multiple function calls
    let calls = vec![
        genai_client::FunctionCall {
            name: "function1".to_string(),
            args: json!({"param": "value1"}),
        },
        genai_client::FunctionCall {
            name: "function2".to_string(),
            args: json!({"param": "value2"}),
        },
        genai_client::FunctionCall {
            name: "function3".to_string(),
            args: json!({"param": "value3"}),
        },
    ];

    let content = model_function_calls_request(calls);

    assert_eq!(content.parts.len(), 3);
    assert_eq!(content.role, Some("model".to_string()));

    for (i, part) in content.parts.iter().enumerate() {
        assert!(part.function_call.is_some());
        let fc = part.function_call.as_ref().unwrap();
        assert_eq!(fc.name, format!("function{}", i + 1));
    }
}

#[test]
fn test_user_tool_response_with_error() {
    // Test tool response containing error information
    let error_response = json!({
        "error": "Division by zero",
        "code": "ERR_DIV_ZERO",
        "details": {
            "numerator": 10,
            "denominator": 0
        }
    });

    let content = user_tool_response("divide".to_string(), error_response.clone());

    assert_eq!(content.parts.len(), 1);
    assert!(content.parts[0].function_response.is_some());
    let fr = content.parts[0].function_response.as_ref().unwrap();
    assert_eq!(fr.name, "divide");
    assert_eq!(fr.response, error_response);
}

#[test]
fn test_user_tool_response_wrapping_primitives() {
    // Test that primitives get wrapped in {"result": ...}

    // String
    let string_val = json!("Hello");
    let content = user_tool_response("func".to_string(), string_val);
    let fr = content.parts[0].function_response.as_ref().unwrap();
    assert_eq!(fr.response, json!({"result": "Hello"}));

    // Number
    let number_val = json!(42);
    let content = user_tool_response("func".to_string(), number_val);
    let fr = content.parts[0].function_response.as_ref().unwrap();
    assert_eq!(fr.response, json!({"result": 42}));

    // Boolean
    let bool_val = json!(true);
    let content = user_tool_response("func".to_string(), bool_val);
    let fr = content.parts[0].function_response.as_ref().unwrap();
    assert_eq!(fr.response, json!({"result": true}));

    // Array
    let array_val = json!([1, 2, 3]);
    let content = user_tool_response("func".to_string(), array_val);
    let fr = content.parts[0].function_response.as_ref().unwrap();
    assert_eq!(fr.response, json!({"result": [1, 2, 3]}));
}

#[test]
fn test_user_tool_response_with_large_data() {
    // Test tool response with large data payload
    let large_array: Vec<i32> = (0..1000).collect();
    let response = json!({
        "data": large_array,
        "count": 1000
    });

    let content = user_tool_response("get_data".to_string(), response.clone());

    assert_eq!(content.parts.len(), 1);
    let fr = content.parts[0].function_response.as_ref().unwrap();
    assert_eq!(fr.name, "get_data");
    assert_eq!(fr.response, response);
}

#[test]
fn test_build_content_request_with_no_tools() {
    // Test building request without tools
    let contents = vec![
        user_text("Hello".to_string()),
        model_text("Hi there!".to_string()),
    ];

    let request = build_content_request(contents.clone(), None);

    assert_eq!(request.contents.len(), 2);
    assert!(request.tools.is_none());
    assert!(request.system_instruction.is_none());
    assert!(request.tool_config.is_none());
}

#[test]
fn test_build_content_request_with_many_contents() {
    // Test building request with many content items
    let mut contents = vec![];
    for i in 0..50 {
        contents.push(user_text(format!("User message {}", i)));
        contents.push(model_text(format!("Model response {}", i)));
    }

    let request = build_content_request(contents.clone(), None);
    assert_eq!(request.contents.len(), 100);
}

#[test]
fn test_function_call_with_empty_args() {
    // Test function call with empty arguments
    let content = model_function_call("no_args_function".to_string(), json!({}));

    assert_eq!(content.parts.len(), 1);
    let fc = content.parts[0].function_call.as_ref().unwrap();
    assert_eq!(fc.name, "no_args_function");
    assert_eq!(fc.args, json!({}));
}

#[test]
fn test_content_roles_are_correct() {
    // Verify all helper functions set the correct role

    let user_content = user_text("test".to_string());
    assert_eq!(user_content.role, Some("user".to_string()));

    let model_content = model_text("test".to_string());
    assert_eq!(model_content.role, Some("model".to_string()));

    let function_call = model_function_call("func".to_string(), json!({}));
    assert_eq!(function_call.role, Some("model".to_string()));

    let function_response = user_tool_response("func".to_string(), json!({}));
    assert_eq!(function_response.role, Some("user".to_string()));
}
