//! Integration tests for multimodal file loading functions.
//!
//! These tests serve dual purposes:
//! 1. Verify file loading, base64 encoding, and MIME type detection
//! 2. Validate that the Gemini API successfully accepts and processes the encoded content
//!
//! # Running Tests
//!
//! ```bash
//! cargo test --test temp_file_tests -- --include-ignored --nocapture
//! ```
//!
//! # Prerequisites
//!
//! - `GEMINI_API_KEY` environment variable must be set
//! - Tests create temporary files that are automatically cleaned up

mod common;

use base64::Engine;
use common::{
    TINY_MP4_BASE64, TINY_PDF_BASE64, TINY_RED_PNG_BASE64, TINY_WAV_BASE64, get_client,
    stateful_builder,
};
use rust_genai::{
    InteractionInput, InteractionStatus, audio_from_file, document_from_file, image_from_file,
    text_content, video_from_file,
};
use tempfile::TempDir;

// =============================================================================
// Image File Tests
// =============================================================================

/// Tests loading an image from a temp file using image_from_file().
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_image_from_temp_file() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Create temp directory and file
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let image_path = temp_dir.path().join("test_image.png");

    // Decode base64 and write to file
    let image_bytes = base64::engine::general_purpose::STANDARD
        .decode(TINY_RED_PNG_BASE64)
        .expect("Failed to decode base64");
    std::fs::write(&image_path, &image_bytes).expect("Failed to write image");

    // Load using image_from_file()
    let image_content = image_from_file(&image_path)
        .await
        .expect("Failed to load image from file");

    // Send to API
    let contents = vec![
        text_content("What color is this image? Answer with just the color name."),
        image_content,
    ];

    let response = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await
        .expect("Image interaction failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");

    let text = response.text().unwrap().to_lowercase();
    println!("Color response: {}", text);

    // The tiny PNG is red
    assert!(
        text.contains("red") || text.contains("pink") || text.contains("magenta"),
        "Response should identify the red color: {}",
        text
    );
}

/// Tests that image_from_file() correctly handles different extensions.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_image_from_file_jpeg_extension() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Test with .jpg extension (should detect as image/jpeg)
    let image_path = temp_dir.path().join("test_image.jpg");

    // Write PNG data but with .jpg extension - API should still process it
    // (MIME type detection is based on extension, but content is what matters for API)
    let image_bytes = base64::engine::general_purpose::STANDARD
        .decode(TINY_RED_PNG_BASE64)
        .expect("Failed to decode base64");
    std::fs::write(&image_path, &image_bytes).expect("Failed to write image");

    let image_content = image_from_file(&image_path)
        .await
        .expect("Failed to load image from file");

    let contents = vec![
        text_content("Is this an image? Answer yes or no."),
        image_content,
    ];

    let response = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await
        .expect("Image interaction failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    println!("Response: {:?}", response.text());
}

// =============================================================================
// Document File Tests (PDF)
// =============================================================================

/// Tests loading a PDF from a temp file using document_from_file().
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_pdf_from_temp_file() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let pdf_path = temp_dir.path().join("test_document.pdf");

    // Decode base64 and write to file
    let pdf_bytes = base64::engine::general_purpose::STANDARD
        .decode(TINY_PDF_BASE64)
        .expect("Failed to decode base64");
    std::fs::write(&pdf_path, &pdf_bytes).expect("Failed to write PDF");

    // Load using document_from_file()
    let doc_content = document_from_file(&pdf_path)
        .await
        .expect("Failed to load PDF from file");

    let contents = vec![
        text_content("What text does this PDF contain? Answer with just the text."),
        doc_content,
    ];

    let response = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await
        .expect("PDF interaction failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");

    let text = response.text().unwrap().to_lowercase();
    println!("PDF response: {}", text);

    // The minimal PDF contains "Hello World"
    assert!(
        text.contains("hello") || text.contains("world"),
        "Response should mention PDF content: {}",
        text
    );
}

// =============================================================================
// Document File Tests (Text Formats)
// =============================================================================

/// Tests loading a plain text file using document_from_file().
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_txt_from_temp_file() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let txt_path = temp_dir.path().join("test_document.txt");

    // Write plain text content
    std::fs::write(&txt_path, "The quick brown fox jumps over the lazy dog.")
        .expect("Failed to write TXT");

    let doc_content = document_from_file(&txt_path)
        .await
        .expect("Failed to load TXT from file");

    let contents = vec![
        text_content("What animal jumps in this text? Answer with one word."),
        doc_content,
    ];

    let response = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await
        .expect("TXT interaction failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    let text = response.text().unwrap().to_lowercase();
    println!("TXT response: {}", text);

    assert!(
        text.contains("fox"),
        "Response should mention the fox: {}",
        text
    );
}

