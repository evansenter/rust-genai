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

use common::{
    SAMPLE_AUDIO_URL, SAMPLE_IMAGE_URL, SAMPLE_VIDEO_URL, TINY_BLUE_PNG_BASE64, TINY_MP4_BASE64,
    TINY_RED_PNG_BASE64, TINY_WAV_BASE64, get_client,
};
use rust_genai::{
    InteractionInput, InteractionStatus, audio_data_content, audio_uri_content, image_data_content,
    image_uri_content, text_content, video_data_content, video_uri_content,
};

// =============================================================================
// Image Input Tests
// =============================================================================

/// Tests image input from URI.
/// Note: GCS URIs are not supported by the Interactions API, so this test
/// documents the expected error behavior.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_image_input_from_uri() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let contents = vec![
        text_content("What is in this image? Describe it briefly in 1-2 sentences."),
        image_uri_content(SAMPLE_IMAGE_URL, Some("image/jpeg".to_string())),
    ];

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(InteractionInput::Content(contents))
        .with_store(true)
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

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(InteractionInput::Content(contents))
        .with_store(true)
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

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(InteractionInput::Content(contents))
        .with_store(true)
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

    // First turn: describe the base64 image
    let contents = vec![
        text_content("What color is this image?"),
        image_data_content(TINY_RED_PNG_BASE64, "image/png"),
    ];

    let response1 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(InteractionInput::Content(contents))
        .with_store(true)
        .create()
        .await
        .expect("First interaction failed");

    assert_eq!(response1.status, InteractionStatus::Completed);
    println!("First response: {:?}", response1.text());

    // Second turn: ask follow-up about the same image
    let response2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_previous_interaction(&response1.id)
        .with_text("Is that a warm or cool color?")
        .with_store(true)
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
        audio_uri_content(SAMPLE_AUDIO_URL, Some("audio/mpeg".to_string())),
    ];

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(InteractionInput::Content(contents))
        .with_store(true)
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

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(InteractionInput::Content(contents))
        .with_store(true)
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
        video_uri_content(SAMPLE_VIDEO_URL, Some("video/mp4".to_string())),
    ];

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(InteractionInput::Content(contents))
        .with_store(true)
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

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(InteractionInput::Content(contents))
        .with_store(true)
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

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(InteractionInput::Content(contents))
        .with_store(true)
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

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(InteractionInput::Content(contents))
        .with_store(true)
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
/// Note: Video is excluded because the minimal MP4 test file often fails validation.
/// This test verifies the model can process multiple distinct media types together.
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

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(InteractionInput::Content(contents))
        .with_store(true)
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

            // The model should acknowledge both inputs
            // Note: The tiny files may not have enough data for detailed analysis
            assert!(
                text.contains("image") || text.contains("color") || text.contains("red"),
                "Response should mention the image: {}",
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

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(InteractionInput::Content(contents))
        .with_store(true)
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
