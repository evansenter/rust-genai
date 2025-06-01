# Rust GenAI

A Rust client library for interacting with Google's Generative AI (Gemini) API.

## Features

- Simple, intuitive API for making requests to Google's Generative AI models
- Support for both single-shot and streaming text generation
- Function calling support for tool use
- Comprehensive error handling
- Async/await support with Tokio
- Well-documented code with examples

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rust-genai = "0.1.0"
rust-genai-macros = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
async-trait = "0.1"
inventory = "0.3"
```

## Usage

### API Key

You'll need a Google API key with access to the Gemini models. You can get one from the [Google AI Studio](https://makersuite.google.com/).

### Simple Request Example

```rust
use rust_genai::Client;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment variable
    let api_key = env::var("GEMINI_API_KEY")?;
    
    // Create the client using the builder
    let client = Client::builder(api_key).build();
    
    // Define model and prompt
    let model = "gemini-2.5-flash-preview-05-20";
    let prompt = "Write a short poem about Rust programming.";
    
    // Send request and get response using the builder pattern
    let response = client
        .with_model(model)
        .with_prompt(prompt)
        .generate()
        .await?;
    
    // Print the generated text
    println!("{}", response.text.unwrap_or_default());
    
    Ok(())
}
```

### Streaming Request Example

```rust
use rust_genai::Client;
use futures_util::{pin_mut, StreamExt};
use std::{env, io::{stdout, Write}};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("GEMINI_API_KEY")?;
    // Create the client using the builder
    let client = Client::builder(api_key).build();
    
    let model = "gemini-2.5-flash-preview-05-20";
    let prompt = "Explain quantum computing in simple terms.";
    
    // Get a stream of response chunks using the builder pattern
    let stream = client
        .with_model(model)
        .with_prompt(prompt)
        .stream();
    pin_mut!(stream);
    
    // Process each chunk as it arrives
    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                if let Some(text) = chunk.text {
                    print!("{}", text);
                    stdout().flush()?;
                }
            }
            Err(e) => {
                eprintln!("\nError: {}", e);
                break;
            }
        }
    }
    
    println!("\n");
    Ok(())
}
```

### Using System Instructions

```rust
use rust_genai::Client;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("GEMINI_API_KEY")?;
    // Create the client using the builder
    let client = Client::builder(api_key).build();
    
    let model = "gemini-2.5-flash-preview-05-20";
    let prompt = "What is the capital of France?";
    let system_instruction = "You are a helpful geography expert.";
    
    // Send request with system instruction using the builder pattern
    let response = client
        .with_model(model)
        .with_prompt(prompt)
        .with_system_instruction(system_instruction)
        .generate()
        .await?;
    
    println!("{}", response.text.unwrap_or_default());
    Ok(())
}
```

### Function Calling Example

```rust
use rust_genai::{Client, FunctionDeclaration};
use serde_json::json;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("GEMINI_API_KEY")?;
    // Create the client using the builder
    let client = Client::builder(api_key).build();
    
    // Define a function that the model can call
    let weather_function = FunctionDeclaration {
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
    
    let model = "gemini-2.5-flash-preview-05-20";
    let prompt = "What's the weather like in London?";
    
    // Send request with function calling enabled
    let response = client
        .with_model(model)
        .with_prompt(prompt)
        .with_function(weather_function)
        .generate()
        .await?;
    
    // Handle the response
    if let Some(text) = response.text {
        println!("Text response: {text}");
    }
    if let Some(function_calls) = response.function_calls {
        for call in function_calls {
            println!("Function call: {} with args: {}", call.name, call.args);
        }
        // Next steps would typically involve: 
        // 1. Executing this function with the provided arguments.
        // 2. Sending the function's output back to the model in a new request
        //    along with the conversation history.
    }
    
    Ok(())
}
```

For a more complete, multi-turn function calling example that shows how to execute the function and send its result back to the model, please see `examples/function_call.rs` in the project repository.

### Automatic Function Calling

The library now supports automatic function discovery and execution. Functions decorated with the `#[generate_function_declaration]` macro are automatically discovered and can be executed when the model requests them:

```rust
use rust_genai::Client;
use rust_genai_macros::generate_function_declaration;
use std::env;

/// Get the current weather in a location
#[generate_function_declaration(
    location(description = "The city and state, e.g. San Francisco, CA"),
    unit(description = "The temperature unit", enum_values = ["celsius", "fahrenheit"])
)]
fn get_weather(location: String, unit: Option<String>) -> String {
    format!("The weather in {} is 22 degrees {}", 
        location, 
        unit.as_deref().unwrap_or("celsius"))
}

/// Get city details
#[generate_function_declaration(
    city(description = "The city to get details for")
)]
async fn get_city_details(city: String) -> serde_json::Value {
    serde_json::json!({
        "city": city,
        "population": 1_000_000,
        "country": "Example Country"
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("GEMINI_API_KEY")?;
    let client = Client::builder(api_key).build();
    
    // Use generate_with_auto_functions() to automatically handle function calls
    let response = client
        .with_model("gemini-2.5-flash-preview-05-20")
        .with_initial_user_text("What's the weather in Tokyo? Also tell me about the city.")
        .generate_with_auto_functions()
        .await?;
    
    // The library automatically:
    // 1. Discovers all functions marked with #[generate_function_declaration]
    // 2. Sends their declarations to the model
    // 3. Executes requested functions when the model calls them
    // 4. Sends results back to the model
    // 5. Continues until the model provides a final response
    
    println!("{}", response.text.unwrap_or_default());
    Ok(())
}
```

Key features of automatic function calling:
- Functions are discovered at compile time using the `inventory` crate
- Both sync and async functions are supported
- The conversation loop handles multiple function calls automatically
- Error handling is built-in - function errors are reported back to the model
- Maximum of 5 conversation turns to prevent infinite loops

### Using the Procedural Macro for Function Declarations

For a more ergonomic way to create function declarations, you can use the provided procedural macro:

```rust
use rust_genai_macros::generate_function_declaration;

/// Function to get the weather in a given location
#[generate_function_declaration(
    location(description = "The city and state, e.g. San Francisco, CA"),
    unit(description = "The temperature unit to use", enum_values = ["celsius", "fahrenheit"])
)]
fn get_weather(location: String, unit: Option<String>) -> String {
    // Your implementation here
    format!("Weather for {}", location)
}

// The macro generates a function called `get_weather_declaration()` 
// that returns a FunctionDeclaration
let weather_function = get_weather_declaration();

// Use it with the client
let response = client
    .with_model(model)
    .with_prompt("What's the weather in Paris?")
    .with_function(weather_function)
    .generate()
    .await?;
```

The macro supports:
- Automatic extraction of function doc comments as descriptions
- Parameter descriptions via the macro attribute
- Enum constraints for parameters with fixed values
- Proper handling of optional parameters (Option<T>)
- Type mapping for common Rust types (String, i32, i64, f32, f64, bool, Vec<T>, serde_json::Value)

## Project Structure

The project consists of two main components:

1. **Public API Crate (`rust-genai`)**: 
   - Provides a user-friendly interface in `src/lib.rs`
   - Handles high-level error representation and client creation

2. **Internal Client (`genai-client/`)**: 
   - Contains the low-level implementation for API communication
   - Defines the JSON serialization/deserialization models
   - Handles the actual HTTP requests and response parsing
   Users of the `rust-genai` crate typically do not need to interact with `genai-client` directly, as its functionalities are exposed through the main `rust-genai` API.

## Available Models

This library should work with all Google Generative AI models, including:

- gemini-2.5-flash-preview-05-20
- (Check Google AI documentation for the latest models)

## Error Handling

The library provides the `GenaiError` enum for comprehensive error handling:

- `Http`: Network-related errors
- `Parse`: Issues parsing the response
- `Json`: JSON deserialization errors
- `Utf8`: Text encoding errors
- `Api`: Errors returned by the Google API
- `Internal`: Other internal errors

## License

[Add your license information here]

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. 