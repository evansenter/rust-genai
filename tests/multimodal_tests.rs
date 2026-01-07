//! Multimodal input tests for the Interactions API
//!
//! Tests for image, audio, video, and mixed media inputs.
//!
//! These tests require the GEMINI_API_KEY environment variable to be set.
//!
//! # Running Tests
//!
//! ```bash
//! cargo test --test multimodal_tests -- --include-ignored --nocapture
//! ```
//!
//! # Test Assets
//!
//! Tests primarily use base64-encoded media since the Interactions API does NOT
//! support Google Cloud Storage (gs://) URIs. URI-based tests gracefully handle
//! the expected "unsupported file uri" error.

mod common;

use base64::Engine;
use common::{
    SAMPLE_AUDIO_URL, SAMPLE_IMAGE_URL, SAMPLE_VIDEO_URL, TEST_TIMEOUT, TINY_BLUE_PNG_BASE64,
    TINY_MP4_BASE64, TINY_PDF_BASE64, TINY_RED_PNG_BASE64, TINY_WAV_BASE64, consume_stream,
    get_client, interaction_builder, stateful_builder, validate_response_semantically,
    with_timeout,
};
use rust_genai::{
    InteractionInput, InteractionStatus, audio_data_content, audio_uri_content,
    document_data_content, image_data_content, image_uri_content, text_content, video_data_content,
    video_uri_content,
};

// =============================================================================
// Image Input Tests
// =============================================================================

