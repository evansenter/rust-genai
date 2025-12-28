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

use futures_util::StreamExt;
use rust_genai::{
    AutoFunctionStreamChunk, Client, GenaiError, InteractionContent, InteractionResponse,
    InteractionStatus, StreamChunk,
};
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
///
/// The detection is specific to avoid false positives - both "spanner" and "utf-8"
/// must appear in the error message.
#[allow(dead_code)]
pub fn is_transient_error(err: &GenaiError) -> bool {
    match err {
        GenaiError::Api { message, .. } => {
            // Spanner UTF-8 errors are transient backend issues
            // See: https://github.com/evansenter/rust-genai/issues/60
            // Check for both "spanner" and "utf-8" to avoid false positives
            let lower = message.to_lowercase();
            lower.contains("spanner") && lower.contains("utf-8")
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
                // Exponential backoff: 1s, 2s, 4s, ...
                let delay = Duration::from_secs(1 << attempt);
                println!(
                    "Transient error on attempt {} of {}, retrying in {:?}: {:?}",
                    attempt + 1,
                    max_retries + 1,
                    delay,
                    err
                );
                last_error = Some(err);
                sleep(delay).await;
            }
            Err(err) => return Err(err),
        }
    }

    Err(last_error.expect("Should have an error if we exhausted retries"))
}

/// Creates a client from the GEMINI_API_KEY environment variable.
/// Returns None if the API key is not set.
#[allow(dead_code)]
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
// Streaming Utilities
// =============================================================================

/// Result of consuming a stream, containing collected deltas and final response.
#[derive(Debug)]
#[allow(dead_code)]
pub struct StreamResult {
    /// Number of delta chunks received during streaming.
    pub delta_count: usize,
    /// All text content collected from delta chunks.
    pub collected_text: String,
    /// The final complete response, if received.
    pub final_response: Option<InteractionResponse>,
    /// Whether any function call deltas were received.
    pub saw_function_call: bool,
    /// Whether any thought deltas were received.
    pub saw_thought: bool,
    /// Whether any thought signature deltas were received.
    pub saw_thought_signature: bool,
    /// All thought content collected from delta chunks.
    pub collected_thoughts: String,
}

impl StreamResult {
    /// Returns true if streaming produced any output (deltas or complete response).
    #[allow(dead_code)]
    pub fn has_output(&self) -> bool {
        self.delta_count > 0 || self.final_response.is_some()
    }
}

/// Consumes a stream, collecting text deltas and the final response.
///
/// This helper standardizes stream consumption across tests, handling:
/// - Counting delta chunks
/// - Collecting text content from deltas
/// - Capturing the final complete response
/// - Detecting function call deltas
/// - Graceful error handling (breaks on error, doesn't panic)
///
/// **Note**: Text content is printed to stdout as it's received for debugging
/// purposes when running tests with `--nocapture`.
///
/// # Arguments
///
/// * `stream` - A boxed stream of `Result<StreamChunk, GenaiError>` that will be
///   fully consumed (ownership is taken)
///
/// # Returns
///
/// A `StreamResult` containing the collected data from the stream.
///
/// # Example
///
/// ```ignore
/// let stream = client.interaction()
///     .with_model("gemini-3-flash-preview")
///     .with_text("Hello")
///     .create_stream();
///
/// let result = consume_stream(stream).await;
/// assert!(result.has_output());
/// assert!(result.collected_text.contains("hello"));
/// ```
#[allow(dead_code)]
pub async fn consume_stream(
    mut stream: futures_util::stream::BoxStream<'_, Result<StreamChunk, GenaiError>>,
) -> StreamResult {
    let mut result = StreamResult {
        delta_count: 0,
        collected_text: String::new(),
        final_response: None,
        saw_function_call: false,
        saw_thought: false,
        saw_thought_signature: false,
        collected_thoughts: String::new(),
    };

    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => match chunk {
                StreamChunk::Delta(delta) => {
                    result.delta_count += 1;
                    if let Some(text) = delta.text() {
                        result.collected_text.push_str(text);
                        print!("{}", text);
                    }
                    if delta.is_function_call() {
                        result.saw_function_call = true;
                    }
                    if delta.is_thought() {
                        result.saw_thought = true;
                        if let InteractionContent::Thought { text: Some(t) } = &delta {
                            result.collected_thoughts.push_str(t);
                        }
                    }
                    if delta.is_thought_signature() {
                        result.saw_thought_signature = true;
                    }
                }
                StreamChunk::Complete(response) => {
                    println!("\nStream complete: {}", response.id);
                    result.final_response = Some(response);
                }
                _ => {} // Handle unknown variants
            },
            Err(e) => {
                println!("Stream error: {:?}", e);
                break;
            }
        }
    }

    result
}

