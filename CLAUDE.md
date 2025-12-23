# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`rust-genai` is a Rust client library for Google's Generative AI (Gemini) API. The project supports both the **GenerateContent API** (legacy) and the **Interactions API** (unified interface for models and agents).

The project is structured as a Cargo workspace with three crates:

- **`rust-genai`** (root): Public API crate that provides the user-facing interface
- **`genai-client/`**: Internal low-level client that handles HTTP communication, JSON serialization/deserialization, and raw API interactions
- **`rust-genai-macros/`**: Procedural macro crate for generating function declarations from Rust functions

## Common Development Commands

### Building and Testing

**IMPORTANT**: By default, always run tests with `cargo test -- --include-ignored` to ensure full end-to-end testing including integration tests that require the `GEMINI_API_KEY` environment variable.

```bash
# Build all workspace members
cargo build

# Build in release mode
cargo build --release

# Run all tests including ignored integration tests (DEFAULT - use this)
cargo test -- --include-ignored

# Run only non-ignored tests
cargo test

# Run tests for a specific test file
cargo test --test integration_tests
cargo test --test macro_tests

# Run only unit tests (no integration tests)
cargo test --lib

# Run a single test by name
cargo test test_name

# Run tests with output visible
cargo test -- --nocapture
```

### Running Examples

All examples require the `GEMINI_API_KEY` environment variable:

```bash
# GenerateContent API examples
cargo run --example simple_request
cargo run --example stream_request
cargo run --example function_call
cargo run --example code_execution

# Interactions API examples
cargo run --example simple_interaction
cargo run --example stateful_interaction
```

### Linting and Formatting

```bash
# Format code
cargo fmt

# Check formatting without making changes
cargo fmt -- --check

# Run clippy for linting (comprehensive check across workspace)
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Architecture

### Layered Architecture

The codebase follows a layered architecture where each layer has distinct responsibilities:

1. **Public API Layer** (`src/lib.rs`, `src/client.rs`, `src/request_builder.rs`):
   - Exposes the user-facing `Client` and `GenerateContentBuilder` types
   - Converts internal errors (`genai_client::InternalError`) to public errors (`GenaiError`)
   - Provides high-level abstractions like automatic function calling

2. **Internal Logic Layer** (`src/internal/`, `src/function_calling.rs`, `src/content_api.rs`):
   - Response processing and streaming logic
   - Function calling registry and execution system using the `inventory` crate
   - Helper functions for building conversation content

3. **HTTP Client Layer** (`genai-client/`):
   - Raw HTTP requests to Google's Generative AI API
   - JSON models for request/response serialization:
     - `models/request.rs` & `models/response.rs`: GenerateContent API
     - `models/interactions.rs`: Interactions API (flat content structure with type tags)
     - `models/shared.rs`: Shared types used by both APIs
   - Error handling for network and API errors
   - SSE (Server-Sent Events) streaming support
   - Endpoint abstraction in `common.rs` for flexible URL construction

4. **Macro Layer** (`rust-genai-macros/`):
   - Procedural macro `#[generate_function_declaration]` for automatic function discovery
   - Generates FunctionDeclaration from Rust function signatures
   - Registers functions in global inventory for automatic execution

### Key Architectural Patterns

**Builder Pattern**: The library uses a fluent builder API throughout:
- `Client::builder(api_key).debug().build()` for client creation
- `client.with_model(...).with_prompt(...).generate()` for requests
- This pattern is implemented in `src/client.rs` and `src/request_builder.rs`

**Function Calling System**: The library supports three levels of function calling:
1. **Manual**: User explicitly passes `FunctionDeclaration` and handles function calls
2. **Semi-automatic**: Macro generates declarations, but user controls execution
3. **Fully automatic**: `generate_with_auto_functions()` discovers and executes functions automatically using the `inventory` crate

The function calling system is implemented across:
- `src/function_calling.rs`: Core traits (`CallableFunction`) and registry
- `rust-genai-macros/src/lib.rs`: Procedural macro for function declaration generation
- `src/request_builder.rs`: The `generate_with_auto_functions()` method that orchestrates automatic execution

**Streaming Architecture**: SSE streaming is implemented using:
- `async-stream` for async generators
- `futures-util::Stream` trait for composable streaming
- Response chunking and text aggregation in `src/internal/response_processing.rs`

