use genai_client::models::request::{
    Content, FunctionDeclaration as InternalFunctionDeclaration,
    FunctionParameters as InternalFunctionParameters, FunctionResponse,
    GenerateContentRequest as InternalGenerateContentRequest, Part, Tool,
};
use rust_genai::{Client, FunctionDeclaration};
use serde_json::json;
use std::env;
use std::error::Error;

// Mock function to simulate getting weather information
fn get_mock_weather_report(location: &str, unit: Option<&str>) -> String {
    let unit_str = unit.unwrap_or("celsius");
    format!("The weather in {location} is currently 451 degrees {unit_str} and sunny.")
}

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<(), Box<dyn Error>> {
    // Get API key from environment variable
    let api_key = env::var("GEMINI_API_KEY").map_err(|e| Box::new(e) as Box<dyn Error>)?;

    // Create the client
    let client = Client::builder(api_key).debug().build();

    // Define the weather function (using the public FunctionDeclaration)
    let weather_function_public_decl = FunctionDeclaration {
        name: "get_weather".to_string(),
        description: "Get the current weather in a given location".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city and state, e.g. San Francisco, CA"
                },
                "unit": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"],
                    "description": "The temperature unit to use"
                }
            },
            "required": ["location"]
        }),
        required: vec!["location".to_string()],
    };

    // Define model and prompt
    let model_name = "gemini-2.5-flash-preview-05-20";
    let initial_prompt_text = "What is the weather like in San Francisco in Fahrenheit? Please use the get_weather tool to find out, and then tell me what I should pack.";

    println!("Sending initial request to model: {model_name}");
    println!("Prompt: {initial_prompt_text}\n");

    // --- First API Call ---
    let response1 = client
        .with_model(model_name)
        .with_prompt(initial_prompt_text)
        .with_function(weather_function_public_decl.clone())
        .generate()
        .await?;

    println!("--- First Model Response ---");
    if let Some(text) = &response1.text {
        println!("Text response: {text}");
    }

    // Check for function calls (now a Vec)
    if let Some(received_function_calls) = response1.function_calls {
        if received_function_calls.is_empty() {
            println!("Model returned an empty list of function calls.");
        } else {
            // For this example, we'll process only the first function call if multiple are present.
            let first_function_call = &received_function_calls[0];
            println!("\nFunction call received (processing first one):");
            println!("  Name: {}", first_function_call.name);
            println!("  Args: {}", first_function_call.args);

            if first_function_call.name == "get_weather" {
                println!("\nExecuting 'get_weather' function client-side...");
                let location = first_function_call
                    .args
                    .get("location")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown location");
                let unit = first_function_call
                    .args
                    .get("unit")
                    .and_then(|v| v.as_str());

                let weather_report = get_mock_weather_report(location, unit);
                println!("  Mock weather report: {weather_report}");

                println!("\nConstructing conversation history for second API call...");

                // 1. User's initial prompt
                let user_content = Content {
                    parts: vec![Part {
                        text: Some(initial_prompt_text.to_string()),
                        function_call: None,
                        function_response: None,
                    }],
                    role: Some("user".to_string()),
                };

                // 2. Model's first response (the function call)
                let model_function_call_part = Part {
                    text: None,
                    function_call: Some(genai_client::models::request::FunctionCall {
                        // Use internal FunctionCall
                        name: first_function_call.name.clone(),
                        args: first_function_call.args.clone(),
                    }),
                    function_response: None,
                };
                let model_content = Content {
                    parts: vec![model_function_call_part],
                    role: Some("model".to_string()),
                };

                // 3. Tool's response (our client's function execution)
                let tool_function_response_part = Part {
                    text: None,
                    function_call: None,
                    function_response: Some(FunctionResponse {
                        // This is genai_client::models::request::FunctionResponse
                        name: "get_weather".to_string(), // Should match the original function call name
                        response: json!({ "weather": weather_report }),
                    }),
                };
                let tool_content = Content {
                    parts: vec![tool_function_response_part],
                    role: Some("tool".to_string()), // Role for function responses is "tool"
                };

                let conversation_history = vec![user_content, model_content, tool_content];

                // We also need to convert the public FunctionDeclaration to the internal Tool format
                // for the request_body.tools field.
                let internal_tool = Tool {
                    function_declarations: Some(vec![InternalFunctionDeclaration {
                        name: weather_function_public_decl.name.clone(),
                        description: weather_function_public_decl.description.clone(),
                        parameters: InternalFunctionParameters {
                            type_: weather_function_public_decl
                                .parameters
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("object")
                                .to_string(),
                            properties: weather_function_public_decl
                                .parameters
                                .get("properties")
                                .cloned()
                                .unwrap_or_else(|| json!({})),
                            required: weather_function_public_decl.required.clone(),
                        },
                    }]),
                    code_execution: None,
                };

                let request_body_for_second_call = InternalGenerateContentRequest {
                    contents: conversation_history,
                    tools: Some(vec![internal_tool]),
                    system_instruction: None,
                    tool_config: None,
                };

                println!("\nSending constructed multi-turn request back to the model...");
                // --- Second API Call using generate_from_request ---
                let response2 = client
                    .generate_from_request(model_name, request_body_for_second_call)
                    .await?;

                println!("\n--- Second Model Response (after function execution) ---");
                if let Some(text) = response2.text {
                    println!("Final text response: {text}");
                }
                if let Some(fcs) = response2.function_calls {
                    if !fcs.is_empty() {
                        println!("\nUnexpected second set of function calls (showing first):");
                        println!("  Name: {}", fcs[0].name);
                        println!("  Args: {}", fcs[0].args);
                    }
                }
            }
        }
    } else if response1.text.is_none() {
        println!("Model did not return text and did not request any function calls.");
    }
    println!("--- End of Interaction ---");

    Ok(())
}
