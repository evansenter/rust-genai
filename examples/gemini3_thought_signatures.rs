use genai_client;
use rust_genai::{
    CallableFunction, Client, WithFunctionCalling, model_function_calls_request_with_signatures,
    user_text, user_tool_response,
};
use rust_genai_macros::generate_function_declaration;
use serde_json::json;
use std::env;
use std::error::Error;

/// Get the current weather for a location
#[generate_function_declaration(
    location(description = "The city and state, e.g., San Francisco, CA"),
    unit(description = "Temperature unit", enum_values = ["celsius", "fahrenheit"])
)]
fn get_weather(location: String, unit: Option<String>) -> String {
    let unit_str = unit.as_deref().unwrap_or("celsius");
    format!("The weather in {location} is 22 degrees {unit_str} and sunny.")
}

/// Get information about a city
#[generate_function_declaration(city(description = "The name of the city, e.g., Tokyo, Paris"))]
async fn get_city_info(city: String) -> serde_json::Value {
    json!({
        "city": city,
        "population": "37 million",
        "country": "Japan",
        "famous_for": ["Skytree", "Shibuya Crossing", "Mount Fuji views"]
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not found in environment");
    let client = Client::builder(api_key).build();

    // IMPORTANT: Use Gemini 3 models for thought signatures
    let model = "gemini-3-flash-preview";

    println!("=== Gemini 3 Thought Signatures Example ===\n");
    println!("This example demonstrates proper multi-turn function calling with Gemini 3.");
    println!("Gemini 3 REQUIRES thought signatures to maintain reasoning context.\n");

    let prompt = "What's the weather in Tokyo and tell me about the city?";

    // Step 1: Initial request with functions
    println!("Step 1: Sending initial request...");
    let response1 = client
        .with_model(model)
        .with_prompt(prompt)
        .with_function(get_weather_declaration())
        .with_function(get_city_info_declaration())
        .generate()
        .await?;

    println!(
        "Response received with {} function calls\n",
        response1
            .function_calls
            .as_ref()
            .map(|f| f.len())
            .unwrap_or(0)
    );

    // Step 2: Extract thought signatures (CRITICAL for Gemini 3!)
    let thought_signatures = response1.thought_signatures.clone();

    if let Some(ref sigs) = thought_signatures {
        println!("✓ Extracted {} thought signature(s)", sigs.len());
        for (i, sig) in sigs.iter().enumerate() {
            println!(
                "  Signature {}: {}...",
                i + 1,
                &sig.chars().take(20).collect::<String>()
            );
        }
    } else {
        println!("⚠ No thought signatures in response (unexpected for Gemini 3)");
    }
    println!();

    if let Some(function_calls) = response1.function_calls.clone() {
        println!(
            "Step 2: Executing {} function call(s)...",
            function_calls.len()
        );

        // Execute each function call
        for call in &function_calls {
            println!("  Calling: {}", call.name);
        }

        // For demo purposes, let's execute them
        let weather_result = get_weather("Tokyo".to_string(), Some("celsius".to_string()));
        let city_info_result = get_city_info("Tokyo".to_string()).await;

        println!("  ✓ Functions executed\n");

        // Step 3: Build conversation history WITH thought signatures
        println!("Step 3: Building conversation history with thought signatures...");

        // Convert to internal FunctionCall type for the helper
        let internal_calls: Vec<genai_client::FunctionCall> = function_calls
            .into_iter()
            .map(|fc| genai_client::FunctionCall {
                name: fc.name,
                args: fc.args,
            })
            .collect();

        let contents = vec![
            user_text(prompt.to_string()),
            // CRITICAL: Pass thought signatures here!
            model_function_calls_request_with_signatures(
                internal_calls,
                thought_signatures, // This is what makes Gemini 3 work!
            ),
            user_tool_response("get_weather".to_string(), json!(weather_result)),
            user_tool_response("get_city_info".to_string(), city_info_result),
        ];

        println!("  ✓ Conversation history built with signatures\n");

        // Step 4: Send follow-up request
        println!("Step 4: Sending follow-up request with conversation history...");
        let response2 = client
            .with_model(model)
            .with_contents(contents)
            .with_function(get_weather_declaration())
            .with_function(get_city_info_declaration())
            .generate()
            .await?;

        // Step 5: Display final response
        println!("\n=== Final Model Response ===");
        if let Some(text) = response2.text {
            println!("{}", text);
        } else {
            println!("(No text response)");
        }

        if let Some(new_calls) = response2.function_calls {
            println!(
                "\n⚠ Model requested {} more function call(s)",
                new_calls.len()
            );
            println!("In a real application, you would continue the loop.");
        }
    } else {
        println!("Model responded directly without function calls:");
        if let Some(text) = response1.text {
            println!("{}", text);
        }
    }

    println!("\n=== Key Takeaways ===");
    println!("1. Extract thought_signatures from GenerateContentResponse");
    println!("2. Pass them to model_function_calls_request_with_signatures()");
    println!("3. Include the result in conversation history");
    println!("4. Without signatures, Gemini 3 returns 400 errors!");

    Ok(())
}
