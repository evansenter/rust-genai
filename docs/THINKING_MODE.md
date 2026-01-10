# Thinking Mode Guide

This guide covers Gemini's thinking capabilities, which expose the model's chain-of-thought reasoning process.

## Table of Contents

- [Overview](#overview)
- [Thinking Levels](#thinking-levels)
- [Basic Usage](#basic-usage)
- [Accessing Thoughts](#accessing-thoughts)
- [Thinking Summaries](#thinking-summaries)
- [Streaming with Thinking](#streaming-with-thinking)
- [Cost and Performance](#cost-and-performance)
- [Best Practices](#best-practices)

## Overview

Thinking mode enables the model to "think out loud" before responding, showing its reasoning process. This is useful for:

- Complex problem solving
- Mathematical calculations
- Multi-step reasoning
- Debugging model behavior
- Understanding how the model reaches conclusions

## Thinking Levels

| Level | Description | Token Cost | Use Case |
|-------|-------------|------------|----------|
| `Off` | No reasoning exposed | Lowest | Simple queries |
| `Minimal` | Minimal reasoning | Low | Quick checks |
| `Low` | Light reasoning | Moderate | Simple problems |
| `Medium` | Balanced reasoning | Higher | Moderate complexity |
| `High` | Extensive reasoning | Highest | Complex problems |

Higher levels produce more detailed reasoning but consume more tokens.

## Basic Usage

### Enable Thinking

```rust,ignore
use genai_rs::{Client, ThinkingLevel};

let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Solve step by step: If a train travels 120 miles in 2 hours, what's its speed?")
    .with_thinking_level(ThinkingLevel::Medium)
    .create()
    .await?;
```

### Check for Thoughts

```rust,ignore
if response.has_thoughts() {
    println!("Model used reasoning!");
}
```

## Accessing Thoughts

> **Note**: Thought blocks contain cryptographic signatures for verification, not human-readable reasoning text. See [Thought Signatures](#thought-signatures) for details.

### Check for Thoughts

```rust,ignore
// Check if model used reasoning
if response.has_thoughts() {
    println!("Model used {} thought blocks", response.thought_signatures().count());
}
```

### Get Thought Signatures

```rust,ignore
// Iterate over thought signatures (cryptographic proofs, not readable text)
for signature in response.thought_signatures() {
    // Signatures are for verification, not display
    println!("Thought signature present");
}

// Get final answer
if let Some(text) = response.text() {
    println!("Final Answer: {}", text);
}
```

### Content Summary

```rust,ignore
let summary = response.content_summary();
println!("Thought blocks: {}", summary.thought_count);
println!("Text blocks: {}", summary.text_count);
```

### Reasoning Token Usage

```rust,ignore
if let Some(reasoning_tokens) = response
    .usage
    .as_ref()
    .and_then(|u| u.total_reasoning_tokens)
{
    println!("Tokens used for reasoning: {}", reasoning_tokens);
}
```

## Thinking Summaries

Request a summary of the reasoning process:

```rust,ignore
use genai_rs::{ThinkingLevel, ThinkingSummaries};

let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Explain photosynthesis step by step")
    .with_thinking_level(ThinkingLevel::High)
    .with_thinking_summaries(ThinkingSummaries::Auto)
    .create()
    .await?;
```

### ThinkingSummaries Options

| Option | Behavior |
|--------|----------|
| `Auto` | API decides whether to include summary |
| `None` | No summary included |

## Streaming with Thinking

Thoughts stream before the final response:

```rust,ignore
use futures_util::StreamExt;
use genai_rs::StreamChunk;

let mut stream = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Solve: What is 15% of 240?")
    .with_thinking_level(ThinkingLevel::Medium)
    .create_stream();

let mut in_thought = false;

while let Some(Ok(event)) = stream.next().await {
    if let StreamChunk::Delta(delta) = event.chunk {
        if delta.is_thought() {
            if !in_thought {
                println!("=== Thinking ===");
                in_thought = true;
            }
            if let Some(text) = delta.text() {
                print!("{}", text);
            }
        } else if delta.is_text() {
            if in_thought {
                println!("\n=== Response ===");
                in_thought = false;
            }
            if let Some(text) = delta.text() {
                print!("{}", text);
            }
        }
    }
}
```

## Cost and Performance

### Token Costs

Thinking increases token usage significantly:

| Level | Typical Overhead |
|-------|------------------|
| Off | Baseline |
| Minimal | +10-20% |
| Low | +20-50% |
| Medium | +50-100% |
| High | +100-300% |

Actual overhead varies based on query complexity.

### When to Use Each Level

```rust,ignore
// Simple factual query - no thinking needed
client.interaction()
    .with_text("What is the capital of France?")
    // No with_thinking_level() - defaults to Off

// Math problem - medium thinking
client.interaction()
    .with_text("Calculate compound interest...")
    .with_thinking_level(ThinkingLevel::Medium)

// Complex reasoning - high thinking
client.interaction()
    .with_text("Analyze this philosophical argument...")
    .with_thinking_level(ThinkingLevel::High)
```

### Monitoring Costs

```rust,ignore
if let Some(usage) = &response.usage {
    println!("Input tokens: {:?}", usage.input_tokens);
    println!("Output tokens: {:?}", usage.output_tokens);
    println!("Reasoning tokens: {:?}", usage.total_reasoning_tokens);

    if let Some(total) = usage.total_tokens {
        println!("Total tokens: {}", total);
    }
}
```

## Best Practices

### 1. Match Level to Task Complexity

```rust,ignore
// DON'T: Use high thinking for simple queries
let response = client.interaction()
    .with_text("What color is the sky?")
    .with_thinking_level(ThinkingLevel::High)  // Wasteful!
    .create().await?;

// DO: Use appropriate level
let response = client.interaction()
    .with_text("What color is the sky?")
    // No thinking needed for simple facts
    .create().await?;
```

### 2. Request Thinking for Problem-Solving Prompts

```rust,ignore
// Good prompts for thinking mode
let prompts = [
    "Solve step by step: ...",
    "Analyze this code for bugs: ...",
    "Compare and contrast: ...",
    "Explain your reasoning: ...",
    "Debug this issue: ...",
];
```

### 3. Handle Missing Thoughts Gracefully

```rust,ignore
// Check for thought presence (signatures are cryptographic, not readable)
if response.has_thoughts() {
    let sig_count = response.thought_signatures().count();
    println!("Model used {} thought blocks for reasoning", sig_count);
} else {
    println!("No thought blocks in response");
}
```

### 4. Use for Debugging Model Behavior

```rust,ignore
// Enable thinking to understand why model gave unexpected answer
let response = client.interaction()
    .with_text(&problematic_prompt)
    .with_thinking_level(ThinkingLevel::High)
    .create().await?;

// Check if model used reasoning (signatures are cryptographic, not readable)
if response.has_thoughts() {
    println!("DEBUG - Model used {} thought blocks", response.thought_signatures().count());
}

// Actual reasoning is reflected in the final response text
if let Some(text) = response.text() {
    println!("DEBUG - Model response: {}", text);
}
```

### 5. Combine with Function Calling

Thinking works with function calling to show reasoning about tool use:

```rust,ignore
let response = client.interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("What's the weather in Tokyo and should I bring an umbrella?")
    .with_thinking_level(ThinkingLevel::Medium)
    .with_function(get_weather.declaration())
    .create_with_auto_functions()
    .await?;

// Thoughts may include reasoning about:
// - Whether to call the function
// - How to interpret function results
// - What recommendation to make
```

## Thought Signatures

Thought signatures provide cryptographic verification of model reasoning. See [Google's documentation](https://ai.google.dev/gemini-api/docs/thought-signatures.md.txt) for details.

```rust,ignore
// Check for thought signatures in streaming
if let StreamChunk::Delta(delta) = event.chunk {
    if delta.is_thought_signature() {
        // Handle signature verification
    }
}
```

## Example

See `cargo run --example thinking` for a complete working example.

```bash
GEMINI_API_KEY=your-key cargo run --example thinking
```
