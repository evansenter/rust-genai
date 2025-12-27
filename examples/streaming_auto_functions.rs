//! Streaming Auto Function Calling Example
//!
//! This example demonstrates streaming with automatic function calling.
//! Unlike regular auto function calling which returns only the final response,
//! this streams content as it arrives while automatically executing functions.
//!
//! Shows how to:
//! - Stream content in real-time with functions executing between streaming rounds
//! - Track function execution events during streaming
//! - Handle multiple function calling rounds
//!
//! # Running
//!
//! ```bash
//! cargo run --example streaming_auto_functions
//! ```
//!
//! # Prerequisites
//!
//! Set the `GEMINI_API_KEY` environment variable with your API key.

use futures_util::StreamExt;
use rust_genai::{AutoFunctionStreamChunk, CallableFunction, Client, InteractionContent};
use rust_genai_macros::generate_function_declaration;
use std::env;
use std::io::{Write, stdout};

// Define functions using the macro - automatically registered for auto-calling.

/// Gets the current weather for a city
#[generate_function_declaration(city(description = "The city to get weather for"))]
fn get_weather(city: String) -> String {
    // Simulate some processing time
    std::thread::sleep(std::time::Duration::from_millis(100));
    format!(
        r#"{{"city": "{}", "temperature": "22°C", "conditions": "partly cloudy", "humidity": "65%"}}"#,
        city
    )
}

/// Gets the current time in a timezone
#[generate_function_declaration(timezone(description = "The timezone like UTC, PST, EST, JST"))]
fn get_time(timezone: String) -> String {
    std::thread::sleep(std::time::Duration::from_millis(100));
    format!(
        r#"{{"timezone": "{}", "time": "14:30:00", "date": "2024-12-24"}}"#,
        timezone
    )
}

/// Converts temperature between units
#[generate_function_declaration(
    value(description = "The temperature value"),
    from_unit(description = "Source unit: celsius or fahrenheit"),
    to_unit(description = "Target unit: celsius or fahrenheit")
)]
fn convert_temperature(value: f64, from_unit: String, to_unit: String) -> String {
    let result = if from_unit.to_lowercase() == "celsius" && to_unit.to_lowercase() == "fahrenheit"
    {
        value * 9.0 / 5.0 + 32.0
    } else if from_unit.to_lowercase() == "fahrenheit" && to_unit.to_lowercase() == "celsius" {
        (value - 32.0) * 5.0 / 9.0
    } else {
        value
    };
    format!(r#"{{"value": {:.1}, "unit": "{}"}}"#, result, to_unit)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable not set");

    let client = Client::builder(api_key).build();

    println!("=== STREAMING AUTO FUNCTION CALLING ===\n");

    // Get function declarations
    let weather_func = GetWeatherCallable.declaration();
    let time_func = GetTimeCallable.declaration();
    let convert_func = ConvertTemperatureCallable.declaration();

    println!("Registered functions:");
    println!(
        "  - {}: {}",
        weather_func.name(),
        weather_func.description()
    );
    println!("  - {}: {}", time_func.name(), time_func.description());
    println!(
        "  - {}: {}",
        convert_func.name(),
        convert_func.description()
    );
    println!();

    // Ask a question that requires multiple function calls
    let prompt = "What's the weather in Tokyo? Also tell me what time it is there (JST timezone).";
    println!("User: {}\n", prompt);
    println!("Response (streaming with auto function execution):\n");

    // Use create_stream_with_auto_functions - combines streaming with auto function calling
    let mut stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(prompt)
        .with_functions(vec![weather_func, time_func, convert_func])
        .create_stream_with_auto_functions();

    let mut function_count = 0;
    let mut delta_count = 0;

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => match chunk {
                AutoFunctionStreamChunk::Delta(content) => {
                    delta_count += 1;
                    // Print text content as it arrives
                    if let InteractionContent::Text { text: Some(ref t) } = content {
                        print!("{}", t);
                        stdout().flush()?;
                    }
                    // Show thoughts if present
                    if let InteractionContent::Thought { text: Some(ref t) } = content {
                        print!("\n[Thinking: {}...]", &t[..t.len().min(50)]);
                        stdout().flush()?;
                    }
                }
                AutoFunctionStreamChunk::ExecutingFunctions(response) => {
                    // Notification that functions are about to execute
                    // Note: In streaming mode, function calls may come via deltas,
                    // so response.function_calls() may be empty. Check response.status
                    // to confirm functions are being requested.
                    println!("\n--- Executing Functions ---");
                    println!("  Status: {:?}", response.status);
                    let calls = response.function_calls();
                    if calls.is_empty() {
                        println!("  (Function calls received via stream deltas)");
                    } else {
                        for call in calls {
                            function_count += 1;
                            println!("  [{}] {}({})", function_count, call.name, call.args);
                        }
                    }
                    println!("---------------------------");
                }
                AutoFunctionStreamChunk::FunctionResults(results) => {
                    // Function execution completed
                    println!("--- Function Results ---");
                    for result in &results {
                        println!("  {} -> {}", result.name, result.result);
                    }
                    println!("------------------------\n");
                    println!("Continuing response...\n");
                }
                AutoFunctionStreamChunk::Complete(response) => {
                    println!("\n\n--- Stream Complete ---");
                    println!("Interaction ID: {}", response.id);
                    println!("Status: {:?}", response.status);
                    if let Some(usage) = &response.usage {
                        println!(
                            "Tokens: {} input, {} output",
                            usage.total_input_tokens.unwrap_or(0),
                            usage.total_output_tokens.unwrap_or(0)
                        );
                    }
                }
                _ => {
                    // Handle unknown future variants gracefully
                    println!("[Unknown event type]");
                }
            },
            Err(e) => {
                eprintln!("\nStream error: {e}");
                break;
            }
        }
    }

    println!("\n--- Statistics ---");
    println!("Total delta chunks: {}", delta_count);
    println!("Functions executed: {}", function_count);

    Ok(())
}
