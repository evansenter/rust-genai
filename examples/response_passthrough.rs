//! Passing InteractionResponse output to follow-up calls.
//!
//! This example demonstrates how to take the output from one model response and
//! pass it directly to a follow-up call using `as_model_turn()`. This pattern is
//! useful for:
//!
//! - Building conversation history from responses without server-side storage
//! - Implementing custom chat loops with stateless deployments
//! - Testing multi-turn conversations with controlled states
//!
//! # Key Concepts
//!
//! - `response.as_model_turn()` converts a response's outputs into a `Turn`
//! - This `Turn` can then be included in `with_history()` for follow-up calls
//! - The pattern enables stateless multi-turn conversations
//!
//! # Run
//!
//! ```bash
//! cargo run --example response_passthrough
//!
//! # With wire-level debugging
//! LOUD_WIRE=1 cargo run --example response_passthrough
//! ```

use genai_rs::{Client, Turn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key =
        std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable not set");
    let client = Client::new(api_key);

    println!("=== RESPONSE PASSTHROUGH EXAMPLE ===\n");

    // ==========================================================================
    // Step 1: Initial interaction
    // ==========================================================================
    println!("--- Step 1: Initial Request ---\n");

    let response1 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What is 15 * 7? Just give me the number.")
        .create()
        .await?;

    let answer1 = response1.as_text().unwrap_or("No response");
    println!("Response 1: {}\n", answer1);

    // ==========================================================================
    // Step 2: Use as_model_turn() to pass response to follow-up
    // ==========================================================================
    println!("--- Step 2: Follow-up Using as_model_turn() ---\n");

    // Convert the response to a Turn for history
    // This captures all the model's outputs (text, function calls, etc.)
    let model_turn = response1.as_model_turn();

    // Build history with the original exchange + new question
    let history = vec![
        Turn::user("What is 15 * 7? Just give me the number."),
        model_turn, // <-- Response passed through directly
        Turn::user("Now divide that by 3. Just give me the number."),
    ];

    let response2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_history(history)
        .create()
        .await?;

    let answer2 = response2.as_text().unwrap_or("No response");
    println!("Response 2: {}\n", answer2);

    // ==========================================================================
    // Step 3: Continue the chain
    // ==========================================================================
    println!("--- Step 3: Continuing the Chain ---\n");

    let history2 = vec![
        Turn::user("What is 15 * 7? Just give me the number."),
        response1.as_model_turn(),
        Turn::user("Now divide that by 3. Just give me the number."),
        response2.as_model_turn(), // <-- Second response passed through
        Turn::user("What was the original calculation I asked about?"),
    ];

    let response3 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_history(history2)
        .create()
        .await?;

    let answer3 = response3.as_text().unwrap_or("No response");
    println!("Response 3 (recall check): {}\n", answer3);

    // ==========================================================================
    // Alternative: Building history incrementally
    // ==========================================================================
    println!("--- Alternative: Incremental History Building ---\n");

    let mut history: Vec<Turn> = Vec::new();

    // Turn 1
    history.push(Turn::user("Name three primary colors."));

    let resp = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_history(history.clone())
        .create()
        .await?;

    println!("Colors: {}", resp.as_text().unwrap_or("No response"));

    // Add model response to history and continue
    history.push(resp.as_model_turn());
    history.push(Turn::user(
        "Which of those is most commonly associated with the sky?",
    ));

    let resp2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_history(history.clone())
        .create()
        .await?;

    println!("Sky color: {}\n", resp2.as_text().unwrap_or("No response"));

    println!("=== Example Complete ===\n");

    println!("--- What You'll See with LOUD_WIRE=1 ---");
    println!("  [REQ#1] POST with text input");
    println!("  [RES#1] completed: numeric answer");
    println!("  [REQ#2] POST with turns (user + model + user)");
    println!("  [RES#2] completed: division result");
    println!("  [REQ#3] POST with turns (full conversation)");
    println!("  [RES#3] completed: recall of original question\n");

    println!("--- Production Considerations ---");
    println!("• as_model_turn() captures ALL outputs (text, function calls, etc.)");
    println!("• For large conversations, consider implementing a sliding window");
    println!("• Token limits apply to the full history sent in each request");
    println!("• Use with_store_enabled() + with_previous_interaction() for server-side storage");

    Ok(())
}
