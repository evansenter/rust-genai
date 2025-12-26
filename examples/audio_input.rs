//! Example: Audio Input with Gemini
//!
//! This example demonstrates how to send audio files to Gemini for transcription,
//! analysis, and question-answering.
//!
//! Supported audio formats: WAV, MP3, AIFF, AAC, OGG, FLAC
//!
//! Run with: cargo run --example audio_input

use rust_genai::{Client, GenaiError, InteractionInput, audio_data_content, text_content};
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
    // Example 1: Basic Audio Transcription
    // =========================================================================
    println!("=== Example 1: Audio Transcription ===\n");

    // Note: This uses a minimal WAV header for demonstration.
    // In real usage, you would provide actual audio content.
    let contents = vec![
        text_content(
            "This is a demo audio file. In real usage, describe what you hear. \
             If the audio is silent or empty, just say 'No audio content detected.'",
        ),
        audio_data_content(DEMO_WAV_BASE64, "audio/wav"),
    ];

    let response = client
        .interaction()
        .with_model(model_name)
        .with_input(InteractionInput::Content(contents))
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

    println!("1. TRANSCRIPTION:");
    println!(
        r#"
   let contents = vec![
       text_content("Transcribe this audio with proper punctuation."),
       audio_data_content(&base64_audio, "audio/mp3"),
   ];

   let response = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_input(InteractionInput::Content(contents))
       .create()
       .await?;
"#
    );

    println!("2. SPEAKER ANALYSIS:");
    println!(
        r#"
   let contents = vec![
       text_content("Analyze this audio:
           - How many speakers are there?
           - What language(s) are spoken?
           - What is the emotional tone?"),
       audio_data_content(&base64_audio, "audio/mp3"),
   ];

   let response = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_input(InteractionInput::Content(contents))
       .create()
       .await?;
"#
    );

    println!("3. CONTENT Q&A:");
    println!(
        r#"
   let contents = vec![
       text_content("In this podcast, what are the main topics discussed?"),
       audio_data_content(&podcast_audio, "audio/mp3"),
   ];

   let response = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_input(InteractionInput::Content(contents))
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
   let contents = vec![
       text_content("Summarize this audio recording."),
       audio_data_content(&base64_audio, "audio/mp3"),
   ];

   let first = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_input(InteractionInput::Content(contents))
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

    let invalid_contents = vec![
        text_content("Transcribe this audio."),
        audio_data_content(invalid_base64, "audio/mp3"),
    ];

    match client
        .interaction()
        .with_model(model_name)
        .with_input(InteractionInput::Content(invalid_contents))
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
    println!("To load an audio file and encode it as base64:\n");
    println!("Note: Add `base64 = \"0.22\"` to your Cargo.toml\n");
    println!(
        r#"
   use std::fs;

   // Read the file
   let audio_bytes = fs::read("path/to/audio.mp3")?;

   // Encode as base64 (requires `base64` crate)
   use base64::Engine;
   let base64_audio = base64::engine::general_purpose::STANDARD.encode(&audio_bytes);

   // Send to Gemini
   let contents = vec![
       text_content("Transcribe this audio."),
       audio_data_content(&base64_audio, "audio/mp3"),
   ];

   let response = client
       .interaction()
       .with_model("gemini-3-flash-preview")
       .with_input(InteractionInput::Content(contents))
       .create()
       .await?;
"#
    );

    println!("=== Examples Complete ===");
    Ok(())
}
