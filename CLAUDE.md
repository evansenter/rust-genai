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
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items  # Build docs (warnings as errors, matches CI)
```

### Examples

All require `GEMINI_API_KEY`:

**Basic Examples:**
```bash
cargo run --example simple_interaction
cargo run --example streaming
cargo run --example system_instructions
cargo run --example stateful_interaction
```

**Function Calling:**
```bash
cargo run --example auto_function_calling
cargo run --example streaming_auto_functions
cargo run --example manual_function_calling
cargo run --example tool_service
```

**Advanced Features:**
```bash
cargo run --example structured_output
cargo run --example google_search
cargo run --example code_execution
cargo run --example url_context
cargo run --example thinking
cargo run --example deep_research
cargo run --example thought_echo
cargo run --example files_api
```

**Multimodal:**
```bash
cargo run --example multimodal_image
cargo run --example audio_input
cargo run --example video_input
cargo run --example pdf_input
cargo run --example text_input
cargo run --example image_generation
```

## Architecture

### Layered Design

1. **Public API** (`src/lib.rs`, `src/client.rs`, `src/request_builder.rs`): User-facing `Client`, `InteractionBuilder`, error conversion
2. **Internal Logic** (`src/function_calling.rs`, `src/interactions_api.rs`, `src/multimodal.rs`): Function registry, content builders, file loading helpers
3. **HTTP Client** (`genai-client/`): Raw API requests, JSON models (`models/interactions/`, `models/shared.rs`), SSE streaming
4. **Macros** (`rust-genai-macros/`): `#[tool]` macro with `inventory` registration

### Key Patterns

**Builder API**: Fluent builders throughout (`Client::builder()`, `client.interaction().with_*()`, `FunctionDeclaration::builder()`)

**Function Calling - Two Categories**:

*Client-Side Tools* (YOUR code executes):
- `#[tool]` macro: Compile-time registration, stateless, auto-discovered
- `ToolService`: Runtime registration, stateful (DB, APIs, config), dependency injection
- Manual: Full control via `create()` + `function_result_content()` loop

*Server-Side Tools* (API executes):
- Google Search, Code Execution, URL Context - enabled via `with_tool()`

**Choosing Client-Side Approach**:
| Approach | Registration | State | Best For |
|----------|-------------|-------|----------|
| `#[tool]` macro | Compile-time | Stateless | Simple tools, clean code |
| `ToolService` | Runtime | Stateful | DB pools, API clients, dynamic config |
| Manual handling | N/A | Flexible | Custom execution logic, rate limiting |

**When You're Blocked - Use ToolService Instead of #[tool]**:
- **Need database access**: `#[tool]` functions can't access connection pools → Use `ToolService` with `Arc<Pool<...>>`
- **Need API client**: `#[tool]` functions can't share HTTP clients → Use `ToolService` with `Arc<reqwest::Client>`
- **Need configuration**: `#[tool]` functions can't read runtime config → Use `ToolService` with `Arc<RwLock<Config>>`
- **Need mutable state**: `#[tool]` functions are stateless → Use `ToolService` with `Arc<RwLock<T>>`
- **Need per-request context**: `#[tool]` functions are global → Use `ToolService` to inject request-specific tools

**ToolService Pattern**: Use `Arc<RwLock<T>>` for interior mutability. Same service instance reused across requests via `service.clone()` (clones the Arc, not the service). See `examples/tool_service.rs`.

**Multi-Turn Inheritance Rules** (with `previousInteractionId`):
| Field | Inherited? | Notes |
|-------|------------|-------|
| `systemInstruction` | ✅ Yes | Only send on first turn |
| `tools` | ❌ No | Must resend on every new user message turn |
| Conversation history | ✅ Yes | Automatically included |

**Function Result Returns**: When returning function results to the model (after executing a tool), you do NOT need to resend `tools`. The model remembers available tools from the interaction that triggered the function call. Only new user message turns require tools to be included.

**Debugging Multi-Turn**: Use `LOUD_WIRE=1` to see exactly what's being sent on each request. This shows tools, systemInstruction, and previousInteractionId for each turn, making it easy to verify correct behavior.

**For comprehensive multi-turn and function calling patterns**: See `docs/MULTI_TURN_FUNCTION_CALLING.md` - covers stateful vs stateless, auto vs manual execution, thought signatures, and design patterns.

**Streaming**: Uses `async-stream` generators and `futures-util::Stream`

### Error Types

- `GenaiError`: API/network errors (thiserror-based), defined in `genai-client/src/errors.rs`, re-exported from `rust-genai`
- `FunctionError`: Function execution errors