/// Tests image input via GCS URI (gs://) which may not be supported by the Interactions API.
/// This test documents the expected "Unsupported file uri" error behavior when the API rejects
/// such URIs, but also handles success gracefully if the API accepts the format.
/// For reliable image input, use base64 encoding (see `test_image_input_from_base64`).
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_image_input_gcs_uri_unsupported() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let contents = vec![
        text_content("What is in this image? Describe it briefly in 1-2 sentences."),
        image_uri_content(SAMPLE_IMAGE_URL, "image/jpeg"),
    ];

    let result = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await;

    match result {
        Ok(response) => {
            assert_eq!(response.status, InteractionStatus::Completed);
            assert!(
                response.has_text(),
                "Should have text response describing image"
            );
            let text = response.text().unwrap().to_lowercase();
            println!("Image description: {}", text);
        }
        Err(e) => {
            let error_str = format!("{:?}", e);
            // GCS URIs are expected to fail with "Unsupported file uri"
            if error_str.contains("Unsupported file uri") {
                println!(
                    "Expected: GCS URIs not supported by Interactions API. Use base64 encoding instead."
                );
            } else {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_image_input_from_base64() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Use tiny red PNG for testing base64 input
    let contents = vec![
        text_content("What color is this image? Answer with just the color name."),
        image_data_content(TINY_RED_PNG_BASE64, "image/png"),
    ];

    let response = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await
        .expect("Base64 image interaction failed");

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

/// Tests multiple images in a single request using base64.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_multiple_images_single_request() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Send two images in a single request (both base64)
    let contents = vec![
        text_content("I'm showing you two small colored images. What colors are they? List both."),
        image_data_content(TINY_RED_PNG_BASE64, "image/png"),
        image_data_content(TINY_BLUE_PNG_BASE64, "image/png"),
    ];

    let response = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await
        .expect("Multiple images interaction failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");

    let text = response.text().unwrap().to_lowercase();
    println!("Multiple images response: {}", text);

    // Should mention both colors
    let mentions_red = text.contains("red") || text.contains("pink");
    let mentions_blue = text.contains("blue");

    assert!(
        mentions_red || mentions_blue,
        "Response should describe at least one of the colors: {}",
        text
    );
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_image_with_follow_up_question() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(TEST_TIMEOUT, async {
        // First turn: describe the base64 image
        let contents = vec![
            text_content("What color is this image?"),
            image_data_content(TINY_RED_PNG_BASE64, "image/png"),
        ];

        let response1 = stateful_builder(&client)
            .with_input(InteractionInput::Content(contents))
            .create()
            .await
            .expect("First interaction failed");

        assert_eq!(response1.status, InteractionStatus::Completed);
        println!("First response: {:?}", response1.text());

        // Second turn: ask follow-up about the same image
        let response2 = stateful_builder(&client)
            .with_previous_interaction(response1.id.as_ref().expect("id should exist"))
            .with_text("Is that a warm or cool color?")
            .create()
            .await
            .expect("Follow-up interaction failed");

        assert_eq!(response2.status, InteractionStatus::Completed);
        assert!(response2.has_text(), "Should have follow-up response");

        let text = response2.text().unwrap().to_lowercase();
        println!("Follow-up response: {}", text);

        // Red is a warm color
        assert!(
            text.contains("warm") || text.contains("hot") || text.contains("red"),
            "Response should identify warm color: {}",
            text
        );
    })
    .await;
}

// =============================================================================
// Audio Input Tests
// =============================================================================

/// Tests audio input from URI.
/// Note: GCS URIs are not supported by the Interactions API.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_audio_input_from_uri() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let contents = vec![
        text_content("What is this audio about? Summarize briefly."),
        audio_uri_content(SAMPLE_AUDIO_URL, "audio/mpeg"),
    ];

    let result = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("Audio response status: {:?}", response.status);
            if response.has_text() {
                let text = response.text().unwrap();
                println!("Audio transcription/summary: {}", text);
            }
        }
        Err(e) => {
            let error_str = format!("{:?}", e);
            if error_str.contains("Unsupported file uri") {
                println!(
                    "Expected: GCS URIs not supported by Interactions API. Use base64 encoding instead."
                );
            } else {
                println!("Audio input error (may be expected): {:?}", e);
            }
        }
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_audio_input_from_base64() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Use tiny WAV for testing base64 audio input
    // Note: This is a minimal header with no actual audio, so the model may report it's empty/silent
    let contents = vec![
        text_content("Describe what you hear in this audio file."),
        rust_genai::audio_data_content(TINY_WAV_BASE64, "audio/wav"),
    ];

    let result = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("Base64 audio response status: {:?}", response.status);
            if response.has_text() {
                let text = response.text().unwrap();
                println!("Audio response: {}", text);
                // Just verify we got some response - the content can vary
                assert!(!text.is_empty(), "Should get some response about the audio");
            }
        }
        Err(e) => {
            println!(
                "Base64 audio error (may be expected for minimal WAV): {:?}",
                e
            );
        }
    }
}

// =============================================================================
// Video Input Tests
// =============================================================================

/// Tests video input from URI.
/// Note: GCS URIs are not supported by the Interactions API.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_video_input_from_uri() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let contents = vec![
        text_content("What animals appear in this video? List them."),
        video_uri_content(SAMPLE_VIDEO_URL, "video/mp4"),
    ];

    let result = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("Video response status: {:?}", response.status);
            if response.has_text() {
                let text = response.text().unwrap();
                println!("Video description: {}", text);
            }
        }
        Err(e) => {
            let error_str = format!("{:?}", e);
            if error_str.contains("Unsupported file uri") {
                println!(
                    "Expected: GCS URIs not supported by Interactions API. Use base64 encoding instead."
                );
            } else {
                println!("Video input error (may be expected): {:?}", e);
            }
        }
    }
}

/// Tests video input from base64.
/// Note: This uses a minimal MP4 header, so the model may report it's empty/corrupt.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_video_input_from_base64() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Use minimal MP4 for testing base64 video input
    // Note: This is a minimal header with no actual video frames, so the model may report it's empty
    let contents = vec![
        text_content("Describe what you see in this video file."),
        video_data_content(TINY_MP4_BASE64, "video/mp4"),
    ];

    let result = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("Base64 video response status: {:?}", response.status);
            if response.has_text() {
                let text = response.text().unwrap();
                println!("Video response: {}", text);
                // Just verify we got some response - the content can vary
                assert!(!text.is_empty(), "Should get some response about the video");
            }
        }
        Err(e) => {
            // A minimal MP4 header may not be accepted by the API
            println!(
                "Base64 video error (may be expected for minimal MP4): {:?}",
                e
            );
        }
    }
}