/// Result of consuming an auto-function stream.
#[derive(Debug)]
#[allow(dead_code)]
pub struct AutoFunctionStreamResult {
    /// Number of delta chunks received during streaming.
    pub delta_count: usize,
    /// All text content collected from delta chunks.
    pub collected_text: String,
    /// Number of times function execution was signaled.
    pub executing_functions_count: usize,
    /// Names of all functions that were executed.
    pub executed_function_names: Vec<String>,
    /// Number of function result events.
    pub function_results_count: usize,
    /// The final complete response, if received.
    pub final_response: Option<InteractionResponse>,
    /// Whether any thought deltas were received.
    pub saw_thought: bool,
    /// All thought content collected from delta chunks.
    pub collected_thoughts: String,
}

impl AutoFunctionStreamResult {
    /// Returns true if streaming produced any output.
    #[allow(dead_code)]
    pub fn has_output(&self) -> bool {
        self.delta_count > 0 || self.final_response.is_some()
    }
}

/// Consumes an auto-function stream, collecting events and the final response.
///
/// This helper standardizes auto-function stream consumption across tests.
///
/// # Arguments
///
/// * `stream` - A boxed stream of `Result<AutoFunctionStreamChunk, GenaiError>`
///
/// # Returns
///
/// An `AutoFunctionStreamResult` containing the collected data from the stream.
#[allow(dead_code)]
pub async fn consume_auto_function_stream(
    mut stream: futures_util::stream::BoxStream<'_, Result<AutoFunctionStreamChunk, GenaiError>>,
) -> AutoFunctionStreamResult {
    let mut result = AutoFunctionStreamResult {
        delta_count: 0,
        collected_text: String::new(),
        executing_functions_count: 0,
        executed_function_names: Vec::new(),
        function_results_count: 0,
        final_response: None,
        saw_thought: false,
        collected_thoughts: String::new(),
    };

    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => match chunk {
                AutoFunctionStreamChunk::Delta(delta) => {
                    result.delta_count += 1;
                    if let Some(text) = delta.text() {
                        result.collected_text.push_str(text);
                        print!("{}", text);
                    }
                    if delta.is_thought() {
                        result.saw_thought = true;
                        if let InteractionContent::Thought { text: Some(t) } = &delta {
                            result.collected_thoughts.push_str(t);
                        }
                    }
                }
                AutoFunctionStreamChunk::ExecutingFunctions(response) => {
                    result.executing_functions_count += 1;
                    for call in response.function_calls() {
                        println!("\n[Executing: {}]", call.name);
                        result.executed_function_names.push(call.name.to_string());
                    }
                }
                AutoFunctionStreamChunk::FunctionResults(results) => {
                    result.function_results_count += 1;
                    println!("[Got {} result(s)]", results.len());
                    // Track executed function names from results
                    for r in &results {
                        if !result.executed_function_names.contains(&r.name) {
                            result.executed_function_names.push(r.name.clone());
                        }
                    }
                }
                AutoFunctionStreamChunk::Complete(response) => {
                    println!("\n[Stream complete: {}]", response.id);
                    result.final_response = Some(response);
                }
                _ => {
                    // Unknown future variants - ignore
                    println!("[Unknown chunk type]");
                }
            },
            Err(e) => {
                println!("Stream error: {:?}", e);
                break;
            }
        }
    }

    result
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

