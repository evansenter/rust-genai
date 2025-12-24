//! Example demonstrating URL context for fetching and analyzing web content.
//!
//! This example shows how to use Gemini's URL context tool to fetch web pages
//! and have the model analyze their content.
//!
//! Run with: cargo run --example url_context

use rust_genai::{Client, GenaiError, UrlRetrievalStatus};
use std::env;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Get API Key from environment variable
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not found in environment");

    // Create the client
    let client = Client::builder(api_key).build();

    // 2. Create an interaction with URL context enabled
    let model_name = "gemini-3-flash-preview";
    let prompt = "Please fetch and summarize the main content from https://example.com. \
                  What is the purpose of this domain?";

    println!("Creating interaction with URL context");
    println!("Model: {model_name}");
    println!("Prompt: {prompt}\n");

    // 3. Send the request with URL context enabled
    match client
        .interaction()
        .with_model(model_name)
        .with_text(prompt)
        .with_url_context() // Enable URL context fetching
        .with_store(true)
        .create()
        .await
    {
        Ok(response) => {
            println!("--- Interaction Response ---");
            println!("Interaction ID: {}", response.id);
            println!("Status: {:?}", response.status);

            // 4. Check URL context metadata (retrieval status for each URL)
            if let Some(metadata) = response.url_context_metadata() {
                println!("\nURL Context Metadata:");
                for entry in &metadata.url_metadata {
                    let status_str = match entry.url_retrieval_status {
                        UrlRetrievalStatus::UrlRetrievalStatusSuccess => "Success",
                        UrlRetrievalStatus::UrlRetrievalStatusUnsafe => "Unsafe (blocked)",
                        UrlRetrievalStatus::UrlRetrievalStatusError => "Error",
                        UrlRetrievalStatus::UrlRetrievalStatusUnspecified => "Unspecified",
                    };
                    println!("  {} - {}", entry.retrieved_url, status_str);
                }
            } else {
                println!("\nNo URL context metadata in response");
            }

            // 5. Display the model's response
            if let Some(text) = response.text() {
                println!("\nModel Response:");
                println!("{text}");
            }

            // 6. Show token usage
            if let Some(usage) = response.usage {
                println!("\nToken Usage:");
                if let Some(input) = usage.total_input_tokens {
                    println!("  Input tokens: {input}");
                }
                if let Some(output) = usage.total_output_tokens {
                    println!("  Output tokens: {output}");
                }
            }
            println!("\n--- End Response ---");
        }
        Err(e) => {
            match &e {
                GenaiError::Api(api_err_msg) => {
                    eprintln!("API Error: {api_err_msg}");
                    // URL context may not be available for all models/regions
                    if api_err_msg.contains("not supported") {
                        eprintln!("Note: URL context may not be available for this model");
                    }
                }
                GenaiError::Http(http_err) => eprintln!("HTTP Error: {http_err}"),
                _ => eprintln!("Error: {e}"),
            }
            return Err(e.into());
        }
    }

    Ok(())
}
