//! Example: Thinking/Reasoning Levels
//!
//! This example demonstrates Gemini's thinking capabilities, which expose
//! the model's chain-of-thought reasoning process.
//!
//! # Running
//!
//! ```bash
//! cargo run --example thinking
//! ```
//!
//! # Prerequisites
//!
//! Set the `GEMINI_API_KEY` environment variable with your API key.
//!
//! # Thinking Levels
//!
//! - `minimal`: Minimal reasoning, fastest responses
//! - `low`: Light reasoning for simple problems
//! - `medium`: Balanced reasoning for moderate complexity
//! - `high`: Extensive reasoning for complex problems
//!
//! Higher levels produce more detailed reasoning but consume more tokens.

use futures_util::StreamExt;
use rust_genai::{Client, StreamChunk, ThinkingLevel};
use std::env;
use std::io::{Write, stdout};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable not set");

    let client = Client::builder(api_key).build()?;

    println!("=== THINKING/REASONING LEVELS EXAMPLE ===\n");

    // ==========================================================================
    // Example 1: Basic Thinking with Medium Level
    // ==========================================================================
    println!("--- Example 1: Medium Thinking Level ---\n");

    let prompt = "Solve this step by step: If a train travels 120 miles in 2 hours, \
                  then stops for 30 minutes, then travels another 60 miles in 1 hour, \
                  what is the average speed for the entire journey?";

    println!("Prompt: {}\n", prompt);

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(prompt)
        .with_thinking_level(ThinkingLevel::Medium)
        .with_store_enabled()
        .create()
        .await?;

    // Check if model produced thoughts with visible text
    let thoughts: Vec<_> = response.thoughts().collect();
    if !thoughts.is_empty() {
        println!("=== Model's Reasoning Process ===\n");
        for thought in thoughts {
            println!("{}\n", thought);
        }
        println!("=== End Reasoning ===\n");
    } else if response.has_thoughts() {
        // The API returned Thought blocks but text may not be exposed
        // This can happen when the thinking is processed internally
        println!("(Model processed reasoning internally)\n");
    }

    // Print the final answer
    if let Some(text) = response.text() {
        println!("Final Answer:\n{}\n", text);
    }

    // Show content summary
    let summary = response.content_summary();
    println!(
        "Content: {} thought blocks, {} text blocks\n",
        summary.thought_count, summary.text_count
    );

    // Show token usage for reasoning
    if let Some(reasoning) = response
        .usage
        .as_ref()
        .and_then(|u| u.total_reasoning_tokens)
    {
        println!("Reasoning tokens: {}", reasoning);
    }

    // ==========================================================================
    // Example 2: Comparing Different Thinking Levels
    // ==========================================================================
    println!("--- Example 2: Comparing Thinking Levels ---\n");

    let complex_prompt = "What is the probability of getting exactly 3 heads \
                          when flipping a fair coin 5 times?";

    println!("Prompt: {}\n", complex_prompt);

    for level in [ThinkingLevel::Low, ThinkingLevel::High] {
        println!(">>> Thinking Level: {:?} <<<\n", level);

        let response = client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_text(complex_prompt)
            .with_thinking_level(level)
            .with_store_enabled()
            .create()
            .await?;

        let summary = response.content_summary();

        if response.has_thoughts() {
            // Just show first thought for comparison
            if let Some(first_thought) = response.thoughts().next() {
                let preview = if first_thought.len() > 200 {
                    format!("{}...", &first_thought[..200])
                } else {
                    first_thought.to_string()
                };
                println!("First thought preview: {}\n", preview);
            }
        }

        if let Some(text) = response.text() {
            let preview = if text.len() > 150 {
                format!("{}...", &text[..150])
            } else {
                text.to_string()
            };
            println!("Answer preview: {}\n", preview);
        }

        println!(
            "Stats: {} thoughts, {} text blocks\n",
            summary.thought_count, summary.text_count
        );

        if let Some(usage) = &response.usage {
            if let Some(reasoning) = usage.total_reasoning_tokens {
                println!("Reasoning tokens used: {}", reasoning);
            }
            if let Some(total) = usage.total_output_tokens {
                println!("Total output tokens: {}", total);
            }
        }
        println!();
    }

    // ==========================================================================
    // Example 3: Streaming with Thinking
    // ==========================================================================
    println!("--- Example 3: Streaming Thoughts ---\n");

    let stream_prompt = "Explain why the sky is blue, showing your reasoning.";
    println!("Prompt: {}\n", stream_prompt);

    let mut stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(stream_prompt)
        .with_thinking_level(ThinkingLevel::Medium)
        .create_stream();

    let mut in_thought = false;

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => match chunk {
                StreamChunk::Delta(content) => {
                    if let Some(t) = content.thought() {
                        if !in_thought {
                            print!("\n[THINKING] ");
                            in_thought = true;
                        }
                        print!("{}", t);
                        stdout().flush()?;
                    } else if let Some(t) = content.text() {
                        if in_thought {
                            println!("\n[END THINKING]\n");
                            in_thought = false;
                            print!("[ANSWER] ");
                        }
                        print!("{}", t);
                        stdout().flush()?;
                    }
                    // ThoughtSignature (thought authenticity verification) is silently ignored
                }
                StreamChunk::Complete(response) => {
                    println!("\n");
                    let summary = response.content_summary();
                    println!(
                        "Complete: {} thoughts, {} text blocks",
                        summary.thought_count, summary.text_count
                    );
                }
                _ => {} // Handle unknown variants
            },
            Err(e) => {
                eprintln!("\nStream error: {}", e);
                break;
            }
        }
    }

    // ==========================================================================
    // Usage Notes
    // ==========================================================================
    println!("\n--- Usage Notes ---\n");
    println!("Thinking Level Guide:");
    println!("  minimal - Quick responses, minimal reasoning overhead");
    println!("  low     - Light reasoning for straightforward problems");
    println!("  medium  - Balanced approach, good for most use cases");
    println!("  high    - Extensive reasoning for complex problems");
    println!("\nBest Practices:");
    println!("  1. Use 'medium' for general problem-solving");
    println!("  2. Use 'high' for math, logic, and complex reasoning");
    println!("  3. Check response.has_thoughts() before iterating");
    println!("  4. Monitor total_reasoning_tokens in usage for cost tracking");

    // =========================================================================
    // Summary
    // =========================================================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Thinking/Reasoning Levels Demo Complete\n");

    println!("--- Key Takeaways ---");
    println!("• with_thinking_level() exposes model's chain-of-thought reasoning");
    println!("• Levels: minimal, low, medium (default), high (extensive reasoning)");
    println!("• response.thoughts() iterates over reasoning blocks");
    println!("• Higher levels use more tokens but improve complex problem solving\n");

    println!("--- What You'll See with LOUD_WIRE=1 ---");
    println!("Non-streaming:");
    println!("  [REQ#1] POST with input + thinkingConfig(medium)");
    println!("  [RES#1] completed: thoughts + text (usage includes reasoningTokens)\n");
    println!("Streaming:");
    println!("  [REQ#2] POST streaming with input + thinkingConfig");
    println!("  [RES#2] SSE stream: thought deltas → text deltas → completed\n");

    println!("--- Production Considerations ---");
    println!("• Monitor total_reasoning_tokens in usage for cost tracking");
    println!("• Use 'high' for math, logic, and complex reasoning tasks");
    println!("• Thought content may be internal (not exposed) in some cases");
    println!("• ThoughtSignature provides authenticity verification");

    Ok(())
}
