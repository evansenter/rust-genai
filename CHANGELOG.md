# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### BREAKING CHANGES

#### Unified Streaming Content Types (#39, #27)
- **`StreamDelta` enum removed**: Streaming deltas now use `InteractionContent` directly
  - `StreamChunk::Delta(InteractionContent)` contains incremental content during streaming
  - `StreamChunk::Complete(InteractionResponse)` contains the final complete response
- **New `InteractionContent::ThoughtSignature` variant**: Captures streaming thought signatures
- **New helper methods on `InteractionContent`**: `text()`, `is_text()`, `is_thought()`, `is_thought_signature()`, `is_function_call()`
- **New type exported**: `StreamChunk` (note: `StreamDelta` is no longer exported)

**Migration guide:**
```rust
// Before:
match chunk {
    StreamChunk::Delta(delta) => match delta {
        StreamDelta::Text { text } => println!("{}", text),
        StreamDelta::Thought { text } => println!("[thinking: {}]", text),
        _ => {}
    }
    StreamChunk::Complete(response) => { /* ... */ }
}

// After:
match chunk {
    StreamChunk::Delta(content) => match content {
        InteractionContent::Text { text } => println!("{}", text.as_deref().unwrap_or("")),
        InteractionContent::Thought { text } => println!("[thinking: {}]", text.as_deref().unwrap_or("")),
        InteractionContent::FunctionCall { name, args, .. } => {
            println!("Function call: {}({:?})", name, args);
        }
        _ => {}
    }
    StreamChunk::Complete(response) => { /* ... */ }
}

// Helper methods still work the same:
if let Some(text) = delta.text() { /* ... */ }
```

