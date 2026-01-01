# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### BREAKING CHANGES

#### Additional Enums Now #[non_exhaustive] (#196)
- **`GenaiError`**: Match statements must include a wildcard arm
- **`FunctionError`**: Match statements must include a wildcard arm
- **`InteractionInput`**: Match statements must include a wildcard arm
- **New `GenaiError::MalformedResponse` variant**: For cases where the API returns 200 OK but with unexpected/malformed content
- This follows [Evergreen principles](https://github.com/google-deepmind/evergreen-spec) for forward-compatible API design

**Migration guide:**
```rust
// Before (exhaustive match):
match error {
    GenaiError::Http(e) => ...,
    GenaiError::Api { .. } => ...,
    // etc.
}

// After (must include wildcard):
match error {
    GenaiError::Http(e) => ...,
    GenaiError::Api { .. } => ...,
    GenaiError::MalformedResponse(msg) => ...,
    _ => ...,  // Required for forward compatibility
}
```

#### URI Content Helpers Require mime_type (#131)
- **Changed signatures**: `image_uri_content()`, `audio_uri_content()`, `video_uri_content()`, `document_uri_content()` now require `mime_type` as a mandatory parameter instead of `Option<String>`
- **Rationale**: Gemini API requires mime_type for URI-based content; making this compile-time enforced prevents runtime API errors

**Migration guide:**
```rust
// Before:
image_uri_content("https://example.com/image.png", Some("image/png".to_string()))

// After:
image_uri_content("https://example.com/image.png", "image/png")
```

#### Tool Enum is Now #[non_exhaustive] (#131)
- **`Tool` enum now includes `#[non_exhaustive]`**: Match statements must include a wildcard arm
- **New `Tool::Unknown` variant**: Captures unrecognized tool types from the API without failing deserialization
- This follows [Evergreen principles](https://github.com/google-deepmind/evergreen-spec) for forward-compatible API design

#### Error Type Consolidation (#131)
- **`InternalError` renamed to `GenaiError`** in `genai-client` crate
- New `Internal` and `InvalidInput` variants for better error categorization
- Users of the public `rust-genai` crate are unaffected (uses the same `GenaiError`)

#### create_with_auto_functions() Returns AutoFunctionResult (#148)
- **Changed return type**: `create_with_auto_functions()` now returns `AutoFunctionResult` instead of `InteractionResponse`
- **New `AutoFunctionResult` type**: Contains both the final response and execution history
- Provides visibility into which functions were called, enabling debugging, logging, and evaluation

**Migration guide:**
```rust
// Before:
let response = builder.create_with_auto_functions().await?;
println!("{}", response.text().unwrap());

// After:
let result = builder.create_with_auto_functions().await?;
println!("{}", result.response.text().unwrap());

// New: Access execution history with timing
for exec in &result.executions {
    println!("Called {} ({:?}) -> {}", exec.name, exec.duration, exec.result);
}
```

### Added

- **Request timeout and token usage helpers** (#228):
  - New `with_timeout(Duration)` on `InteractionBuilder` for per-request timeouts
  - For `create()`: Overall request timeout
  - For `create_stream()`: Per-chunk timeout to detect stalled connections
  - New `GenaiError::Timeout(Duration)` variant returned when requests exceed timeout
  - Token usage helper methods on `InteractionResponse`:
    - `input_tokens()`, `output_tokens()`, `total_tokens()`
    - `reasoning_tokens()`, `cached_tokens()`, `tool_use_tokens()`
  - Warning logged when timeout used with auto-function methods (not yet supported)

- **ToolService trait for dependency injection** (#197):
  - New `ToolService` trait enables tools to access shared state (DB connections, API clients, config)
  - Use `with_tool_service(Arc<dyn ToolService>)` on `InteractionBuilder` to provide tools
  - Service-provided functions take precedence over global `#[tool]` registry functions
  - When a service function shadows a global function, a warning is logged
  - Works with both `create_with_auto_functions()` and `create_stream_with_auto_functions()`

- **Partial results when max_function_call_loops exceeded** (#172):
  - `create_with_auto_functions()` now returns partial results instead of error when limit is hit
  - New `reached_max_loops: bool` field on `AutoFunctionResult` indicates if limit was reached
  - The `response` field contains the last API response (likely with pending function calls)
  - The `executions` vector preserves all function calls that were executed before hitting the limit
  - Enables debugging stuck function loops and accessing partial work
  - New `AutoFunctionStreamChunk::MaxLoopsReached` variant for streaming (parallel change)
  - `AutoFunctionResultAccumulator` now handles `MaxLoopsReached` and sets `reached_max_loops: true`
  - Legacy JSON without `reached_max_loops` field deserializes with default `false`

- **Function execution timing** (#148):
  - `FunctionExecutionResult.duration` tracks how long each function took to execute
  - Duration is serialized as milliseconds for JSON compatibility
  - Useful for performance monitoring, debugging, and optimization

- **Streaming accumulator helper** (#148):
  - New `AutoFunctionResultAccumulator` type to collect `AutoFunctionResult` from streaming
  - Allows combining streaming UI updates with execution history collection
  - Example:
    ```rust
    let mut accumulator = AutoFunctionResultAccumulator::new();
    while let Some(chunk) = stream.next().await {
        if let Some(result) = accumulator.push(chunk?) {
            // Stream complete, result contains full execution history
            println!("Executed {} functions", result.executions.len());
        }
    }
    ```

- **Full `Serialize`/`Deserialize` support for save/resume semantics** (#148, #151):
  - `InteractionResponse` now implements `Serialize` for logging, caching, and persistence
  - `AutoFunctionResult` implements `Serialize` and `Deserialize` for full execution history
  - `FunctionExecutionResult` now implements `Deserialize` for roundtrip serialization
  - `StreamChunk` and `AutoFunctionStreamChunk` implement both traits for streaming event replay
  - New `AutoFunctionStreamChunk::Unknown` variant for forward-compatible deserialization
  - Enables offline replay, testing/mocking, and state persistence for long-running agents

- **New convenience helpers on `InteractionResponse`** (#131):
  - `google_search_call()` - returns first Google Search call (singular)
  - `code_execution_call()` - returns first Code Execution call (singular)
  - `url_context_call()` - returns first URL Context call (singular)

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

- **Logging strategy documentation** (#203): New `docs/LOGGING_STRATEGY.md` with comprehensive guidelines
  - Log level definitions (error/warn/debug) with concrete examples
  - Sensitive data handling (API keys redacted, user content at debug only)
  - Evergreen pattern logging (Unknown variants log at warn level)
  - Debug logging for auto-function loop lifecycle (iteration tracking, execution timing)
  - Enable with `RUST_LOG=rust_genai=debug`

### Changed
- **`InteractionContent` is now `#[non_exhaustive]`** (#44): Match statements must include a wildcard arm (`_ => {}`). This allows adding new variants in minor version updates without breaking downstream code.
- **Deep Research example now requires background mode** (#179): Updated `deep_research.rs` example to reflect API requirement that `background=true` is mandatory for agent interactions. Removed synchronous mode demonstration since it is no longer supported by the API.
- **Function execution failures now log at `warn!` instead of `error!`** (#203): Since function failures are recoverable (the error is sent to the model which can retry or adapt), they are now correctly logged as warnings rather than errors. This aligns with the new logging strategy documented in `docs/LOGGING_STRATEGY.md`.

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