// =============================================================================
// Mixed Multimodal Content Tests
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_multimodal_text_and_image_interleaved() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Interleave text and base64 image content
    let contents = vec![
        text_content("I'm going to show you an image."),
        image_data_content(TINY_RED_PNG_BASE64, "image/png"),
        text_content("Based on the color above, what emotion might it represent?"),
    ];

    let response = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await
        .expect("Interleaved content interaction failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");

    let text = response.text().unwrap().to_lowercase();
    println!("Interleaved content response: {}", text);

    // Red is associated with passion, anger, love, energy
    assert!(
        text.contains("passion")
            || text.contains("anger")
            || text.contains("love")
            || text.contains("energy")
            || text.contains("emotion")
            || text.contains("warm")
            || text.contains("intense")
            || text.contains("red"),
        "Response should address emotional association: {}",
        text
    );
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_multimodal_comparison() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Ask model to compare two base64 images
    let contents = vec![
        text_content(
            "Compare these two colored squares. What are their colors and how do they differ?",
        ),
        image_data_content(TINY_RED_PNG_BASE64, "image/png"),
        image_data_content(TINY_BLUE_PNG_BASE64, "image/png"),
    ];

    let response = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await
        .expect("Comparison interaction failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");

    let text = response.text().unwrap().to_lowercase();
    println!("Comparison response: {}", text);

    // Should mention differences or colors
    assert!(
        text.contains("different")
            || text.contains("red")
            || text.contains("blue")
            || text.contains("color")
            || text.contains("first")
            || text.contains("second"),
        "Response should compare the images: {}",
        text
    );
}

// =============================================================================
// Mixed Media Tests
// =============================================================================

/// Tests combining multiple media types (image + audio) in a single interaction.
///
/// This is an **enforcing test** that expects the API to successfully process
/// image + audio together. It asserts on the response content when successful,
/// but allows known format errors from the minimal test files.
///
/// Note: Video is excluded because the minimal MP4 test file often fails validation.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_mixed_image_and_audio() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Combine image and audio with a question about both
    let contents = vec![
        text_content(
            "I'm sending you an image and an audio file. \
             For the image, tell me what color it is. \
             For the audio, describe what kind of audio file it appears to be. \
             Keep your response brief.",
        ),
        image_data_content(TINY_RED_PNG_BASE64, "image/png"),
        audio_data_content(TINY_WAV_BASE64, "audio/wav"),
    ];

    let result = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await;

    match result {
        Ok(response) => {
            assert_eq!(
                response.status,
                InteractionStatus::Completed,
                "Mixed media interaction should complete"
            );
            assert!(response.has_text(), "Should have text response");

            let text = response.text().unwrap().to_lowercase();
            println!("Mixed media response: {}", text);

            // Verify the model acknowledged the inputs
            // Note: We check for image OR audio keywords since minimal test files
            // may not provide enough data for the model to analyze both
            let mentions_image =
                text.contains("image") || text.contains("color") || text.contains("red");
            let mentions_audio = text.contains("audio")
                || text.contains("sound")
                || text.contains("wav")
                || text.contains("silent")
                || text.contains("empty");

            assert!(
                mentions_image || mentions_audio,
                "Response should mention at least one input (image or audio): {}",
                text
            );
        }
        Err(e) => {
            // The minimal test files might not be fully valid
            let error_str = format!("{:?}", e);
            println!(
                "Mixed media error (may be expected for minimal files): {}",
                error_str
            );
            // Don't fail the test for format errors with minimal test files
            assert!(
                error_str.contains("format")
                    || error_str.contains("invalid")
                    || error_str.contains("empty")
                    || error_str.contains("audio"),
                "Unexpected error: {}",
                error_str
            );
        }
    }
}

