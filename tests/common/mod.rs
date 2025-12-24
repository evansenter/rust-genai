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

use rust_genai::{Client, GenaiError, InteractionResponse, InteractionStatus};
use std::env;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Creates a client from the GEMINI_API_KEY environment variable.
/// Returns None if the API key is not set.
pub fn get_client() -> Option<Client> {
    env::var("GEMINI_API_KEY")
        .ok()
        .map(|key| Client::builder(key).build())
}

// =============================================================================
// Polling Utilities
// =============================================================================

/// Error type for polling operations.
#[derive(Debug)]
#[allow(dead_code)]
pub enum PollError {
    /// Polling timed out before the interaction completed.
    Timeout,
    /// The interaction failed.
    Failed,
    /// An API error occurred during polling.
    Api(GenaiError),
}

impl From<GenaiError> for PollError {
    fn from(err: GenaiError) -> Self {
        PollError::Api(err)
    }
}

/// Polls an interaction until it completes or times out, using exponential backoff.
///
/// Starts with a 1-second delay and doubles each iteration up to a maximum of 10 seconds.
/// This is more efficient than fixed-interval polling: faster initial detection of
/// quick completions, fewer API calls for long-running tasks.
///
/// # Arguments
///
/// * `client` - The API client to use for polling
/// * `interaction_id` - The ID of the interaction to poll
/// * `max_wait` - Maximum duration to wait before timing out
///
/// # Returns
///
/// * `Ok(InteractionResponse)` - The completed (or failed) interaction
/// * `Err(PollError::Timeout)` - If max_wait elapsed without completion
/// * `Err(PollError::Failed)` - If the interaction status became Failed
/// * `Err(PollError::Api(_))` - If an API error occurred
///
/// # Example
///
/// ```ignore
/// let response = poll_until_complete(&client, &interaction_id, Duration::from_secs(60)).await?;
/// ```
#[allow(dead_code)]
pub async fn poll_until_complete(
    client: &Client,
    interaction_id: &str,
    max_wait: Duration,
) -> Result<InteractionResponse, PollError> {
    const INITIAL_DELAY: Duration = Duration::from_secs(1);
    const MAX_DELAY: Duration = Duration::from_secs(10);

    let mut delay = INITIAL_DELAY;
    let start = Instant::now();

    loop {
        if start.elapsed() > max_wait {
            return Err(PollError::Timeout);
        }

        sleep(delay).await;

        let response = client.get_interaction(interaction_id).await?;
        println!(
            "Poll after {:?}: status={:?}",
            start.elapsed(),
            response.status
        );

        match response.status {
            InteractionStatus::Completed => return Ok(response),
            InteractionStatus::Failed => return Err(PollError::Failed),
            _ => {
                // Continue polling with exponential backoff
                delay = (delay * 2).min(MAX_DELAY);
            }
        }
    }
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
