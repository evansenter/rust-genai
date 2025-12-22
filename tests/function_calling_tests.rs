// Tests for function calling functionality
use rust_genai::{Client, FunctionDeclaration, WithFunctionCalling};
use serde_json::json;
use std::env;

#[tokio::test]
#[ignore = "Makes real API calls - may hit rate limits"]
async fn test_function_calling_integration() {
    let Ok(api_key) = env::var("GEMINI_API_KEY") else {
        println!("Skipping test_function_calling_integration: GEMINI_API_KEY not set.");
        return;
    };

    let client = Client::builder(api_key).build();

    // Define a simple weather function
    let weather_function = FunctionDeclaration::builder("get_weather")
        .description("Get the current weather in a given location")
        .parameter(
            "location",
            json!({
                "type": "string",
                "description": "The city and state, e.g. San Francisco, CA"
            }),
        )
        .parameter(
            "unit",
            json!({
                "type": "string",
                "enum": ["celsius", "fahrenheit"],
                "description": "The temperature unit"
            }),
        )
        .required(vec!["location".to_string()])
        .build();

    let model = "gemini-3-flash-preview";
    let prompt = "What's the weather like in London?";

    // First request - expect function call
    let response = client
        .with_model(model)
        .with_prompt(prompt)
        .with_function(weather_function.clone())
        .generate()
        .await;

    assert!(
        response.is_ok(),
        "First request failed: {:?}",
        response.err()
    );
    let response = response.unwrap();

    // Should have function calls
    assert!(
        response.function_calls.is_some(),
        "Expected function calls in response"
    );
    let function_calls = response.function_calls.unwrap();
    assert!(
        !function_calls.is_empty(),
        "Expected at least one function call"
    );

    // Verify the function call
    let call = &function_calls[0];
    assert_eq!(call.name, "get_weather");
    assert!(call.args["location"].is_string());
    assert!(
        call.args["location"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("london")
    );
}

#[test]
fn test_function_declaration_edge_cases() {
    // Test function with empty name (should still work)
    let func = FunctionDeclaration::builder("")
        .description("Empty name function")
        .build();
    let tool = func.into_tool();
    assert_eq!(tool.function_declarations.unwrap()[0].name(), "");

    // Test function with very long description
    let long_desc = "x".repeat(10000);
    let func = FunctionDeclaration::builder("long_desc")
        .description(long_desc.clone())
        .build();
    let tool = func.into_tool();
    assert_eq!(
        tool.function_declarations.unwrap()[0].description(),
        long_desc
    );

    // Test function with nested objects in parameters
    let func = FunctionDeclaration::builder("nested")
        .description("Nested parameters")
        .parameter(
            "outer",
            json!({
                "type": "object",
                "properties": {
                    "inner": {
                        "type": "object",
                        "properties": {
                            "value": {"type": "string"}
                        }
                    }
                }
            }),
        )
        .build();

    let tool = func.into_tool();
    assert!(tool.function_declarations.is_some());
}
