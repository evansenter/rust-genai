//! Auto Function Calling Example
//!
//! This example demonstrates automatic function calling where the library
//! handles the function execution loop for you.
//!
//! Shows both non-streaming (with auto-execution) and streaming usage.
//!
//! # Running
//!
//! ```bash
//! cargo run --example auto_function_calling
//! ```
//!
//! # Prerequisites
//!
//! Set the `GEMINI_API_KEY` environment variable with your API key.

use futures_util::StreamExt;
use rust_genai::{CallableFunction, Client, StreamChunk};
use rust_genai_macros::generate_function_declaration;
use std::env;
use std::io::{Write, stdout};

// Define a function using the macro - this automatically registers it
// in the global function registry for auto-calling.

/// Gets the current weather for a city
#[generate_function_declaration(city(description = "The city to get weather for"))]
fn get_weather(city: String) -> String {
    // In a real application, this would call a weather API
    println!("  [Function called: get_weather(city={})]", city);
    format!(
        r#"{{"city": "{}", "temperature": "22°C", "conditions": "partly cloudy", "humidity": "65%"}}"#,
        city
    )
}

/// Gets the current time in a timezone
#[generate_function_declaration(timezone(description = "The timezone like UTC, PST, EST, JST"))]
fn get_time(timezone: String) -> String {
    println!("  [Function called: get_time(timezone={})]", timezone);
    format!(
        r#"{{"timezone": "{}", "time": "14:30:00", "date": "2024-12-24"}}"#,
        timezone
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable not set");

    let client = Client::builder(api_key).build();

    println!("=== AUTO FUNCTION CALLING EXAMPLE ===\n");

    // Get the function declarations from our registered functions
    let weather_func = GetWeatherCallable.declaration();
    let time_func = GetTimeCallable.declaration();

    println!("Registered functions:");
    println!(
        "  - {}: {}",
        weather_func.name(),
        weather_func.description()
    );
    println!("  - {}: {}", time_func.name(), time_func.description());
    println!();

    // Ask a question that requires calling a function
    let prompt = "What's the weather like in Tokyo and what time is it there (JST)?";
    println!("User: {}\n", prompt);
    println!("Processing (functions will be called automatically)...\n");

    // Use create_with_auto_functions - the library handles the entire loop:
    // 1. Send request to model
    // 2. If model returns function calls, execute them
    // 3. Send results back to model
    // 4. Repeat until model returns text
    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(prompt)
        .with_functions(vec![weather_func, time_func])
        .create_with_auto_functions()
        .await?;

    println!("\n--- Final Response ---");
    println!("Status: {:?}", response.status);

    if let Some(text) = response.text() {
        println!("\nAssistant: {}", text);
    }

    println!("\n--- End Auto Function Calling ---");

    // Streaming example with function calling
    // Note: Streaming shows the response as it's generated, but function calls
    // are typically returned as complete chunks rather than streamed piece by piece.
    println!("\n=== STREAMING WITH FUNCTION CALLING ===\n");

    let stream_prompt = "What's the weather in London right now?";
    println!("User: {}\n", stream_prompt);
    println!("Response (streaming):");

    let weather_func_stream = GetWeatherCallable.declaration();

    let mut stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(stream_prompt)
        .with_function(weather_func_stream)
        .create_stream();

    let mut saw_function_call = false;

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => match chunk {
                StreamChunk::Delta(content) => {
                    // Check for text content
                    if let Some(text) = content.text() {
                        print!("{}", text);
                        stdout().flush()?;
                    }
                    // Check for function calls
                    if content.is_function_call() {
                        saw_function_call = true;
                        println!("\n  [Received function call in stream]");
                    }
                }
                StreamChunk::Complete(response) => {
                    println!();
                    if response.has_function_calls() {
                        let calls = response.function_calls();
                        println!("\nFunction calls in final response:");
                        for (id, name, args, _) in &calls {
                            println!("  - {}({}) [id: {:?}]", name, args, id);
                        }
                        println!("\nNote: To complete the conversation, you would execute");
                        println!("the function and send results back to the model.");
                    }
                }
            },
            Err(e) => {
                eprintln!("\nStream error: {e}");
                break;
            }
        }
    }

    if saw_function_call {
        println!("\n(Function call was streamed successfully)");
    }

    println!("\n--- End Streaming ---");

    Ok(())
}
