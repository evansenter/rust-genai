//! Example: Video Input with Gemini
//!
//! This example demonstrates how to send video files to Gemini for analysis,
//! including scene description, object detection, and content understanding.
//!
//! Supported video formats: MP4, MPEG, MOV, AVI, FLV, MPG, WEBM, WMV, 3GP
//!
//! Run with: cargo run --example video_input

use genai_rs::{Client, GenaiError};
use std::env;
use std::error::Error;

// A minimal valid MP4 file header (ftyp box only) - for demonstration purposes only.
// This contains no actual video frames, so the API may reject it or report no content.
// In real usage, load actual video files with content.
const DEMO_MP4_BASE64: &str = "AAAAIGZ0eXBpc29tAAACAGlzb21pc28yYXZjMW1wNDE=";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not found in environment");
    let client = Client::builder(api_key).build()?;
    let model_name = "gemini-3-flash-preview";

    // =========================================================================
    // Example 1: Basic Video Analysis (Fluent Builder Pattern)
    // =========================================================================
    println!("=== Example 1: Video Analysis ===\n");

    // Note: This uses a minimal MP4 header for demonstration.
    // In real usage, you would use add_video_file() for automatic file loading:
    //   .add_video_file("video.mp4").await?
    let response = client
        .interaction()
        .with_model(model_name)
        .with_text(
            "This is a demo video file. In real usage, describe what you see. \
             If the video is empty or corrupted, just say 'No video content detected.'",
        )
        .add_video_data(DEMO_MP4_BASE64, "video/mp4")
        .create()
        .await;

    match response {
        Ok(r) => {
            if let Some(text) = r.text() {
                println!("Response: {text}\n");
            }
        }
        Err(e) => {
            println!("Note: Demo MP4 may not be processable: {e}\n");
        }
    }

    // =========================================================================
    // Example 2: Code Patterns for Video Analysis
    // =========================================================================
    println!("=== Example 2: Video Analysis Patterns ===\n");

    println!("Here are common patterns for working with video:\n");

    println!("1. SCENE DESCRIPTION (Fluent Pattern):");
    println!(
        r#"
   // Most ergonomic: fluent builder pattern
   let response = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_text("Describe the key scenes in this video. What's happening?")
       .add_video_data(&base64_video, "video/mp4")
       .create()
       .await?;

   // Or load from file with automatic MIME detection:
   let response = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_text("Describe the key scenes in this video.")
       .add_video_file("video.mp4").await?
       .create()
       .await?;
"#
    );

    println!("2. OBJECT/PERSON DETECTION:");
    println!(
        r#"
   let response = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_text("List all the objects and people visible in this video.
           For each, note when they first appear (approximate timestamp).")
       .add_video_data(&base64_video, "video/mp4")
       .create()
       .await?;
"#
    );

    println!("3. ACTION RECOGNITION:");
    println!(
        r#"
   let response = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_text("What actions or activities are being performed in this video?
           Describe the sequence of events.")
       .add_video_data(&base64_video, "video/mp4")
       .create()
       .await?;
"#
    );

    println!("4. VIDEO Q&A:");
    println!(
        r#"
   let response = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_text("How many people are in this video? What are they wearing?")
       .add_video_data(&base64_video, "video/mp4")
       .create()
       .await?;
"#
    );

    // =========================================================================
    // Example 3: Multi-turn Conversation about Video
    // =========================================================================
    println!("=== Example 3: Multi-turn Video Conversation ===\n");

    println!("Use stateful conversations for follow-up questions:\n");
    println!(
        r#"
   // First turn: Send video and get initial analysis
   let first = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_text("Describe what's happening in this video.")
       .add_video_data(&base64_video, "video/mp4")
       .with_store_enabled()  // Enable conversation storage
       .create()
       .await?;

   // Second turn: Ask follow-up (video is remembered)
   let second = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_text("What happens at the 30-second mark?")
       .with_previous_interaction(&first.id)
       .create()
       .await?;

   // Third turn: More specific questions
   let third = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_text("What color is the car in the background?")
       .with_previous_interaction(&second.id)
       .create()
       .await?;
"#
    );

    // =========================================================================
    // Example 4: Combining Video with Audio Analysis
    // =========================================================================
    println!("=== Example 4: Video + Audio Analysis ===\n");

    println!("Gemini can analyze both video and audio tracks:\n");
    println!(
        r#"
   let response = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_text("Analyze this video:
           1. What is shown visually?
           2. What sounds or speech can you hear?
           3. How do the visuals and audio relate?")
       .add_video_data(&base64_video, "video/mp4")
       .create()
       .await?;
"#
    );

    // =========================================================================
    // Example 5: Error Handling
    // =========================================================================
    println!("=== Example 5: Error Handling ===\n");

    // Demonstrate error handling with invalid video
    let invalid_base64 = "not_valid_video_data_at_all";

    match client
        .interaction()
        .with_model(model_name)
        .with_text("Describe this video.")
        .add_video_data(invalid_base64, "video/mp4")
        .create()
        .await
    {
        Ok(response) => {
            if let Some(text) = response.text() {
                println!("Response: {text}\n");
            }
        }
        Err(e) => match &e {
            GenaiError::Api {
                status_code,
                message,
                ..
            } => {
                println!("API error for invalid video:");
                println!("  Status: {status_code}");
                println!("  Message: {message}\n");
            }
            _ => println!("Error: {e}\n"),
        },
    }

    // =========================================================================
    // Reference: Supported Video Formats
    // =========================================================================
    println!("=== Supported Video Formats ===\n");
    println!("Gemini supports these video formats:");
    println!("  - MP4  (video/mp4)");
    println!("  - MPEG (video/mpeg)");
    println!("  - MOV  (video/mov, video/quicktime)");
    println!("  - AVI  (video/avi, video/x-msvideo)");
    println!("  - FLV  (video/x-flv)");
    println!("  - MPG  (video/mpg)");
    println!("  - WEBM (video/webm)");
    println!("  - WMV  (video/wmv)");
    println!("  - 3GP  (video/3gpp)");
    println!();
    println!("Maximum video length: ~1 hour");
    println!("Maximum file size (base64): ~20MB recommended");
    println!("For larger files (20MB-2GB): Use Files API (not yet implemented in this SDK)\n");

    // =========================================================================
    // Reference: Loading Video Files
    // =========================================================================
    println!("=== Loading Video Files ===\n");
    println!("Option 1: Use the built-in file loading helper (recommended):\n");
    println!(
        r#"
   use genai_rs::video_from_file;

   // Load video file with automatic MIME detection and base64 encoding
   let video_content = video_from_file("path/to/video.mp4").await?;

   // Build the request using with_content
   let response = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_content(vec![
           InteractionContent::new_text("Describe what's happening in this video."),
           video_content,
       ])
       .create()
       .await?;

   // Or use the async builder method directly
   let response = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_text("Describe what's happening in this video.")
       .add_video_file("path/to/video.mp4").await?
       .create()
       .await?;
"#
    );

    println!("Option 2: Manual file loading and encoding:\n");
    println!(
        r#"
   use std::fs;
   use base64::Engine;

   // Read and encode
   let video_bytes = fs::read("path/to/video.mp4")?;
   let base64_video = base64::engine::general_purpose::STANDARD.encode(&video_bytes);

   // Send with fluent builder
   let response = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_text("Describe what's happening in this video.")
       .add_video_data(&base64_video, "video/mp4")
       .create()
       .await?;
"#
    );

    println!("Note: For large videos (>20MB), use the Files API to upload first,");
    println!("then reference by URI. This avoids base64 encoding overhead.\n");

    // =========================================================================
    // Summary
    // =========================================================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Video Input Demo Complete\n");

    println!("--- Key Takeaways ---");
    println!("• add_video_data(base64, mime_type) for inline video content");
    println!("• add_video_file(path) loads and encodes video automatically");
    println!("• video_from_file() helper for programmatic content building");
    println!("• Model analyzes both visual content and audio track\n");

    println!("--- What You'll See with LOUD_WIRE=1 ---");
    println!("  [REQ#1] POST with text + inlineData (video base64 truncated)");
    println!("  [RES#1] completed: scene description or analysis\n");
    println!("Multi-turn:");
    println!("  [REQ#2] POST with text + previousInteractionId");
    println!("  [RES#2] completed: timestamp-specific questions answered\n");

    println!("--- Production Considerations ---");
    println!("• Supports MP4, MPEG, MOV, AVI, FLV, WEBM, WMV, 3GP");
    println!("• Maximum video length: ~1 hour");
    println!("• For files >20MB, use Files API (upload_file)");
    println!("• Base64 encoding adds ~33% overhead");

    Ok(())
}
