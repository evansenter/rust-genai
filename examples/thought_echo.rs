//! Example: Multi-Turn Conversations with Thinking
//!
//! This example demonstrates multi-turn conversations when thinking mode is enabled.
//!
//! # Important API Limitation
//!
//! The Gemini API does **NOT** allow thought content in user input turns. Attempting to
//! send thought blocks in user turns returns: "User turns cannot contain thought blocks."
//!
//! This means the `thought_content()` helper should NOT be used to echo thoughts back
//! to the API. Instead, use `with_previous_interaction(id)` which handles thought
//! context automatically on the server side.
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

use genai_rs::{Client, Content, InteractionInput, ThinkingLevel};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable not set");

    let client = Client::builder(api_key).build()?;

    println!("=== MULTI-TURN WITH THINKING EXAMPLE ===\n");

    // ==========================================================================
    // Method 1: Using previous_interaction_id (RECOMMENDED)
    // ==========================================================================
    println!("--- Method 1: Using previous_interaction_id (Recommended) ---\n");
    println!("This is the correct approach for multi-turn with thinking.\n");

    let initial_prompt = "What is 17 * 23? Think through this step by step.";
    println!("User: {}\n", initial_prompt);

    // First interaction - must enable store for multi-turn
    let response1 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(initial_prompt)
        .with_thinking_level(ThinkingLevel::Medium)
        .with_store_enabled()
        .create()
        .await?;

    // Display thought count (signatures are cryptographic, not human-readable)
    if response1.has_thoughts() {
        let sig_count = response1.thought_signatures().count();
        println!(
            "Model used internal reasoning ({} thought signature(s))",
            sig_count
        );
    }

    if let Some(text) = response1.as_text() {
        println!("\nModel's answer: {}\n", text);
    }

    let interaction_id = response1
        .id
        .as_ref()
        .expect("id should exist when store=true");
    println!("Interaction ID: {}\n", interaction_id);

    // Follow-up using previous_interaction_id - server preserves thought context
    let followup = "Now what is that result divided by 17?";
    println!("User: {}\n", followup);

    let response2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(followup)
        .with_previous_interaction(interaction_id)
        .with_thinking_level(ThinkingLevel::Medium)
        .with_store_enabled()
        .create()
        .await?;

    if response2.has_thoughts() {
        let sig_count = response2.thought_signatures().count();
        println!(
            "Model used internal reasoning ({} thought signature(s))",
            sig_count
        );
    }

    if let Some(text) = response2.as_text() {
        println!("\nModel's answer: {}\n", text);
    }

    // ==========================================================================
    // Method 2: Manual History (TEXT ONLY - thoughts NOT allowed)
    // ==========================================================================
    println!("--- Method 2: Manual History (Text Only) ---\n");
    println!("Note: The API does NOT allow thought blocks in user turns.");
    println!("Only text content can be echoed in manual history.\n");

    let prompt = "What is 13 * 19?";
    println!("User: {}\n", prompt);

    let resp_manual = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(prompt)
        .with_thinking_level(ThinkingLevel::Low)
        .create()
        .await?;

    let answer = resp_manual.as_text().unwrap_or("(no answer)");
    println!("Model's answer: {}\n", answer);

    // Build manual history - TEXT ONLY (no thoughts!)
    let history: Vec<Content> = vec![
        Content::text(prompt),
        Content::text(answer),
        Content::text("Now divide that by 13."),
    ];

    println!("User: Now divide that by 13.\n");

    let resp_followup = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(InteractionInput::Content(history))
        .with_thinking_level(ThinkingLevel::Low)
        .create()
        .await?;

    if let Some(text) = resp_followup.as_text() {
        println!("Model's answer: {}\n", text);
    }

    // =========================================================================
    // Summary
    // =========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Multi-Turn with Thinking Demo Complete\n");

    println!("--- Key Takeaways ---");
    println!("• Use with_previous_interaction() for multi-turn with thinking");
    println!("• The server preserves thought context automatically");
    println!("• Manual history can only contain TEXT - thoughts are rejected");
    println!("• Thought signatures are cryptographic, not human-readable text\n");

    println!("--- What You'll See with LOUD_WIRE=1 ---");
    println!("Method 1 (previous_interaction_id):");
    println!("  [REQ#1] POST with input + thinkingConfig + store:true");
    println!("  [RES#1] completed: thoughts (signature) + text + interaction ID");
    println!("  [REQ#2] POST with input + previousInteractionId");
    println!("  [RES#2] completed: server-side thought context preserved\n");
    println!("Method 2 (manual history, text only):");
    println!("  [REQ#3] POST with input + thinkingConfig");
    println!("  [RES#3] completed: thoughts + text");
    println!("  [REQ#4] POST with manual history (text content only)\n");

    println!("--- Production Considerations ---");
    println!("• Always use with_previous_interaction() for thinking conversations");
    println!("• Manual history is limited to text - thoughts cannot be echoed");
    println!("• Enable with_store_enabled() to get interaction IDs for chaining");
    println!("• Thought signatures are for verification, not user display");

    Ok(())
}
