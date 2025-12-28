//! Built-in tools multi-turn tests
//!
//! Tests for Google Search, URL Context, and Code Execution across multiple
//! conversation turns.
//!
//! These tests require the GEMINI_API_KEY environment variable to be set.
//!
//! # Running Tests
//!
//! ```bash
//! cargo test --test tools_multiturn_tests -- --include-ignored --nocapture
//! ```

mod common;

use common::{DEFAULT_MAX_RETRIES, get_client, retry_on_transient};
use rust_genai::InteractionStatus;

/// Checks if an error is a known API limitation for long conversation chains.
/// These errors (UTF-8 encoding issues, spanner errors, truncation) can occur
/// when the conversation context becomes too large.
fn is_long_conversation_api_error(error: &rust_genai::GenaiError) -> bool {
    let error_str = format!("{:?}", error);
    error_str.contains("UTF-8") || error_str.contains("spanner") || error_str.contains("truncated")
}

// =============================================================================
// Multi-turn: Built-in Tools
// =============================================================================

/// Test Google Search grounding across multiple conversation turns.
///
/// This validates that:
/// - Google Search grounding works in stateful conversations
/// - Search results from Turn 1 are accessible in follow-up turns
/// - The model can reason about previously fetched search data
///
/// Turn 1: Ask about current information (triggers search)
/// Turn 2: Ask follow-up about the search results
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_google_search_multi_turn() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    println!("=== Google Search + Multi-turn ===");

    // Turn 1: Ask about current weather (requires real-time data)
    println!("\n--- Turn 1: Initial search query ---");
    let result1 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(
            "What is the current weather in Tokyo, Japan today? Use search to find current data.",
        )
        .with_google_search()
        .with_store(true)
        .create()
        .await;

    let response1 = match result1 {
        Ok(response) => {
            println!("Turn 1 status: {:?}", response.status);
            if let Some(text) = response.text() {
                println!("Turn 1 response: {}", text);
            }
            // Log grounding metadata if available
            if let Some(metadata) = response.google_search_metadata() {
                println!("Grounding metadata found:");
                println!("  Search queries: {:?}", metadata.web_search_queries);
                println!("  Grounding chunks: {}", metadata.grounding_chunks.len());
            } else {
                println!("Note: No grounding metadata returned (may vary by API response)");
            }
            response
        }
        Err(e) => {
            let error_str = format!("{:?}", e);
            println!("Google Search error: {}", error_str);
            // Google Search may not be available in all accounts
            if error_str.contains("not supported")
                || error_str.contains("not available")
                || error_str.contains("permission")
            {
                println!("Google Search tool not available - skipping test");
                return;
            }
            panic!("Turn 1 failed unexpectedly: {:?}", e);
        }
    };

    assert_eq!(
        response1.status,
        InteractionStatus::Completed,
        "Turn 1 should complete successfully"
    );

    // Turn 2: Ask follow-up referencing the search results
    println!("\n--- Turn 2: Follow-up about search ---");
    let result2 = retry_on_transient(DEFAULT_MAX_RETRIES, || async {
        client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_previous_interaction(&response1.id)
            .with_text("Based on the weather information you just found, should I bring an umbrella if I visit Tokyo today?")
            .with_store(true)
            .create()
            .await
    })
    .await;

    match result2 {
        Ok(response2) => {
            println!("Turn 2 status: {:?}", response2.status);
            if let Some(text) = response2.text() {
                println!("Turn 2 response: {}", text);
                // Verify response references the previous weather context
                let text_lower = text.to_lowercase();
                assert!(
                    text_lower.contains("tokyo")
                        || text_lower.contains("weather")
                        || text_lower.contains("umbrella")
                        || text_lower.contains("rain")
                        || text_lower.contains("sun")
                        || text_lower.contains("yes")
                        || text_lower.contains("no"),
                    "Turn 2 should reference weather context. Got: {}",
                    text
                );
            }
            assert_eq!(
                response2.status,
                InteractionStatus::Completed,
                "Turn 2 should complete successfully"
            );
        }
        Err(e) => {
            // Check for transient errors that might occur with multi-turn
            if is_long_conversation_api_error(&e) {
                println!(
                    "API limitation encountered (expected for some contexts): {:?}",
                    e
                );
                return;
            }
            panic!("Turn 2 failed unexpectedly: {:?}", e);
        }
    }

    println!("\n✓ Google Search + multi-turn completed successfully");
}

