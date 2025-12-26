# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`rust-genai` is a Rust client library for Google's Generative AI (Gemini) API using the **Interactions API** for unified model/agent interactions.

**Workspace structure:**
- **`rust-genai`** (root): Public API crate with user-facing `Client` and `InteractionBuilder`
- **`genai-client/`**: Internal HTTP client, JSON models, and SSE streaming
- **`rust-genai-macros/`**: Procedural macro for automatic function declaration generation

## Development Commands

### Testing

**Default**: Always run `cargo test -- --include-ignored` for full integration testing.

```bash
cargo test -- --include-ignored           # Full test suite (requires GEMINI_API_KEY)
cargo test                                 # Unit tests only
cargo test --test interactions_api_tests  # Specific test file
cargo test test_name -- --include-ignored # Single test by name
cargo test -- --nocapture                 # Show test output
```

**Environment**: `GEMINI_API_KEY` required for integration tests. Tests take 2-5 minutes; some may flake due to LLM variability.

### Quality Checks

```bash
cargo fmt                                                            # Format
cargo fmt -- --check                                                 # Check format
cargo clippy --workspace --all-targets --all-features -- -D warnings # Lint
cargo doc --workspace --no-deps --document-private-items             # Build docs
```

### Examples

All require `GEMINI_API_KEY`:
```bash
cargo run --example simple_interaction
cargo run --example streaming
cargo run --example auto_function_calling
cargo run --example structured_output
cargo run --example google_search
cargo run --example code_execution
cargo run --example url_context
cargo run --example thinking
cargo run --example multimodal_image
cargo run --example pdf_input
cargo run --example image_generation
cargo run --example stateful_interaction
```

## Architecture

### Layered Design

1. **Public API** (`src/lib.rs`, `src/client.rs`, `src/request_builder.rs`): User-facing `Client`, `InteractionBuilder`, error conversion
2. **Internal Logic** (`src/function_calling.rs`, `src/interactions_api.rs`): Function registry, content builders
3. **HTTP Client** (`genai-client/`): Raw API requests, JSON models (`models/interactions.rs`, `models/shared.rs`), SSE streaming
4. **Macros** (`rust-genai-macros/`): `#[generate_function_declaration]` macro with `inventory` registration

### Key Patterns

**Builder API**: Fluent builders throughout (`Client::builder()`, `client.interaction().with_*()`, `FunctionDeclaration::builder()`)

**Function Calling Levels**:
1. Manual: User provides `FunctionDeclaration` and handles calls
2. Semi-automatic: Macro generates declarations, user controls execution
3. Fully automatic: `create_with_auto_functions()` discovers and executes via `inventory` crate

**Streaming**: Uses `async-stream` generators and `futures-util::Stream`

### Error Types

- `GenaiError`: API/network errors (thiserror-based), converted from `genai_client::InternalError` in `src/lib.rs:43`
- `FunctionError`: Function execution errors

### Content Export Strategy

**Re-exported** (user-constructed): `image_data_content`, `audio_uri_content`, `function_result_content`, `function_call_content`

**Not re-exported** (model-generated): Built-in tool outputs accessed via response methods like `response.google_search_results()`, `response.code_execution_results()`

## Test Organization

- **Unit tests**: Inline in source files
- **Integration tests** (`tests/`):
  - `builder_tests.rs`, `macro_tests.rs`: No API key needed
  - `interactions_api_tests.rs`: Core CRUD, streaming
  - `advanced_function_calling_tests.rs`: Complex function scenarios
  - `agents_and_multiturn_tests.rs`: Stateful conversations
  - `multimodal_tests.rs`: Image/media handling
  - `tools_and_config_tests.rs`: Built-in tools
  - `api_canary_tests.rs`: API compatibility checks
  - `common/`: Shared test utilities

## Claude Code Configuration

### Hooks (automatic)

- **PostToolUse**: Auto-runs `cargo fmt` after editing Rust files
- **SessionStart**: Verifies `GEMINI_API_KEY` and build status (`.claude/hooks/session_init.sh`)
- **Stop**: Pre-push validation matching CI (`.claude/hooks/stop.sh`)

### Skills (auto-invoked)

- **`test-full`**: Complete test suite with `--include-ignored`
- **`review-workspace`**: Health check (cargo check, clippy, unit tests)
- **`check-docs`**: Documentation build with warning checks
- **`run-examples`**: Verify all examples work

## CI/CD

GitHub Actions (`.github/workflows/rust.yml`) runs 6 parallel jobs: check, test, test-integration, fmt, clippy, doc. Integration tests require same-repo origin (protects API key).

## Project Conventions

- **Model name**: Always use `gemini-3-flash-preview` throughout the project (tests, examples, documentation). Do not reference other model names.

## Technical Notes

- Rust edition 2024, minimum 1.75
- Uses `rustls-tls` (not native TLS)
- Tokio async runtime
- API version: Gemini V1Beta (configured in `genai-client/src/common.rs`, not user-configurable)
- See `CHANGELOG.md` for breaking changes and migration guides
