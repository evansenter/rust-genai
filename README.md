# Rust GenAI

A Rust client library for interacting with Google's Generative AI (Gemini) API using the Interactions API.

## Features

- Simple, intuitive API for making requests to Google's Generative AI models
- Support for both single-shot and streaming interactions
- Stateful conversations with automatic context management via `previous_interaction_id`
- Function calling with both manual and automatic execution
- Automatic function discovery at compile time using procedural macros
- Structured output with JSON schema enforcement via `with_response_format()`
- Built-in tool support: Google Search grounding, URL context, and code execution
- Comprehensive error handling with detailed error types
- Async/await support with Tokio

## External Documentation

For authoritative Gemini API documentation, consult these sources:

| Document | Description |
|----------|-------------|
| [Interactions API Reference](https://ai.google.dev/static/api/interactions.md.txt) | API specification and endpoint details |
| [Interactions API Guide](https://ai.google.dev/static/api/interactions-api.md.txt) | Usage patterns and best practices |
| [Function Calling Guide](https://ai.google.dev/gemini-api/docs/function-calling.md.txt) | Function declaration and execution |
| [Thought Signatures](https://ai.google.dev/gemini-api/docs/thought-signatures.md.txt) | Reasoning and thought content |

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rust-genai = "0.3.0"
rust-genai-macros = "0.3.0"  # Only if using the procedural macros
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
futures-util = "0.3"  # Only if using streaming responses
```

### Prerequisites

- Rust 1.85 or later (edition 2024)
- A Google AI API key with access to Gemini models (get one from [Google AI Studio](https://ai.dev/))

## Usage

See `examples/` for complete, runnable examples covering all features.

### Simple Interaction

```rust
use rust_genai::Client;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("GEMINI_API_KEY")?;
    let client = Client::builder(api_key).build()?;

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Write a short poem about Rust programming.")
        .create()
        .await?;

    println!("{}", response.text().unwrap_or("No response"));
    Ok(())
}
```

### Streaming Responses

```rust
use rust_genai::Client;
use futures_util::StreamExt;

let stream = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Explain quantum computing.")
    .create_stream();

futures_util::pin_mut!(stream);
while let Some(result) = stream.next().await {
    if let Ok(chunk) = result {
        if let Some(text) = chunk.text() {
            print!("{}", text);
        }
    }
}
```

See [`docs/STREAMING_API.md`](docs/STREAMING_API.md) for stream types, resume capability, and patterns.

### Stateful Conversations

```rust
// First turn - set system instruction
let response1 = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("My name is Alice.")
    .with_system_instruction("You are a helpful assistant.")
    .with_store_enabled()
    .create()
    .await?;

// Second turn - chain with previous
let response2 = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_previous_interaction(&response1.id)
    .with_text("What is my name?")  // Model remembers: "Alice"
    .create()
    .await?;
```

**Key inheritance rules:**
- `systemInstruction`: Inherited (only send on first turn)
- `tools`: NOT inherited (must resend on each user message turn)

See [`docs/MULTI_TURN_FUNCTION_CALLING.md`](docs/MULTI_TURN_FUNCTION_CALLING.md) for comprehensive patterns.

### Function Calling

Three approaches for client-side function calling:

| Approach | State | Best For |
|----------|-------|----------|
| `#[tool]` macro | Stateless | Simple tools, quick prototyping |
| `ToolService` | Stateful | DB connections, API clients, shared config |
| Manual | Flexible | Custom execution logic, rate limiting |

#### Automatic with `#[tool]` Macro

```rust
use rust_genai_macros::tool;

/// Get the weather in a location
#[tool(location(description = "City and state, e.g. San Francisco, CA"))]
fn get_weather(location: String) -> String {
    format!("Weather in {}: 72°F, sunny", location)
}

let result = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("What's the weather in Tokyo?")
    .with_function(GetWeatherCallable.declaration())
    .create_with_auto_functions()
    .await?;

println!("{}", result.response.text().unwrap());
```

#### Stateful with ToolService

For tools that need shared state (database pools, API clients, configuration):

```rust
use rust_genai::{ToolService, CallableFunction};

struct MyService {
    db: Arc<DatabasePool>,
}

impl ToolService for MyService {
    fn tools(&self) -> Vec<Arc<dyn CallableFunction>> {
        vec![Arc::new(DbQueryTool { pool: self.db.clone() })]
    }
}

let service = Arc::new(MyService { db });
let result = client.interaction()
    .with_tool_service(service)
    .create_with_auto_functions()
    .await?;
```

See `examples/tool_service.rs` for a complete example.

### Built-in Tools

```rust
// Google Search grounding
let response = client.interaction()
    .with_google_search()
    .with_text("What are today's top news stories?")
    .create().await?;

// Code execution (Python sandbox)
let response = client.interaction()
    .with_code_execution()
    .with_text("Calculate the first 10 Fibonacci numbers")
    .create().await?;

// URL context
let response = client.interaction()
    .with_url_context()
    .with_text("Summarize https://example.com")
    .create().await?;
```

### Thinking Mode

```rust
use rust_genai::ThinkingLevel;

let response = client.interaction()
    .with_thinking_level(ThinkingLevel::Medium)
    .with_text("Solve step by step: If a train travels 120 km in 2 hours...")
    .create().await?;

for thought in response.thoughts() {
    println!("Thinking: {}", thought);
}
println!("Answer: {}", response.text().unwrap());
```

### Multimodal Input

```rust
// Add images, audio, video, or documents
let response = client.interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("What's in this image?")
    .add_image_file("photo.jpg").await?
    .create().await?;
```

All media types follow the same pattern: `add_*_file()`, `add_*_data()`, `add_*_uri()`.

## Logging

Enable debug logging:

```bash
RUST_LOG=rust_genai=debug cargo run --example simple_interaction
```

For wire-level API debugging without configuring a logging backend:

```bash
LOUD_WIRE=1 cargo run --example simple_interaction
```

See [`docs/LOGGING_STRATEGY.md`](docs/LOGGING_STRATEGY.md) for details on log levels and `LOUD_WIRE` output.

## Project Structure

- **`rust-genai`** (root): Public API crate with `Client`, `InteractionBuilder`, HTTP layer (`src/http/`), and type modules
- **`rust-genai-macros/`**: Procedural macro for `#[tool]`

## Forward Compatibility

This library follows the [Evergreen spec](https://github.com/google-deepmind/evergreen-spec) philosophy: unknown API types deserialize into `Unknown` variants instead of failing. Always include wildcard arms in match statements:

```rust
match output {
    InteractionContent::Text { text } => println!("{}", text.unwrap_or_default()),
    _ => {}  // Handle unknown future variants
}
```

## Error Handling

Two main error types:

- **`GenaiError`**: API/network errors (Http, Parse, Json, Api, InvalidInput)
- **`FunctionError`**: Function calling errors (ArgumentMismatch, ExecutionError)

## Troubleshooting

- **"API key not valid"**: Verify `GEMINI_API_KEY` is set correctly
- **"Model not found"**: Use valid model name (e.g., `gemini-3-flash-preview`)
- **Functions not executing**: Use `create_with_auto_functions()` for automatic execution
- **Image URL blocked**: Use Google Cloud Storage URLs or base64-encoded images

## Testing

```bash
cargo test -- --include-ignored  # Full test suite (requires GEMINI_API_KEY)
cargo test                       # Unit tests only
```

See [CLAUDE.md](CLAUDE.md) for test assertion strategies and development guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions welcome! See [CLAUDE.md](CLAUDE.md) for development guidelines.