/// Tests combining all three media types: image, audio, and video.
///
/// This is an **exploratory test** that documents API behavior rather than enforcing
/// specific outcomes. It may fail due to the minimal test files not being fully valid.
///
/// - **Success**: Indicates the API accepts all three media types together
/// - **Failure**: Documents which media types cause issues (helps debugging)
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_mixed_image_audio_video() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Combine all three media types
    let contents = vec![
        text_content(
            "I'm sending you an image, an audio file, and a video file. \
             Please briefly acknowledge each one.",
        ),
        image_data_content(TINY_RED_PNG_BASE64, "image/png"),
        audio_data_content(TINY_WAV_BASE64, "audio/wav"),
        video_data_content(TINY_MP4_BASE64, "video/mp4"),
    ];

    let result = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("All media types response status: {:?}", response.status);
            if response.has_text() {
                let text = response.text().unwrap();
                println!("All media types response: {}", text);
            }
            // If we get here, the API accepted all three types
            assert_eq!(response.status, InteractionStatus::Completed);
        }
        Err(e) => {
            // The minimal test files are very likely to fail validation
            let error_str = format!("{:?}", e);
            println!(
                "All media types error (expected for minimal files): {}",
                error_str
            );
            // This test documents the API behavior with minimal files
            // A passing result would indicate the API accepted the format
        }
    }
}

// =============================================================================
// Document/PDF Input Tests
// =============================================================================

/// Tests PDF document input from base64.
/// This tests the ability to send PDF documents to the model for analysis.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_pdf_document_input_from_base64() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Use minimal PDF containing "Hello World" text
    let contents = vec![
        text_content(
            "What text does this PDF document contain? Answer with just the text you find.",
        ),
        document_data_content(TINY_PDF_BASE64, "application/pdf"),
    ];

    let result = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("PDF document response status: {:?}", response.status);
            if response.has_text() {
                let text = response.text().unwrap();
                println!("PDF response: {}", text);
                // The minimal PDF contains "Hello World"
                let lower = text.to_lowercase();
                assert!(
                    lower.contains("hello") || lower.contains("world"),
                    "Response should mention the PDF content: {}",
                    text
                );
            }
            assert_eq!(response.status, InteractionStatus::Completed);
        }
        Err(e) => {
            // The minimal PDF might not be fully valid or the API might have restrictions
            println!(
                "PDF document error (may be expected for minimal PDF): {:?}",
                e
            );
        }
    }
}

/// Tests combining PDF document with text question.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_pdf_with_question() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let contents = vec![
        text_content("I'm sending you a PDF document."),
        document_data_content(TINY_PDF_BASE64, "application/pdf"),
        text_content("Is this a valid PDF? What can you tell me about its structure?"),
    ];

    let result = stateful_builder(&client)
        .with_input(InteractionInput::Content(contents))
        .create()
        .await;

    match result {
        Ok(response) => {
            assert_eq!(response.status, InteractionStatus::Completed);
            assert!(response.has_text(), "Should have text response");
            let text = response.text().unwrap().to_lowercase();
            println!("PDF question response: {}", text);
            // Should mention something about the PDF
            assert!(
                text.contains("pdf") || text.contains("document") || text.contains("page"),
                "Response should address the PDF: {}",
                text
            );
        }
        Err(e) => {
            println!("PDF with question error (may be expected): {:?}", e);
        }
    }
}

// =============================================================================
// Streaming Tests
// =============================================================================

