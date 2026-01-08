# Conversation Patterns Guide

This guide covers patterns for multi-turn conversations, including stateless approaches using Turn arrays and the ConversationBuilder.

## Table of Contents

- [Overview](#overview)
- [Stateful vs Stateless](#stateful-vs-stateless)
- [ConversationBuilder](#conversationbuilder)
- [Turn Arrays](#turn-arrays)
- [Dynamic History Management](#dynamic-history-management)
- [Advanced Patterns](#advanced-patterns)
- [Choosing an Approach](#choosing-an-approach)

## Overview

`rust-genai` supports three approaches to multi-turn conversations:

| Approach | State Storage | Best For |
|----------|--------------|----------|
| **Stateful** (`previous_interaction_id`) | Server-side | Simple apps, persistent context |
| **ConversationBuilder** | Client-side | Inline conversation construction |
| **Turn Arrays** (`with_turns()`) | Client-side | External history, custom management |

## Stateful vs Stateless

### Stateful (Server Storage)

The server maintains conversation history:

```rust,ignore
// Turn 1: Start conversation
let response1 = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("My name is Alice")
    .with_store_enabled()
    .create()
    .await?;

// Turn 2: Server remembers context
let response2 = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("What's my name?")
    .with_previous_interaction(response1.id.as_ref().unwrap())
    .with_store_enabled()
    .create()
    .await?;

// Model responds: "Your name is Alice"
```

**Pros**: Simple, no history management needed
**Cons**: Requires storage, less control over context

### Stateless (Client History)

You manage conversation history:

```rust,ignore
let history = vec![
    Turn::user("My name is Alice"),
    Turn::model("Nice to meet you, Alice!"),
    Turn::user("What's my name?"),
];

let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_turns(history)
    .create()
    .await?;
```

**Pros**: Full control, no server storage needed, portable
**Cons**: Must manage history, larger requests

## ConversationBuilder

Fluent API for inline conversation construction.

### Basic Usage

```rust,ignore
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .conversation()
    .user("What is 2+2?")
    .model("2+2 equals 4.")
    .user("And what's that times 3?")
    .done()
    .create()
    .await?;

// Model responds about 4 * 3 = 12
```

### With System Instructions

```rust,ignore
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_system_instruction("You are a helpful math tutor")
    .conversation()
    .user("I need help with fractions")
    .model("I'd be happy to help! What would you like to know?")
    .user("How do I add 1/2 + 1/4?")
    .done()
    .create()
    .await?;
```

### With Multimodal Content

```rust,ignore
use rust_genai::{Turn, TurnContent, image_data_content, text_content};

// Build multimodal turn manually
let multimodal_turn = Turn {
    role: Role::User,
    content: TurnContent::Parts(vec![
        text_content("What's in this image?"),
        image_data_content(base64_image, "image/png"),
    ]),
};

let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_turns(vec![multimodal_turn])
    .create()
    .await?;
```

## Turn Arrays

Direct array of Turn objects for external history management.

### Creating Turns

```rust,ignore
use rust_genai::Turn;

// Simple text turns
let user_turn = Turn::user("Hello!");
let model_turn = Turn::model("Hi there! How can I help?");

// Build history
let history = vec![
    Turn::user("I'm planning a trip to Tokyo"),
    Turn::model("Tokyo is wonderful! What aspects interest you?"),
    Turn::user("I love food and temples"),
    Turn::model("Great choices! For food, try Tsukiji for sushi..."),
    Turn::user("What's one must-see temple?"),
];

let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_turns(history)
    .create()
    .await?;
```

### From External Sources

```rust,ignore
// Load from database
let db_history = load_conversation_from_db(conversation_id)?;

let history: Vec<Turn> = db_history
    .iter()
    .map(|msg| {
        if msg.is_user {
            Turn::user(&msg.content)
        } else {
            Turn::model(&msg.content)
        }
    })
    .collect();

let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_turns(history)
    .create()
    .await?;
```

## Dynamic History Management

Build history incrementally during a conversation.

### Chat Loop Pattern

```rust,ignore
let mut history: Vec<Turn> = Vec::new();

loop {
    // Get user input
    let user_input = get_user_input()?;
    if user_input == "quit" {
        break;
    }

    // Add user message to history
    history.push(Turn::user(&user_input));

    // Send full history
    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_turns(history.clone())
        .create()
        .await?;

    let model_response = response.text().unwrap_or("No response");
    println!("Model: {}", model_response);

    // Add model response to history
    history.push(Turn::model(model_response));
}
```

### Sliding Window

Limit context to recent turns to manage token costs:

```rust,ignore
const MAX_TURNS: usize = 10;

fn add_to_history(history: &mut Vec<Turn>, turn: Turn) {
    history.push(turn);

    // Keep only recent turns
    if history.len() > MAX_TURNS {
        history.drain(0..history.len() - MAX_TURNS);
    }
}
```

### With Summarization

Summarize old context to preserve information while reducing tokens:

```rust,ignore
async fn summarize_and_trim(
    client: &Client,
    history: &mut Vec<Turn>,
    max_turns: usize,
) -> Result<(), GenaiError> {
    if history.len() <= max_turns {
        return Ok(());
    }

    // Extract old turns to summarize
    let old_turns: Vec<_> = history.drain(0..history.len() - max_turns + 1).collect();

    // Generate summary
    let summary_prompt = format!(
        "Summarize this conversation in 2-3 sentences:\n{}",
        old_turns
            .iter()
            .map(|t| format!("{}: {}", t.role, t.content))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let summary = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(&summary_prompt)
        .create()
        .await?;

    // Insert summary as context at the beginning
    history.insert(0, Turn::user("Previous conversation summary:"));
    history.insert(1, Turn::model(summary.text().unwrap_or("...")));

    Ok(())
}
```

## Advanced Patterns

### Combining with System Instructions

```rust,ignore
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_system_instruction("You are a Python expert. Always provide code examples.")
    .with_turns(history)
    .create()
    .await?;
```

### With Function Calling

Turn arrays work with all features:

```rust,ignore
use rust_genai_macros::tool;

#[tool(description = "Get current weather")]
fn get_weather(city: String) -> String {
    format!("Weather in {}: Sunny, 72°F", city)
}

let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_turns(history)
    .with_function::<get_weather>()
    .create_with_auto_functions()
    .await?;
```

### With Streaming

```rust,ignore
use futures_util::StreamExt;

let mut stream = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_turns(history)
    .create_stream();

while let Some(result) = stream.next().await {
    // Process stream events
}
```

### Branching Conversations

Create conversation branches by cloning history:

```rust,ignore
let base_history = vec![
    Turn::user("I want to learn a programming language"),
    Turn::model("Great! What's your goal?"),
    Turn::user("I want to build web applications"),
];

// Branch 1: Explore Rust
let mut rust_branch = base_history.clone();
rust_branch.push(Turn::user("Tell me about Rust for web development"));

// Branch 2: Explore TypeScript
let mut ts_branch = base_history.clone();
ts_branch.push(Turn::user("Tell me about TypeScript for web development"));

// Both branches maintain the same context up to the branching point
```

## Choosing an Approach

| Scenario | Recommended Approach |
|----------|---------------------|
| Simple chatbot | Stateful (`previous_interaction_id`) |
| Serverless/Lambda | Stateless (Turn arrays) |
| Custom history storage | Turn arrays with `with_turns()` |
| Inline test conversations | ConversationBuilder |
| Migration from other APIs | Turn arrays (convert existing format) |
| Context window management | Turn arrays with sliding window |
| Conversation branching | Turn arrays (clone and modify) |

### Decision Tree

```text
Need persistent server storage?
├── Yes → Use stateful with previous_interaction_id
└── No
    ├── Building conversation inline?
    │   └── Yes → Use ConversationBuilder
    └── Managing external history?
        └── Yes → Use with_turns()
```

## Wire Format

Both ConversationBuilder and `with_turns()` produce the same wire format:

```json
{
  "model": "gemini-3-flash-preview",
  "input": {
    "turns": [
      { "role": "user", "parts": [{ "text": "Hello" }] },
      { "role": "model", "parts": [{ "text": "Hi!" }] },
      { "role": "user", "parts": [{ "text": "How are you?" }] }
    ]
  }
}
```

## Example

See `cargo run --example explicit_turns` for a complete working example.

```bash
GEMINI_API_KEY=your-key cargo run --example explicit_turns
```
