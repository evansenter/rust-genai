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
use std::future::Future;
use std::time::{Duration, Instant};
use tokio::time::sleep;

// =============================================================================
// Retry Utilities for Transient API Errors
// =============================================================================

/// Maximum number of retries for transient API errors.
#[allow(dead_code)]
pub const DEFAULT_MAX_RETRIES: u32 = 3;

/// Checks if an error is a transient API error that should be retried.
///
/// Currently detects:
/// - Spanner UTF-8 errors (Google backend issue with stateful conversations)
#[allow(dead_code)]
pub fn is_transient_error(err: &GenaiError) -> bool {
    match err {
        GenaiError::Api(msg) => {
            // Spanner UTF-8 errors are transient backend issues
            // See: https://github.com/evansenter/rust-genai/issues/60
            msg.to_lowercase().contains("spanner")
        }
        _ => false,
    }
}

/// Retries an async operation on transient API errors with exponential backoff.
///
/// This is useful for working around transient Google API backend issues,
/// such as the Spanner UTF-8 error that occasionally occurs with stateful
/// conversations (see issue #60).
///
/// # Arguments
///
/// * `max_retries` - Maximum number of retry attempts (0 = no retries, just run once)
/// * `operation` - A closure that returns a future producing `Result<T, GenaiError>`
///
/// # Returns
///
/// The result of the operation if it succeeds, or the last error if all retries fail.
///
/// # Example
///
/// ```ignore
/// let response = retry_on_transient(3, || async {
///     client
///         .interaction()
///         .with_model("gemini-3-flash-preview")
///         .with_text("Hello")
///         .with_store(true)
///         .create()
///         .await
/// }).await.expect("Request failed after retries");
/// ```
#[allow(dead_code)]
pub async fn retry_on_transient<F, Fut, T>(max_retries: u32, operation: F) -> Result<T, GenaiError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, GenaiError>>,
{
    let mut last_error = None;

    for attempt in 0..=max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(err) if is_transient_error(&err) && attempt < max_retries => {
                println!(
                    "Transient error on attempt {} of {}: {:?}",
                    attempt + 1,
                    max_retries + 1,
                    err
                );
                last_error = Some(err);
                // Exponential backoff: 1s, 2s, 4s, ...
                let delay = Duration::from_secs(1 << attempt);
                sleep(delay).await;
            }
            Err(err) => return Err(err),
        }
    }

    Err(last_error.expect("Should have an error if we exhausted retries"))
}

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
/// Checks the status immediately on first call (no initial delay), then uses exponential
/// backoff starting at 1 second and doubling up to a maximum of 10 seconds. This is more
/// efficient than fixed-interval polling: instant detection of already-completed tasks,
/// faster initial detection of quick completions, and fewer API calls for long-running tasks.
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
    let mut first_poll = true;
    let start = Instant::now();

    loop {
        if start.elapsed() > max_wait {
            return Err(PollError::Timeout);
        }

        // Skip delay on first poll to detect instant completions
        if first_poll {
            first_poll = false;
        } else {
            sleep(delay).await;
            delay = (delay * 2).min(MAX_DELAY);
        }

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