/// Tests loading a JSON file using document_from_file().
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_json_from_temp_file() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let json_path = temp_dir.path().join("test_data.json");

    let json_content = r#"{"name": "Alice", "age": 30, "city": "Paris"}"#;
    std::fs::write(&json_path, json_content).expect("Failed to write JSON");

    let doc_content = document_from_file(&json_path)
        .await
        .expect("Failed to load JSON from file");

    let contents = vec![
        text_content("What city is mentioned in this JSON? Answer with just the city name."),
        doc_content,
    ];

    let response = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await
        .expect("JSON interaction failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    let text = response.text().unwrap().to_lowercase();
    println!("JSON response: {}", text);

    assert!(
        text.contains("paris"),
        "Response should mention Paris: {}",
        text
    );
}

/// Tests loading a Markdown file using document_from_file().
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_markdown_from_temp_file() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let md_path = temp_dir.path().join("README.md");

    let md_content = r#"# Project Title

## Features
- Fast performance
- Easy to use
- Well documented
"#;
    std::fs::write(&md_path, md_content).expect("Failed to write Markdown");

    let doc_content = document_from_file(&md_path)
        .await
        .expect("Failed to load Markdown from file");

    let contents = vec![
        text_content("How many features are listed in this markdown? Answer with just a number."),
        doc_content,
    ];

    let response = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await
        .expect("Markdown interaction failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    let text = response.text().unwrap().to_lowercase();
    println!("Markdown response: {}", text);

    assert!(
        text.contains("3") || text.contains("three"),
        "Response should mention 3 features: {}",
        text
    );
}

/// Tests loading a CSV file using document_from_file().
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_csv_from_temp_file() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let csv_path = temp_dir.path().join("data.csv");

    let csv_content = "name,score\nAlice,95\nBob,87\nCarol,92";
    std::fs::write(&csv_path, csv_content).expect("Failed to write CSV");

    let doc_content = document_from_file(&csv_path)
        .await
        .expect("Failed to load CSV from file");

    let contents = vec![
        text_content("Who has the highest score in this CSV? Answer with just the name."),
        doc_content,
    ];

    let response = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await
        .expect("CSV interaction failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    let text = response.text().unwrap().to_lowercase();
    println!("CSV response: {}", text);

    assert!(
        text.contains("alice"),
        "Response should identify Alice as highest: {}",
        text
    );
}

// =============================================================================
// Audio File Tests
// =============================================================================

/// Tests loading an audio file from a temp file using audio_from_file().
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_audio_from_temp_file() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let audio_path = temp_dir.path().join("test_audio.wav");

    let audio_bytes = base64::engine::general_purpose::STANDARD
        .decode(TINY_WAV_BASE64)
        .expect("Failed to decode base64");
    std::fs::write(&audio_path, &audio_bytes).expect("Failed to write audio");

    let audio_content = audio_from_file(&audio_path)
        .await
        .expect("Failed to load audio from file");

    let contents = vec![
        text_content("Is this an audio file? Answer yes or no."),
        audio_content,
    ];

    let response = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await
        .expect("Audio interaction failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");
    println!("Audio response: {:?}", response.text());
}

// =============================================================================
// Video File Tests
// =============================================================================

/// Tests loading a video file from a temp file using video_from_file().
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_video_from_temp_file() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let video_path = temp_dir.path().join("test_video.mp4");

    let video_bytes = base64::engine::general_purpose::STANDARD
        .decode(TINY_MP4_BASE64)
        .expect("Failed to decode base64");
    std::fs::write(&video_path, &video_bytes).expect("Failed to write video");

    let video_content = video_from_file(&video_path)
        .await
        .expect("Failed to load video from file");

    let contents = vec![
        text_content("Is this a video file? Answer yes or no."),
        video_content,
    ];

    let response = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await
        .expect("Video interaction failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");
    println!("Video response: {:?}", response.text());
}

// =============================================================================
// Error Handling Tests
// =============================================================================

/// Tests that image_from_file() returns appropriate error for missing file.
#[tokio::test]
async fn test_image_from_file_not_found() {
    let result = image_from_file("/nonexistent/path/image.png").await;

    assert!(result.is_err(), "Should return error for missing file");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Failed to read file") || err.contains("No such file"),
        "Error should mention file not found: {}",
        err
    );
}

/// Tests that document_from_file() returns appropriate error for unsupported extension.
#[tokio::test]
async fn test_document_from_file_unsupported_extension() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let docx_path = temp_dir.path().join("document.docx");
    std::fs::write(&docx_path, "fake content").expect("Failed to write file");

    let result = document_from_file(&docx_path).await;

    assert!(
        result.is_err(),
        "Should return error for unsupported extension"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Unsupported document extension"),
        "Error should mention unsupported extension: {}",
        err
    );
}

/// Tests that image_from_file() returns appropriate error for wrong category.
#[tokio::test]
async fn test_image_from_file_wrong_category() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mp3_path = temp_dir.path().join("audio.mp3");
    std::fs::write(&mp3_path, "fake content").expect("Failed to write file");

    let result = image_from_file(&mp3_path).await;

    assert!(
        result.is_err(),
        "Should return error for audio file used as image"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("not an image type") || err.contains("audio_from_file"),
        "Error should suggest correct function: {}",
        err
    );
}