### Multimodal Input

**Fluent Builder Pattern** (recommended for inline content):
```rust
// Images, audio, video, documents - all use the same pattern
client.interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Analyze this image")
    .add_image_file("photo.jpg").await?        // From file (auto MIME detection)
    .add_image_data(base64_data, "image/png")  // From base64
    .add_image_uri("gs://bucket/img.jpg", "image/jpeg")  // From URI
    .create().await?
```

**Content Vector** (for programmatic/dynamic content):
```rust
use rust_genai::{text_content, image_from_file};

let contents = vec![
    text_content("Analyze this image"),
    image_from_file("photo.jpg").await?,
];
client.interaction()
    .with_content(contents)
    .create().await?
```

**File Loading Helpers** (in `multimodal` module):
- `image_from_file()`, `audio_from_file()`, `video_from_file()`, `document_from_file()`
- Auto-detect MIME type from extension, load file, base64 encode
- `*_from_file_with_mime()` variants for explicit MIME override

### Multimodal Output (Image Generation)

**Generate images** by setting response modalities:
```rust
let response = client.interaction()
    .with_model("gemini-3-pro-image-preview")
    .with_text("Generate an image of a sunset over mountains")
    .with_response_modalities(vec!["IMAGE".to_string()])
    .create().await?;

// Extract and save generated images
for output in &response.outputs {
    if let InteractionContent::Image {
        data: Some(base64_data),
        mime_type,
        ..
    } = output {
        // Decode base64 image data
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD.decode(base64_data)?;

        // Save to file (mime_type typically "image/png" or "image/jpeg")
        std::fs::write("generated.png", bytes)?;
    }
}
```

**Response Modalities**:
- `"TEXT"` - Text generation (default)
- `"IMAGE"` - Image generation
- Can specify both: `vec!["TEXT".to_string(), "IMAGE".to_string()]`

See `examples/image_generation.rs` for complete example.

### Content Export Strategy

**Re-exported** (user-constructed): `image_data_content`, `audio_uri_content`, `function_result_content`, `function_call_content`, `image_from_file`, `audio_from_file`, `video_from_file`, `document_from_file`, `detect_mime_type`

**Not re-exported** (model-generated): Built-in tool outputs accessed via response methods like `response.google_search_results()`, `response.code_execution_results()`

### Files API

The Files API allows uploading large files (up to 2GB) that can be referenced across multiple interactions. Files are stored for 48 hours.

**Standard upload** (loads entire file into memory):
```rust
let file = client.upload_file("video.mp4").await?;
// Use in interaction
client.interaction().with_file(&file).with_text("Describe this").create().await?;
// Clean up
client.delete_file(&file.name).await?;
```

**Chunked upload** (memory-efficient for large files 500MB-2GB):
```rust
// Uses ~8MB memory regardless of file size
let (file, resumable) = client.upload_file_chunked("large_video.mp4").await?;

// With custom chunk size
let (file, resumable) = client.upload_file_chunked_with_options(
    "large_video.mp4",
    "video/mp4",
    16 * 1024 * 1024  // 16MB chunks
).await?;
```

**When to use chunked:**
- Large files (500MB-2GB) to avoid memory pressure
- Memory-constrained environments (containers, edge devices)
- Batch processing multiple large files

**Memory characteristics:**
- Standard: Requires `file_size` bytes of RAM
- Chunked: Uses fixed ~`chunk_size` bytes (default 8MB)

**ResumableUpload handle** returned from chunked methods:
- `file_size()`: Total file size
- `mime_type()`: File MIME type
- `upload_url()`: Resumable upload URL
- `query_offset()`: Query server for bytes uploaded (for resume logic)
- `resume()`: Resume upload from offset with a reader

## Core Design Philosophy: Evergreen-Inspired Soft-Typing