### Added
- **Google Search grounding support** (#25): Enable real-time web search integration with Gemini models
  - New `with_google_search()` builder method on `InteractionBuilder`
  - New types: `GroundingMetadata`, `GroundingChunk`, `WebSource`
  - New helper methods: `has_google_search_metadata()`, `google_search_metadata()`, `has_google_search_calls()`, `google_search_calls()` on `InteractionResponse`
  - Full streaming support via `StreamChunk::Complete`

- **Code execution support** (#26): Enable Python code execution via Gemini's built-in sandbox
  - New `with_code_execution()` builder method on `InteractionBuilder`
  - New `CodeExecutionOutcome` enum with `Ok`, `Failed`, `DeadlineExceeded`, `Unspecified` variants
  - Updated `InteractionContent::CodeExecutionCall` with typed fields: `id`, `language`, `code`
  - Updated `InteractionContent::CodeExecutionResult` with typed fields: `call_id`, `outcome`, `output`
  - New helper methods on `InteractionResponse`: `code_execution_calls()`, `code_execution_results()`, `successful_code_output()`
  - New helper functions: `code_execution_call_content()`, `code_execution_result_content()`, `code_execution_success()`, `code_execution_error()`
  - Backward-compatible deserialization for old API response format
  - **Breaking (serialization)**: `CodeExecutionCall` now serializes `language` and `code` as top-level fields instead of nested in `arguments`. Deserialization remains backward-compatible with both formats.

- **URL context support** (#63): Enable URL content fetching and analysis
  - New `with_url_context()` builder method on `InteractionBuilder`
  - New types: `UrlContextMetadata`, `UrlMetadataEntry`, `UrlRetrievalStatus`
  - New helper methods: `has_url_context_metadata()`, `url_context_metadata()`, `has_url_context_calls()`, `url_context_calls()` on `InteractionResponse`
  - Supports up to 20 URLs per request, max 34MB per URL

- **Structured output JSON schema support** (#80): Enforce JSON schema constraints on model responses
  - Use `.with_response_format(schema)` to specify a JSON schema for structured output
  - Works standalone for structured data extraction
  - Combines with built-in tools (Google Search, URL Context)
  - New comprehensive example: `examples/structured_output.rs`

- **Function call/result structs**: New `FunctionCallInfo` and `FunctionResultInfo` structs with named fields for cleaner access
  - `function_calls()` now returns `Vec<FunctionCallInfo>` with fields: `id`, `name`, `args`, `thought_signature`
  - `function_results()` now returns `Vec<FunctionResultInfo>` with fields: `name`, `call_id`, `result`
  - New `has_function_results()` method on `InteractionResponse` for parity with `has_function_calls()`

### Changed
- **`InteractionContent` is now `#[non_exhaustive]`** (#44): Match statements must include a wildcard arm (`_ => {}`). This allows adding new variants in minor version updates without breaking downstream code.

### Fixed
- **Streaming with function calls now works** (#27): Function call deltas are now properly parsed instead of causing errors
- **Streaming now properly yields content chunks** (#17): The streaming API was returning 0 chunks because the code expected all SSE events to have an `interaction` field, but the API sends different event types (`content.delta` and `interaction.complete`)

#### Simplified Client API
- **`Client::new()` signature simplified**: No longer takes `api_version` parameter
  - Before: `Client::new(api_key, None)`
  - After: `Client::new(api_key)`
  - The `api_version` was stored but never used; the library defaults to V1Beta internally
- **`ApiVersion` no longer re-exported** from rust-genai (still available in genai-client for internal use)

#### Removed deprecated function calling helpers
- **`function_response_content()` helper removed**: Use `function_result_content()` instead
  - Before: `function_response_content("get_weather", json!({"temp": 72}))`
  - After: `function_result_content("get_weather", "call_123", json!({"temp": 72}))`
  - The `call_id` parameter is required for proper API response matching
- **`InteractionContent::FunctionResponse` variant removed**: Use `FunctionResult` variant instead

#### UsageMetadata field names updated (#24)
- **Field names now match Interactions API**: The old GenerateContent API field names have been replaced
  - `prompt_tokens` → `total_input_tokens`
  - `candidates_tokens` → `total_output_tokens`
  - `total_tokens` remains unchanged
- **New fields added**: `total_cached_tokens`, `total_reasoning_tokens`, `total_tool_use_tokens`
- **Token usage now works**: Previously always returned `None` due to field name mismatch

## [0.2.0] - 2025-12-23

### BREAKING CHANGES

This release removes the legacy GenerateContent API in favor of the unified Interactions API. This is a major breaking change that requires code migration.

#### Removed
- **GenerateContent API**: All `GenerateContentBuilder` methods and related functionality removed
  - `Client::with_model()` method removed
  - `GenerateContentBuilder` type removed
  - `generate_from_request()` and `stream_from_request()` methods removed
  - `GenerateContentResponse` type removed (use `InteractionResponse` instead)

- **Helper modules**:
  - `content_api` module removed (use `interactions_api` instead)
  - `internal/response_processing` module removed

- **Examples**: Removed all GenerateContent examples
  - `simple_request.rs`
  - `stream_request.rs`
  - `code_execution.rs`
  - `function_call.rs`
  - `gemini3_thought_signatures.rs`

- **Internal crates**:
  - `genai-client/src/core.rs` removed
  - `genai-client/src/models/request.rs` removed
  - `genai-client/src/models/response.rs` removed

### Added

- **Enhanced InteractionResponse**:
  - New `.text()` convenience method to extract text from interaction responses
  - New `.function_calls()` convenience method to extract function calls with thought signatures

- **Automatic function calling for Interactions API**:
  - New `InteractionBuilder::create_with_auto_functions()` method
  - Auto-discovers and executes functions from the global registry
  - Supports multi-turn function calling with automatic loop handling

- **New helper functions**:
  - `function_result_content()` for sending function execution results (correct API format)
  - Enhanced `function_call_content_with_signature()` to include optional call ID

### Fixed

- **Function calling implementation** now correctly follows Google's Interactions API specification:
  - Added `id` field to `FunctionCall` to capture the call identifier from the API
  - Added new `FunctionResult` content type with `call_id` field (replaces `FunctionResponse`)
  - `create_with_auto_functions()` now sends only function results (not the original calls)
  - The API server maintains function call context via `previous_interaction_id`
  - Deprecated `FunctionResponse` variant (use `FunctionResult` instead)
  - Improved error message when max function call loops (5) is exceeded

### Changed

- **Primary API**: The Interactions API is now the only supported API
- **Migration Path**:
  - Replace `client.with_model(...).with_prompt(...).generate()`
  - With `client.interaction().with_model(...).with_text(...).create()`
  - Replace `generate_with_auto_functions()` with `create_with_auto_functions()`
  - Use `interactions_api` helper functions instead of `content_api`

### Migration Guide

#### Before (v0.1.x - GenerateContent API):
```rust
let response = client
    .with_model("gemini-3-flash-preview")
    .with_prompt("Hello, world!")
    .generate()
    .await?;

println!("{}", response.text.unwrap());
```

#### After (v0.2.0 - Interactions API):
```rust
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Hello, world!")
    .create()
    .await?;

println!("{}", response.text().unwrap_or("No text"));
```

#### Streaming:
```rust
// Before
let stream = client
    .with_model("gemini-3-flash-preview")
    .with_prompt("Hello")
    .generate_stream()?;

// After
let stream = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Hello")
    .create_stream();
```

#### Automatic Function Calling:
```rust
// Before
let response = client
    .with_model("gemini-3-flash-preview")
    .with_prompt("What's the weather?")
    .generate_with_auto_functions()
    .await?;

// After
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("What's the weather?")
    .create_with_auto_functions()
    .await?;
```

## [0.1.0] - 2024-12-XX

### Added
- Initial release
- Support for GenerateContent API
- Support for Interactions API
- Function calling with automatic discovery via macros
- Streaming support for both APIs
- Comprehensive test suite
- Example programs for both APIs
