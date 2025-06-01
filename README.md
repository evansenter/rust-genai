# Rust GenAI

A Rust client library for interacting with Google's Generative AI (Gemini) API.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
  - [API Key](#api-key)
  - [Simple Request Example](#simple-request-example)
  - [Streaming Request Example](#streaming-request-example)
  - [Using System Instructions](#using-system-instructions)
  - [Function Calling Example](#function-calling-example)
  - [Manual Function Calling with Multi-Turn Conversations](#manual-function-calling-with-multi-turn-conversations)
  - [Helper Functions for Building Conversations](#helper-functions-for-building-conversations)
  - [Automatic Function Calling](#automatic-function-calling)
  - [Using the Procedural Macro for Function Declarations](#using-the-procedural-macro-for-function-declarations)
  - [Code Execution](#code-execution)
- [API Reference](#api-reference)
- [Project Structure](#project-structure)
- [Available Models](#available-models)
- [Error Handling](#error-handling)
- [License](#license)
- [Contributing](#contributing)

## Features

- Simple, intuitive API for making requests to Google's Generative AI models
- Support for both single-shot and streaming text generation
- Function calling support with both manual and automatic execution
- Automatic function discovery at compile time using procedural macros
- Multi-turn conversation handling with function execution
- Helper functions for building complex conversations
- Comprehensive error handling with detailed error types
- Async/await support with Tokio
- Type-safe function argument handling with serde
- Support for both synchronous and asynchronous functions
- Well-documented code with examples

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rust-genai = "0.1.0"
rust-genai-macros = "0.1.0"  # Only if using the procedural macros
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
> - `simple_request.rs` - Basic text generation
> - `stream_request.rs` - Streaming responses
> - `function_call.rs` - Both manual and automatic function calling
> - `code_execution.rs` - Code execution in Python

### API Key

You'll need a Google API key with access to the Gemini models. You can get one from the [Google AI Studio](https://ai.dev/).

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
        .generate_stream()?;
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

### Manual Function Calling with Multi-Turn Conversations

For more control over function execution, you can manually handle function calls and build multi-turn conversations:

```rust
use rust_genai::{Client, model_function_calls_request, user_text, user_tool_response};
use rust_genai_macros::generate_function_declaration;
use serde_json::json;
use std::env;

#[generate_function_declaration(
    location(description = "The city and state, e.g. San Francisco, CA")
)]
fn get_weather(location: String) -> String {
    format!("The weather in {} is sunny and 72°F", location)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("GEMINI_API_KEY")?;
    let client = Client::builder(api_key).build();
    
    let prompt = "What's the weather in London?";
    let weather_func = get_weather_declaration();
    
    // First request with function available
    let response = client
        .with_model("gemini-2.5-flash-preview-05-20")
        .with_prompt(prompt)
        .with_function(weather_func.clone())
        .generate()
        .await?;
    
    // Check if the model wants to call a function
    if let Some(function_calls) = response.function_calls {
        for call in &function_calls {
            if call.name == "get_weather" {
                // Extract arguments and execute function
                let location = call.args.get("location")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let result = get_weather(location.to_string());
                
                // Build conversation history
                let contents = vec![
                    user_text(prompt.to_string()),
                    model_function_calls_request(function_calls.clone()),
                    user_tool_response(call.name.clone(), json!({ "result": result })),
                ];
                
                // Send function result back to model
                let final_response = client
                    .with_model("gemini-2.5-flash-preview-05-20")
                    .with_contents(contents)
                    .with_function(weather_func)
                    .generate()
                    .await?;
                
                println!("{}", final_response.text.unwrap_or_default());
            }
        }
    }
    
    Ok(())
}
```

### Helper Functions for Building Conversations

The library provides several helper functions for building complex conversation histories:

```rust
use rust_genai::{user_text, model_text, model_function_call, user_tool_response};
use serde_json::json;

// Create user messages
let user_msg = user_text("Hello, how are you?".to_string());

// Create model text responses
let model_msg = model_text("I'm doing well, thank you!".to_string());

// Record function calls made by the model
let function_call = model_function_call(
    "get_weather".to_string(),
    json!({ "location": "New York" })
);

// Create tool/function responses
let tool_response = user_tool_response(
    "get_weather".to_string(),
    json!({ "temperature": "72°F", "condition": "sunny" })
);

// Use these to build complex conversations
let contents = vec![
    user_text("What's the weather?".to_string()),
    model_function_call("get_weather".to_string(), json!({"location": "NYC"})),
    user_tool_response("get_weather".to_string(), json!({"temp": "72°F"})),
    model_text("The weather in NYC is 72°F.".to_string()),
    user_text("Thanks!".to_string()),
];

let response = client
    .with_model("gemini-2.5-flash-preview-05-20")
    .with_contents(contents)
    .generate()
    .await?;
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

### Code Execution

The library supports code execution, allowing the model to run Python code and return results:

```rust
use rust_genai::Client;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("GEMINI_API_KEY")?;
    let client = Client::builder(api_key).build();
    
    let response = client
        .with_model("gemini-2.5-flash-preview-05-20")
        .with_prompt("Calculate the factorial of 7 using Python")
        .with_code_execution()  // Enable code execution
        .generate()
        .await?;
    
    // Handle the response
    if let Some(text) = response.text {
        println!("Model says: {}", text);
    }
    
    if let Some(code_results) = response.code_execution_results {
        for result in code_results {
            println!("Executed code:\n{}", result.code);
            println!("Output:\n{}", result.output);
        }
    }
    
    Ok(())
}
```

## API Reference

### Client Builder

```rust
// Create a client with API key
let client = Client::builder(api_key).build();

// Create a client with debug logging enabled
let client = Client::builder(api_key).debug().build();
```

### Request Builder Methods

The `GenerateContentBuilder` provides a fluent API for constructing requests:

```rust
client
    .with_model("model-name")              // Set the model (required)
    .with_prompt("prompt")                  // Set initial prompt text
    .with_initial_user_text("text")         // Alternative to with_prompt
    .with_system_instruction("instruction") // Set system instruction
    .with_contents(vec![...])               // Set full conversation history
    .with_function(func_decl)               // Add a function declaration
    .with_functions(vec![...])              // Add multiple function declarations
    .with_code_execution()                  // Enable code execution
    .with_tool_config(config)               // Set tool configuration
    .generate()                             // Execute and get response
    .generate_stream()                      // Get streaming response
    .generate_with_auto_functions()         // Execute with automatic function calling
```

### Response Types

```rust
// GenerateContentResponse
pub struct GenerateContentResponse {
    pub text: Option<String>,
    pub function_calls: Option<Vec<FunctionCall>>,
    pub code_execution_results: Option<Vec<CodeExecutionResult>>,
}

// FunctionCall
pub struct FunctionCall {
    pub name: String,
    pub args: serde_json::Value,
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
// registered in a global registry and discovered by generate_with_auto_functions()
```

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

This library has been tested with the following Google Generative AI models:

- `gemini-2.5-flash-preview-05-20`

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

[Add your license information here]

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. 