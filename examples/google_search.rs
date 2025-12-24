//! Example: Google Search Grounding with Gemini
//!
//! This example demonstrates how to use Gemini's built-in Google Search
//! grounding capability to get responses informed by real-time web data.
//!
//! Run with: cargo run --example google_search

use rust_genai::{Client, GenaiError};
use std::env;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Get API Key from environment variable
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not found in environment");

    // Create the client
    let client = Client::builder(api_key).build();

    // 2. Create an interaction with Google Search enabled
    let model_name = "gemini-3-flash-preview";
    let prompt = "What are the latest developments in Rust programming language in 2024? \
                  Include specific version numbers and features.";

    println!("Creating interaction with model: {model_name}");
    println!("Prompt: {prompt}\n");

    // 3. Send the interaction request with Google Search grounding
    match client
        .interaction()
        .with_model(model_name)
        .with_text(prompt)
        .with_google_search() // Enable real-time web search
        .with_store(true)
        .create()
        .await
    {
        Ok(response) => {
            println!("--- Google Search Grounded Response ---");
            println!("Interaction ID: {}", response.id);
            println!("Status: {:?}\n", response.status);

            // 4. Display the model's text response
            if let Some(text) = response.text() {
                println!("Model Response:\n{text}\n");
            }

            // 5. Check if response is grounded and display sources
            if response.has_grounding() {
                println!("--- Grounding Information ---");

                if let Some(metadata) = response.grounding_metadata() {
                    // Display search queries used
                    if !metadata.web_search_queries.is_empty() {
                        println!("Search Queries:");
                        for query in &metadata.web_search_queries {
                            println!("  - {query}");
                        }
                        println!();
                    }

                    // Display web sources
                    if !metadata.grounding_chunks.is_empty() {
                        println!("Web Sources ({} total):", metadata.grounding_chunks.len());
                        for (i, chunk) in metadata.grounding_chunks.iter().enumerate() {
                            println!("  {}. {} [{}]", i + 1, chunk.web.title, chunk.web.domain);
                            println!("     {}", chunk.web.uri);
                        }
                    }
                }
            } else {
                println!("Note: No grounding metadata returned (may vary by API response)");
            }

            // 6. Show token usage
            if let Some(usage) = response.usage {
                println!("\n--- Token Usage ---");
                if let Some(input) = usage.total_input_tokens {
                    println!("  Input tokens: {input}");
                }
                if let Some(output) = usage.total_output_tokens {
                    println!("  Output tokens: {output}");
                }
            }
        }
        Err(e) => {
            match &e {
                GenaiError::Api(api_err_msg) => {
                    eprintln!("API Error: {api_err_msg}");
                    if api_err_msg.contains("not supported")
                        || api_err_msg.contains("not available")
                    {
                        eprintln!(
                            "Note: Google Search grounding may not be available in all regions or accounts."
                        );
                    }
                }
                GenaiError::Http(http_err) => eprintln!("HTTP Error: {http_err}"),
                GenaiError::Json(json_err) => eprintln!("JSON Error: {json_err}"),
                _ => eprintln!("Error: {e}"),
            }
            return Err(e.into());
        }
    }

    println!("\n--- End Response ---");
    Ok(())
}
