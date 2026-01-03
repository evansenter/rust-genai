//! Streaming Example
//!
//! This example demonstrates streaming responses from the Gemini API,
//! where text is printed as it arrives rather than waiting for the complete response.
//!
//! # Running
//!
//! ```bash
//! cargo run --example streaming
//! ```
//!
//! # Prerequisites
//!
//! Set the `GEMINI_API_KEY` environment variable with your API key.

use futures_util::StreamExt;
use rust_genai::{Client, StreamChunk};
use std::env;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable not set");

    let client = Client::builder(api_key).build()?;

    println!("=== STREAMING EXAMPLE ===\n");

    let prompt = "Write a short poem about programming. Be creative!";
    println!("User: {}\n", prompt);
    println!("Assistant (streaming): ");

    // Create a streaming request
    let mut stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(prompt)
        .with_store_enabled()
        .create_stream();

    let mut delta_count = 0;
    let mut total_chars = 0;

    // Process the stream as chunks arrive
    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => match chunk {
                StreamChunk::Delta(delta) => {
                    delta_count += 1;
                    // Print text deltas as they arrive
                    if let Some(text) = delta.text() {
                        print!("{}", text);
                        io::stdout().flush()?; // Flush to show immediately
                        total_chars += text.len();
                    }
                    // You could also handle thought deltas here:
                    // if delta.is_thought() { ... }
                }
                StreamChunk::Complete(response) => {
                    // Final response with full metadata
                    println!("\n");
                    println!("--- Stream Complete ---");
                    println!("Interaction ID: {:?}", response.id);
                    println!("Status: {:?}", response.status);
                    if let Some(usage) = response.usage
                        && let Some(total) = usage.total_tokens
                    {
                        println!("Total tokens: {}", total);
                    }
                }
                _ => {} // Handle unknown variants
            },
            Err(e) => {
                eprintln!("\nStream error: {:?}", e);
                break;
            }
        }
    }

    println!("\n--- Stream Stats ---");
    println!("Delta chunks received: {}", delta_count);
    println!("Total characters: {}", total_chars);

    // =========================================================================
    // Summary
    // =========================================================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Streaming Demo Complete\n");

    println!("--- Key Takeaways ---");
    println!("• create_stream() returns a Stream of chunks instead of waiting for full response");
    println!("• StreamChunk::Delta contains incremental text/thought content");
    println!("• StreamChunk::Complete provides final response with usage metadata");
    println!("• Flush stdout after each delta for immediate display\n");

    println!("--- What You'll See with LOUD_WIRE=1 ---");
    println!("  [REQ#1] POST with input text + model + store:true");
    println!("  [RES#1] SSE stream: multiple text deltas → completed\n");

    println!("--- Production Considerations ---");
    println!("• Handle stream errors gracefully (connection drops, timeouts)");
    println!("• Use buffering strategies for high-frequency deltas");
    println!("• Consider progress indicators for long-running streams");
    println!("• StreamChunk::Complete contains the same data as non-streaming response");

    Ok(())
}
