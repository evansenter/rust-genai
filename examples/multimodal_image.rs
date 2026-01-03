//! Multimodal Image Input Example
//!
//! This example demonstrates sending images to the Gemini API for analysis
//! using base64-encoded image data. It shows both the fluent builder pattern
//! and the manual content vector approach.
//!
//! # Running
//!
//! ```bash
//! cargo run --example multimodal_image
//! ```
//!
//! # Prerequisites
//!
//! Set the `GEMINI_API_KEY` environment variable with your API key.

use rust_genai::{Client, image_data_content, text_content};
use std::env;

// A tiny red PNG image (1x1 pixel) encoded as base64
const TINY_RED_PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==";

// A tiny blue PNG image (1x1 pixel) encoded as base64
const TINY_BLUE_PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable not set");

    let client = Client::builder(api_key).build()?;

    println!("=== MULTIMODAL IMAGE INPUT EXAMPLE ===\n");

    // Method 1: Fluent builder pattern with add_image_data()
    // This is the most ergonomic approach for inline multimodal content
    println!("Sending image to Gemini for analysis...\n");

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What color is this image? Describe it.")
        .add_image_data(TINY_RED_PNG_BASE64, "image/png")
        .with_store_enabled()
        .create()
        .await?;

    println!("--- Response ---");
    println!("Status: {:?}", response.status);
    println!();

    if let Some(text) = response.text() {
        println!("Image Description:");
        println!("{}", text);
    }

    if let Some(usage) = response.usage {
        println!();
        if let Some(total) = usage.total_tokens {
            println!("Total tokens: {}", total);
        }
    }

    println!("\n--- End ---");

    // Method 2: Using with_content() for multiple items
    // Useful when building content programmatically
    println!("\n=== IMAGE COMPARISON ===\n");

    let comparison_contents = vec![
        text_content("Compare these two colored images. What are their colors?"),
        image_data_content(TINY_RED_PNG_BASE64, "image/png"),
        image_data_content(TINY_BLUE_PNG_BASE64, "image/png"),
    ];

    let comparison = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_content(comparison_contents)
        .with_store_enabled()
        .create()
        .await?;

    if let Some(text) = comparison.text() {
        println!("Comparison: {}", text);
    }

    // Demonstrate a follow-up question using conversation context
    println!("\n=== FOLLOW-UP QUESTION ===\n");
    println!("User: Which of those colors is warmer?\n");

    let follow_up = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_previous_interaction(
            comparison
                .id
                .as_ref()
                .expect("id should exist when store=true"),
        )
        .with_text("Which of those colors is warmer?")
        .with_store_enabled()
        .create()
        .await?;

    if let Some(text) = follow_up.text() {
        println!("Assistant: {}", text);
    }

    // =========================================================================
    // Summary
    // =========================================================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Multimodal Image Input Demo Complete\n");

    println!("--- Key Takeaways ---");
    println!("• add_image_data(base64, mime_type) for inline image content");
    println!("• with_content() accepts mixed text + image vectors");
    println!("• image_data_content() helper for building content programmatically");
    println!("• Follow-up questions work with with_previous_interaction()\n");

    println!("--- What You'll See with LOUD_WIRE=1 ---");
    println!("Single image:");
    println!("  [REQ#1] POST with text + inlineData (base64 truncated in logs)");
    println!("  [RES#1] completed: text describing the image\n");
    println!("Multiple images:");
    println!("  [REQ#2] POST with text + 2x inlineData");
    println!("  [RES#2] completed: comparison of both images\n");
    println!("Follow-up:");
    println!("  [REQ#3] POST with text + previousInteractionId");
    println!("  [RES#3] completed: answer using image context\n");

    println!("--- Production Considerations ---");
    println!("• Base64 encoding increases payload size ~33%");
    println!("• For large/repeated files, use Files API instead (upload_file)");
    println!("• MIME type must match actual image format");
    println!("• Model supports PNG, JPEG, GIF, WebP images");

    Ok(())
}