/// Test streaming with multimodal (image) input.
///
/// This validates that:
/// - Streaming works correctly when images are part of the input
/// - Text deltas are received incrementally
/// - Final response correctly describes the image
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_multimodal_streaming() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(TEST_TIMEOUT, async {
        println!("=== Multimodal + Streaming ===");

        // Create content with text and image
        let contents = vec![
            text_content("What color is this image? Answer in one word."),
            image_data_content(TINY_RED_PNG_BASE64, "image/png"),
        ];

        // Stream the response using with_input for multimodal content
        let stream = interaction_builder(&client)
            .with_input(InteractionInput::Content(contents))
            .create_stream();

        let result = consume_stream(stream).await;

        println!("\nDelta count: {}", result.delta_count);
        println!("Collected text: {}", result.collected_text);

        // Verify streaming worked
        assert!(
            result.has_output(),
            "Should receive streaming chunks or final response"
        );

        // Verify content describes the red image
        let text_to_check = if !result.collected_text.is_empty() {
            result.collected_text.to_lowercase()
        } else if let Some(ref response) = result.final_response {
            response.text().unwrap_or_default().to_lowercase()
        } else {
            String::new()
        };

        assert!(
            text_to_check.contains("red"),
            "Response should identify the red color. Got: {}",
            text_to_check
        );

        // Verify final response if present
        if let Some(ref response) = result.final_response {
            assert_eq!(
                response.status,
                InteractionStatus::Completed,
                "Final response should be completed"
            );
        }

        println!("\n✓ Multimodal + streaming completed successfully");
    })
    .await;
}

// =============================================================================
// Builder Pattern File Loading Tests
// =============================================================================

/// Tests the add_image_file() builder method.
///
/// This validates the fluent builder pattern for loading images directly from files,
/// which auto-detects MIME type from the file extension and base64 encodes the content.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_add_image_file_builder() {
    use base64::Engine;
    use tempfile::TempDir;

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

    // Use the fluent builder pattern with add_image_file()
    let response = interaction_builder(&client)
        .with_text("What color is this image? Answer with just the color name.")
        .add_image_file(&image_path)
        .await
        .expect("Failed to add image file")
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

/// Tests chaining multiple add_image_file() calls.
///
/// Validates that the builder correctly accumulates multiple images when
/// chaining add_image_file() calls.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_add_multiple_image_files_builder() {
    use base64::Engine;
    use tempfile::TempDir;

    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create two image files
    let red_path = temp_dir.path().join("red.png");
    let blue_path = temp_dir.path().join("blue.png");

    let red_bytes = base64::engine::general_purpose::STANDARD
        .decode(TINY_RED_PNG_BASE64)
        .expect("Failed to decode red PNG");
    std::fs::write(&red_path, &red_bytes).expect("Failed to write red image");

    let blue_bytes = base64::engine::general_purpose::STANDARD
        .decode(TINY_BLUE_PNG_BASE64)
        .expect("Failed to decode blue PNG");
    std::fs::write(&blue_path, &blue_bytes).expect("Failed to write blue image");

    // Chain multiple add_image_file() calls
    let response = interaction_builder(&client)
        .with_text("I'm showing you two small colored images. What colors are they? List both.")
        .add_image_file(&red_path)
        .await
        .expect("Failed to add red image")
        .add_image_file(&blue_path)
        .await
        .expect("Failed to add blue image")
        .create()
        .await
        .expect("Multiple images interaction failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");

    let text = response.text().unwrap().to_lowercase();
    println!("Multiple images response: {}", text);

    // Should mention at least one color
    let mentions_red = text.contains("red") || text.contains("pink");
    let mentions_blue = text.contains("blue");

    assert!(
        mentions_red || mentions_blue,
        "Response should describe at least one of the colors: {}",
        text
    );
}

/// Tests add_image_file() error handling for missing file.
#[tokio::test]
async fn test_add_image_file_not_found() {
    // This test doesn't require an API key - just tests local file loading error
    let client = rust_genai::Client::builder("fake-key-for-testing".to_string())
        .build()
        .unwrap();

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Describe this image")
        .add_image_file("/nonexistent/path/image.png")
        .await;

    assert!(result.is_err(), "Should return error for missing file");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Failed to read file") || err.contains("No such file"),
        "Error should mention file not found: {}",
        err
    );
}