/// Test URL context fetching across multiple conversation turns.
///
/// This validates that:
/// - URL context tool works in stateful conversations
/// - Fetched URL content from Turn 1 is preserved in conversation context
/// - The model can answer follow-up questions about fetched content
///
/// Turn 1: Fetch and summarize https://example.com
/// Turn 2: Ask specific question about the fetched content
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_url_context_multi_turn() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    println!("=== URL Context + Multi-turn ===");

    // Turn 1: Fetch example.com content
    println!("\n--- Turn 1: Fetch URL content ---");
    let result1 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(
            "Fetch and summarize the main content from https://example.com using URL context.",
        )
        .with_url_context()
        .with_store(true)
        .create()
        .await;

    let response1 = match result1 {
        Ok(response) => {
            println!("Turn 1 status: {:?}", response.status);
            if let Some(text) = response.text() {
                println!("Turn 1 response: {}", text);
            }
            // Log URL context metadata if available
            if let Some(metadata) = response.url_context_metadata() {
                println!("URL context metadata found:");
                for entry in &metadata.url_metadata {
                    println!(
                        "  URL: {} - Status: {:?}",
                        entry.retrieved_url, entry.url_retrieval_status
                    );
                }
            } else {
                println!("Note: No URL context metadata returned (may vary by API response)");
            }
            response
        }
        Err(e) => {
            let error_str = format!("{:?}", e);
            println!("URL Context error: {}", error_str);
            // URL Context may not be available in all accounts
            if error_str.contains("not supported") || error_str.contains("not available") {
                println!("URL Context tool not available - skipping test");
                return;
            }
            panic!("Turn 1 failed unexpectedly: {:?}", e);
        }
    };

    assert_eq!(
        response1.status,
        InteractionStatus::Completed,
        "Turn 1 should complete successfully"
    );

    // Turn 2: Ask follow-up about the fetched content
    println!("\n--- Turn 2: Follow-up about URL content ---");
    let result2 = retry_on_transient(DEFAULT_MAX_RETRIES, || async {
        client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_previous_interaction(&response1.id)
            .with_text("What is the main purpose of that website you just fetched? Is it a real company or an example domain?")
            .create()
            .await
    })
    .await;

    match result2 {
        Ok(response2) => {
            println!("Turn 2 status: {:?}", response2.status);
            if let Some(text) = response2.text() {
                println!("Turn 2 response: {}", text);
                // Verify response references the example.com content
                let text_lower = text.to_lowercase();
                assert!(
                    text_lower.contains("example")
                        || text_lower.contains("domain")
                        || text_lower.contains("placeholder")
                        || text_lower.contains("illustrative")
                        || text_lower.contains("documentation")
                        || text_lower.contains("reserved"),
                    "Turn 2 should reference example.com content. Got: {}",
                    text
                );
            }
            assert_eq!(
                response2.status,
                InteractionStatus::Completed,
                "Turn 2 should complete successfully"
            );
        }
        Err(e) => {
            // Check for transient errors that might occur with multi-turn
            if is_long_conversation_api_error(&e) {
                println!(
                    "API limitation encountered (expected for some contexts): {:?}",
                    e
                );
                return;
            }
            panic!("Turn 2 failed unexpectedly: {:?}", e);
        }
    }

    println!("\n✓ URL Context + multi-turn completed successfully");
}

/// Test code execution across multiple conversation turns.
///
/// This validates that:
/// - Code execution works in stateful conversations
/// - Results from Turn 1 code execution can be referenced in Turn 2
/// - The model can build upon previous calculations
///
/// Turn 1: Calculate factorial of 5 (= 120)
/// Turn 2: Multiply that result by 2 (= 240)
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_code_execution_multi_turn() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    println!("=== Code Execution + Multi-turn ===");

    // Turn 1: Calculate factorial of 5
    println!("\n--- Turn 1: Calculate factorial ---");
    let result1 = retry_on_transient(DEFAULT_MAX_RETRIES, || async {
        client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_text("Calculate the factorial of 5 using code execution. Return just the number.")
            .with_code_execution()
            .with_store(true)
            .create()
            .await
    })
    .await;

    let response1 = match result1 {
        Ok(response) => {
            println!("Turn 1 status: {:?}", response.status);
            if let Some(text) = response.text() {
                println!("Turn 1 response: {}", text);
            }
            // Log code execution results if available
            let results = response.code_execution_results();
            if !results.is_empty() {
                println!("Code execution results:");
                for result in &results {
                    println!("  Outcome: {:?}", result.outcome);
                    println!("  Output: {}", result.output);
                }
            }
            response
        }
        Err(e) => {
            let error_str = format!("{:?}", e);
            println!("Code Execution error: {}", error_str);
            if error_str.contains("not supported") || error_str.contains("not available") {
                println!("Code Execution tool not available - skipping test");
                return;
            }
            panic!("Turn 1 failed unexpectedly: {:?}", e);
        }
    };

    assert_eq!(
        response1.status,
        InteractionStatus::Completed,
        "Turn 1 should complete successfully"
    );

    // Turn 2: Multiply the result by 2
    println!("\n--- Turn 2: Multiply result by 2 ---");
    let result2 = retry_on_transient(DEFAULT_MAX_RETRIES, || async {
        client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_previous_interaction(&response1.id)
            .with_text(
                "Multiply the factorial result you just calculated by 2. What is the answer?",
            )
            .with_code_execution()
            .with_store(true)
            .create()
            .await
    })
    .await;

    match result2 {
        Ok(response2) => {
            println!("Turn 2 status: {:?}", response2.status);
            if let Some(text) = response2.text() {
                println!("Turn 2 response: {}", text);
                // 5! = 120, 120 * 2 = 240
                assert!(
                    text.contains("240"),
                    "Turn 2 should calculate 120 * 2 = 240. Got: {}",
                    text
                );
            }
            // Also check code execution output
            let results = response2.code_execution_results();
            for result in &results {
                if result.output.contains("240") {
                    println!("Verified: Code output contains 240");
                }
            }
            assert_eq!(
                response2.status,
                InteractionStatus::Completed,
                "Turn 2 should complete successfully"
            );
        }
        Err(e) => {
            if is_long_conversation_api_error(&e) {
                println!(
                    "API limitation encountered (expected for some contexts): {:?}",
                    e
                );
                return;
            }
            panic!("Turn 2 failed unexpectedly: {:?}", e);
        }
    }

    println!("\n✓ Code Execution + multi-turn completed successfully");
}
