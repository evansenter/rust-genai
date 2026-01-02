# Logging Strategy

This document defines the logging strategy for `rust-genai`, ensuring consistent, secure, and useful log output across the codebase.

## Log Levels

### `error!` - Unrecoverable failures
Use for situations where the operation cannot continue and needs user intervention.

**Use when:**
- Internal logic errors that indicate bugs
- Fatal configuration issues
- Malformed API responses that prevent operation (e.g., missing required fields)

**Examples from codebase:**
```rust
// Missing required call_id field (src/request_builder/auto_functions.rs)
error!(
    "Function call '{}' is missing required call_id field.",
    function_name
);
```

**Not for:**
- API errors (those are returned as `GenaiError`, not logged)
- Recoverable function execution failures (use `warn!` instead)

### `warn!` - Recoverable issues requiring attention
Use for situations that are unusual, potentially problematic, or indicate degraded operation, but where the code can continue.

**Use when:**
- Unknown API types encountered (Evergreen pattern)
- Configuration issues that have reasonable fallbacks
- Large file uploads exceeding recommended limits
- Validation warnings (e.g., empty function names)
- Malformed API responses that can be recovered from
- Function execution failures in auto-function mode (error sent to model for recovery)

**Examples from codebase:**
```rust
// Function execution failure - recoverable (src/request_builder/auto_functions.rs)
warn!(
    "Function execution failed (recoverable): function='{}', error='{}'. \
     The error will be sent to the model, which may retry or adapt.",
    call.name, e
);

// Large file warning (src/multimodal.rs)
log::warn!(
    "File '{}' is {:.1}MB which exceeds the recommended 20MB limit for inline data...",
    path.display(), size_mb
);

// Unknown API type (genai-client/src/models/interactions/content.rs)
log::warn!(
    "Encountered unknown InteractionContent type '{}'. \
     Parse error: {}. \
     This may indicate a new API feature or a malformed response. \
     The content will be preserved in the Unknown variant.",
    content_type, parse_error
);

// Validation warning (genai-client/src/models/shared.rs)
log::warn!(
    "FunctionDeclaration '{}' requires parameter '{}' which is not defined in properties...",
    self.name, req
);
```

### `debug!` - Development and troubleshooting details
Use for information useful during development or when diagnosing issues.

**Use when:**
- API request/response lifecycle events
- Request body contents (JSON format preferred)
- SSE streaming events
- Internal state transitions

**Examples from codebase:**
```rust
// Request body logging (src/client.rs)
fn log_request_body<T: std::fmt::Debug + serde::Serialize>(body: &T) {
    match serde_json::to_string_pretty(body) {
        Ok(json) => log::debug!("Request Body (JSON):\n{json}"),
        Err(_) => log::debug!("Request Body: {body:#?}"),
    }
}

// Interaction lifecycle (src/client.rs)
log::debug!("Creating interaction");
log::debug!("Interaction created: ID={}", response.id);

// SSE events (genai-client/src/interactions.rs)
debug!(
    "SSE event received: event_type={:?}, has_delta={}, has_interaction={}...",
    event.event_type, event.delta.is_some(), event.interaction.is_some()
);

// Auto-function loop lifecycle (src/request_builder/auto_functions.rs)
debug!("Auto-function loop iteration {}/{}", loop_count + 1, max_loops);
debug!("Executing {} function call(s)", function_calls.len());
debug!("Function '{}' executed in {:?}", call.name, duration);
```

### `trace!` - Verbose low-level details
Use for very detailed information that would be overwhelming at debug level.

**Use when:**
- Byte-level streaming data
- Individual token processing
- Parser state transitions

**Note:** Currently not used in the codebase. Consider adding for SSE parser internals if deeper debugging is needed.

## Logging Categories

### 1. API Request/Response Logging

**Current state:** ✅ Implemented at `debug` level

