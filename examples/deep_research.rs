//! Example: Deep Research Agent with Gemini
//!
//! This example demonstrates how to use Gemini's Deep Research agent, which conducts
//! multi-step research tasks by executing iterative searches, synthesizing information
//! across multiple sources, and generating comprehensive research reports.
//!
//! The Deep Research agent operates as a long-running asynchronous operation. This
//! example shows two modes of operation:
//!
//! 1. **Synchronous mode**: Waits for the research to complete (simpler, may timeout)
//! 2. **Background mode with polling**: Starts research asynchronously and polls for
//!    completion (recommended for long-running research tasks)
//!
//! **Expected runtime**: Deep research queries typically take 30-120 seconds depending
//! on query complexity. Simple queries may complete in under 30 seconds, while complex
//! multi-source research can take several minutes.
//!
//! Note: The Deep Research agent may not be available in all accounts or regions.
//!
//! Run with: cargo run --example deep_research

use rust_genai::{Client, GenaiError, InteractionStatus};
use std::env;
use std::error::Error;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Maximum time to wait for research to complete when polling
const MAX_POLL_DURATION: Duration = Duration::from_secs(120);

/// Initial delay between polls (will increase with exponential backoff).
///
/// Note: This is intentionally more conservative (2s) than test utilities (1s)
/// to reduce API calls in production usage where research typically takes longer.
const INITIAL_POLL_DELAY: Duration = Duration::from_secs(2);

/// Maximum delay between polls
const MAX_POLL_DELAY: Duration = Duration::from_secs(10);

/// Maximum characters to display for synchronous mode results.
/// Sync mode uses a shorter limit since it's the "simple" demo.
const SYNC_DISPLAY_LIMIT: usize = 1500;

/// Maximum characters to display for background mode results.
/// Background mode shows more since it's the "full" demo with polling.
const BACKGROUND_DISPLAY_LIMIT: usize = 2000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Get API Key from environment variable
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not found in environment");

    // Create the client
    let client = Client::builder(api_key).build();

    // The Deep Research agent identifier
    let agent_name = "deep-research-pro-preview-12-2025";

    println!("=== Deep Research Agent Example ===\n");

    // 2. First, demonstrate synchronous mode (simple but may timeout for complex queries)
    println!("--- Part 1: Synchronous Research ---\n");
    synchronous_research(&client, agent_name).await?;

    println!("\n--- Part 2: Background Mode with Polling ---\n");
    background_research_with_polling(&client, agent_name).await?;

    Ok(())
}

/// Demonstrates synchronous deep research (waits for completion).
///
/// This is simpler but may timeout for complex research queries that take a long time.
async fn synchronous_research(client: &Client, agent_name: &str) -> Result<(), Box<dyn Error>> {
    let prompt = "What are the key differences between Rust's ownership model and \
                  C++'s RAII pattern? Focus on memory safety guarantees.";

    println!("Research query: {prompt}\n");
    println!("Starting synchronous research (waiting for completion)...\n");

    // 3. Create an interaction with the Deep Research agent
    let result = client
        .interaction()
        .with_agent(agent_name)
        .with_text(prompt)
        .with_store(true) // Required for agent interactions
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("Research completed!");
            println!("Status: {:?}", response.status);
            println!("Interaction ID: {}\n", response.id);

            // 4. Display the research results
            if let Some(text) = response.text() {
                // Truncate for display if very long
                let display_text = if text.len() > SYNC_DISPLAY_LIMIT {
                    format!(
                        "{}...\n\n[Response truncated, {} total chars]",
                        &text[..SYNC_DISPLAY_LIMIT],
                        text.len()
                    )
                } else {
                    text.to_string()
                };
                println!("Research Results:\n{display_text}\n");
            } else {
                println!("No text response received.");
            }

            // 5. Show token usage if available
            if let Some(usage) = response.usage {
                println!("--- Token Usage ---");
                if let Some(input) = usage.total_input_tokens {
                    println!("  Input tokens: {input}");
                }
                if let Some(output) = usage.total_output_tokens {
                    println!("  Output tokens: {output}");
                }
            }
        }
        Err(e) => {
            handle_research_error(&e);
        }
    }

    Ok(())
}

/// Demonstrates background mode with polling for long-running research.
///
/// This approach is recommended for complex research queries that may take
/// significant time to complete. The research runs asynchronously, and we
/// poll for status updates with exponential backoff.
async fn background_research_with_polling(
    client: &Client,
    agent_name: &str,
) -> Result<(), Box<dyn Error>> {
    let prompt = "What are the current best practices for building production-ready \
                  REST APIs in Rust? Include framework comparisons and security considerations.";

    println!("Research query: {prompt}\n");
    println!("Starting background research...\n");

    // 6. Start the research in background mode
    let result = client
        .interaction()
        .with_agent(agent_name)
        .with_text(prompt)
        .with_background(true) // Enable background mode
        .with_store(true) // Required for stateful interactions
        .create()
        .await;

    match result {
        Ok(initial_response) => {
            println!("Research initiated!");
            println!("Initial status: {:?}", initial_response.status);
            println!("Interaction ID: {}\n", initial_response.id);

            // 7. If already completed (fast response), display results
            if initial_response.status == InteractionStatus::Completed {
                println!("Research completed immediately (very fast response).\n");
                display_research_results(&initial_response);
                return Ok(());
            }

            // 8. Poll for completion with exponential backoff
            println!(
                "Polling for completion (max wait: {:?})...\n",
                MAX_POLL_DURATION
            );

            match poll_for_completion(client, &initial_response.id).await {
                Ok(final_response) => {
                    println!("\nResearch completed!");
                    display_research_results(&final_response);
                }
                Err(PollError::Timeout) => {
                    println!(
                        "\nPolling timed out after {:?}. The research may still be running.",
                        MAX_POLL_DURATION
                    );
                    println!(
                        "You can retrieve results later using interaction ID: {}",
                        initial_response.id
                    );
                }
                Err(PollError::Failed) => {
                    println!("\nResearch task failed. Check the interaction for error details.");
                }
                Err(PollError::Api(e)) => {
                    println!("\nAPI error during polling: {:?}", e);
                }
            }
        }
        Err(e) => {
            handle_research_error(&e);
        }
    }

    Ok(())
}

