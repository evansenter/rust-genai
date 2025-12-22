# Rust GenAI - Project Context for AI Assistants

## Project Overview

Rust GenAI is a Rust client library for interacting with Google's Generative AI (Gemini) API. It provides a simple, type-safe, and ergonomic interface for text generation, streaming responses, function calling, and code execution.

## Project Structure

This is a Rust workspace with three main components:

### 1. Main Crate (`rust-genai`)
**Location**: `src/`

The public API that users interact with. Key modules:

- `lib.rs` - Main entry point and public API exports
- `client.rs` - Client builder and initialization
- `request_builder.rs` - Fluent API for building requests
- `content_api.rs` - Helper functions for building conversation histories
- `function_calling.rs` - Automatic function discovery and execution
- `types.rs` - Public type definitions and error types
- `internal/` - Internal implementation details

### 2. Internal Client (`genai-client/`)
**Location**: `genai-client/`

Low-level HTTP client and API models. Handles:
- JSON serialization/deserialization for Gemini API
- HTTP requests via reqwest
- Response parsing and streaming
- Raw API error handling

Users of `rust-genai` don't directly interact with this crate.

### 3. Procedural Macros (`rust-genai-macros/`)
**Location**: `rust-genai-macros/`

Provides the `#[generate_function_declaration]` procedural macro for:
- Auto-generating function declarations from Rust functions
- Registering functions globally via the `inventory` crate
- Supporting both sync and async functions

## Key Architecture Patterns

### Builder Pattern
The library uses a fluent builder pattern for client and request construction:

```rust
let client = Client::builder(api_key).build();
let response = client
    .with_model("gemini-2.5-flash-preview-05-20")
    .with_prompt("Hello")
    .generate()
    .await?;
```

### Error Handling
Two main error types:
- `GenaiError` - For API and HTTP errors
- `FunctionError` - For function calling errors

Both use `thiserror` for ergonomic error definitions.

### Async/Await
The entire API is async-first using Tokio. Streaming uses `async-stream` and `futures-util`.

### Function Calling
Two approaches:
1. **Manual**: User explicitly handles function calls and multi-turn conversations
2. **Automatic**: Library discovers functions via `inventory` and handles execution loop

## Testing Strategy

**Location**: `tests/`

Comprehensive test suite including:
- Unit tests for individual modules
- Integration tests for end-to-end workflows
- Macro tests for procedural macro functionality
- Debug mode tests
- Error handling tests

Tests use mock responses where possible to avoid API calls.

## Code Style Guidelines

- **Rust 2024 Edition** (see Cargo.toml edition field)
- **Async/await** for all I/O operations
- **Builder patterns** for complex object construction
- **Type safety** - leverage Rust's type system extensively
- **Error handling** - use `Result` types, no panics in library code
- **Documentation** - doc comments on all public items

## Important Files to Know

- `README.md` - Comprehensive user documentation with examples
- `TEST_COVERAGE.md` - Test coverage documentation
- `examples/` - Runnable example programs:
  - `simple_request.rs` - Basic usage
  - `stream_request.rs` - Streaming
  - `function_call.rs` - Function calling
  - `code_execution.rs` - Code execution

## Common Development Tasks

### Adding a New Feature
1. Update `genai-client/` if new API models needed
2. Add public API in `src/`
3. Update builder in `request_builder.rs` if applicable
4. Add tests in `tests/`
5. Update README.md with examples
6. Update TEST_COVERAGE.md

### Modifying Function Calling
- Core logic: `src/function_calling.rs`
- Macro implementation: `rust-genai-macros/src/lib.rs`
- Test with: `tests/function_calling_tests.rs`, `tests/macro_execution_tests.rs`

### Updating API Models
- Location: `genai-client/src/`
- Ensure serde serialization is correct
- Test with integration tests

## Dependencies

Key dependencies and their purposes:
- `tokio` - Async runtime
- `reqwest` - HTTP client
- `serde`/`serde_json` - Serialization
- `async-trait` - Async traits
- `inventory` - Global function registry
- `thiserror` - Error handling
- `futures-util` - Stream utilities
- `async-stream` - Stream macros

## API Key Management

Examples expect `GEMINI_API_KEY` environment variable. The library itself doesn't enforce this - it's up to the user to provide the key.

## Current Model Support

Tested primarily with:
- `gemini-2.5-flash-preview-05-20`

Should work with other Gemini models that support the same API schema.

## Known Limitations

- No retry logic for API calls (user's responsibility)
- No rate limiting built-in
- Function execution limited to 5 turns to prevent infinite loops
- Code execution only supports Python (Gemini API limitation)

## When Making Changes

1. **Read before editing** - Always read existing code before modifications
2. **Test coverage** - Add tests for new functionality
3. **Update docs** - Keep README.md and doc comments current
4. **Examples** - Update or add examples if API changes
5. **Error handling** - Ensure proper error types and messages
6. **Backwards compatibility** - Consider impact on existing users

## Design Principles

- **Simple over complex** - Keep the API surface minimal and intuitive
- **Type safety** - Use Rust's type system to catch errors at compile time
- **Ergonomic** - Builder patterns and sensible defaults
- **Async-first** - All I/O is async
- **No surprises** - Explicit over implicit behavior
- **Well-documented** - Every public item has doc comments with examples

## Questions to Ask Before Changes

1. Does this fit the existing API design patterns?
2. Is this the simplest solution that could work?
3. Have I added appropriate error handling?
4. Are there tests covering this functionality?
5. Is the documentation clear and complete?
6. Does this maintain backwards compatibility?

## Helpful Context

- The project uses Rust 2024 edition features
- Function declarations follow JSON Schema format for Gemini API
- The `Content` type represents messages in multi-turn conversations
- Streaming responses are `impl Stream<Item = Result<GenerateContentResponse>>`
- The macro uses `syn` for parsing and `quote` for code generation
