//! Common test utilities shared across all integration test files.
//!
//! Usage in test files:
//! ```ignore
//! mod common;
//! use common::*;
//! ```
//!
//! # Note on URI Support
//!
//! The Interactions API does NOT support Google Cloud Storage (gs://) URIs.
//! Tests that need external media should use base64-encoded data or
//! gracefully handle the unsupported URI error.

use rust_genai::Client;
use std::env;
use std::future::Future;
use std::time::Duration;

/// Creates a client from the GEMINI_API_KEY environment variable.
/// Returns None if the API key is not set.
pub fn get_client() -> Option<Client> {
    env::var("GEMINI_API_KEY")
        .ok()
        .map(|key| Client::builder(key).build())
}

// =============================================================================
// Timeout Utilities
// =============================================================================

/// Default timeout for long-running integration tests (60 seconds).
///
/// This provides a safety net to prevent tests from hanging indefinitely
/// when interacting with external APIs.
#[allow(dead_code)]
pub const TEST_TIMEOUT: Duration = Duration::from_secs(60);

/// Extended timeout for tests that make many sequential API calls (120 seconds).
///
/// Use this for tests like multi-turn conversations that make 10+ API calls.
#[allow(dead_code)]
pub const EXTENDED_TEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Wraps a future with a timeout, panicking if the timeout is exceeded.
///
/// Use this to prevent integration tests from hanging indefinitely when
/// interacting with external APIs.
///
/// # Arguments
///
/// * `duration` - Maximum time to wait for the future to complete
/// * `future` - The async operation to wrap with a timeout
///
/// # Panics
///
/// Panics with a descriptive message if the timeout is exceeded.
///
/// # Example
///
/// ```ignore
/// use common::{TEST_TIMEOUT, get_client, with_timeout};
///
/// #[tokio::test]
/// #[ignore = "Requires API key"]
/// async fn test_something() {
///     let Some(client) = get_client() else {
///         println!("Skipping: GEMINI_API_KEY not set");
///         return;
///     };
///
///     with_timeout(TEST_TIMEOUT, async {
///         // test logic that might hang
///         let response = client.interaction()
///             .with_model("gemini-3-flash-preview")
///             .with_text("Hello")
///             .create()
///             .await
///             .expect("Request failed");
///     }).await;
/// }
/// ```
#[allow(dead_code)]
pub async fn with_timeout<F, T>(duration: Duration, future: F) -> T
where
    F: Future<Output = T>,
{
    tokio::time::timeout(duration, future)
        .await
        .unwrap_or_else(|_| panic!("Test timed out after {:?}", duration))
}

// =============================================================================
// Test Asset URLs
// =============================================================================

// Allow dead_code because these are shared utilities and not all test files use all constants

/// Google Cloud Storage sample image URL (scones/pastries)
/// NOTE: GCS URIs are NOT supported by the Interactions API - tests should handle errors gracefully
#[allow(dead_code)]
pub const SAMPLE_IMAGE_URL: &str = "gs://cloud-samples-data/generative-ai/image/scones.jpg";

/// Google Cloud Storage sample audio URL (Pixel phone promo)
/// NOTE: GCS URIs are NOT supported by the Interactions API - tests should handle errors gracefully
#[allow(dead_code)]
pub const SAMPLE_AUDIO_URL: &str = "gs://cloud-samples-data/generative-ai/audio/pixel.mp3";

/// Google Cloud Storage sample video URL
/// NOTE: GCS URIs are NOT supported by the Interactions API - tests should handle errors gracefully
#[allow(dead_code)]
pub const SAMPLE_VIDEO_URL: &str = "gs://cloud-samples-data/video/animals.mp4";

/// Small 1x1 red PNG image encoded as base64
/// This is a minimal valid PNG for testing base64 image input
#[allow(dead_code)]
pub const TINY_RED_PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==";

/// Small WAV audio header (minimal valid WAV for testing)
/// This is a 44-byte WAV header with no actual audio data
#[allow(dead_code)]
pub const TINY_WAV_BASE64: &str = "UklGRiQAAABXQVZFZm10IBAAAAABAAEARKwAAIhYAQACABAAZGF0YQAAAAA=";

/// Minimal MP4 video file (ftyp box only) for testing base64 video input
/// This is a minimal valid MP4 container header - the model may report it's empty/corrupt
#[allow(dead_code)]
pub const TINY_MP4_BASE64: &str = "AAAAIGZ0eXBpc29tAAACAGlzb21pc28yYXZjMW1wNDE=";

/// Small 1x1 blue PNG image encoded as base64
/// This is a minimal valid PNG for testing multi-image comparisons
#[allow(dead_code)]
pub const TINY_BLUE_PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";
