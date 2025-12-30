//! Example: Echoing Thoughts in Multi-Turn Conversations
//!
//! This example demonstrates how to use `thought_content()` to manually echo
//! the model's thoughts when constructing multi-turn conversations without
//! using `previous_interaction_id`.
//!
//! # When to Use This Pattern
//!
//! In most cases, you should use `with_previous_interaction(id)` which automatically
//! handles thought context on the server. However, manual thought echoing is useful when:
//!
//! - You need to filter or modify the conversation history
//! - You want to store and replay conversations from a database
//! - You're building custom conversation management
//!
//! # Running
//!
//! ```bash
//! cargo run --example thought_echo
//! ```
//!
//! # Prerequisites
//!
//! Set the `GEMINI_API_KEY` environment variable with your API key.

use rust_genai::interactions_api::{text_content, thought_content};
use rust_genai::{Client, InteractionContent, InteractionInput, ThinkingLevel};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable not set");

    let client = Client::builder(api_key).build();

    println!("=== THOUGHT ECHO EXAMPLE ===\n");

    // ==========================================================================
    // Step 1: Initial interaction with thinking enabled
    // ==========================================================================
    println!("--- Step 1: Initial Problem ---\n");

    let initial_prompt = "What is 17 * 23? Think through this step by step.";
    println!("User: {}\n", initial_prompt);

    let response1 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(initial_prompt)
        .with_thinking_level(ThinkingLevel::Medium)
        .with_store(true)
        .create()
        .await?;

    // Display the response
    if response1.has_thoughts() {
        println!("Model's reasoning:");
        for thought in response1.thoughts() {
            println!("  [Thought] {}", thought);
        }
    }

    if let Some(text) = response1.text() {
        println!("\nModel's answer: {}\n", text);
    }

    // ==========================================================================
    // Step 2: Manual multi-turn using thought_content()
    // ==========================================================================
    println!("--- Step 2: Follow-up Question (Manual History) ---\n");

    // Build conversation history manually
    // We include the original prompt, the model's thoughts, and the model's answer
    let mut history: Vec<InteractionContent> = vec![
        // User's original message
        text_content(initial_prompt),
    ];

    // Echo back the model's thoughts using thought_content()
    for thought in response1.thoughts() {
        history.push(thought_content(thought));
    }

    // Echo back the model's answer
    if let Some(text) = response1.text() {
        history.push(text_content(text));
    }

    // Add the follow-up question
    let followup = "Now what is that result divided by 17?";
    history.push(text_content(followup));

    println!("User: {}\n", followup);

    // Create the follow-up request with manual history
    let response2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(InteractionInput::Content(history))
        .with_thinking_level(ThinkingLevel::Medium)
        .with_store(true)
        .create()
        .await?;

    // Display the response
    if response2.has_thoughts() {
        println!("Model's reasoning:");
        for thought in response2.thoughts() {
            println!("  [Thought] {}", thought);
        }
    }

    if let Some(text) = response2.text() {
        println!("\nModel's answer: {}\n", text);
    }

    // ==========================================================================
    // Comparison: Using previous_interaction_id (Recommended)
    // ==========================================================================
    println!("--- Alternative: Using previous_interaction_id (Recommended) ---\n");
    println!("Note: In production, prefer using .with_previous_interaction(id)");
    println!("which handles thought context automatically on the server.\n");

    // First interaction
    let response_auto = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What is 17 * 23?")
        .with_thinking_level(ThinkingLevel::Low)
        .with_store(true)
        .create()
        .await?;

    println!("First response ID: {:?}", response_auto.id);

    // Follow-up using previous_interaction_id
    let followup_auto = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Now divide that by 17.")
        .with_previous_interaction(
            response_auto
                .id
                .as_ref()
                .expect("id should exist when store=true"),
        )
        .with_thinking_level(ThinkingLevel::Low)
        .with_store(true)
        .create()
        .await?;

    if let Some(text) = followup_auto.text() {
        println!("Follow-up answer: {}\n", text);
    }

    // ==========================================================================
    // Usage Notes
    // ==========================================================================
    println!("--- Usage Notes ---\n");
    println!("When to use thought_content():");
    println!("  - Building custom conversation stores/databases");
    println!("  - Filtering or modifying conversation history");
    println!("  - Replaying saved conversations");
    println!();
    println!("When to use with_previous_interaction():");
    println!("  - Simple multi-turn conversations (recommended)");
    println!("  - Server handles thought signatures automatically");
    println!("  - No need to manually track conversation content");

    println!("\n=== END EXAMPLE ===");

    Ok(())
}
