//! Example: Google Search Grounding with Gemini
//!
//! This example demonstrates how to use Gemini's built-in Google Search
//! grounding capability to get responses informed by real-time web data.
//!
//! Shows both non-streaming and streaming usage.
//!
//! Run with: cargo run --example google_search

use futures_util::StreamExt;
use rust_genai::{Client, GenaiError, StreamChunk};
use std::env;
use std::error::Error;
use std::io::{Write, stdout};

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
            println!("Interaction ID: {:?}", response.id);
            println!("Status: {:?}\n", response.status);

            // 4. Display the model's text response
            if let Some(text) = response.text() {
                println!("Model Response:\n{text}\n");
            }

            // 5. Check if response is grounded and display sources
            if response.has_google_search_metadata() {
                println!("--- Grounding Information ---");

                if let Some(metadata) = response.google_search_metadata() {
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
                GenaiError::Api {
                    status_code,
                    message,
                    request_id,
                } => {
                    eprintln!("API Error (HTTP {}): {}", status_code, message);
                    if let Some(id) = request_id {
                        eprintln!("  Request ID: {}", id);
                    }
                    if message.contains("not supported") || message.contains("not available") {
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

    println!("\n--- End Non-Streaming Response ---");

    // 7. Streaming example with Google Search
    println!("\n=== Streaming with Google Search ===\n");

    let stream_prompt = "What are the top 3 tech news stories today?";
    println!("Prompt: {stream_prompt}\n");
    println!("Response (streaming):");

    let mut stream = client
        .interaction()
        .with_model(model_name)
        .with_text(stream_prompt)
        .with_google_search()
        .create_stream();

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => match chunk {
                StreamChunk::Delta(content) => {
                    if let Some(text) = content.text() {
                        print!("{}", text);
                        stdout().flush()?;
                    }
                }
                StreamChunk::Complete(response) => {
                    println!("\n");
                    if let Some(metadata) = response.google_search_metadata() {
                        println!("Sources ({} total):", metadata.grounding_chunks.len());
                        for chunk in metadata.grounding_chunks.iter().take(3) {
                            println!("  - {} [{}]", chunk.web.title, chunk.web.domain);
                        }
                    }
                }
                _ => {} // Handle unknown variants
            },
            Err(e) => {
                eprintln!("\nStream error: {e}");
                break;
            }
        }
    }

    println!("\n--- End Streaming Response ---");
    Ok(())
}
