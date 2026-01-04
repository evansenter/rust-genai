# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## External Gemini API Documentation

**Important**: When working on API integration or troubleshooting, consult these authoritative sources:

| Document | URL |
|----------|-----|
| Interactions API Reference | https://ai.google.dev/static/api/interactions.md.txt |
| Interactions API Guide | https://ai.google.dev/static/api/interactions-api.md.txt |
| Function Calling Guide | https://ai.google.dev/gemini-api/docs/function-calling.md.txt |
| Thought Signatures | https://ai.google.dev/gemini-api/docs/thought-signatures.md.txt |

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
cargo test test_name -- --include-ignored # Single test by name
```

**Environment**: `GEMINI_API_KEY` required for integration tests. Tests take 2-5 minutes; some may flake due to LLM variability.

### Quality Checks

```bash
cargo fmt -- --check                                                 # Check format
cargo clippy --workspace --all-targets --all-features -- -D warnings # Lint
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items  # Docs
```

### Examples

All require `GEMINI_API_KEY`. Key examples:
- `cargo run --example simple_interaction` - Basic usage
- `cargo run --example auto_function_calling` - Function calling
- `cargo run --example streaming` - SSE streaming

See `examples/` for full list (multimodal, thinking, files API, image generation, etc.)

## Architecture

### Layered Design

1. **Public API** (`src/lib.rs`, `src/client.rs`, `src/request_builder/`): User-facing `Client`, `InteractionBuilder`
2. **Internal Logic** (`src/function_calling.rs`, `src/interactions_api.rs`, `src/multimodal.rs`): Function registry, content builders
3. **HTTP Client** (`genai-client/`): Raw API requests, JSON models, SSE streaming
4. **Macros** (`rust-genai-macros/`): `#[tool]` macro with `inventory` registration

### Key Patterns

**Builder API**: Fluent builders throughout (`Client::builder()`, `client.interaction().with_*()`)

**Function Calling** - Two categories:

| Category | Tools | Who Executes |
|----------|-------|--------------|
| Client-Side | `#[tool]` macro, `ToolService`, Manual | YOUR code |
| Server-Side | Google Search, Code Execution, URL Context | API |

**Choosing Client-Side Approach**:
| Approach | Registration | State | Best For |
|----------|-------------|-------|----------|
| `#[tool]` macro | Compile-time | Stateless | Simple tools, clean code |
| `ToolService` | Runtime | Stateful | DB pools, API clients, dynamic config |
| Manual handling | N/A | Flexible | Custom execution logic, rate limiting |

**Function Calling Modes**:
| Mode | Behavior |
|------|----------|
| Auto (default) | Model decides whether to call functions |
| Any | Model must call a function |
| None | Function calling disabled |
| Validated | Schema adherence for both calls and natural language |

**Multi-Turn Inheritance Rules** (critical gotcha):
| Field | Inherited? | Notes |
|-------|------------|-------|
| `systemInstruction` | ✅ Yes | Only send on first turn |
| `tools` | ❌ No | Must resend on every new user message turn |
| Conversation history | ✅ Yes | Automatically included |

**Debugging**: Use `LOUD_WIRE=1` to see wire-level request/response details.

**Comprehensive Guides** (see `docs/`):
- `docs/MULTI_TURN_FUNCTION_CALLING.md` - Stateful/stateless, auto/manual execution, thought signatures
- `docs/STREAMING_API.md` - Stream types, resume capability, auto-function streaming
- `docs/LOGGING_STRATEGY.md` - Log levels, sensitive data handling

### Error Types

- `GenaiError`: API/network errors (thiserror-based), defined in `genai-client/src/errors.rs`
- `FunctionError`: Function execution errors

## Core Design Philosophy: Evergreen Soft-Typing

This library follows the [Evergreen spec](https://github.com/google-deepmind/evergreen-spec) philosophy: **unknown data should be preserved, not rejected**.

### Key Principles

1. **Graceful Unknown Handling**: Unrecognized API types deserialize into `Unknown` variants
2. **Non-Exhaustive Enums**: Use `#[non_exhaustive]` on enums that may grow
3. **Preserve Data Roundtrip**: `Unknown` variants serialize back with original data intact
4. **Continue on Unknown Status**: When polling, continue on unrecognized status (use timeouts)

### Standard Unknown Variant Pattern

All enums use consistent naming - field names follow `<context>_type` (e.g., `content_type`, `tool_type`, `status_type`):

```rust
Unknown {
    <context>_type: String,      // The unrecognized type from API
    data: serde_json::Value,     // Full JSON preserved for roundtrip
}
```

Helper methods: `is_unknown()`, `unknown_<context>_type()`, `unknown_data()`

See `InteractionContent` in `genai-client/src/models/interactions/content.rs` for reference implementation.

**Implementation status** (all ✅): `InteractionContent`, `Tool`, `InteractionStatus`, `StreamChunk`, `AutoFunctionStreamChunk`, `FunctionCallingMode`

## Test Organization

- **Unit tests**: Inline in source files
- **Integration tests** (`tests/`): Require `GEMINI_API_KEY` for most; see file names for categories
- **Property-based tests** (proptest): Serialization roundtrip verification
  - `genai-client/src/models/interactions/proptest_tests.rs`: Strategy generators
  - `tests/proptest_roundtrip_tests.rs`: Integration proptests

### Test Assertion Strategies

- **Structural**: Verify API mechanics (status, field presence) - default for most tests
- **Semantic**: Use `validate_response_semantically()` for behavioral tests (adds ~1-2s API call)
- **Avoid**: Brittle `text.contains("word")` assertions - LLM outputs vary

## CI/CD

GitHub Actions runs: check, test, test-strict-unknown, test-integration (4 matrix groups), fmt, clippy, doc, security, build-metrics. Integration tests require same-repo origin (protects API key).

## Project Conventions

- **Model name**: Always use `gemini-3-flash-preview` (exception: `gemini-3-pro-image-preview` for image generation)

### Naming Conventions

| Suffix | Meaning | Example |
|--------|---------|---------|
| `*_stream()` | Returns `Stream<Item>` for async iteration | `create_stream()` |
| `*_chunked()` | Uses chunked I/O internally, returns single result | `upload_file_chunked()` |
| `*_with_auto_functions()` | Automatically executes functions in a loop | `create_stream_with_auto_functions()` |

### #[must_use] Annotation

Apply `#[must_use]` to getters, handles, and boolean checks where ignoring the result is likely a bug.

## Versioning Philosophy

Breaking changes are permitted and preferred when they simplify the API or align with Evergreen principles. Prefer clean breaks over backwards-compatibility shims.

## Logging

See `docs/LOGGING_STRATEGY.md`. Key points:
- `error` for unrecoverable, `warn` for recoverable (including Evergreen unknowns), `debug` for API lifecycle
- API keys redacted; user content only at `debug` level
- Enable: `RUST_LOG=rust_genai=debug cargo run --example simple_interaction`

## Technical Notes

- Rust edition 2024 (requires Rust 1.85+)
- Uses `rustls-tls` (not native TLS)
- Tokio async runtime
- API version: Gemini V1Beta (configured in `genai-client/src/common.rs`)
- See `CHANGELOG.md` for breaking changes and migration guides
