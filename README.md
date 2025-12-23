# Rust GenAI

A Rust client library for interacting with Google's Generative AI (Gemini) API using the Interactions API.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
  - [API Key](#api-key)
  - [Simple Interaction](#simple-interaction)
  - [Streaming Responses](#streaming-responses)
  - [System Instructions](#system-instructions)
  - [Stateful Conversations](#stateful-conversations)
  - [Function Calling](#function-calling)
  - [Automatic Function Calling](#automatic-function-calling)
  - [Using the Procedural Macro](#using-the-procedural-macro)
- [API Reference](#api-reference)
- [Project Structure](#project-structure)
- [Available Models](#available-models)
- [Error Handling](#error-handling)
- [Logging](#logging)
- [License](#license)
- [Contributing](#contributing)

## Features

- Simple, intuitive API for making requests to Google's Generative AI models
- Support for both single-shot and streaming interactions
- Stateful conversations with automatic context management via `previous_interaction_id`
- Function calling with both manual and automatic execution
- Automatic function discovery at compile time using procedural macros
- Helper functions for building multi-turn conversations
- Comprehensive error handling with detailed error types
- Async/await support with Tokio
- Type-safe function argument handling with serde
- Support for both synchronous and asynchronous functions

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rust-genai = "0.2.0"
rust-genai-macros = "0.2.0"  # Only if using the procedural macros
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"  # For JSON values in function calls
futures-util = "0.3"  # Only if using streaming responses
```

Note: `async-trait` and `inventory` are already included as dependencies of `rust-genai`, so you don't need to add them unless you're implementing custom `CallableFunction` traits.

### Prerequisites

- Rust 1.75 or later (for stable async traits and other features)
- A Google AI API key with access to Gemini models (get one from [Google AI Studio](https://ai.dev/))

## Usage

> **Note**: For complete, runnable examples, check out the `examples/` directory in the repository:
> - `simple_interaction.rs` - Basic text generation
> - `stateful_interaction.rs` - Multi-turn conversations

### API Key

You'll need a Google API key with access to the Gemini models. You can get one from the [Google AI Studio](https://ai.dev/).

### Simple Interaction

```rust
use rust_genai::Client;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment variable
    let api_key = env::var("GEMINI_API_KEY")?;

    // Create the client
    let client = Client::builder(api_key).build();

    // Send an interaction and get response
    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Write a short poem about Rust programming.")
        .create()
        .await?;

    // Print the generated text
    println!("{}", response.text().unwrap_or("No response"));

    Ok(())
}
```

### Streaming Responses

```rust
use rust_genai::Client;
use futures_util::{pin_mut, StreamExt};
use std::{env, io::{stdout, Write}};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("GEMINI_API_KEY")?;
    let client = Client::builder(api_key).build();

    // Get a stream of response chunks
    let stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Explain quantum computing in simple terms.")
        .create_stream();
    pin_mut!(stream);

    // Process each chunk as it arrives
    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                if let Some(text) = chunk.text() {
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

### System Instructions

```rust
use rust_genai::Client;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("GEMINI_API_KEY")?;
    let client = Client::builder(api_key).build();

    // Send request with system instruction
    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_system_instruction("You are a helpful geography expert.")
        .with_text("What is the capital of France?")
        .create()
        .await?;

    println!("{}", response.text().unwrap_or("No response"));
    Ok(())
}
```

### Stateful Conversations

The Interactions API supports stateful conversations using `previous_interaction_id`. The server automatically maintains context:

```rust
use rust_genai::Client;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("GEMINI_API_KEY")?;
    let client = Client::builder(api_key).build();

    // First interaction - establish context
    let response1 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("My name is Alice and I live in New York.")
        .with_store(true)  // Store for later reference
        .create()
        .await?;

    println!("Response 1: {}", response1.text().unwrap_or(""));

    // Second interaction - reference the first
    let response2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_previous_interaction(&response1.id)  // Link to previous
        .with_text("What is my name and where do I live?")
        .create()
        .await?;

    // Model remembers: "Your name is Alice and you live in New York."
    println!("Response 2: {}", response2.text().unwrap_or(""));

    Ok(())
}
```

### Function Calling

For manual function calling with full control over execution:

```rust
use rust_genai::{Client, FunctionDeclaration, function_result_content, WithFunctionCalling};
use serde_json::json;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("GEMINI_API_KEY")?;
    let client = Client::builder(api_key).build();

    // Define a function using the builder
    let weather_function = FunctionDeclaration::builder("get_weather")
        .description("Get the current weather in a given location")
        .parameter("location", json!({
            "type": "string",
            "description": "The city and state, e.g. San Francisco, CA"
        }))
        .required(vec!["location".to_string()])
        .build();

    // First request with function declaration
    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's the weather like in London?")
        .with_function(weather_function.clone())
        .create()
        .await?;

    // Check if model wants to call a function
    let function_calls = response.function_calls();
    if !function_calls.is_empty() {
        let (call_id, name, args, _signature) = &function_calls[0];
        println!("Function call: {} with args: {}", name, args);

        // Execute your function logic here...
        let weather_result = json!({"temperature": "72°F", "conditions": "sunny"});

        // Send the result back using function_result_content
        let call_id = call_id.clone().expect("call_id required");
        let function_result = function_result_content(name, call_id, weather_result);

        let final_response = client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_previous_interaction(&response.id)
            .with_content(vec![function_result])
            .with_function(weather_function)
            .create()
            .await?;

        println!("{}", final_response.text().unwrap_or("No response"));
    }

    Ok(())
}
```

### Automatic Function Calling

The library supports automatic function discovery and execution. Functions decorated with `#[generate_function_declaration]` are automatically discovered and executed when the model requests them:

```rust
use rust_genai::{Client, CallableFunction, WithFunctionCalling};
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("GEMINI_API_KEY")?;
    let client = Client::builder(api_key).build();

    // Get the function declaration from the generated callable
    let weather_func = GetWeatherCallable.declaration();

    // Use create_with_auto_functions() to automatically handle function calls
    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's the weather in Tokyo?")
        .with_function(weather_func)
        .create_with_auto_functions()
        .await?;

    // The library automatically:
    // 1. Sends the function declaration to the model
    // 2. Executes the function when the model calls it
    // 3. Sends results back to the model
    // 4. Continues until the model provides a final response

    println!("{}", response.text().unwrap_or("No response"));
    Ok(())
}
```

Key features of automatic function calling:
- Functions are discovered at compile time using the `inventory` crate
- Both sync and async functions are supported
- The conversation loop handles multiple function calls automatically
- Error handling is built-in - function errors are reported back to the model
- Maximum of 5 conversation turns to prevent infinite loops

### Using the Procedural Macro

The `#[generate_function_declaration]` macro provides an ergonomic way to create function declarations:

```rust
use rust_genai_macros::generate_function_declaration;

/// Function to get the weather in a given location
#[generate_function_declaration(
    location(description = "The city and state, e.g. San Francisco, CA"),
    unit(description = "The temperature unit to use", enum_values = ["celsius", "fahrenheit"])
)]
fn get_weather(location: String, unit: Option<String>) -> String {
    format!("Weather for {}", location)
}

// The macro generates:
// - A `get_weather_declaration()` function returning FunctionDeclaration
// - A `GetWeatherCallable` struct implementing CallableFunction
// - Automatic registration in the global function registry
```

The macro supports:
- Automatic extraction of function doc comments as descriptions
- Parameter descriptions via the macro attribute
- Enum constraints for parameters with fixed values
- Proper handling of optional parameters (Option<T>)
- Type mapping for common Rust types (String, i32, i64, f32, f64, bool, Vec<T>, serde_json::Value)

## API Reference

### Client Builder

```rust
// Create a client with API key
let client = Client::builder(api_key).build();
```

For debug logging, see the [Logging](#logging) section below.

### InteractionBuilder Methods

The `InteractionBuilder` provides a fluent API for constructing requests:

```rust
client
    .interaction()                              // Start building an interaction
    .with_model("model-name")                   // Set the model (required unless using agent)
    .with_agent("agent-name")                   // Use an agent instead of model
    .with_text("prompt")                        // Set input as simple text
    .with_input(InteractionInput::...)          // Set complex input
    .with_content(vec![...])                    // Set content array directly
    .with_system_instruction("instruction")     // Set system instruction
    .with_previous_interaction("id")            // Link to previous interaction
    .with_function(func_decl)                   // Add a function declaration
    .with_functions(vec![...])                  // Add multiple function declarations
    .with_generation_config(config)             // Set generation parameters
    .with_response_modalities(vec![...])        // Set response modalities
    .with_response_format(schema)               // Set JSON schema for output
    .with_store(true)                           // Whether to store the interaction
    .with_background(false)                     // Run in background mode
    .create()                                   // Execute and get response
    .create_stream()                            // Get streaming response
    .create_with_auto_functions()               // Execute with auto function calling
```

### Response Types

```rust
// InteractionResponse provides convenience methods
impl InteractionResponse {
    fn text(&self) -> Option<String>;           // Get first text output
    fn all_text(&self) -> String;               // Concatenate all text outputs
    fn has_text(&self) -> bool;                 // Check if response has text
    fn has_function_calls(&self) -> bool;       // Check for function calls
    fn has_thoughts(&self) -> bool;             // Check for thought content
    fn function_calls(&self) -> Vec<(Option<String>, String, Value, Option<String>)>;
    // Returns: (call_id, name, args, thought_signature)
}
```

### Advanced: Function Registry and CallableFunction Trait

For advanced users who want to implement custom function handling:

```rust
use rust_genai::{CallableFunction, FunctionError, FunctionDeclaration};
use async_trait::async_trait;

pub struct MyFunction;

#[async_trait]
impl CallableFunction for MyFunction {
    fn declaration(&self) -> FunctionDeclaration {
        // Return function declaration
    }

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, FunctionError> {
        // Implement function logic
    }
}

// Functions marked with #[generate_function_declaration] are automatically
// registered in a global registry and discovered by create_with_auto_functions()
```

## Logging

The library uses the standard Rust `log` crate for structured logging. To see debug output, initialize a logging backend in your application:

```toml
# Add to your Cargo.toml
[dependencies]
env_logger = "0.11"  # or simplelog, tracing-subscriber, etc.
```

```rust
// Initialize logging in your application
fn main() {
    env_logger::init();
    // ... your code
}
```

Control log levels via the `RUST_LOG` environment variable:

```bash
# Show all debug logs from rust-genai
RUST_LOG=rust_genai=debug cargo run

# Show only warnings and errors
RUST_LOG=rust_genai=warn cargo run
```

The library logs request/response details, streaming events, and interaction lifecycle at the `debug` level.

## Project Structure

The project consists of three main components:

1. **Public API Crate (`rust-genai`)**:
   - Provides a user-friendly interface in `src/lib.rs`
   - Handles high-level error representation and client creation
   - Contains the `InteractionBuilder` for fluent API construction

2. **Internal Client (`genai-client/`)**:
   - Contains the low-level implementation for API communication
   - Defines the JSON serialization/deserialization models
   - Handles the actual HTTP requests and response parsing

3. **Macro Crate (`rust-genai-macros/`)**:
   - Procedural macro for `#[generate_function_declaration]`
   - Automatic function registration via `inventory` crate

Users of the `rust-genai` crate typically do not need to interact with `genai-client` directly, as its functionalities are exposed through the main `rust-genai` API.

## Available Models

This library has been tested with the following Google Generative AI models:

- `gemini-3-flash-preview`

For the latest available models, check the [Google AI documentation](https://ai.google.dev/models).

## Error Handling

The library provides comprehensive error handling with two main error types:

### GenaiError

The main error type for API interactions:

- `Http`: Network-related errors
- `Parse`: Issues parsing the response
- `Json`: JSON deserialization errors
- `Utf8`: Text encoding errors
- `Api`: Errors returned by the Google API
- `Internal`: Other internal errors
- `InvalidInput`: Validation errors (missing model, missing input, etc.)

### FunctionError

Specific to function calling:

- `ArgumentMismatch`: When function arguments don't match the expected schema
- `ExecutionError`: When a function fails during execution

```rust
use rust_genai::FunctionError;

// Example of handling function errors
match result {
    Err(FunctionError::ArgumentMismatch(msg)) => {
        eprintln!("Invalid arguments: {}", msg);
    }
    Err(FunctionError::ExecutionError(msg)) => {
        eprintln!("Function failed: {}", msg);
    }
    Ok(value) => {
        // Process result
    }
}
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

The MIT License is a permissive license that is short and to the point. It lets people do anything they want with your code as long as they provide attribution back to you and don't hold you liable.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