/// Error type for polling operations
#[derive(Debug)]
enum PollError {
    /// Polling timed out before completion
    Timeout,
    /// The interaction failed
    Failed,
    /// An API error occurred
    Api(GenaiError),
}

impl From<GenaiError> for PollError {
    fn from(err: GenaiError) -> Self {
        PollError::Api(err)
    }
}

/// Polls for interaction completion with exponential backoff.
///
/// This function queries the API for the interaction status, using exponential
/// backoff to reduce API calls while still detecting completion quickly.
///
/// Note: This polling logic is intentionally implemented inline rather than
/// importing from test utilities. Examples should be self-contained so users
/// can copy them directly. Similar logic exists in `tests/common/mod.rs` for
/// internal test use with slightly different parameters.
async fn poll_for_completion(
    client: &Client,
    interaction_id: &str,
) -> Result<rust_genai::InteractionResponse, PollError> {
    let start = Instant::now();
    let mut delay = INITIAL_POLL_DELAY;
    let mut poll_count = 0;

    loop {
        // Check if we've exceeded the maximum wait time
        if start.elapsed() > MAX_POLL_DURATION {
            return Err(PollError::Timeout);
        }

        // Wait before polling (skip on first iteration for instant detection)
        if poll_count > 0 {
            sleep(delay).await;
            // Exponential backoff up to maximum
            delay = (delay * 2).min(MAX_POLL_DELAY);
        }
        poll_count += 1;

        // Query the interaction status
        let response = client.get_interaction(interaction_id).await?;

        println!(
            "  Poll #{}: status={:?} (elapsed: {:.1}s)",
            poll_count,
            response.status,
            start.elapsed().as_secs_f64()
        );

        // Check the status
        match response.status {
            InteractionStatus::Completed => return Ok(response),
            InteractionStatus::Failed => return Err(PollError::Failed),
            InteractionStatus::InProgress => {
                // Continue polling
            }
            InteractionStatus::RequiresAction => {
                println!("    Note: Interaction requires action (unusual for deep research)");
            }
            InteractionStatus::Cancelled => {
                println!("    Interaction was cancelled");
                return Err(PollError::Failed);
            }
            _ => {
                // Unknown status - continue polling but log it
                println!("    Unknown status, continuing to poll...");
            }
        }
    }
}

/// Displays the research results from a completed interaction
fn display_research_results(response: &rust_genai::InteractionResponse) {
    println!("Status: {:?}", response.status);
    println!("Interaction ID: {}\n", response.id);

    if let Some(text) = response.text() {
        // Truncate for display if very long
        let display_text = if text.len() > BACKGROUND_DISPLAY_LIMIT {
            format!(
                "{}...\n\n[Response truncated, {} total chars]",
                &text[..BACKGROUND_DISPLAY_LIMIT],
                text.len()
            )
        } else {
            text.to_string()
        };
        println!("Research Results:\n{display_text}\n");
    } else {
        println!("No text response received.\n");
    }

    // Show token usage
    if let Some(usage) = &response.usage {
        println!("--- Token Usage ---");
        if let Some(input) = usage.total_input_tokens {
            println!("  Input tokens: {input}");
        }
        if let Some(output) = usage.total_output_tokens {
            println!("  Output tokens: {output}");
        }
    }
}

/// Handles errors from the Deep Research agent with helpful messages
fn handle_research_error(e: &GenaiError) {
    match e {
        GenaiError::Api {
            status_code,
            message,
            request_id,
        } => {
            eprintln!("API Error (HTTP {}): {}", status_code, message);
            if let Some(id) = request_id {
                eprintln!("  Request ID: {}", id);
            }

            // Provide helpful context for common errors
            if message.contains("not found") || message.contains("not available") {
                eprintln!("\nNote: The Deep Research agent may not be available in your account.");
                eprintln!("This is a preview feature that requires specific access permissions.");
            } else if message.contains("quota") || message.contains("rate") {
                eprintln!("\nNote: You may have exceeded API quota or rate limits.");
            }
        }
        GenaiError::Http(http_err) => {
            eprintln!("HTTP Error: {http_err}");
            eprintln!("Check your network connection and try again.");
        }
        GenaiError::Json(json_err) => {
            eprintln!("JSON Error: {json_err}");
        }
        _ => {
            eprintln!("Error: {e}");
        }
    }
}
