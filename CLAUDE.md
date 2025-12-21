# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`rust-genai` is a Rust client library for Google's Generative AI (Gemini) API. The project is structured as a Cargo workspace with three crates:

- **`rust-genai`** (root): Public API crate that provides the user-facing interface
- **`genai-client/`**: Internal low-level client that handles HTTP communication, JSON serialization/deserialization, and raw API interactions
- **`rust-genai-macros/`**: Procedural macro crate for generating function declarations from Rust functions

## Common Development Commands

### Building and Testing

```bash
# Build all workspace members
cargo build

# Build in release mode
cargo build --release

# Run all tests (requires GEMINI_API_KEY environment variable for integration tests)
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
# Run examples
cargo run --example simple_request
cargo run --example stream_request
cargo run --example function_call
cargo run --example code_execution
```

### Linting and Formatting

```bash
# Format code
cargo fmt

# Check formatting without making changes
cargo fmt -- --check

# Run clippy for linting
cargo clippy

# Run clippy with all features and deny warnings
cargo clippy --all-features -- -D warnings
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
   - JSON models for request/response serialization (models/request.rs, models/response.rs)
   - Error handling for network and API errors
   - SSE (Server-Sent Events) streaming support

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

### Workspace Member Relationships

- The root crate (`rust-genai`) depends on both `genai-client` and `rust-genai-macros`
- `genai-client` is completely independent and could theoretically be used standalone
- `rust-genai-macros` uses the `inventory` crate to register functions at compile time
- Functions marked with `#[generate_function_declaration]` are automatically collected via `inventory::collect!` and can be discovered at runtime

### Test Organization

Tests are organized into two categories:
- **Unit tests**: Inline with source code (e.g., `src/lib.rs:55-134`)
- **Integration tests**: In `tests/` directory, each file tests a specific feature:
  - `integration_tests.rs`: Full API workflow tests (requires API key)
  - `macro_tests.rs`: Procedural macro functionality
  - `function_calling_tests.rs`: Function execution system
  - `content_api_tests.rs`: Conversation helper functions

Integration tests that require a real API key use `#[ignore]` attribute and must be run with `cargo test -- --ignored`.

## API Version Support

The library supports different API versions via the `ApiVersion` enum in `genai-client`:
- Currently defaults to `V1Beta`
- API version affects URL construction in `genai-client/src/core.rs`

## Development Notes

- Rust edition: 2024
- Minimum Rust version: 1.75 (for stable async traits)
- The project uses `rustls-tls` instead of native TLS for better portability
- All async operations use Tokio as the runtime