| Event | Level | Location |
|-------|-------|----------|
| Request body | `debug` | `src/client.rs::log_request_body()` |
| Interaction created | `debug` | `src/client.rs::create_interaction()` |
| Interaction retrieved | `debug` | `src/client.rs::get_interaction()` |
| Interaction deleted | `debug` | `src/client.rs::delete_interaction()` |
| Stream chunk received | `debug` | `src/client.rs::create_interaction_stream()` |
| SSE event details | `debug` | `genai-client/src/interactions.rs` |
| Auto-function loop iteration | `debug` | `src/request_builder/auto_functions.rs` |
| Function execution timing | `debug` | `src/request_builder/auto_functions.rs` |

### 2. Unknown/Evergreen Type Handling

**Current state:** ✅ Implemented at `warn` level

All Evergreen-pattern `Unknown` variants log when encountered:

| Type | Location |
|------|----------|
| `InteractionContent::Unknown` | `genai-client/src/models/interactions/content.rs` |
| `InteractionStatus::Unknown` | `genai-client/src/models/interactions/response.rs` |
| `Tool::Unknown` | `genai-client/src/models/shared.rs` |
| `StreamChunk::Unknown` | `genai-client/src/models/interactions/streaming.rs` |
| `AutoFunctionStreamChunk::Unknown` | `src/streaming.rs` |

### 3. Validation Warnings

**Current state:** ✅ Implemented at `warn` level

| Condition | Location |
|-----------|----------|
| Large file (>20MB) | `src/multimodal.rs::load_and_encode_file()` |
| Empty function name | `genai-client/src/models/shared.rs::build()` |
| Missing required parameters | `genai-client/src/models/shared.rs::build()` |
| Empty call_id/thought_signature | `src/interactions_api.rs` |
| max_function_call_loops=0 | `src/request_builder/mod.rs` |
| Function execution failure | `src/request_builder/auto_functions.rs` |
| Function not found in registry | `src/request_builder/auto_functions.rs` |

### 4. Silent Operations

**Current state:** ⚠️ Partially implemented

The following operations silently handle edge cases and may benefit from logging:

| Operation | Current Behavior | Recommendation |
|-----------|------------------|----------------|
| SSE lifecycle events | Silently skipped | Keep silent (expected behavior) |
| Unknown SSE events with interaction field | `warn` logged | ✅ Already logged |
| CodeExecutionCall language fallback | `warn` logged | ✅ Already logged |
| Unknown InteractionInput variant | `warn` logged | ✅ Already logged |

## Sensitive Data Handling

### API Key Protection

**Current state:** ✅ Protected

- `Client` and `ClientBuilder` implement custom `Debug` that shows `[REDACTED]` for API keys
- API keys are passed via URL query parameter (standard for Google APIs)
- Request bodies do not contain API keys

### Request Body Content

**Current state:** ⚠️ Logs at debug level

The `log_request_body()` function logs full request contents including:
- User prompts (text content)
- Base64-encoded media (images, audio, video, documents)
- Function call parameters

**Recommendation:** This is appropriate at `debug` level since:
1. Debug logs are not enabled by default
2. Users who enable debug logging expect detailed output
3. The library is a client-side tool (logs stay local)

**Consider:** Adding a separate `trace` level for base64 content to keep debug logs more readable.

## Structured Logging Considerations

### Current State

The codebase uses string-based log messages. For better machine parseability, consider structured fields in future iterations.

### Potential Improvements

```rust
// Current style
log::debug!("Interaction created: ID={}", response.id);

// Structured style (future consideration)
log::debug!(
    interaction_id = %response.id,
    model = ?response.model,
    "Interaction created"
);
```

This would require:
1. Enabling `kv` feature in the `log` crate
2. Subscribers that support structured logging (e.g., `tracing`)

**Decision:** Keep current string-based approach. The library should remain logging-framework-agnostic. Users can use `tracing-log` bridge if they want structured logs.

