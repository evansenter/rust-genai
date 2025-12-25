# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`rust-genai` is a Rust client library for Google's Generative AI (Gemini) API. The project uses the **Interactions API** which provides a unified interface for working with both models and agents.

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
cargo test --test interactions_api_tests
cargo test --test builder_tests
cargo test --test macro_tests

# Run only unit tests (no integration tests)
cargo test --lib

# Run a single test by name (including ignored tests)
cargo test test_simple_interaction -- --include-ignored

# Run tests with output visible
cargo test -- --nocapture

# Run integration tests in parallel (faster, but may hit rate limits)
cargo test --test interactions_api_tests -- --include-ignored --test-threads=4
```

**Test Execution Time**: Running all integration tests takes approximately 2-5 minutes depending on API response times and network latency. Using `--test-threads=4` can speed this up but may trigger rate limits. Individual tests typically complete in 2-10 seconds.

**Known Test Flakiness**: Some integration tests may occasionally fail due to LLM behavior variability (model may paraphrase data, not follow instructions perfectly, etc.). Re-running usually succeeds.

**Environment Variables for Tests**:
- `GEMINI_API_KEY` (required): API key for running integration tests
- `TEST_IMAGE_URL` (optional): Custom image URL for `test_image_input_from_uri` (defaults to Google's sample scones.jpg)

### Running Examples

All examples require the `GEMINI_API_KEY` environment variable:

```bash
# Interactions API examples
cargo run --example simple_interaction
cargo run --example stateful_interaction
cargo run --example streaming
cargo run --example auto_function_calling
cargo run --example multimodal_image
cargo run --example url_context
cargo run --example thinking
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
   - Exposes the user-facing `Client` and `InteractionBuilder` types
   - Converts internal errors (`genai_client::InternalError`) to public errors (`GenaiError`)
   - Provides high-level abstractions like automatic function calling

2. **Internal Logic Layer** (`src/function_calling.rs`, `src/interactions_api.rs`):
   - Function calling registry and execution system using the `inventory` crate
   - Helper functions for building Interactions API content (text, images, function calls, etc.)

3. **HTTP Client Layer** (`genai-client/`):
   - Raw HTTP requests to Google's Generative AI API
   - JSON models for request/response serialization:
     - `models/interactions.rs`: Interactions API (flat content structure with type tags)
     - `models/shared.rs`: Shared types (FunctionDeclaration, Tool, etc.)
   - Error handling for network and API errors
   - SSE (Server-Sent Events) streaming support
   - Endpoint abstraction in `common.rs` for flexible URL construction

4. **Macro Layer** (`rust-genai-macros/`):
   - Procedural macro `#[generate_function_declaration]` for automatic function discovery
   - Generates FunctionDeclaration from Rust function signatures
   - Registers functions in global inventory for automatic execution

### Key Architectural Patterns

**Builder Pattern**: The library uses a fluent builder API throughout:
- `Client::builder(api_key).build()` for client creation
- `client.interaction().with_model(...).with_text(...).create()` for requests
- `FunctionDeclaration::builder()` for creating function declarations ergonomically
- This pattern is implemented in `src/client.rs` and `src/request_builder.rs`

**Function Calling System**: The library supports three levels of function calling:
1. **Manual**: User explicitly passes `FunctionDeclaration` and handles function calls
2. **Semi-automatic**: Macro generates declarations, but user controls execution
3. **Fully automatic**: `create_with_auto_functions()` discovers and executes functions automatically using the `inventory` crate

The function calling system is implemented across:
- `src/function_calling.rs`: Core traits (`CallableFunction`) and registry
- `rust-genai-macros/src/lib.rs`: Procedural macro for function declaration generation
- `src/request_builder.rs`: The `create_with_auto_functions()` method that orchestrates automatic execution

**Configurable Max Loops**: Use `with_max_function_call_loops(n)` to control the maximum iterations for automatic function calling (default: 5).

**Streaming Architecture**: SSE streaming is implemented using:
- `async-stream` for async generators
- `futures-util::Stream` trait for composable streaming
- Response chunking handled by the Interactions API streaming endpoint

### Error Handling Strategy

The library uses two distinct error types:
- `GenaiError`: For API and network errors (thiserror-based)
- `FunctionError`: Specific to function execution errors

Internal errors from `genai-client` are converted to public `GenaiError` variants via `From` trait implementation in `src/lib.rs:43`.

### Export Strategy for Helper Functions

The library re-exports helper functions based on how they're used:

**Re-exported (user-constructed content):**
- Multimodal inputs (`image_data_content`, `audio_uri_content`, etc.) - users build these to send to the API
- Function results (`function_result_content`) - users send these after executing functions
- Function calls (`function_call_content`) - needed for multi-turn conversations to echo back the model's call

