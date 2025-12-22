# Project Backlog

This document tracks future improvements, refactoring opportunities, and feature ideas for rust-genai.

## High Priority

### Interactions API Builder Pattern
**Impact:** High | **Effort:** ~2-3 hours | **Type:** Enhancement

Add fluent builder API for Interactions API to match the ergonomics of GenerateContent API.

**Current:**
```rust
let request = CreateInteractionRequest {
    model: Some("gemini-3-flash-preview".to_string()),
    agent: None,
    input: InteractionInput::Text("Hello".to_string()),
    previous_interaction_id: None,
    tools: None,
    response_modalities: None,
    response_format: None,
    generation_config: None,
    stream: None,
    background: None,
    store: None,
    system_instruction: None,
};
client.create_interaction(request).await?;
```

**Proposed:**
```rust
client.interaction()
    .with_model("gemini-3-flash-preview")
    .with_input("Hello")
    .with_previous_interaction(&id)
    .with_tools(vec![...])
    .create()  // or .create_stream()
```

**Benefits:**
- Consistent with existing GenerateContent builder pattern
- Better IDE autocomplete and discoverability
- Reduces boilerplate for common use cases
- More Rust-idiomatic API

---

## Medium Priority

### Unify Logging Approach
**Impact:** Medium | **Effort:** ~1 hour | **Type:** Refactoring

Replace ad-hoc println!/eprintln! with structured logging using the `log` crate.

**Current Issues:**
- Mix of `println!` in debug mode
- Inconsistent error logging with `eprintln!`
- No log levels (debug, info, warn, error)
- Debug mode is binary (on/off) rather than filtered by level

**Proposed Changes:**
- Add `log` crate dependency
- Replace all println! with log::debug!
- Replace all eprintln! with log::warn! or log::error!
- Make debug mode control log filtering
- Users can integrate with their preferred logging backend (env_logger, tracing, etc.)

**Files to Update:**
- `src/client.rs` - ~8 println! statements
- `genai-client/src/core.rs` - Error messages
- `genai-client/src/interactions.rs` - Error messages

---

### Consolidate Error Response Handling
**Impact:** Low | **Effort:** ~1-2 hours | **Type:** Refactoring

Extract common error handling pattern into shared helper function.

**Current Pattern (repeated 3+ times):**
```rust
if !response.status().is_success() {
    let error_text = response.text().await.map_err(InternalError::Http)?;
    return Err(InternalError::Api(error_text));
}
```

**Proposed:**
```rust
async fn handle_api_error(response: Response) -> Result<Response, InternalError> {
    if !response.status().is_success() {
        let error_text = response.text().await.map_err(InternalError::Http)?;
        return Err(InternalError::Api(error_text));
    }
    Ok(response)
}

// Usage:
let response = handle_api_error(response).await?;
```

**Files to Update:**
- `genai-client/src/core.rs`
- `genai-client/src/interactions.rs`
- `src/client.rs` (create_interaction, get_interaction, delete_interaction)

---

## Future Features

### Agentic Capabilities
**Impact:** High | **Effort:** ~4-6 hours | **Type:** Feature

Add high-level abstractions for building agentic workflows on top of the Interactions API.

**Status:** 80% ready - Interactions API provides the foundation

**Proposed APIs:**
```rust
// Agent builder
let agent = Agent::builder()
    .with_model("gemini-3-flash-preview")
    .with_tools(vec![...])
    .with_system_instruction("You are a helpful coding assistant")
    .build();

// Conversational agent with memory
let mut conversation = agent.start_conversation();
let response = conversation.send("Hello").await?;
let response2 = conversation.send("What did I just say?").await?;

// Multi-step agent task
let result = agent
    .execute_task("Research and summarize the latest Rust features")
    .with_max_steps(10)
    .with_callback(|step| println!("Step: {step:?}"))
    .await?;
```

**Components Needed:**
- Agent struct wrapping Interactions API (~50 lines)
- Conversation state management (~100 lines)
- Task execution with step tracking (~150 lines)
- Tool execution coordination (~100 lines)

**Estimated Total:** ~400 lines

---

### Gemini Live API Support
**Impact:** High | **Effort:** ~2-3 weeks | **Type:** Feature

Add support for Gemini's real-time bidirectional voice/text API.

**Status:** Not started - significant new work required

**Requirements:**
- WebSocket support (not currently in dependencies)
- Audio streaming (PCM format handling)
- Real-time state management
- Interruption handling
- Voice activity detection integration

**New Dependencies:**
- `tokio-tungstenite` or `async-tungstenite` for WebSocket
- Audio codec support (possibly `opus` or similar)

**Proposed API:**
```rust
let session = client
    .live_session()
    .with_model("gemini-3-flash-preview")
    .with_modalities(vec![Modality::Audio, Modality::Text])
    .connect()
    .await?;

// Send audio
session.send_audio(audio_chunk).await?;

// Receive responses
while let Some(response) = session.next().await {
    match response? {
        LiveResponse::Audio(data) => play_audio(data),
        LiveResponse::Text(text) => println!("{text}"),
        LiveResponse::ToolCall(call) => handle_tool(call),
    }
}
```

**Estimated Total:** ~1500 lines

---

## Completed

### ✅ Extract SSE Parser to Shared Module
**Completed:** 2024 (commit 23ab1ee)

Created `genai-client/src/sse_parser.rs` with generic parsing function, eliminating ~75 lines of duplicated code across 3 files.

### ✅ Implement Interactions API (Phase 2)
**Completed:** 2024

Added full support for Gemini's Interactions API including models, client functions, examples, and tests.

### ✅ Refactor to Endpoint Abstraction (Phase 1)
**Completed:** 2024

Introduced `Endpoint` enum for flexible URL construction supporting multiple API versions.

---

## Contributing

When working on items from this backlog:

1. Create a feature branch from `master`
2. Update this document to move items from their current section to "In Progress" (add a new section if needed)
3. When complete, move to "Completed" section with completion date and relevant commit SHA
4. Consider breaking large features into smaller milestones