## Guidelines for Adding New Logs

### When to Log

1. **Always log** Unknown variants in Evergreen enums
2. **Always log** validation issues that don't fail but may cause problems
3. **Consider logging** fallback behaviors (e.g., defaulting values)
4. **Never log** sensitive user data at `info` level or above

### Message Format

Use consistent message formatting:

```rust
// Good: Action-oriented with key=value context
log::debug!("Creating interaction");
log::debug!("Interaction created: ID={}", response.id);
log::warn!("File '{}' is {:.1}MB which exceeds the recommended 20MB limit...", path, size);

// Good: Explains why something is unusual
log::warn!(
    "Encountered unknown Tool type '{}'. \
     This may indicate a new API feature or a malformed response.",
    tool_type
);

// Bad: Too terse, no context
log::warn!("Unknown type");

// Bad: Exposes implementation details without context
log::debug!("{:?}", internal_state);
```

### Testing Logs

The `test_log` crate can be used for tests that need to verify logging behavior:

```rust
#[test_log::test]
fn test_unknown_type_logs_warning() {
    // ... test that triggers Unknown variant creation
    // Verify warning appears in test output
}
```

> **Note**: `test_log` is not currently used in the codebase. The logging changes are straightforward enough that manual verification via `RUST_LOG=rust_genai=debug` is adequate. Consider adding logging tests if regressions become an issue.

## Integration with User Code

Users can configure logging using any `log`-compatible backend:

```rust
// Simple setup with env_logger
env_logger::init();

// Or with tracing for structured logs
tracing_subscriber::fmt::init();
```

Log filtering by level:
```bash
RUST_LOG=rust_genai=debug cargo run --example simple_interaction
RUST_LOG=genai_client=debug cargo run --example streaming
```

## Wire-Level Debugging with LOUD_WIRE

For zero-config debugging of raw API traffic, use the `LOUD_WIRE` environment variable:

```bash
LOUD_WIRE=1 cargo run --example simple_interaction
```

### LOUD_WIRE vs RUST_LOG

| Feature | RUST_LOG | LOUD_WIRE |
|---------|----------|-----------|
| **Purpose** | Structured logging for all modules | Raw API traffic inspection |
| **Output** | Plain text to configured logger | Pretty-printed JSON to stderr |
| **Filtering** | Per-module level control | All-or-nothing |
| **Colors** | Depends on backend | Always (green requests, red responses, blue SSE) |
| **Base64 data** | Full content at debug level | Truncated to 100 chars |
| **Timestamps** | Depends on backend | Always included with request IDs |
| **SSE streaming** | Individual events at debug | Always included with correlation |

### When to Use Each

**Use RUST_LOG when:**
- Diagnosing internal library behavior
- Filtering specific modules or log levels
- Integrating with your application's logging pipeline
- Production debugging with controlled verbosity

**Use LOUD_WIRE when:**
- "What exactly is being sent to the API?"
- Debugging request/response mismatches
- Sharing API traces for bug reports
- Quick development iteration

### LOUD_WIRE Output Format

```
[LOUD_WIRE] 2026-01-02T10:30:45Z [REQ#1] >>> POST https://...
[LOUD_WIRE] 2026-01-02T10:30:45Z [REQ#1] Body:
{
  "model": "gemini-3-flash-preview",
  "input": "Hello",
  ...
}
[LOUD_WIRE] 2026-01-02T10:30:46Z [REQ#1] <<< 200 OK
[LOUD_WIRE] 2026-01-02T10:30:46Z [REQ#1] SSE:
{
  "delta": {
    "text": "Hello"
  }
}
```

- Request IDs (`[REQ#N]`) correlate requests with their responses and SSE chunks
- Base64 `"data"` fields are truncated: `"data": "AAAA..."`
- File uploads show progress: `>>> UPLOAD "video.mp4" (video/mp4, 150.25 MB)`
