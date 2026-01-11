//! Demonstrates retry logic with exponential backoff using `is_retryable()`.
//!
//! This example shows how to build requests separately from execution, enabling
//! retry patterns for transient failures like rate limits (429) and server errors (5xx).
//!
//! Key concepts demonstrated:
//! - `InteractionBuilder::build()` to create a reusable `InteractionRequest`
//! - `Client::execute()` to run a pre-built request
//! - `GenaiError::is_retryable()` to identify transient errors
//! - Exponential backoff with jitter for retry delays
//!
//! Run with: cargo run --example retry_with_backoff

use genai_rs::{Client, GenaiError, InteractionRequest};
use rand::Rng;
use std::env;
use std::error::Error;
use std::time::Duration;

/// Configuration for retry behavior
struct RetryConfig {
    /// Maximum number of retry attempts (not including initial attempt)
    max_retries: u32,
    /// Base delay between retries (doubles each attempt)
    base_delay: Duration,
    /// Maximum delay between retries
    max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
        }
    }
}

/// Executes a request with automatic retry on transient errors.
///
/// Uses exponential backoff with jitter to spread out retries and avoid
/// thundering herd problems when multiple clients retry simultaneously.
async fn execute_with_retry(
    client: &Client,
    request: InteractionRequest,
    config: &RetryConfig,
) -> Result<genai_rs::InteractionResponse, GenaiError> {
    let mut attempt = 0;

    loop {
        // Clone the request for this attempt (original is preserved for retries)
        let request_clone = request.clone();

        match client.execute(request_clone).await {
            Ok(response) => {
                if attempt > 0 {
                    println!("✓ Request succeeded after {} retry attempt(s)", attempt);
                }
                return Ok(response);
            }
            Err(e) => {
                // Check if we should retry
                if !e.is_retryable() {
                    println!("✗ Non-retryable error: {}", e);
                    return Err(e);
                }

                if attempt >= config.max_retries {
                    println!(
                        "✗ Max retries ({}) exceeded. Last error: {}",
                        config.max_retries, e
                    );
                    return Err(e);
                }

                // Calculate delay with exponential backoff
                let base_delay_ms = config.base_delay.as_millis() as u64;
                let delay_ms = base_delay_ms * 2u64.pow(attempt);
                let delay = Duration::from_millis(delay_ms).min(config.max_delay);

                // Add jitter (±25%) to spread out retries.
                //
                // IMPORTANT: Use a proper random number generator here. Poor entropy
                // (e.g., timestamps) causes multiple clients to compute similar jitter
                // values, defeating the "thundering herd" mitigation. When a service
                // recovers from an outage, all waiting clients would retry at nearly
                // the same time, overwhelming the service again.
                let jitter_factor = 0.75 + (rand::rng().random::<f64>() * 0.5);
                let jittered_delay =
                    Duration::from_millis((delay.as_millis() as f64 * jitter_factor) as u64);

                println!(
                    "⟳ Retryable error (attempt {}/{}): {}",
                    attempt + 1,
                    config.max_retries,
                    e
                );
                println!("  Waiting {:?} before retry...", jittered_delay);

                tokio::time::sleep(jittered_delay).await;
                attempt += 1;
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Get API key
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not found in environment");

    let client = Client::builder(api_key).build()?;

    let model = "gemini-3-flash-preview";
    let prompt = "What is the Rust programming language known for? Answer in one sentence.";

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Retry with Backoff Example");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Step 1: Build the request (without executing)
    println!("Step 1: Building request...");
    let request: InteractionRequest = client
        .interaction()
        .with_model(model)
        .with_text(prompt)
        .build()?;

    println!("  Model: {:?}", request.model);
    println!("  Request is Clone + Serialize + Deserialize");
    println!("  Request can be retried, logged, or persisted\n");

    // Step 2: Execute with retry logic
    println!("Step 2: Executing with retry logic...");
    let config = RetryConfig::default();
    println!(
        "  Max retries: {}, Base delay: {:?}\n",
        config.max_retries, config.base_delay
    );

    let response = execute_with_retry(&client, request, &config).await?;

    // Step 3: Process response
    println!("\n--- Response ---");
    if let Some(text) = response.text() {
        println!("{}", text);
    }

    if let Some(usage) = &response.usage
        && let Some(total) = usage.total_tokens
    {
        println!("\nTokens used: {}", total);
    }

    println!("\n=== Example Complete ===\n");

    println!("--- What You'll See with LOUD_WIRE=1 ---");
    println!("  [REQ#1] POST with input text + model");
    println!("  [RES#1] completed: text response with usage stats");
    println!("  (If retries occur, you'll see REQ#2, REQ#3, etc.)\n");

    println!("--- Production Considerations ---");
    println!("• InteractionBuilder::build() creates a reusable InteractionRequest");
    println!("• InteractionRequest implements Clone for retry patterns");
    println!("• Client::execute() runs a pre-built request");
    println!("• GenaiError::is_retryable() identifies transient errors (429, 5xx, timeouts)");
    println!("• Set appropriate max_retries based on your SLA requirements");
    println!("• Consider circuit breakers for sustained failures");
    println!("• For 429 errors, check Retry-After header if available");
    println!("• Serialize requests for dead-letter queues or audit logs");

    Ok(())
}