/// Minimal PDF document containing "Hello World" text
/// This is a complete valid PDF for testing document input
#[allow(dead_code)]
pub const TINY_PDF_BASE64: &str = "JVBERi0xLjQKMSAwIG9iago8PCAvVHlwZSAvQ2F0YWxvZyAvUGFnZXMgMiAwIFIgPj4KZW5kb2JqCjIgMCBvYmoKPDwgL1R5cGUgL1BhZ2VzIC9LaWRzIFszIDAgUl0gL0NvdW50IDEgPj4KZW5kb2JqCjMgMCBvYmoKPDwgL1R5cGUgL1BhZ2UgL1BhcmVudCAyIDAgUiAvTWVkaWFCb3ggWzAgMCA3MiA3Ml0gL0NvbnRlbnRzIDQgMCBSIC9SZXNvdXJjZXMgPDwgPj4gPj4KZW5kb2JqCjQgMCBvYmoKPDwgL0xlbmd0aCA0NCA+PgpzdHJlYW0KQlQgL0YxIDEyIFRmIDEwIDUwIFRkIChIZWxsbyBXb3JsZCkgVGogRVQKZW5kc3RyZWFtCmVuZG9iagp4cmVmCjAgNQowMDAwMDAwMDAwIDY1NTM1IGYgCjAwMDAwMDAwMDkgMDAwMDAgbiAKMDAwMDAwMDA1OCAwMDAwMCBuIAowMDAwMDAwMTE1IDAwMDAwIG4gCjAwMDAwMDAyMjQgMDAwMDAgbiAKdHJhaWxlcgo8PCAvU2l6ZSA1IC9Sb290IDEgMCBSID4+CnN0YXJ0eHJlZgozMjAKJSVFT0Y=";

// =============================================================================
// Test Fixture Builders (Issue #82)
// =============================================================================

/// Default model used across all tests.
pub const DEFAULT_MODEL: &str = "gemini-3-flash-preview";

/// Creates a pre-configured interaction builder with the default model.
///
/// This is the standard entry point for integration tests, reducing boilerplate.
///
/// # Example
///
/// ```ignore
/// use common::{get_client, interaction_builder};
///
/// let client = get_client().unwrap();
/// let response = interaction_builder(&client)
///     .with_text("Hello!")
///     .create()
///     .await
///     .expect("Request failed");
/// ```
#[allow(dead_code)]
pub fn interaction_builder(client: &Client) -> rust_genai::InteractionBuilder<'_> {
    client.interaction().with_model(DEFAULT_MODEL)
}

/// Creates a stateful interaction builder with storage enabled.
///
/// Use this when testing multi-turn conversations that need server-side state.
///
/// # Example
///
/// ```ignore
/// let response = stateful_builder(&client)
///     .with_text("Remember my name is Alice")
///     .create()
///     .await?;
/// ```
#[allow(dead_code)]
pub fn stateful_builder(client: &Client) -> rust_genai::InteractionBuilder<'_> {
    interaction_builder(client).with_store(true)
}

/// Creates an interaction builder with thinking enabled.
///
/// Use this for tests that need model reasoning. Note that thoughts may not
/// always be visible in the response depending on API behavior.
///
/// # Example
///
/// ```ignore
/// let response = thinking_builder(&client)
///     .with_text("Solve this step by step: 2+2")
///     .create()
///     .await?;
/// if response.has_thoughts() {
///     println!("Model reasoning: {:?}", response.thoughts());
/// }
/// ```
#[allow(dead_code)]
pub fn thinking_builder(client: &Client) -> rust_genai::InteractionBuilder<'_> {
    interaction_builder(client).with_thinking_level(rust_genai::ThinkingLevel::Medium)
}