### Error Handling Strategy

The library uses two distinct error types:
- `GenaiError`: For API and network errors (thiserror-based)
- `FunctionError`: Specific to function execution errors

Internal errors from `genai-client` are converted to public `GenaiError` variants via `From` trait implementation in `src/lib.rs:43`.

## Important Implementation Details

### Content API Helper Functions

The `content_api` module (`src/content_api.rs`) provides builder functions for constructing multi-turn conversations:
- `user_text()`: Create user messages
- `model_text()`: Create model responses
- `model_function_call()` / `model_function_calls_request()`: Record function calls
- `user_tool_response()`: Send function results back to model

These are essential for implementing multi-turn conversations with function calling.

### Gemini 3 Thought Signatures

Gemini 3 models require **thought signatures** for multi-turn function calling. Without these signatures, Gemini 3 returns 400 errors when receiving function call responses.

**What are thought signatures?**
- Opaque encrypted tokens that represent the model's internal reasoning state
- Generated by Gemini 3 during responses that include function calls
- Must be preserved and passed back unchanged in subsequent conversation turns

**Implementation:**
- Extract from `GenerateContentResponse.thought_signatures` field
- Pass to `model_function_calls_request_with_signatures()` when building conversation history
- For Interactions API, use `function_call_content_with_signature()`

**Example workflow:**
```rust
// Step 1: Initial request with function declaration
let response = client
    .with_model("gemini-3-flash-preview")
    .with_prompt("What's the weather in Tokyo?")
    .with_function(weather_fn)
    .generate()
    .await?;

// Step 2: Extract thought signatures (critical for Gemini 3!)
let signatures = response.thought_signatures.clone();

// Step 3: Build conversation history WITH signatures
let contents = vec![
    user_text("What's the weather in Tokyo?"),
    model_function_calls_request_with_signatures(calls, signatures),
    user_tool_response("get_weather", json!({"temp": "22°C"})),
];

// Step 4: Continue conversation
let response2 = client
    .with_model("gemini-3-flash-preview")
    .with_contents(contents)
    .with_function(weather_fn)
    .generate()
    .await?;
```

**See:** `examples/gemini3_thought_signatures.rs` for a complete working example.

**Note:** This is only required for Gemini 3 models. Earlier Gemini models (1.5, 2.0) don't use thought signatures.

### Interactions API Implementation

The Interactions API provides a unified interface for both models and agents. Key implementation details:

**Client Methods** (`src/client.rs`):
- `create_interaction()`: Non-streaming interaction creation
- `create_interaction_stream()`: Streaming with SSE for real-time updates
- `get_interaction()`: Retrieve interaction by ID
- `delete_interaction()`: Remove interaction from server

**Core Functions** (`genai-client/src/interactions.rs`):
- HTTP client functions that handle the underlying API requests
- Use the `Endpoint` abstraction for URL construction
- Support for stateful conversations via `previous_interaction_id`

**Content Structure** (`genai-client/src/models/interactions.rs`):
- Uses flat `InteractionContent` enum with type-tagged variants (Text, Thought, Image, Audio, Video, FunctionCall, FunctionResponse)
- Different from GenerateContent API which uses nested `Content` with `parts` arrays
- Fields are often optional as API doesn't always return all data

**Stateful Conversations**:
- Pass `previous_interaction_id` to reference earlier interactions
- Server maintains conversation context automatically
- Enables implicit caching for improved performance and reduced costs

### Workspace Member Relationships

- The root crate (`rust-genai`) depends on both `genai-client` and `rust-genai-macros`
- `genai-client` is completely independent and could theoretically be used standalone
- `rust-genai-macros` uses the `inventory` crate to register functions at compile time
- Functions marked with `#[generate_function_declaration]` are automatically collected via `inventory::collect!` and can be discovered at runtime

### Test Organization

Tests are organized into two categories:
- **Unit tests**: Inline with source code (e.g., `src/lib.rs:55-134`)
- **Integration tests**: In `tests/` directory, each file tests a specific feature:
  - `integration_tests.rs`: GenerateContent API workflow tests (requires API key)
  - `interactions_tests.rs`: Interactions API tests (requires API key)
  - `macro_tests.rs`: Procedural macro functionality
  - `function_calling_tests.rs`: Function execution system
  - `content_api_tests.rs`: Conversation helper functions
  - `auto_function_tests.rs`: Auto-function execution tests (requires HTTP mocking)
  - `content_api_edge_cases.rs`: Edge cases for content API helper functions
  - `interaction_builder_edge_cases.rs`: InteractionBuilder edge cases and validation tests

