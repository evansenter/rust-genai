# Rust GenAI

A Rust client library for interacting with Google's Generative AI (Gemini) API using the Interactions API.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
  - [Simple Interaction](#simple-interaction)
  - [Streaming Responses](#streaming-responses)
  - [System Instructions](#system-instructions)
  - [Stateful Conversations](#stateful-conversations)
  - [Function Calling](#function-calling)
  - [Automatic Function Calling](#automatic-function-calling)
  - [Built-in Tools](#built-in-tools)
  - [Thinking Mode](#thinking-mode)
  - [Multimodal Input](#multimodal-input)
- [API Reference](#api-reference)
- [Project Structure](#project-structure)
- [Error Handling](#error-handling)
- [Logging](#logging)
- [Troubleshooting](#troubleshooting)
- [Claude Code Integration](#claude-code-integration)
- [License](#license)
- [Contributing](#contributing)

## Features

- Simple, intuitive API for making requests to Google's Generative AI models
- Support for both single-shot and streaming interactions
- Stateful conversations with automatic context management via `previous_interaction_id`
- Function calling with both manual and automatic execution
- Automatic function discovery at compile time using procedural macros
- Structured output with JSON schema enforcement via `with_response_format()`
- Built-in tool support: Google Search grounding, URL context, and code execution
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

- Rust 1.85 or later (edition 2024)
- A Google AI API key with access to Gemini models (get one from [Google AI Studio](https://ai.dev/))

## Usage

> **Note**: For complete, runnable examples, check out the `examples/` directory:
> - `simple_interaction.rs` - Basic text generation
> - `streaming.rs` - Real-time streaming responses
> - `system_instructions.rs` - Custom system prompts
> - `stateful_interaction.rs` - Multi-turn conversations
> - `auto_function_calling.rs` - Automatic function execution
> - `structured_output.rs` - JSON schema enforcement
> - `google_search.rs` - Web search grounding
> - `code_execution.rs` - Python code execution
> - `url_context.rs` - URL content fetching
> - `thinking.rs` - Reasoning with thought content
> - `multimodal_image.rs` - Image input handling
> - `audio_input.rs` - Audio input handling
> - `video_input.rs` - Video input handling
> - `pdf_input.rs` - PDF document processing
> - `image_generation.rs` - Image generation

You'll need a Google API key from [Google AI Studio](https://ai.dev/).

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
        let call = &function_calls[0];
        println!("Function call: {} with args: {}", call.name, call.args);

        // Execute your function logic here...
        let weather_result = json!({"temperature": "72°F", "conditions": "sunny"});

        // Send the result back using function_result_content
        let call_id = call.id.expect("call_id required");
        let function_result = function_result_content(call.name, call_id, weather_result);

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

The library supports automatic function discovery and execution. Functions decorated with `#[tool]` are automatically discovered and executed when the model requests them:

```rust
use rust_genai::{Client, CallableFunction, WithFunctionCalling};
use rust_genai_macros::tool;
use std::env;

/// Get the current weather in a location
#[tool(
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
    let result = client
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

    // Access the final response and execution history
    println!("{}", result.response.text().unwrap_or("No response"));

    // You can also inspect which functions were called
    for exec in &result.executions {
        println!("Called {} -> {}", exec.name, exec.result);
    }
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

The `#[tool]` macro provides an ergonomic way to create function declarations:

```rust
use rust_genai_macros::tool;

/// Function to get the weather in a given location
#[tool(
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

### Built-in Tools

The library supports Gemini's built-in tools for enhanced capabilities:

```rust
// Google Search grounding - real-time web search
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_google_search()
    .with_text("What are today's top news stories?")
    .create()
    .await?;

// Access grounding metadata
if let Some(metadata) = response.google_search_metadata() {
    for chunk in &metadata.grounding_chunks {
        println!("Source: {:?}", chunk.web);
    }
}

// Code execution - run Python in a sandbox
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_code_execution()
    .with_text("Calculate the first 10 Fibonacci numbers")
    .create()
    .await?;

// Get successful output
if let Some(output) = response.successful_code_output() {
    println!("Result: {}", output);
}

// URL context - fetch and analyze web content
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_url_context()
    .with_text("Summarize https://example.com")
    .create()
    .await?;
```

### Thinking Mode

Enable reasoning/thinking for complex problem-solving:

```rust
use rust_genai::ThinkingLevel;

let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_thinking_level(ThinkingLevel::Medium)
    .with_text("Solve this step by step: If a train travels 120 km in 2 hours...")
    .create()
    .await?;

// Access the model's reasoning process
for thought in response.thoughts() {
    println!("Thinking: {}", thought);
}

// Get the final answer
println!("Answer: {}", response.text().unwrap_or(""));
```

### Multimodal Input

Send images, audio, video, and documents to the model for analysis:

```rust
use rust_genai::Client;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("GEMINI_API_KEY")?;
    let client = Client::builder(api_key).build();

    // Method 1: Fluent builder pattern (recommended)
    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's in this image?")
        .add_image_file("photo.jpg").await?  // Auto-detects MIME type
        .create()
        .await?;

    println!("{}", response.text().unwrap_or("No response"));

    // Add multiple media files
    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Compare these two images")
        .add_image_file("photo1.jpg").await?
        .add_image_file("photo2.png").await?
        .create()
        .await?;

    // Use base64 data directly
    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Describe this image")
        .add_image_data(base64_encoded_image, "image/jpeg")
        .create()
        .await?;

    Ok(())
}
```

All media types use the same pattern:
- **Images**: `add_image_file()`, `add_image_data()`, `add_image_uri()`
- **Audio**: `add_audio_file()`, `add_audio_data()`, `add_audio_uri()`
- **Video**: `add_video_file()`, `add_video_data()`, `add_video_uri()`
- **Documents**: `add_document_file()`, `add_document_data()`, `add_document_uri()`

For programmatic content building, use the helper functions:

```rust
use rust_genai::{image_from_file, text_content};

// Load files with automatic MIME detection
let image = image_from_file("photo.jpg").await?;

let contents = vec![
    text_content("Analyze this image"),
    image,
];

let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_content(contents)
    .create()
    .await?;
```

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
    .with_google_search()                       // Enable Google Search grounding
    .with_code_execution()                      // Enable Python code execution
    .with_url_context()                         // Enable URL content fetching
    .with_thinking_level(ThinkingLevel::Medium) // Enable reasoning mode
    .with_generation_config(config)             // Set generation parameters
    .with_response_modalities(vec![...])        // Set response modalities
    .with_response_format(schema)               // Set JSON schema for output
    .with_store(true)                           // Whether to store the interaction
    .with_background(false)                     // Run in background mode
    // Multimodal content (accumulate, don't replace)
    .add_image_file("path").await?              // Add image from file
    .add_image_data(base64, "image/png")        // Add image from base64
    .add_image_uri(uri, "image/jpeg")           // Add image from URI
    .add_audio_file("path").await?              // Add audio from file
    .add_video_file("path").await?              // Add video from file
    .add_document_file("path").await?           // Add document from file
    // Execute
    .create()                                   // Execute and get response
    .create_stream()                            // Get streaming response
    .create_with_auto_functions()               // Execute with auto function calling
```

### Response Types

```rust
// InteractionResponse provides convenience methods
impl InteractionResponse {
    // Text content
    fn text(&self) -> Option<&str>;             // Get first text output
    fn all_text(&self) -> String;               // Concatenate all text outputs
    fn has_text(&self) -> bool;

    // Function calling
    fn function_calls(&self) -> Vec<FunctionCallInfo>;
    fn function_results(&self) -> Vec<FunctionResultInfo>;
    fn has_function_calls(&self) -> bool;
    fn has_function_results(&self) -> bool;

    // Thinking/reasoning
    fn thoughts(&self) -> impl Iterator<Item = &str>;
    fn has_thoughts(&self) -> bool;

    // Built-in tools
    fn google_search_metadata(&self) -> Option<&GroundingMetadata>;
    fn code_execution_results(&self) -> Vec<(CodeExecutionOutcome, &str)>;
    fn url_context_metadata(&self) -> Option<&UrlContextMetadata>;
}

// Function call/result info structs with named fields
pub struct FunctionCallInfo<'a> {
    pub id: Option<&'a str>,                // Call ID for sending results back
    pub name: &'a str,                      // Function name
    pub args: &'a Value,                    // Function arguments
    pub thought_signature: Option<&'a str>, // For reasoning continuity
}

pub struct FunctionResultInfo<'a> {
    pub name: &'a str,                      // Function name
    pub call_id: &'a str,                   // Matches the call's ID
    pub result: &'a Value,                  // Function result
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

// Functions marked with #[tool] are automatically registered in a global
// registry and discovered by create_with_auto_functions()
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
   - Procedural macro for `#[tool]`
   - Automatic function registration via `inventory` crate

Users of the `rust-genai` crate typically do not need to interact with `genai-client` directly, as its functionalities are exposed through the main `rust-genai` API.

## Forward Compatibility

This library follows the [Evergreen spec](https://github.com/google-deepmind/evergreen-spec) philosophy for graceful API evolution:

- **Unknown types are preserved, not rejected**: When the API returns content types this library doesn't recognize yet, they're captured in `Unknown` variants rather than causing deserialization errors.
- **Non-exhaustive enums**: Key types like `InteractionContent` and `Tool` use `#[non_exhaustive]`, so your match statements should include wildcard arms.
- **Roundtrip preservation**: Unknown content can be serialized back without data loss - the `Unknown` variants store both the type name and full JSON data.

```rust
// Handle unknown content gracefully
for output in response.outputs {
    match output {
        InteractionContent::Text { text } => println!("{}", text.unwrap_or_default()),
        InteractionContent::Unknown { type_name, data } => {
            log::warn!("Unknown content type '{}': {:?}", type_name, data);
        }
        _ => {} // Future variants
    }
}
```

**Design principle**: All `Unknown` variants use a data-preserving pattern with `type_name: String` and `data: serde_json::Value` fields. This ensures you can always inspect what the API sent and roundtrip serialize it. See [CLAUDE.md](CLAUDE.md) for implementation details.

For strict validation during development, enable the `strict-unknown` feature flag - unknown types will error instead of being captured.

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

## Troubleshooting

### Common Issues

**"API key not valid" error**
- Ensure `GEMINI_API_KEY` environment variable is set correctly
- Verify your API key has access to the Gemini models at [Google AI Studio](https://ai.dev/)

**"Model not found" error**
- Check that you're using a valid model name (e.g., `gemini-3-flash-preview`)
- Some models may require specific API access or be in preview

**Function calls not being executed**
- Ensure you're using `create_with_auto_functions()` for automatic execution
- For manual execution, check that you're sending the `FunctionResult` back correctly
- Verify your function is registered via `#[tool]`

**Image URL not accessible**
- The API blocks most public HTTP URLs for security
- Use Google Cloud Storage URLs (`gs://...`) or base64-encoded images
- See `image_data_content()` for base64 encoding

**Rate limiting errors**
- The free tier has request limits; wait and retry
- Consider adding delays between requests in batch operations

## Claude Code Integration

This project includes configuration for [Claude Code](https://claude.ai/code) to assist with development:

- **Session initialization**: Displays recent PRs, open issues, and build status
- **PR feedback processing**: Automated review comment handling with critical thinking
- **Quality gates**: Automatic formatting and linting checks

See `CLAUDE.md` for detailed guidance and `.claude/` for configuration files.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
