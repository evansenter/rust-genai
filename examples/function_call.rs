use rust_genai::CallableFunction;
use rust_genai::Client;
use rust_genai::{model_function_calls_request, user_text, user_tool_response};
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
fn get_weather(location: String, unit: Option<String>) -> String {
    // In a real app, you might have more complex logic here.
    let unit_str = unit.as_deref().unwrap_or("celsius");
    format!("The weather in {location} is currently 22 degrees {unit_str} and mostly sunny.")
}

/// Function to get the details on a given city (with simulated delay)
#[generate_function_declaration(city(description = "The city to get details for, e.g. Tokyo"))]
async fn get_city_details(city: String) -> serde_json::Value {
    // Simulate some async work, e.g., a network call
    println!("(Simulating async fetch for city: {city}...)");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    println!("(Async fetch for {city} complete.)");
    json!({
        "city_name": city,
        "population": 37_000_000, // Example data
        "country": "Japan",
        "description": format!("{} is a vibrant metropolis, known for its unique blend of modern and traditional culture.", city),
        "highlights": ["Skytree Tower", "Shibuya Crossing", "Senso-ji Temple"]
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not found in environment");
    let client = Client::builder(api_key).debug().build();

    let model_name = "gemini-2.5-flash-preview-05-20"; // Or your preferred model like "gemini-pro"

    // Test Case 1: Manual function calling (demonstrating the traditional approach)
    println!("\n--- Test Case 1: Weather in London (Manual Function Calling) ---");
    let prompt1 = "What is the weather like in London in Fahrenheit?";
    
    // Get the function declaration from the macro-generated function
    let weather_func_decl = get_weather_declaration();
    
    // Initial request with function available
    let response1 = client
        .with_model(model_name)
        .with_prompt(prompt1)
        .with_function(weather_func_decl)
        .generate()
        .await?;
    
    // Check if the model wants to call a function
    if let Some(function_calls) = response1.function_calls {
        println!("Model requested function calls:");
        for call in &function_calls {
            println!("  Function: {} with args: {}", call.name, call.args);
            
            // Manually execute the function based on its name
            if call.name == "get_weather" {
                // Extract arguments
                let location = call.args.get("location")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let unit = call.args.get("unit")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                
                // Call the function
                let result = get_weather(location.to_string(), unit);
                println!("Function result: {}", result);
                
                // Build conversation history with the function result
                let contents = vec![
                    user_text(prompt1.to_string()),
                    model_function_calls_request(function_calls.clone().into_iter()
                        .map(|fc| genai_client::models::request::FunctionCall {
                            name: fc.name,
                            args: fc.args,
                        })
                        .collect()),
                    user_tool_response(call.name.clone(), json!({ "result": result })),
                ];
                
                // Send the function result back to the model
                let final_response = client
                    .with_model(model_name)
                    .with_contents(contents)
                    .with_function(get_weather_declaration())
                    .generate()
                    .await?;
                
                if let Some(text) = final_response.text {
                    println!("Final Model Response:\n{text}");
                }
            }
        }
    } else if let Some(text) = response1.text {
        // Model responded directly without calling functions
        println!("Model Response:\n{text}");
    }

    // Test Case 2: Automatic function calling for multiple tools
    println!("\n\n--- Test Case 2: Weather and Details for Tokyo (Automatic) ---");
    let prompt2 = "Tell me about Tokyo: What is the weather like there in Celsius, and also give me some details about the city. Finally, suggest what I should pack if I go there tomorrow.";
    match client
        .with_model(model_name)
        .with_initial_user_text(prompt2)
        .generate_with_auto_functions()
        .await
    {
        Ok(response) => {
            if let Some(text) = response.text {
                println!("Final Model Response:\n{text}");
            } else {
                println!("Model did not provide a final text response.");
            }
            if let Some(function_calls) = response.function_calls {
                if !function_calls.is_empty() {
                    eprintln!(
                        "Warning: Final response still contained unhandled function calls: {function_calls:?}"
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("Error in Test Case 2: {e}");
        }
    }

    // Test Case 3: Automatic function calling with system prompt
    println!("\n\n--- Test Case 3: Weather in Paris with System Prompt (Automatic) ---");
    let prompt3 = "How is the weather in Paris (France)?";
    let system_prompt = "You are a laconic weather bot. Only state the weather using the available tool. No extra words.";
    match client
        .with_model(model_name)
        .with_initial_user_text(prompt3)
        .with_system_instruction(system_prompt)
        .generate_with_auto_functions()
        .await
    {
        Ok(response) => {
            if let Some(text) = response.text {
                println!("Final Model Response:\n{text}");
            } else {
                println!("Model did not provide a final text response.");
            }
        }
        Err(e) => {
            eprintln!("Error in Test Case 3: {e}");
        }
    }

    Ok(())
}