This library follows the [Evergreen spec](https://github.com/google-deepmind/evergreen-spec) philosophy for graceful API evolution. The core principle: **unknown data should be preserved, not rejected**.

### Key Principles

1. **Graceful Unknown Handling**: Unrecognized API types deserialize into `Unknown` variants instead of failing
2. **Non-Exhaustive Enums**: Use `#[non_exhaustive]` on enums that may grow (e.g., `InteractionContent`, `Tool`)
3. **Soft-Typed Where Appropriate**: Use `serde_json::Value` for evolving structures (e.g., function args)
4. **Preserve Data Roundtrip**: `Unknown` variants serialize back with their original data intact
5. **Continue on Unknown Status**: When polling for interaction completion, continue polling on unrecognized status variants rather than failing immediately. This ensures forward compatibility when the API adds new transient states. Use timeouts to protect against infinite loops (see `examples/deep_research.rs`).

### DO:
```rust
// Use non_exhaustive for API-driven enums
#[non_exhaustive]
pub enum Tool {
    GoogleSearch,
    CodeExecution,
    Unknown { tool_type: String, data: serde_json::Value },
}

// Handle unknown variants in match
match tool {
    Tool::GoogleSearch => ...,
    Tool::CodeExecution => ...,
    _ => log::warn!("Unknown tool type, ignoring"),
}

// Use serde_json::Value for flexible/evolving fields
pub struct FunctionCall {
    pub name: String,
    pub args: serde_json::Value,  // Schema may change
}

// Continue polling on unknown status variants
match response.status {
    InteractionStatus::Completed => return Ok(response),
    InteractionStatus::Failed => return Err(...),
    InteractionStatus::InProgress => { /* continue */ }
    other => {
        // Don't fail - continue polling with timeout protection
        eprintln!("Unhandled status {:?}, continuing...", other);
    }
}
```

### DON'T:
```rust
// Don't use exhaustive enums for API types - breaks when API adds variants
pub enum Tool {
    GoogleSearch,
    CodeExecution,
    // API adds "NewTool" -> all client code breaks!
}

// Don't fail on unknown data
let content: InteractionContent = serde_json::from_str(json)?;
// If json has type "future_feature", this should NOT error
```

### Standard Unknown Variant Pattern

All enums with `Unknown` variants use the **data-preserving pattern** with consistent naming:

**Field names** follow `<context>_type`:
- `InteractionContent`: `content_type`
- `Tool`: `tool_type`
- `InteractionStatus`: `status_type`
- `StreamChunk` / `AutoFunctionStreamChunk`: `chunk_type`

**Helper methods** are consistent across all types:
- `is_unknown()` - Check if this is an Unknown variant
- `unknown_<context>_type()` - Get the unrecognized type name
- `unknown_data()` - Get the preserved JSON data

```rust
Unknown {
    /// The unrecognized type from the API
    <context>_type: String,
    /// The full JSON data, preserved for debugging and roundtrip serialization
    data: serde_json::Value,
}
```

This requires a custom `Deserialize` implementation. See `InteractionContent` in `content.rs` for the reference implementation.

**Why not `#[serde(other)] Unknown`?** The unit variant pattern loses all data - you can't inspect what the API sent or roundtrip serialize it. Always prefer the data-preserving pattern.

### Implementation Locations

- `InteractionContent` (content.rs): `content_type` field, `unknown_content_type()` helper ✅
- `Tool` (shared.rs): `tool_type` field, `unknown_tool_type()` helper ✅
- `InteractionStatus` (response.rs): `status_type` field, `unknown_status_type()` helper ✅
- `StreamChunk` (streaming.rs): `chunk_type` field, `unknown_chunk_type()` helper ✅
- `AutoFunctionStreamChunk` (streaming.rs): `chunk_type` field, `unknown_chunk_type()` helper ✅
- `strict-unknown` feature flag: Optional strict mode for development/testing

## Test Organization

- **Unit tests**: Inline in source files
- **Integration tests** (`tests/`):
  - `function_declaration_builder_tests.rs`, `interaction_builder_tests.rs`, `macro_tests.rs`, `ui_tests.rs`: No API key needed
  - `interactions_api_tests.rs`: Core CRUD, streaming
  - `advanced_function_calling_tests.rs`: Complex function scenarios
  - `agents_tests.rs`, `multiturn_tests.rs`, `streaming_multiturn_tests.rs`: Stateful conversations
  - `thinking_function_tests.rs`, `tools_multiturn_tests.rs`: Thinking and tool multi-turn tests
  - `multimodal_tests.rs`: Image/media handling
  - `tools_and_config_tests.rs`: Built-in tools
  - `api_canary_tests.rs`: API compatibility checks
  - `common/`: Shared test utilities
- **Property-based tests** (proptest): Serialization roundtrip verification
  - `genai-client/src/models/interactions/proptest_tests.rs`: Strategy generators and unit proptests
  - `tests/proptest_roundtrip_tests.rs`: Integration proptests for public API types
  - To add coverage for a new type: create an `arb_<type>()` strategy function, then add a `<type>_roundtrip` test
  - Use `#[cfg(feature = "strict-unknown")]` variants for strategies that include `Unknown` variants

### Test Assertion Strategies

Integration tests should use the appropriate assertion strategy based on what they're testing:

**Structural Assertions** (default for most tests):
- Verify API mechanics work (status codes, response structure, field presence)
- Check non-empty responses, valid IDs, correct status
- Fast execution, no extra API calls
- Example: `assert!(!response.text().unwrap().is_empty())`

**Semantic Validation** (for high-value behavioral tests):
- Use `validate_response_semantically()` from `tests/common/mod.rs`
- Validates responses are contextually appropriate using Gemini as a judge
- Uses structured output for deterministic yes/no validation with explanation
- Apply to critical tests where behavior matters: context preservation, function calling effectiveness, multi-turn coherence
- Adds ~1-2 seconds per validation (extra API call)
- Example:
  ```rust
  let is_valid = validate_response_semantically(
      &client,
      "User asked about weather in Tokyo",
      response.text().unwrap(),
      "Does this response provide weather information?"
  ).await?;
  assert!(is_valid, "Response should address weather question");
  ```

**DO NOT use brittle content assertions:**
- ❌ Avoid `text.contains("specific word")` - LLM outputs vary
- ❌ Avoid checking for exact phrases or specific numbers in generated text
- ✅ Use structural checks for mechanics, semantic validation for behavior

**When to use semantic validation:**
- Multi-turn context preservation tests
- Function calling integration tests where the function result should inform the response
- Tests validating the model understood and addressed a complex question
- Any test where "did it do the right thing?" matters more than "did it respond?"

See `tests/thinking_function_tests.rs::test_thinking_with_function_calling_multi_turn` and `tests/interactions_api_tests.rs::test_stateful_conversation` for examples.

## CI/CD

GitHub Actions (`.github/workflows/rust.yml`) runs 9 parallel jobs: check, test, test-strict-unknown, 4× test-integration (core, tools, functions, multimodal), fmt, clippy, doc, security. Integration tests are split into 4 matrix jobs for faster execution (~2.5 min vs ~4 min). Integration tests require same-repo origin (protects API key). CI runs on all PRs regardless of file type.

## Project Conventions

- **Model name**: Always use `gemini-3-flash-preview` throughout the project (tests, examples, documentation). Exception: Image generation requires `gemini-3-pro-image-preview` since it's the only model supporting image output.

### Naming Conventions

**Suffix patterns for method names:**

| Suffix | Meaning | Example |
|--------|---------|---------|
| `*_stream()` | Returns a `Stream<Item>` for async iteration | `create_stream()` returns `Stream<StreamChunk>` |
| `*_chunked()` | Uses chunked/streaming internally, returns single result | `upload_file_chunked()` returns `(FileMetadata, ResumableUpload)` |
| `*_with_auto_functions()` | Automatically executes functions in a loop | `create_stream_with_auto_functions()` |

The distinction matters: `*_stream()` methods return async iterators that yield multiple items, while `*_chunked()` methods use chunked I/O internally but return a single final result.

### #[must_use] Annotation

Use `#[must_use]` liberally to prevent users from silently dropping values with side effects:

```rust
// Good - prevents ignoring important return values
#[must_use]
pub fn upload_url(&self) -> &str { ... }

#[must_use]
pub fn is_active(&self) -> bool { ... }
```

Apply `#[must_use]` to:
- Getters that return important state
- Methods returning handles needed for further operations (e.g., `ResumableUpload`)
- Boolean status checks
- Any function where ignoring the result is likely a bug

The attribute generates a compiler warning if the return value is unused, catching bugs early.

## Versioning Philosophy

Breaking changes are always permitted, and preferred when they:
- Simplify the API surface
- Remove unnecessary abstractions
- Align with Evergreen principles

Prefer clean breaks over backwards-compatibility shims. Don't add deprecation warnings or migration layers—just make the change.

## Logging

See `docs/LOGGING_STRATEGY.md` for the full logging strategy. Key points:

- **Log levels**: `error` for unrecoverable failures, `warn` for recoverable issues (including Evergreen unknown variants), `debug` for API lifecycle events and request bodies
- **Sensitive data**: API keys are redacted in `Debug` output. User prompts/media logged only at `debug` level (disabled by default)
- **Evergreen logging**: All `Unknown` variants log at `warn` level to surface API evolution
- **Maintenance**: When adding new logging, update the strategy doc to keep examples accurate

Enable debug logging with:
```bash
RUST_LOG=rust_genai=debug cargo run --example simple_interaction
```

## Technical Notes

- Rust edition 2024 (requires Rust 1.85+)
- Uses `rustls-tls` (not native TLS)
- Tokio async runtime
- API version: Gemini V1Beta (configured in `genai-client/src/common.rs`, not user-configurable)
- See `CHANGELOG.md` for breaking changes and migration guides
