# InteractionBuilder API Guide

This guide covers the `InteractionBuilder` fluent API, including method naming conventions, interactions, and common patterns.

## Table of Contents

- [Overview](#overview)
- [Method Naming Conventions](#method-naming-conventions)
- [Input Methods](#input-methods)
- [Method Interactions](#method-interactions)
- [Validation Errors](#validation-errors)
- [Typestate Pattern](#typestate-pattern)
- [Best Practices](#best-practices)

## Overview

The `InteractionBuilder` provides a fluent interface for constructing requests to the Gemini API. Methods can be chained in any order (within typestate constraints), and the request is built when you call `create()`, `create_stream()`, or `build()`.

```rust,ignore
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_system_instruction("You are helpful")
    .with_text("Hello!")
    .create()
    .await?;
```

## Method Naming Conventions

Methods follow a consistent naming pattern based on their behavior:

| Prefix | Behavior | Example |
|--------|----------|---------|
| `with_*` | **Configures** a setting (replaces if called twice) | `with_model()`, `with_text()`, `with_history()` |
| `set_*` | **Replaces** a collection entirely | `set_content()`, `set_tools()` |
| `add_*` | **Accumulates** items to a collection | `add_function()`, `add_image_file()`, `add_file()` |

### Complete Method Reference

| Method | Prefix | Behavior | Notes |
|--------|--------|----------|-------|
| **Configuration** |
| `with_model()` | with | replaces | Mutually exclusive with `with_agent()` |
| `with_agent()` | with | replaces | Mutually exclusive with `with_model()` |
| `with_agent_config()` | with | replaces | Requires `with_agent()` |
| `with_system_instruction()` | with | replaces | |
| `with_timeout()` | with | replaces | |
| **Input** |
| `with_text()` | with | replaces | Composes with `with_history()` |
| `with_history()` | with | replaces | Composes with `with_text()` |
| `set_content()` | set | replaces | For function results; incompatible with history |
| **Multimodal (accumulate)** |
| `add_image_file()` | add | accumulates | |
| `add_image_data()` | add | accumulates | |
| `add_image_uri()` | add | accumulates | |
| `add_audio_file()` | add | accumulates | |
| `add_video_file()` | add | accumulates | |
| `add_document_file()` | add | accumulates | |
| `add_file()` | add | accumulates | Files API |
| `add_file_uri()` | add | accumulates | Files API |
| **Tools** |
| `set_tools()` | set | replaces | Raw tool array |
| `add_function()` | add | accumulates | Single function declaration |
| `add_functions()` | add | accumulates | Multiple function declarations |
| `with_tool_service()` | with | replaces | Dependency-injected tools |
| **Server-Side Tools (enable capabilities)** |
| `with_google_search()` | with | accumulates | Enables Google Search |
| `with_code_execution()` | with | accumulates | Enables code execution |
| `with_url_context()` | with | accumulates | Enables URL fetching |
| `with_computer_use()` | with | accumulates | Enables computer use |
| `with_mcp_server()` | with | accumulates | Adds MCP server |
| `with_file_search()` | with | accumulates | Enables file search |

## Input Methods

The builder has several ways to set the input content:

| Method | Purpose | Composes With |
|--------|---------|---------------|
| `with_text(str)` | Simple text message | `with_history()`, `add_*()` |
| `with_history(Vec<Turn>)` | Conversation history | `with_text()` |
| `set_content(Vec<InteractionContent>)` | Raw content (function results) | `with_text()`, `add_*()` |
| `add_*()` methods | Multimodal content | `with_text()`, `set_content()` |
| `conversation()...done()` | Fluent conversation builder | — |

### How Inputs Compose at Build Time

```text
content_input set?
├── Yes
│   └── history set?
│       ├── Yes → ERROR (incompatible)
│       └── No
│           └── current_message set?
│               ├── Yes → Content([text, ...content_items])
│               └── No  → Content([...content_items])
└── No
    └── history set?
        ├── Yes
        │   └── current_message set?
        │       ├── Yes → Turns([...history, Turn::user(text)])
        │       └── No  → Turns([...history])
        └── No
            └── current_message set?
                ├── Yes → Text(message)
                └── No  → ERROR ("Input is required")
```

## Method Interactions

### Order Independence

Most input methods are **order-independent** - calling them in different orders produces the same result:

```rust,ignore
// These are equivalent:
.with_history(h).with_text("question")
.with_text("question").with_history(h)

// These are also equivalent:
.with_text("Describe this").add_image_file("photo.jpg")
.add_image_file("photo.jpg").with_text("Describe this")
```

### Replacement vs Accumulation

```rust,ignore
// Replacement: second call wins
.with_text("first").with_text("second")  // → "second"

// Accumulation: both are included
.add_image_file("a.jpg").add_image_file("b.jpg")  // → [image_a, image_b]

// set_* replaces, add_* accumulates on the same collection
.set_tools(tools1).add_function(func)  // → tools1 + func
.add_function(func1).set_tools(tools2)  // → tools2 only (set replaces!)
```

## Validation Errors

The builder validates configuration at `build()` time and returns clear errors:

### 1. Content Cannot Combine with History

Content input (via `set_content()` or `add_*()` methods) is for single-turn multimodal messages. It cannot be combined with multi-turn history.

```rust,ignore
// ERROR: Cannot combine content with history
client.interaction()
    .with_model("gemini-3-flash-preview")
    .with_history(conversation_history)
    .add_image_file("photo.jpg").await?
    .build()  // Returns Err!
```

**Workaround**: For multimodal multi-turn, build `Turn` objects with content arrays:

```rust,ignore
use genai_rs::{Turn, TurnContent, InteractionContent};

let multimodal_turn = Turn {
    role: Role::User,
    content: TurnContent::Parts(vec![
        InteractionContent::new_text("What's in this image?"),
        InteractionContent::new_image_data(base64_data, "image/png"),
    ]),
};

client.interaction()
    .with_model("gemini-3-flash-preview")
    .with_history(vec![...history, multimodal_turn])
    .create()
    .await?;
```

### 2. Model vs Agent is Mutually Exclusive

You must specify exactly one of `with_model()` or `with_agent()`:

```rust,ignore
// ERROR: Both specified
client.interaction()
    .with_model("gemini-3-flash-preview")
    .with_agent("deep-research-pro-preview-12-2025")
    .build()  // Returns Err!

// ERROR: Neither specified
client.interaction()
    .with_text("Hello")
    .build()  // Returns Err!
```

### 3. Agent Config Requires Agent

`with_agent_config()` is only valid when using `with_agent()`:

```rust,ignore
// ERROR: agent_config without agent
client.interaction()
    .with_model("gemini-3-flash-preview")
    .with_agent_config(DeepResearchConfig::new())
    .with_text("Research AI trends")
    .build()  // Returns Err!
```

## Typestate Pattern

The builder uses Rust's type system to enforce API constraints at compile time. The `State` parameter tracks builder state:

```text
                     FirstTurn
                         │
        ┌────────────────┴────────────────┐
        │                                 │
        ▼                                 ▼
     Chained                        StoreDisabled
(via previous_interaction)       (via store_disabled)
```

### State Constraints

| State | Unavailable Methods | Reason |
|-------|---------------------|--------|
| `Chained` | `with_store_disabled()` | Chained requires storage |
| `StoreDisabled` | `with_previous_interaction()` | Requires storage |
| `StoreDisabled` | `with_background(true)` | Requires storage |
| `StoreDisabled` | `create_with_auto_functions()` | Requires storage |

```rust,ignore
// Compile-time error: can't disable store after chaining
client.interaction()
    .with_previous_interaction("id-123")  // Now in Chained state
    .with_store_disabled()  // ERROR: method not available
```

## Best Practices

### 1. Use Specific Input Methods

Prefer the specific method for your use case:

```rust,ignore
// Good: Clear intent
.with_text("Hello")  // Simple text
.with_history(turns)  // Multi-turn
.add_image_file("photo.jpg")  // Multimodal

// Avoid: Generic method is less clear
.with_input(InteractionInput::Text("Hello".to_string()))
```

### 2. Chain Related Configuration

Group related builder calls together:

```rust,ignore
client.interaction()
    // Target
    .with_model("gemini-3-flash-preview")
    // Context
    .with_system_instruction("You are helpful")
    .with_history(history)
    // Input
    .with_text("Current question")
    // Tools
    .add_function(get_weather.declaration())
    // Execute
    .create()
    .await?;
```

### 3. Handle Errors at Build Time

Call `build()` explicitly when you want to validate without executing:

```rust,ignore
let request = client.interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Hello")
    .build()?;  // Validate configuration

// Later: execute
let response = client.execute(request).await?;
```

### 4. Use ConversationBuilder for Inline Conversations

For test fixtures or inline conversation construction:

```rust,ignore
let response = client.interaction()
    .with_model("gemini-3-flash-preview")
    .conversation()
        .user("What is 2+2?")
        .model("4")
        .user("Times 3?")
        .done()
    .create()
    .await?;
```

## Related Documentation

- [Conversation Patterns](CONVERSATION_PATTERNS.md) - Multi-turn conversation strategies
- [Multimodal](MULTIMODAL.md) - Working with images, audio, video
- [Function Calling](FUNCTION_CALLING.md) - Tool integration
- [Multi-Turn Function Calling](MULTI_TURN_FUNCTION_CALLING.md) - Function calling in conversations