**NOT re-exported (model-generated content):**
- `google_search_call_content`, `google_search_result_content`
- `code_execution_call_content`, `code_execution_result_content`
- `url_context_call_content`, `url_context_result_content`

Built-in tool outputs are generated by the model and users read them from responses via helper methods like `response.google_search_results()` or `response.url_context_metadata()`. These helpers are still accessible via `rust_genai::interactions_api::*` if needed for advanced use cases.

## Important Implementation Details

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
- Uses flat `InteractionContent` enum with type-tagged variants (Text, Thought, Image, Audio, Video, FunctionCall, FunctionResult)
- Fields are often optional as API doesn't always return all data
- Helper functions in `src/interactions_api.rs` provide ergonomic content builders

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
- **Unit tests**: Inline with source code (e.g., `src/lib.rs`, `src/request_builder.rs`, `src/interactions_api.rs`, `genai-client/src/models/interactions.rs`)
- **Integration tests**: In `tests/` directory:
  - `builder_tests.rs`: Unit tests for FunctionDeclaration and InteractionBuilder (no API key required)
  - `macro_tests.rs`: Procedural macro functionality (no API key required)
  - `interactions_api_tests.rs`: Core API integration tests (CRUD, streaming, basic function calling)
  - `advanced_function_calling_tests.rs`: Complex function calling scenarios (multi-function, parallel calls, error handling)
  - `agents_and_multiturn_tests.rs`: Stateful conversations, multi-turn interactions, agent features
  - `multimodal_tests.rs`: Image and other multimodal input handling
  - `tools_and_config_tests.rs`: Built-in tools, generation config, system instructions
  - `common/`: Shared test utilities and fixtures

Integration tests that require a real API key use `#[ignore]` attribute and must be run with `cargo test -- --include-ignored`.

## API Version

The library uses the Gemini V1Beta API internally. The API version is configured in `genai-client/src/common.rs` and is not user-configurable.

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
- Runs all example programs to verify they work with current API
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
- Request URLs and bodies for Interactions API
- Response content (success and error cases)
- Streaming events and chunks
- Interaction lifecycle events (create, retrieve, delete)

## Unreleased Breaking Changes

See `CHANGELOG.md` for full details. Key changes pending for next release:

**Unified Streaming Content Types** (#52):
- `StreamDelta` enum removed - streaming now uses `InteractionContent` directly
- `StreamChunk::Delta(InteractionContent)` for incremental content
- `StreamChunk::Complete(InteractionResponse)` for final response
- New `InteractionContent::ThoughtSignature` variant for streaming thought signatures
- Streaming with function calls now works (fixes #27)

**UsageMetadata Field Names Updated** (#53):
- `prompt_tokens` → `total_input_tokens`
- `candidates_tokens` → `total_output_tokens`
- New fields: `total_cached_tokens`, `total_reasoning_tokens`, `total_tool_use_tokens`
- Token usage now works (previously always returned `None`)

**Client API Simplified**:
- `Client::new(api_key)` no longer takes `api_version` parameter
- `ApiVersion` no longer re-exported from rust-genai

**Deprecated Helpers Removed**:
- `function_response_content()` removed - use `function_result_content()` with `call_id`
- `InteractionContent::FunctionResponse` removed - use `FunctionResult`

## Known Issues & Gaps

Active issues to be aware of (see GitHub issues for current status):
- **#28**: Response parser doesn't support built-in tool call types (code execution, Google Search)
- **#25, #26**: Google Search grounding and code execution not yet supported in Interactions API

## Backlog & Roadmap

See `BACKLOG.md` for detailed feature planning. High priority items:
- **MCP Support**: Model Context Protocol for tool interoperability
- **ReAct Pattern**: Reasoning + Acting agent loop
- **Google Search Grounding**: Real-time web grounding (Gemini-specific)
- **Rate Limiting & Retry Logic**: Production-ready retry with exponential backoff

## CI/CD

The project uses GitHub Actions (`.github/workflows/rust.yml`) with 6 parallel jobs:
- **check**: `cargo check --workspace --all-targets --all-features`
- **test**: Unit tests only (`cargo test --workspace`)
- **test-integration**: Full tests with API key (`cargo test --workspace -- --include-ignored`)
- **fmt**: `cargo fmt --all -- --check`
- **clippy**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- **doc**: `cargo doc --workspace --no-deps --document-private-items` with `-D warnings`

Integration tests only run on pushes or PRs from the same repository (not external forks) to protect the API key.

## Development Notes

- Rust edition: 2024
- Minimum Rust version: 1.75 (for stable async traits)
- The project uses `rustls-tls` instead of native TLS for better portability
- All async operations use Tokio as the runtime
