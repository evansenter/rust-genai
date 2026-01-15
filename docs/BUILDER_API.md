# InteractionBuilder API Guide

This guide covers the `InteractionBuilder` fluent API, including method interactions, sharp edges, and common patterns.

## Table of Contents

- [Overview](#overview)
- [Input Methods](#input-methods)
- [Method Interactions](#method-interactions)
- [Sharp Edges](#sharp-edges)
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

## Input Methods

The builder has several ways to set the input content:

| Method | Purpose | Sets Field |
|--------|---------|------------|
| `with_text(str)` | Simple text message | `current_message` |
| `with_history(Vec<Turn>)` | Conversation history | `history` |
| `with_content(Vec<InteractionContent>)` | Raw content (function results) | `content_input` |
| `add_image_file()` / `add_image_data()` / etc. | Multimodal content | `content_input` |
| `conversation()...done()` | Fluent conversation builder | `history` |

### How Inputs Compose at Build Time

At `build()`, inputs are composed based on what's set:

1. **`content_input` + `current_message`**: Merged (text prepended to content)
2. **`history` + `current_message`**: Merged (text appended as user turn)
3. **`content_input` + `history`**: **Error** - incompatible modes
4. **Single input**: Used directly

```text
Priority: content_input > history > current_message

content_input set?
├── Yes
│   └── current_message set?
│       ├── Yes → Content([text, ...content_items])
│       └── No  → Content([...content_items])
└── No
    └── history set?
        ├── Yes
        │   └── current_message set?
        │       ├── Yes → Turns([...history, Turn::user(text)])
        │       └── No  → Turns([...history])
        └── No
            └── current_message set?
                ├── Yes → Text(message)
                └── No  → Error: "Input is required"
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

| Method | Behavior |
|--------|----------|
| `with_text()` | **Replaces** current message |
| `with_history()` | **Replaces** history |
| `with_content()` | **Replaces** content input |
| `add_*()` methods | **Accumulates** content items |

```rust,ignore
// Replacement: second call wins
.with_text("first").with_text("second")  // → "second"

// Accumulation: both are included
.add_image_file("a.jpg").add_image_file("b.jpg")  // → [image_a, image_b]
```

## Sharp Edges

### 1. `content_input` Cannot Combine with `history`

Content input (via `with_content()` or `add_*()` methods) is for single-turn multimodal messages. It cannot be combined with multi-turn history.

```rust,ignore
// ERROR: Cannot combine content with history
client.interaction()
    .with_model("gemini-3-flash-preview")
    .with_history(conversation_history)
    .add_image_file("photo.jpg").await?  // Sets content_input
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

### 2. `with_content()` Replaces, `add_*()` Accumulates

```rust,ignore
// with_content replaces everything
.with_content(items1).with_content(items2)  // → items2 only

// add_* accumulates
.add_image_data(d1, m1).add_image_data(d2, m2)  // → [image1, image2]

// Mixing: with_content then add_* works (accumulates)
.with_content(items).add_image_data(d, m)  // → items + image
```

### 3. Model vs Agent is Mutually Exclusive

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

### 4. `with_agent_config()` Without `with_agent()` is Silently Ignored

```rust,ignore
// agent_config is ignored when using a model
client.interaction()
    .with_model("gemini-3-flash-preview")  // Using model, not agent
    .with_agent_config(DeepResearchConfig::new())  // Silently ignored!
    .with_text("Research AI trends")
    .create()
    .await?;
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

### Methods Available on All States

These methods work regardless of state:
- `with_model()` / `with_agent()`
- `with_text()` / `with_history()` / `with_content()`
- `with_system_instruction()`
- `with_tools()` / `with_function()`
- All `add_*()` multimodal methods
- `create()` / `create_stream()` / `build()`

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
// Good: Grouped by concern
client.interaction()
    // Target
    .with_model("gemini-3-flash-preview")
    // Context
    .with_system_instruction("You are helpful")
    .with_history(history)
    // Input
    .with_text("Current question")
    // Tools
    .with_function(get_weather.declaration())
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
// Good: Readable inline conversation
let response = client.interaction()
    .with_model("gemini-3-flash-preview")
    .conversation()
        .user("What is 2+2?")
        .model("4")
        .user("Times 3?")
        .done()
    .create()
    .await?;

// Alternative: Explicit history (better for dynamic conversations)
let history = vec![
    Turn::user("What is 2+2?"),
    Turn::model("4"),
    Turn::user("Times 3?"),
];
let response = client.interaction()
    .with_model("gemini-3-flash-preview")
    .with_history(history)
    .create()
    .await?;
```

## Related Documentation

- [Conversation Patterns](CONVERSATION_PATTERNS.md) - Multi-turn conversation strategies
- [Multimodal](MULTIMODAL.md) - Working with images, audio, video
- [Function Calling](FUNCTION_CALLING.md) - Tool integration
- [Multi-Turn Function Calling](MULTI_TURN_FUNCTION_CALLING.md) - Function calling in conversations
