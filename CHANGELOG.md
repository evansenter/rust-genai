# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### BREAKING CHANGES

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
