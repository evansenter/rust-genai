//! Structured Output Example
//!
//! This example demonstrates how to use JSON schema to enforce structured output
//! from the Gemini API. The model will return responses that conform to your schema.
//!
//! Run with: cargo run --example structured_output

use rust_genai::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::error::Error;

/// A struct representing the structured output we want from the model.
/// Using serde, we can easily parse the JSON response into this type.
#[derive(Debug, Serialize, Deserialize)]
struct MovieReview {
    title: String,
    year: i32,
    rating: f64,
    genre: String,
    summary: String,
    pros: Vec<String>,
    cons: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Get API key from environment
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not found in environment");
    let client = Client::builder(api_key).build();

    println!("=== Structured Output Examples ===\n");

    // Example 1: Basic structured output
    println!("--- Example 1: Basic Structured Output ---");
    basic_structured_output(&client).await?;

    // Example 2: Structured output with complex nested schema
    println!("\n--- Example 2: Complex Nested Schema ---");
    complex_nested_schema(&client).await?;

    // Example 3: Structured output combined with Google Search
    println!("\n--- Example 3: Structured Output + Google Search ---");
    structured_with_search(&client).await?;

    Ok(())
}

/// Basic example: Extract structured movie review data
async fn basic_structured_output(client: &Client) -> Result<(), Box<dyn Error>> {
    // Define the JSON schema for our expected output
    let schema = json!({
        "type": "object",
        "properties": {
            "title": {"type": "string"},
            "year": {"type": "integer"},
            "rating": {"type": "number"},
            "genre": {"type": "string"},
            "summary": {"type": "string"},
            "pros": {
                "type": "array",
                "items": {"type": "string"}
            },
            "cons": {
                "type": "array",
                "items": {"type": "string"}
            }
        },
        "required": ["title", "year", "rating", "genre", "summary", "pros", "cons"]
    });

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Write a review for the movie 'Inception' (2010). Be concise.")
        .with_response_format(schema)
        .create()
        .await?;

    // The response is guaranteed to be valid JSON matching our schema
    let text = response.text().expect("Should have text response");
    println!("Raw JSON response:\n{}\n", text);

    // Parse into our Rust struct
    let review: MovieReview = serde_json::from_str(text)?;
    println!("Parsed MovieReview struct:");
    println!("  Title: {} ({})", review.title, review.year);
    println!("  Rating: {}/10", review.rating);
    println!("  Genre: {}", review.genre);
    println!("  Summary: {}", review.summary);
    println!("  Pros: {:?}", review.pros);
    println!("  Cons: {:?}", review.cons);

    Ok(())
}

/// Complex example: Nested objects and arrays
async fn complex_nested_schema(client: &Client) -> Result<(), Box<dyn Error>> {
    // A more complex schema with nested structures
    let schema = json!({
        "type": "object",
        "properties": {
            "recipe": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "cuisine": {"type": "string"},
                    "difficulty": {
                        "type": "string",
                        "enum": ["easy", "medium", "hard"]
                    },
                    "prep_time_minutes": {"type": "integer"},
                    "cook_time_minutes": {"type": "integer"}
                },
                "required": ["name", "cuisine", "difficulty"]
            },
            "ingredients": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "item": {"type": "string"},
                        "amount": {"type": "string"},
                        "optional": {"type": "boolean"}
                    },
                    "required": ["item", "amount"]
                }
            },
            "steps": {
                "type": "array",
                "items": {"type": "string"}
            }
        },
        "required": ["recipe", "ingredients", "steps"]
    });

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Give me a simple pasta recipe with 5 ingredients and 4 steps.")
        .with_response_format(schema)
        .create()
        .await?;

    let text = response.text().expect("Should have text response");

    // Parse and pretty-print the JSON
    let json: serde_json::Value = serde_json::from_str(text)?;
    println!("Recipe JSON:\n{}", serde_json::to_string_pretty(&json)?);

    // Extract and display specific fields
    if let Some(recipe) = json.get("recipe") {
        println!(
            "\nRecipe: {} ({} cuisine, {} difficulty)",
            recipe.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown"),
            recipe.get("cuisine").and_then(|v| v.as_str()).unwrap_or("Unknown"),
            recipe.get("difficulty").and_then(|v| v.as_str()).unwrap_or("Unknown")
        );
    }

    Ok(())
}

/// Combining structured output with Google Search for real-time data
async fn structured_with_search(client: &Client) -> Result<(), Box<dyn Error>> {
    // Schema for stock information
    let schema = json!({
        "type": "object",
        "properties": {
            "company": {"type": "string"},
            "ticker": {"type": "string"},
            "current_status": {"type": "string"},
            "recent_news": {
                "type": "array",
                "items": {"type": "string"}
            }
        },
        "required": ["company", "ticker", "current_status"]
    });

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's the current status of Apple Inc (AAPL) stock? Include any recent news.")
        .with_google_search() // Enable real-time web search
        .with_response_format(schema)
        .create()
        .await?;

    let text = response.text().expect("Should have text response");
    let json: serde_json::Value = serde_json::from_str(text)?;
    println!("Stock Info JSON:\n{}", serde_json::to_string_pretty(&json)?);

    // Show grounding metadata if available
    if let Some(metadata) = response.grounding_metadata() {
        println!(
            "\nGrounded with {} sources from web search",
            metadata.grounding_chunks.len()
        );
    }

    Ok(())
}
