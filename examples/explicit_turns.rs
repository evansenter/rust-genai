//! Explicit multi-turn conversation using Turn arrays.
//!
//! This example demonstrates two ways to have multi-turn conversations without
//! relying on server-side storage (`previous_interaction_id`):
//!
//! 1. **ConversationBuilder** - Fluent API for inline conversation construction
//! 2. **with_turns()** - Direct array of Turn objects for external history
//!
//! Use these approaches when you need:
//! - Stateless deployments where interaction storage isn't used
//! - Custom history management (sliding window, summarization)
//! - Migration from other providers with existing conversation history
//! - Testing with controlled conversation states

use rust_genai::{Client, Turn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");
    let client = Client::new(api_key);

    // Approach 1: Using ConversationBuilder fluent API
    // This is best for building conversations inline with readable syntax
    println!("=== ConversationBuilder Example ===\n");

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

    println!(
        "Model response: {}\n",
        response.text().unwrap_or("No response")
    );

    // Approach 2: Using with_turns() with pre-built history
    // This is best when you have conversation history from an external source
    println!("=== with_turns() Example ===\n");

    let history = vec![
        Turn::user("I'm planning a trip to Paris."),
        Turn::model(
            "Paris is a wonderful destination! The city offers incredible art, cuisine, and architecture. What aspects of Paris are you most interested in exploring?",
        ),
        Turn::user("I love museums and good food."),
        Turn::model(
            "Perfect! For museums, I'd recommend the Louvre, Musée d'Orsay, and Centre Pompidou. For food, try Le Marais for falafel, Saint-Germain for classic bistros, and don't miss the bakeries everywhere for croissants and pain au chocolat.",
        ),
        Turn::user("What's one thing I absolutely shouldn't miss?"),
    ];

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_turns(history)
        .create()
        .await?;

    println!(
        "Model response: {}\n",
        response.text().unwrap_or("No response")
    );

    // Approach 3: Building history dynamically
    // Useful for chatbot applications that manage their own history
    println!("=== Dynamic History Example ===\n");

    let mut history: Vec<Turn> = Vec::new();

    // Simulating a conversation loop
    let user_messages = [
        "Let's play a word game. I'll say a word and you respond with a word that starts with my word's last letter.",
        "Apple",
        "Elephant",
    ];

    for user_msg in user_messages {
        println!("User: {}", user_msg);

        // Add user message to history
        history.push(Turn::user(user_msg));

        // Send full conversation history
        let response = client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_turns(history.clone())
            .create()
            .await?;

        let model_response = response.text().unwrap_or("No response");
        println!("Model: {}\n", model_response);

        // Add model response to history for next turn
        history.push(Turn::model(model_response));
    }

    println!("=== Done ===");

    Ok(())
}