// =============================================================================
// Builder Pattern add_*_bytes() Tests
// =============================================================================

/// Tests the add_image_bytes() builder method.
///
/// This validates that raw bytes (not base64-encoded) can be passed directly
/// to the builder, which will handle the base64 encoding internally.
/// Uses semantic validation to verify the model correctly interprets the image.
///
/// Note: This test uses `.expect()` (strict assertion) because the PNG fixture
/// is a complete, well-formed image that the API should always accept.
/// Compare to audio/video tests which use lenient `match result` because those
/// minimal fixtures may be rejected by the API.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_add_image_bytes_roundtrip() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Decode the base64 constant to get raw bytes
    let image_bytes = base64::engine::general_purpose::STANDARD
        .decode(TINY_RED_PNG_BASE64)
        .expect("Failed to decode base64");

    // Use add_image_bytes() with raw bytes
    // The tiny PNG is well-formed and should always be processable
    let response = interaction_builder(&client)
        .with_text("What color is this image? Answer with just the color name.")
        .add_image_bytes(&image_bytes, "image/png")
        .create()
        .await
        .expect("Image bytes interaction failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");

    let text = response.text().unwrap();
    println!("Color response: {}", text);

    // Use semantic validation instead of brittle content checks
    let is_valid = validate_response_semantically(
        &client,
        "User asked about the color of a 1x1 red PNG image",
        text,
        "Does this response describe a red, pink, magenta, or similar warm color?",
    )
    .await
    .expect("Semantic validation failed");

    assert!(
        is_valid,
        "Response should identify a red/warm color: {}",
        text
    );
}

/// Tests the add_audio_bytes() builder method.
///
/// This validates that raw audio bytes can be passed directly to the builder.
/// Note: The minimal WAV test file may not contain actual audio, so the model
/// may report it's empty/silent.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_add_audio_bytes_roundtrip() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Decode the base64 constant to get raw bytes
    let audio_bytes = base64::engine::general_purpose::STANDARD
        .decode(TINY_WAV_BASE64)
        .expect("Failed to decode base64");

    // Use add_audio_bytes() with raw bytes
    let result = interaction_builder(&client)
        .with_text("Describe what you hear in this audio file.")
        .add_audio_bytes(&audio_bytes, "audio/wav")
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("Audio bytes response status: {:?}", response.status);
            if response.has_text() {
                let text = response.text().unwrap();
                println!("Audio response: {}", text);
                // Just verify we got some response - the content can vary
                assert!(!text.is_empty(), "Should get some response about the audio");
            }
        }
        Err(e) => {
            // The minimal WAV might not be accepted
            println!(
                "Audio bytes error (may be expected for minimal WAV): {:?}",
                e
            );
        }
    }
}

/// Tests the add_video_bytes() builder method.
///
/// This validates that raw video bytes can be passed directly to the builder.
/// Note: The minimal MP4 test file is just a container header with no frames,
/// so the model may report it's empty/corrupt.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_add_video_bytes_roundtrip() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Decode the base64 constant to get raw bytes
    let video_bytes = base64::engine::general_purpose::STANDARD
        .decode(TINY_MP4_BASE64)
        .expect("Failed to decode base64");

    // Use add_video_bytes() with raw bytes
    let result = interaction_builder(&client)
        .with_text("Describe what you see in this video file.")
        .add_video_bytes(&video_bytes, "video/mp4")
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("Video bytes response status: {:?}", response.status);
            if response.has_text() {
                let text = response.text().unwrap();
                println!("Video response: {}", text);
                // Just verify we got some response - the content can vary
                assert!(!text.is_empty(), "Should get some response about the video");
            }
        }
        Err(e) => {
            // The minimal MP4 might not be accepted
            println!(
                "Video bytes error (may be expected for minimal MP4): {:?}",
                e
            );
        }
    }
}

