use rust_genai::{
    FunctionDeclaration, build_content_request, model_function_call, user_text, user_tool_response,
};
use serde_json::json;

#[test]
fn test_user_text() {
    let content = user_text("Hello, world!".to_string());

    // Verify the content structure
    assert_eq!(content.parts.len(), 1);
    assert_eq!(content.parts[0].text, Some("Hello, world!".to_string()));
    assert_eq!(content.role, Some("user".to_string()));
}

#[test]
fn test_model_function_call() {
    let args = json!({
        "location": "San Francisco, CA",
        "unit": "fahrenheit"
    });

    let content = model_function_call("get_weather".to_string(), args.clone());

    // Verify the content structure
    assert_eq!(content.parts.len(), 1);
    assert_eq!(content.role, Some("model".to_string()));

    let function_call = &content.parts[0].function_call;
    assert!(function_call.is_some());
    let fc = function_call.as_ref().unwrap();
    assert_eq!(fc.name, "get_weather");
    assert_eq!(fc.args, args);
}

#[test]
fn test_user_tool_response() {
    let response_data = json!({
        "temperature": 72,
        "condition": "sunny"
    });

    let content = user_tool_response("get_weather".to_string(), response_data.clone());

    // Verify the content structure
    assert_eq!(content.parts.len(), 1);
    assert_eq!(content.role, Some("user".to_string()));

    let function_response = &content.parts[0].function_response;
    assert!(function_response.is_some());
    let fr = function_response.as_ref().unwrap();
    assert_eq!(fr.name, "get_weather");
    assert_eq!(fr.response, response_data);
}

#[test]
fn test_user_tool_response_with_string() {
    // Test with a primitive string value
    let response_data = json!("The weather is sunny and 72°F");

    let content = user_tool_response("get_weather".to_string(), response_data.clone());

    // Verify the response is wrapped in an object
    let function_response = &content.parts[0].function_response;
    assert!(function_response.is_some());
    let fr = function_response.as_ref().unwrap();
    assert_eq!(fr.name, "get_weather");

    // Should be wrapped in { "result": ... }
    let expected = json!({ "result": "The weather is sunny and 72°F" });
    assert_eq!(fr.response, expected);
}

#[test]
fn test_user_tool_response_with_number() {
    // Test with a primitive number value
    let response_data = json!(42);

    let content = user_tool_response("calculate".to_string(), response_data.clone());

    let function_response = &content.parts[0].function_response;
    assert!(function_response.is_some());
    let fr = function_response.as_ref().unwrap();

    // Should be wrapped in { "result": ... }
    let expected = json!({ "result": 42 });
    assert_eq!(fr.response, expected);
}

#[test]
fn test_user_tool_response_with_bool() {
    // Test with a primitive boolean value
    let response_data = json!(true);

    let content = user_tool_response("check_status".to_string(), response_data.clone());

    let function_response = &content.parts[0].function_response;
    assert!(function_response.is_some());
    let fr = function_response.as_ref().unwrap();

    // Should be wrapped in { "result": ... }
    let expected = json!({ "result": true });
    assert_eq!(fr.response, expected);
}

#[test]
fn test_user_tool_response_with_array() {
    // Test with an array value
    let response_data = json!(["item1", "item2", "item3"]);

    let content = user_tool_response("list_items".to_string(), response_data.clone());

    let function_response = &content.parts[0].function_response;
    assert!(function_response.is_some());
    let fr = function_response.as_ref().unwrap();

    // Arrays should also be wrapped in { "result": ... }
    let expected = json!({ "result": ["item1", "item2", "item3"] });
    assert_eq!(fr.response, expected);
}

#[test]
fn test_user_tool_response_with_null() {
    // Test with null value
    let response_data = json!(null);

    let content = user_tool_response("check_null".to_string(), response_data.clone());

    let function_response = &content.parts[0].function_response;
    assert!(function_response.is_some());
    let fr = function_response.as_ref().unwrap();

    // null should be wrapped in { "result": ... }
    let expected = json!({ "result": null });
    assert_eq!(fr.response, expected);
}

#[test]
fn test_build_content_request_simple() {
    let conversation = vec![user_text("What's the weather?".to_string())];

    let request = build_content_request(conversation, None);

    // Verify request structure
    assert_eq!(request.contents.len(), 1);
    assert!(request.system_instruction.is_none());
    assert!(request.tools.is_none());
    assert!(request.tool_config.is_none());
}

#[test]
fn test_build_content_request_with_tools() {
    let function = FunctionDeclaration {
        name: "get_weather".to_string(),
        description: "Get weather information".to_string(),
        parameters: Some(json!({
            "type": "object",
            "properties": {
                "location": {"type": "string"}
            }
        })),
        required: vec!["location".to_string()],
    };

    let conversation = vec![user_text("What's the weather in Paris?".to_string())];

    let tools = vec![function.to_tool()];
    let request = build_content_request(conversation, Some(tools));

    // Verify request structure
    assert_eq!(request.contents.len(), 1);
    assert!(request.tools.is_some());
    assert_eq!(request.tools.as_ref().unwrap().len(), 1);
}

#[test]
fn test_build_content_request_multi_turn() {
    let args = json!({"location": "Tokyo"});
    let response = json!({"temperature": 68, "condition": "cloudy"});

    let conversation = vec![
        user_text("What's the weather in Tokyo?".to_string()),
        model_function_call("get_weather".to_string(), args),
        user_tool_response("get_weather".to_string(), response),
    ];

    let request = build_content_request(conversation, None);

    // Verify all parts are included
    assert_eq!(request.contents.len(), 3);

    // Verify roles
    assert_eq!(request.contents[0].role, Some("user".to_string()));
    assert_eq!(request.contents[1].role, Some("model".to_string()));
    assert_eq!(request.contents[2].role, Some("user".to_string()));
}

#[test]
fn test_content_api_edge_cases() {
    // Test empty string
    let content = user_text(String::new());
    assert_eq!(content.parts[0].text, Some(String::new()));

    // Test very long text
    let long_text = "x".repeat(100_000);
    let content = user_text(long_text.clone());
    assert_eq!(content.parts[0].text, Some(long_text));

    // Test special characters
    let special_text = "Hello 世界 🦀\n\t\"quotes\" 'single'".to_string();
    let content = user_text(special_text.clone());
    assert_eq!(content.parts[0].text, Some(special_text));

    // Test empty JSON objects
    let empty_args = json!({});
    let content = model_function_call("empty_func".to_string(), empty_args.clone());
    assert_eq!(
        content.parts[0].function_call.as_ref().unwrap().args,
        empty_args
    );
}
