//! Example: Audio Input with Gemini
//!
//! This example demonstrates how to send audio files to Gemini for transcription,
//! analysis, and question-answering.
//!
//! Supported audio formats: WAV, MP3, AIFF, AAC, OGG, FLAC
//!
//! Run with: cargo run --example audio_input

use rust_genai::{Client, GenaiError};
use std::env;
use std::error::Error;

// A minimal valid WAV file header (44 bytes) - for demonstration purposes only.
// This contains no actual audio data, so the API may reject it or report it as empty/silent.
// In real usage, load actual audio files with content.
const DEMO_WAV_BASE64: &str = "UklGRiQAAABXQVZFZm10IBAAAAABAAEARKwAAIhYAQACABAAZGF0YQAAAAA=";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not found in environment");
    let client = Client::builder(api_key).build();
    let model_name = "gemini-3-flash-preview";

    // =========================================================================
    // Example 1: Basic Audio Transcription (Fluent Builder Pattern)
    // =========================================================================
    println!("=== Example 1: Audio Transcription ===\n");

    // Note: This uses a minimal WAV header for demonstration.
    // In real usage, you would provide actual audio content.
    // Using the fluent builder pattern with add_audio_data()
    let response = client
        .interaction()
        .with_model(model_name)
        .with_text(
            "This is a demo audio file. In real usage, describe what you hear. \
             If the audio is silent or empty, just say 'No audio content detected.'",
        )
        .add_audio_data(DEMO_WAV_BASE64, "audio/wav")
        .create()
        .await;

    match response {
        Ok(r) => {
            if let Some(text) = r.text() {
                println!("Response: {text}\n");
            }
        }
        Err(e) => {
            println!("Note: Demo WAV may not be processable: {e}\n");
        }
    }

    // =========================================================================
    // Example 2: Code Patterns for Audio Analysis
    // =========================================================================
    println!("=== Example 2: Audio Analysis Patterns ===\n");

    println!("Here are common patterns for working with audio:\n");

    println!("1. TRANSCRIPTION (Fluent Pattern):");
    println!(
        r#"
   // Most ergonomic: fluent builder pattern
   let response = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_text("Transcribe this audio with proper punctuation.")
       .add_audio_data(&base64_audio, "audio/mp3")
       .create()
       .await?;

   // Alternative: content vector (useful for dynamic content)
   let contents = vec![
       text_content("Transcribe this audio with proper punctuation."),
       audio_data_content(&base64_audio, "audio/mp3"),
   ];
   let response = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_content(contents)
       .create()
       .await?;
"#
    );

    println!("2. SPEAKER ANALYSIS:");
    println!(
        r#"
   let response = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_text("Analyze this audio:
           - How many speakers are there?
           - What language(s) are spoken?
           - What is the emotional tone?")
       .add_audio_data(&base64_audio, "audio/mp3")
       .create()
       .await?;
"#
    );

    println!("3. CONTENT Q&A:");
    println!(
        r#"
   let response = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_text("In this podcast, what are the main topics discussed?")
       .add_audio_data(&podcast_audio, "audio/mp3")
       .create()
       .await?;
"#
    );

    // =========================================================================
    // Example 3: Multi-turn Conversation about Audio
    // =========================================================================
    println!("=== Example 3: Multi-turn Audio Conversation ===\n");

    println!("Use stateful conversations for follow-up questions:\n");
    println!(
        r#"
   // First turn: Send audio and get initial analysis
   let first = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_text("Summarize this audio recording.")
       .add_audio_data(&base64_audio, "audio/mp3")
       .with_store(true)  // Enable conversation storage
       .create()
       .await?;

   // Second turn: Ask follow-up (audio is remembered)
   let second = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_text("What emotions did you detect in the speaker's voice?")
       .with_previous_interaction(&first.id)
       .create()
       .await?;
"#
    );

    // =========================================================================
    // Example 4: Error Handling
    // =========================================================================
    println!("=== Example 4: Error Handling ===\n");

    // Demonstrate error handling with invalid audio
    let invalid_base64 = "not_valid_audio_data_at_all";

    match client
        .interaction()
        .with_model(model_name)
        .with_text("Transcribe this audio.")
        .add_audio_data(invalid_base64, "audio/mp3")
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
                println!("API error for invalid audio:");
                println!("  Status: {status_code}");
                println!("  Message: {message}\n");
            }
            _ => println!("Error: {e}\n"),
        },
    }

    // =========================================================================
    // Reference: Supported Audio Formats
    // =========================================================================
    println!("=== Supported Audio Formats ===\n");
    println!("Gemini supports these audio formats:");
    println!("  - WAV  (audio/wav)");
    println!("  - MP3  (audio/mp3, audio/mpeg)");
    println!("  - AIFF (audio/aiff)");
    println!("  - AAC  (audio/aac)");
    println!("  - OGG  (audio/ogg)");
    println!("  - FLAC (audio/flac)");
    println!();
    println!("Maximum audio length: ~9.5 hours");
    println!("For files larger than 20MB, use the Files API (not yet implemented).\n");

    // =========================================================================
    // Reference: Loading Audio Files
    // =========================================================================
    println!("=== Loading Audio Files ===\n");
    println!("Option 1: Use the built-in file loading helper (recommended):\n");
    println!(
        r#"
   use rust_genai::audio_from_file;

   // Load audio file with automatic MIME detection and base64 encoding
   let audio_content = audio_from_file("path/to/audio.mp3").await?;

   // Build the request using with_content
   let response = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_content(vec![
           text_content("Transcribe this audio."),
           audio_content,
       ])
       .create()
       .await?;

   // Or use the async builder method
   let response = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_text("Transcribe this audio.")
       .add_audio_file("path/to/audio.mp3").await?
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
   let audio_bytes = fs::read("path/to/audio.mp3")?;
   let base64_audio = base64::engine::general_purpose::STANDARD.encode(&audio_bytes);

   // Send with fluent builder
   let response = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_text("Transcribe this audio.")
       .add_audio_data(&base64_audio, "audio/mp3")
       .create()
       .await?;
"#
    );

    println!("=== Examples Complete ===");
    Ok(())
}
