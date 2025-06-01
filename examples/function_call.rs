use rust_genai::{Client, model_function_call, user_text, user_tool_response, build_content_request};
use rust_genai_macros::generate_function_declaration;
use serde_json::json;
use std::env;
use std::error::Error;

/// Function to get the weather in a given location
#[generate_function_declaration(
    location(description = "The city and state, e.g. San Francisco, CA"),
    unit(description = "The temperature unit to use", enum_values = ["celsius", "fahrenheit"])
)]
#[allow(clippy::needless_pass_by_value)]
fn get_weather(
    location: String, 
    unit: Option<String>,
) -> String {
    let unit_str = unit.as_deref().unwrap_or("celsius");
    format!("The weather in {location} is currently 72 degrees {unit_str} and sunny.")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let api_key = env::var("GEMINI_API_KEY")?;
    let client = Client::builder(api_key).debug().build();

    // Get the function declaration from the macro-generated function
    let weather_function_decl = get_weather_declaration();

    let model_name = "gemini-2.5-flash-preview-05-20";
    let initial_prompt = "What is the weather like in San Francisco in Fahrenheit? Please use the get_weather tool to find out, and then tell me what I should pack.";

    let response1 = client
        .with_model(model_name)
        .with_prompt(initial_prompt)
        .with_function(weather_function_decl.clone())
        .generate()
        .await?;

    if let Some(text) = &response1.text {
        println!("Text: {text}");
    }

    if let Some(function_calls) = response1.function_calls {
        if let Some(call) = function_calls.first() {
            if call.name == "get_weather" {
                // Execute the function
                let location = call.args.get("location")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let unit = call.args.get("unit")
                    .and_then(|v| v.as_str());

                let result = get_weather(location.to_string(), unit.map(std::string::ToString::to_string));
                println!("\nFunction result: {result}");

                // Send the result back to the model
                let conversation = vec![
                    user_text(initial_prompt.to_string()),
                    model_function_call(call.name.clone(), call.args.clone()),
                    user_tool_response("get_weather".to_string(), json!({ "result": result })),
                ];

                let response2 = client
                    .generate_from_request(
                        model_name,
                        build_content_request(conversation, Some(vec![weather_function_decl.to_tool()]))
                    )
                    .await?;

                if let Some(text) = response2.text {
                    println!("\nFinal response: {text}");
                }
            }
        }
    }

    Ok(())
}