Integration tests that require a real API key use `#[ignore]` attribute and must be run with `cargo test -- --ignored`.

## API Version Support

The library supports different API versions via the `ApiVersion` enum in `genai-client`:
- Currently defaults to `V1Beta`
- API version affects URL construction in `genai-client/src/core.rs`

## Claude Code Configuration

This project has Claude Code hooks and skills configured in `.claude/` to streamline development workflow.

### Hooks

Hooks are automatically executed at specific points during development:

**PostToolUse Hook (Auto-Format)**:
- Automatically runs `cargo fmt` on any Rust file after editing
- Ensures consistent code formatting without manual intervention
- Configured to run quietly and not fail the workflow

**SessionStart Hook (Environment Check)**:
- Runs at the start of each Claude Code session
- Verifies `GEMINI_API_KEY` is set for integration tests
- Checks if the project builds successfully
- Located at: `.claude/hooks/session_init.sh`

**Stop Hook (Pre-Push Validation)**:
- Automatically runs before considering work complete
- Matches CI checks exactly to catch issues before pushing
- Runs: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo check`, unit tests
- Ensures all changes will pass CI before being pushed
- Located at: `.claude/hooks/stop.sh`

### Skills

Skills provide reusable workflows that are automatically invoked by Claude Code when relevant to your request:

**`test-full` skill** (auto-invoked when you ask to run tests):
- Runs complete test suite: `cargo test --all -- --include-ignored --nocapture`
- Includes integration tests that require `GEMINI_API_KEY`
- Shows full test output for debugging
- Example trigger: "Can you run the full test suite?"

**`review-workspace` skill** (auto-invoked when you ask for a health check):
- Comprehensive workspace health check
- Runs: `cargo check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, unit tests
- Shows recent git commits and workspace summary
- Useful before commits or when starting work
- Example trigger: "Can you review the workspace health?"

**`check-docs` skill** (auto-invoked when you ask about documentation):
- Builds documentation for all workspace crates
- Verifies no documentation warnings exist
- Checks for missing docs, broken links, and invalid code examples
- Example trigger: "Can you check the documentation?"

**`run-examples` skill** (auto-invoked when you ask to run examples):
- Runs all 6 example programs to verify they work with current API
- Requires `GEMINI_API_KEY` environment variable
- Useful for catching API breaking changes
- Example trigger: "Can you run all the examples?"

### Configuration Files

- `.claude/settings.json`: Main configuration with hooks and permissions
- `.claude/hooks/session_init.sh`: Session initialization script
- `.claude/skills/test-full.yaml` & `test-full.sh`: Full test suite skill
- `.claude/skills/review-workspace.yaml` & `review-workspace.sh`: Health check skill

**Note**: Changes to hooks require restarting the Claude Code session to take effect.

## Logging

The library uses the standard Rust `log` crate for logging. Users need to initialize their preferred logging backend to see log output.

### Setting Up Logging

Add a logging backend to your `Cargo.toml`:
```toml
[dependencies]
env_logger = "0.11"  # or simplelog, tracing-subscriber, etc.
```

Initialize the logger in your application:
```rust
fn main() {
    env_logger::init();
    // ... rest of your code
}
```

### Controlling Log Levels

Use the `RUST_LOG` environment variable to control logging:
```bash
# Show all debug logs from rust-genai
RUST_LOG=rust_genai=debug cargo run

# Show only warnings and errors
RUST_LOG=rust_genai=warn cargo run

# Show debug logs from rust-genai and info from other crates
RUST_LOG=rust_genai=debug,info cargo run
```

### What Gets Logged

At the `debug` level, the library logs:
- Request URLs and bodies (both GenerateContent and Interactions APIs)
- Response content (success and error cases)
- Streaming events and chunks
- Interaction lifecycle events (create, retrieve, delete)

## Development Notes

- Rust edition: 2024
- Minimum Rust version: 1.75 (for stable async traits)
- The project uses `rustls-tls` instead of native TLS for better portability
- All async operations use Tokio as the runtime