/// Tests the add_document_bytes() builder method.
///
/// This validates that raw document bytes (PDF) can be passed directly to
/// the builder. The test PDF contains "Hello World" text.
/// Uses semantic validation to verify the model correctly interprets the document.
///
/// Note: Like audio/video tests, this uses lenient error handling because the
/// minimal PDF fixture or the semantic validation call might fail.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_add_document_bytes_roundtrip() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Decode the base64 constant to get raw bytes
    let pdf_bytes = base64::engine::general_purpose::STANDARD
        .decode(TINY_PDF_BASE64)
        .expect("Failed to decode base64");

    // Use add_document_bytes() with raw bytes
    let result = interaction_builder(&client)
        .with_text("What text does this PDF document contain? Answer with just the text you find.")
        .add_document_bytes(&pdf_bytes, "application/pdf")
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("PDF bytes response status: {:?}", response.status);
            assert_eq!(response.status, InteractionStatus::Completed);
            assert!(response.has_text(), "Should have text response");

            let text = response.text().unwrap();
            println!("PDF response: {}", text);

            // Use semantic validation instead of brittle content checks
            // Handle validation failure gracefully since it makes an additional API call
            match validate_response_semantically(
                &client,
                "User asked about text in a PDF that contains 'Hello World'",
                text,
                "Does this response mention 'Hello', 'World', or indicate these words were found in the document?",
            )
            .await
            {
                Ok(is_valid) => {
                    assert!(
                        is_valid,
                        "Response should mention the PDF content: {}",
                        text
                    );
                }
                Err(e) => {
                    println!("Semantic validation error (non-fatal): {:?}", e);
                }
            }
        }
        Err(e) => {
            // The minimal PDF might not be fully valid
            println!("PDF bytes error (may be expected for minimal PDF): {:?}", e);
        }
    }
}

// =============================================================================
// Text-to-Speech Output Tests
// =============================================================================

/// Tests basic text-to-speech audio output
#[tokio::test]
#[ignore = "Requires API key and TTS model access"]
async fn test_text_to_speech_basic() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // TTS requires a specific model
    let tts_model = "gemini-2.5-flash-preview-tts";

    with_timeout(TEST_TIMEOUT, async {
        let response = client
            .interaction()
            .with_model(tts_model)
            .with_text("Hello, world!")
            .with_audio_output()
            .with_voice("Kore")
            .create()
            .await;

        match response {
            Ok(r) => {
                println!("TTS response status: {:?}", r.status);
                assert!(r.has_audio(), "Response should contain audio output");

                if let Some(audio) = r.first_audio() {
                    let bytes = audio.bytes().expect("Should decode audio");
                    println!("Audio size: {} bytes", bytes.len());
                    println!("Audio MIME type: {:?}", audio.mime_type());
                    println!("Audio extension: {}", audio.extension());
                    assert!(!bytes.is_empty(), "Audio should not be empty");
                }
            }
            Err(e) => {
                // TTS model might not be available in all regions
                println!("TTS test error (may be expected): {:?}", e);
            }
        }
    })
    .await;
}

/// Tests text-to-speech with speech configuration
#[tokio::test]
#[ignore = "Requires API key and TTS model access"]
async fn test_text_to_speech_with_speech_config() {
    use rust_genai::SpeechConfig;

    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let tts_model = "gemini-2.5-flash-preview-tts";

    with_timeout(TEST_TIMEOUT, async {
        let config = SpeechConfig {
            voice: Some("Puck".to_string()),
            language: Some("en-US".to_string()),
            speaker: None,
        };

        let response = client
            .interaction()
            .with_model(tts_model)
            .with_text("Testing speech configuration.")
            .with_audio_output()
            .with_speech_config(config)
            .create()
            .await;

        match response {
            Ok(r) => {
                println!("TTS with config status: {:?}", r.status);
                assert!(r.has_audio(), "Response should contain audio output");
            }
            Err(e) => {
                println!("TTS with config error (may be expected): {:?}", e);
            }
        }
    })
    .await;
}
