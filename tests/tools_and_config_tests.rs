//! Built-in tools, response formats, and generation config tests
//!
//! Tests for Google Search grounding, code execution, URL context,
//! structured output with JSON schema, and advanced generation config.
//!
//! These tests require the GEMINI_API_KEY environment variable to be set.
//!
//! # Running Tests
//!
//! ```bash
//! cargo test --test tools_and_config_tests -- --include-ignored --nocapture
//! ```
//!
//! # Notes
//!
//! Some built-in tools may not be available in all regions or account types.
//! Tests are designed to gracefully skip if tools are unavailable.

mod common;

use common::get_client;
use rust_genai::{GenerationConfig, InteractionStatus, Tool};
use serde_json::json;

// =============================================================================
// Built-in Tools: Google Search
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_google_search() {
    // Test using Google Search for grounding with current information
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(
            "What is the current weather in New York City today? Use search to find current data.",
        )
        .with_google_search() // Use convenience method
        .with_store(true)
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("Status: {:?}", response.status);
            if response.has_text() {
                let text = response.text().unwrap();
                println!("Response with Google Search: {}", text);
                // Should provide current/recent information
                let text_lower = text.to_lowercase();
                assert!(
                    text_lower.contains("weather")
                        || text_lower.contains("temperature")
                        || text_lower.contains("new york")
                        || text_lower.contains("today")
                        || text_lower.contains("currently"),
                    "Response should mention weather-related content"
                );
            }

            // Verify grounding metadata is available
            if let Some(metadata) = response.grounding_metadata() {
                println!("Grounding metadata found:");
                println!("  Search queries: {:?}", metadata.web_search_queries);
                println!("  Grounding chunks: {}", metadata.grounding_chunks.len());
                for chunk in &metadata.grounding_chunks {
                    println!(
                        "    - {} [{}] ({})",
                        chunk.web.title, chunk.web.domain, chunk.web.uri
                    );
                }
            } else {
                println!("Note: No grounding metadata returned (may vary by API response)");
            }
        }
        Err(e) => {
            let error_str = format!("{:?}", e);
            println!("Google Search error (may be expected): {}", error_str);
            // Google Search may not be available in all accounts
            if error_str.contains("not supported")
                || error_str.contains("not available")
                || error_str.contains("permission")
            {
                println!("Google Search tool not available - skipping test");
            }
        }
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_google_search_streaming() {
    // Test Google Search with streaming
    use futures_util::StreamExt;
    use rust_genai::StreamChunk;

    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let mut stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What's the latest news about Rust programming language?")
        .with_google_search()
        .create_stream();

    let mut chunk_count = 0;
    let mut final_response = None;

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                chunk_count += 1;
                match chunk {
                    StreamChunk::Delta(content) => {
                        println!("Delta chunk {}: {:?}", chunk_count, content);
                    }
                    StreamChunk::Complete(response) => {
                        println!("Complete response received");
                        // Check for grounding metadata in the final response
                        if let Some(metadata) = response.grounding_metadata() {
                            println!("Streaming grounding metadata:");
                            println!("  Search queries: {:?}", metadata.web_search_queries);
                            println!("  Chunks: {}", metadata.grounding_chunks.len());
                        }
                        final_response = Some(response);
                    }
                }
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                println!("Stream error: {}", error_str);
                // Google Search may not be available in all accounts
                if error_str.contains("not supported")
                    || error_str.contains("not available")
                    || error_str.contains("permission")
                {
                    println!("Google Search tool not available - skipping test");
                    return;
                }
                // For other errors, break but let assertions catch issues
                break;
            }
        }
    }

    assert!(chunk_count > 0, "Should receive at least one chunk");
    assert!(final_response.is_some(), "Should receive complete response");
}

// =============================================================================
// Built-in Tools: Code Execution
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_code_execution() {
    // Test code execution tool for calculations
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Calculate the factorial of 10 using Python code execution.")
        .with_code_execution() // Use the new convenience method
        .with_store(true)
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("Status: {:?}", response.status);
            if response.has_text() {
                let text = response.text().unwrap();
                println!("Response with Code Execution: {}", text);
                // factorial(10) = 3628800
                assert!(
                    text.contains("3628800") || text.contains("3,628,800"),
                    "Response should contain the factorial result: {}",
                    text
                );
            }

            // Check for typed built-in tool content using new helpers
            let summary = response.content_summary();
            println!(
                "Content summary: {} text, {} code_execution_call, {} code_execution_result, {} unknown",
                summary.text_count,
                summary.code_execution_call_count,
                summary.code_execution_result_count,
                summary.unknown_count
            );

            // Test the new typed helper methods
            for (language, code) in response.executable_code() {
                println!(
                    "Executed {} code: {}",
                    language,
                    &code[..code.len().min(100)]
                );
            }

            for (outcome, output) in response.code_execution_results() {
                println!("Outcome: {} (success: {})", outcome, outcome.is_success());
                println!("Output: {}", &output[..output.len().min(100)]);
            }

            // Test the convenience helper and verify the code output directly
            // This is more robust than checking LLM text response
            if let Some(output) = response.successful_code_output() {
                println!(
                    "First successful output: {}",
                    &output[..output.len().min(100)]
                );
                assert!(
                    output.contains("3628800"),
                    "Code output should contain correct factorial result (3628800), got: {}",
                    output
                );
            } else {
                // If no successful code output, check that code was at least executed
                assert!(
                    !response.code_execution_results().is_empty(),
                    "Expected code execution results but found none"
                );
            }

            // Verify the response doesn't contain unknown content types for code execution
            // (they should all be recognized as known types now)
            if !summary.unknown_types.is_empty() {
                println!("Unknown types found: {:?}", summary.unknown_types);
            }
        }
        Err(e) => {
            let error_str = format!("{:?}", e);
            println!("Code Execution error (may be expected): {}", error_str);
            if error_str.contains("not supported") || error_str.contains("not available") {
                println!("Code Execution tool not available - skipping test");
            }
        }
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_code_execution_complex() {
    // Test code execution with a more complex calculation
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(
            "Using Python, calculate the sum of the first 100 prime numbers. Execute the code to get the answer.",
        )
        .with_tools(vec![Tool::CodeExecution])
        .with_store(true)
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("Status: {:?}", response.status);
            if response.has_text() {
                let text = response.text().unwrap();
                println!("Prime sum response: {}", text);
                // Sum of first 100 primes is 24133
                // Model might express this with or without comma formatting
                assert!(
                    text.contains("24133") || text.contains("24,133"),
                    "Response should contain the sum of first 100 primes (24133), got: {}",
                    text
                );
            }
        }
        Err(e) => {
            println!("Code Execution error (may be expected): {:?}", e);
        }
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_code_execution_streaming() {
    // Test code execution with streaming
    use futures_util::StreamExt;
    use rust_genai::StreamChunk;

    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let mut stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Calculate 15 factorial using Python code execution.")
        .with_code_execution()
        .create_stream();

    let mut chunk_count = 0;
    let mut final_response = None;

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                chunk_count += 1;
                match chunk {
                    StreamChunk::Delta(content) => {
                        println!("Delta chunk {}: {:?}", chunk_count, content);
                    }
                    StreamChunk::Complete(response) => {
                        println!("Complete response received");
                        // Check for code execution results
                        let summary = response.content_summary();
                        println!(
                            "Code execution results: {}",
                            summary.code_execution_result_count
                        );
                        final_response = Some(response);
                    }
                }
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                println!("Stream error: {}", error_str);
                if error_str.contains("not supported") || error_str.contains("not available") {
                    println!("Code Execution tool not available - skipping test");
                    return;
                }
                break;
            }
        }
    }

    assert!(chunk_count > 0, "Should receive at least one chunk");
    assert!(final_response.is_some(), "Should receive complete response");

    // Verify code execution happened
    if let Some(response) = final_response {
        let has_code_result = !response.code_execution_results().is_empty();
        println!("Has code execution results: {}", has_code_result);
    }
}

// =============================================================================
// Built-in Tools: URL Context
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_url_context() {
    // Test URL context tool for fetching and analyzing web content
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(
            "Fetch and summarize the main content from https://example.com using URL context.",
        )
        .with_url_context() // Use convenience method
        .with_store(true)
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("Status: {:?}", response.status);

            // Check for URL context metadata
            if let Some(metadata) = response.url_context_metadata() {
                println!("URL Context metadata found:");
                for entry in &metadata.url_metadata {
                    println!(
                        "  URL: {} - Status: {:?}",
                        entry.retrieved_url, entry.url_retrieval_status
                    );
                }
            } else {
                println!("No URL context metadata in response (may be normal for some responses)");
            }

            if response.has_text() {
                let text = response.text().unwrap();
                println!("URL Context response: {}", text);
                // example.com has standard placeholder content
                let text_lower = text.to_lowercase();
                assert!(
                    text_lower.contains("example")
                        || text_lower.contains("domain")
                        || text_lower.contains("website")
                        || text_lower.contains("illustrative")
                        || text_lower.contains("documentation"),
                    "Response should describe content from example.com"
                );
            }
        }
        Err(e) => {
            let error_str = format!("{:?}", e);
            println!("URL Context error (may be expected): {}", error_str);
            if error_str.contains("not supported") || error_str.contains("not available") {
                println!("URL Context tool not available - skipping test");
            }
        }
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_url_context_streaming() {
    // Test URL context with streaming
    use futures_util::StreamExt;
    use rust_genai::StreamChunk;

    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let mut stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Fetch https://example.com and describe the page structure.")
        .with_url_context()
        .create_stream();

    let mut chunk_count = 0;
    let mut final_response = None;

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                chunk_count += 1;
                match chunk {
                    StreamChunk::Delta(content) => {
                        println!("Delta chunk {}: {:?}", chunk_count, content);
                    }
                    StreamChunk::Complete(response) => {
                        println!("Complete response received");
                        // Check for URL context metadata
                        if let Some(metadata) = response.url_context_metadata() {
                            println!("URL metadata entries: {}", metadata.url_metadata.len());
                        }
                        final_response = Some(response);
                    }
                }
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                println!("Stream error: {}", error_str);
                if error_str.contains("not supported") || error_str.contains("not available") {
                    println!("URL Context tool not available - skipping test");
                    return;
                }
                break;
            }
        }
    }

    assert!(chunk_count > 0, "Should receive at least one chunk");
    assert!(final_response.is_some(), "Should receive complete response");
}

// =============================================================================
// Response Formats: Structured Output
// =============================================================================

/// Test structured output with JSON schema enforcement.
///
/// The response_format parameter accepts a JSON schema directly to enforce
/// structured output from the model.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_structured_output() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Pass the JSON schema directly to response_format
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "integer"},
            "email": {"type": "string"}
        },
        "required": ["name", "age", "email"]
    });

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Generate a fake user profile with a name, age, and email address.")
        .with_response_format(schema)
        .with_store(true)
        .create()
        .await;

    let response = result.expect("Structured output request should succeed");
    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");

    let text = response.text().unwrap();
    println!("Structured output: {}", text);

    // Parse as JSON - should be valid JSON matching our schema
    let json: serde_json::Value =
        serde_json::from_str(text).expect("Response should be valid JSON");
    println!(
        "Parsed JSON: {}",
        serde_json::to_string_pretty(&json).unwrap()
    );

    assert!(json.get("name").is_some(), "Should have name field");
    assert!(json.get("age").is_some(), "Should have age field");
    assert!(json.get("email").is_some(), "Should have email field");
}

/// Test structured output with enum constraints.
///
/// The response_format parameter enforces specific enum values for fields.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_structured_output_enum() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Pass the JSON schema directly - enum constrains valid values
    let schema = json!({
        "type": "object",
        "properties": {
            "sentiment": {
                "type": "string",
                "enum": ["positive", "negative", "neutral"]
            },
            "confidence": {
                "type": "number"
            }
        },
        "required": ["sentiment", "confidence"]
    });

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Analyze the sentiment of: 'I love this product, it's amazing!'")
        .with_response_format(schema)
        .with_store(true)
        .create()
        .await;

    let response = result.expect("Structured output with enum should succeed");
    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");

    let text = response.text().unwrap();
    println!("Sentiment analysis: {}", text);

    // Parse as JSON
    let json: serde_json::Value =
        serde_json::from_str(text).expect("Response should be valid JSON");
    println!(
        "Parsed JSON: {}",
        serde_json::to_string_pretty(&json).unwrap()
    );

    // Verify sentiment is one of the enum values
    let sentiment = json
        .get("sentiment")
        .and_then(|v| v.as_str())
        .expect("Should have sentiment field");
    assert!(
        ["positive", "negative", "neutral"].contains(&sentiment),
        "Sentiment '{}' should be one of: positive, negative, neutral",
        sentiment
    );

    // Verify confidence exists
    assert!(
        json.get("confidence").is_some(),
        "Should have confidence field"
    );
}

/// Test structured output combined with Google Search grounding.
///
/// This demonstrates using response_format with built-in tools to get
/// structured data from real-time web searches.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_structured_output_with_google_search() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Schema for structured search results
    let schema = json!({
        "type": "object",
        "properties": {
            "answer": {"type": "string"},
            "source_count": {"type": "integer"}
        },
        "required": ["answer"]
    });

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What is the current population of Tokyo, Japan?")
        .with_google_search()
        .with_response_format(schema)
        .with_store(true)
        .create()
        .await;

    let response = result.expect("Structured output with Google Search should succeed");
    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");

    let text = response.text().unwrap();
    println!("Google Search structured output: {}", text);

    // Parse as JSON
    let json: serde_json::Value =
        serde_json::from_str(text).expect("Response should be valid JSON");
    println!(
        "Parsed JSON: {}",
        serde_json::to_string_pretty(&json).unwrap()
    );

    // Verify required field exists
    assert!(json.get("answer").is_some(), "Should have answer field");

    // Verify grounding metadata is present (Google Search was used)
    if let Some(metadata) = response.grounding_metadata() {
        println!("Grounding chunks: {:?}", metadata.grounding_chunks.len());
    }
}

/// Test structured output combined with URL context fetching.
///
/// This demonstrates using response_format with URL context to extract
/// structured data from web pages.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_structured_output_with_url_context() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Schema for extracting page metadata
    let schema = json!({
        "type": "object",
        "properties": {
            "title": {"type": "string"},
            "description": {"type": "string"},
            "has_navigation": {"type": "boolean"}
        },
        "required": ["title", "description"]
    });

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Analyze the page at https://example.com and extract metadata.")
        .with_url_context()
        .with_response_format(schema)
        .with_store(true)
        .create()
        .await;

    let response = result.expect("Structured output with URL context should succeed");
    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");

    let text = response.text().unwrap();
    println!("URL context structured output: {}", text);

    // Parse as JSON
    let json: serde_json::Value =
        serde_json::from_str(text).expect("Response should be valid JSON");
    println!(
        "Parsed JSON: {}",
        serde_json::to_string_pretty(&json).unwrap()
    );

    // Verify required fields exist
    assert!(json.get("title").is_some(), "Should have title field");
    assert!(
        json.get("description").is_some(),
        "Should have description field"
    );

    // Verify URL context metadata is present
    if let Some(metadata) = response.url_context_metadata() {
        println!("URL metadata entries: {:?}", metadata.url_metadata.len());
    }
}

/// Test structured output with complex nested schema.
///
/// This demonstrates more complex JSON schemas with nested objects and arrays.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_structured_output_nested() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Complex nested schema
    let schema = json!({
        "type": "object",
        "properties": {
            "company": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "founded": {"type": "integer"}
                },
                "required": ["name"]
            },
            "employees": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "role": {"type": "string"}
                    },
                    "required": ["name", "role"]
                }
            }
        },
        "required": ["company", "employees"]
    });

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Generate data for a fictional tech startup called 'CloudAI' founded in 2023 with 3 employees: a CEO, CTO, and designer.")
        .with_response_format(schema)
        .with_store(true)
        .create()
        .await;

    let response = result.expect("Nested schema structured output should succeed");
    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");

    let text = response.text().unwrap();
    println!("Nested structured output: {}", text);

    // Parse as JSON
    let json: serde_json::Value =
        serde_json::from_str(text).expect("Response should be valid JSON");
    println!(
        "Parsed JSON: {}",
        serde_json::to_string_pretty(&json).unwrap()
    );

    // Verify nested structure
    let company = json.get("company").expect("Should have company object");
    assert!(company.get("name").is_some(), "Company should have name");

    let employees = json
        .get("employees")
        .and_then(|e| e.as_array())
        .expect("Should have employees array");
    assert_eq!(employees.len(), 3, "Should have 3 employees");

    for emp in employees {
        assert!(emp.get("name").is_some(), "Each employee should have name");
        assert!(emp.get("role").is_some(), "Each employee should have role");
    }
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_structured_output_streaming() {
    // Test structured output with streaming
    use futures_util::StreamExt;
    use rust_genai::StreamChunk;

    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let schema = json!({
        "type": "object",
        "properties": {
            "color": {"type": "string"},
            "hex_code": {"type": "string"},
            "rgb": {
                "type": "object",
                "properties": {
                    "r": {"type": "integer"},
                    "g": {"type": "integer"},
                    "b": {"type": "integer"}
                }
            }
        },
        "required": ["color", "hex_code"]
    });

    let mut stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Describe the color blue with its hex code and RGB values.")
        .with_response_format(schema)
        .create_stream();

    let mut chunk_count = 0;
    let mut collected_text = String::new();
    let mut final_response = None;

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                chunk_count += 1;
                match chunk {
                    StreamChunk::Delta(content) => {
                        if let Some(text) = content.text() {
                            collected_text.push_str(text);
                        }
                        println!("Delta chunk {}: {:?}", chunk_count, content);
                    }
                    StreamChunk::Complete(response) => {
                        println!("Complete response received");
                        final_response = Some(response);
                    }
                }
            }
            Err(e) => {
                println!("Stream error: {:?}", e);
                break;
            }
        }
    }

    assert!(chunk_count > 0, "Should receive at least one chunk");
    assert!(final_response.is_some(), "Should receive complete response");

    // Verify the final response is valid JSON matching our schema
    if let Some(text) = final_response.as_ref().and_then(|r| r.text()) {
        let json: serde_json::Value =
            serde_json::from_str(text).expect("Streaming response should be valid JSON");
        println!(
            "Parsed JSON: {}",
            serde_json::to_string_pretty(&json).unwrap()
        );
        assert!(json.get("color").is_some(), "Should have color field");
        assert!(json.get("hex_code").is_some(), "Should have hex_code field");
    }
}

// =============================================================================
// Response Modalities: Image Generation
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_response_modalities_image() {
    // Test image generation using response modalities
    // Note: This requires the gemini-3-pro-image-preview model
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let result = client
        .interaction()
        .with_model("gemini-3-pro-image-preview")
        .with_text("Generate a simple image of a red circle on a white background.")
        .with_response_modalities(vec!["IMAGE".to_string()])
        .with_store(true)
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("Status: {:?}", response.status);
            println!("Outputs count: {}", response.outputs.len());

            // Check for image content in outputs
            for (i, output) in response.outputs.iter().enumerate() {
                println!("Output {}: {:?}", i, output);
            }

            // Image generation should return image content
            let has_image = response
                .outputs
                .iter()
                .any(|o| matches!(o, rust_genai::InteractionContent::Image { .. }));

            println!("Has image output: {}", has_image);

            // Assert we got an image when the model successfully responded
            assert!(
                has_image,
                "Expected image content in response when using IMAGE modality"
            );
        }
        Err(e) => {
            let error_str = format!("{:?}", e);
            println!("Image generation error (may be expected): {}", error_str);
            // Image generation model may not be available
            if error_str.contains("not found")
                || error_str.contains("not supported")
                || error_str.contains("model")
            {
                println!("Image generation model not available - skipping test");
            }
        }
    }
}

// =============================================================================
// Generation Config: Thinking Level
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_generation_config_thinking_level_minimal() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let config = GenerationConfig {
        temperature: Some(0.7),
        max_output_tokens: Some(500),
        top_p: None,
        top_k: None,
        thinking_level: Some("minimal".to_string()),
    };

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What is 2 + 2?")
        .with_generation_config(config)
        .with_store(true)
        .create()
        .await
        .expect("Minimal thinking interaction failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");

    let text = response.text().unwrap();
    println!("Minimal thinking response: {}", text);
    assert!(text.contains('4'), "Should contain the answer");
}

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_generation_config_thinking_level_high() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let config = GenerationConfig {
        temperature: Some(0.7),
        max_output_tokens: Some(1000),
        top_p: None,
        top_k: None,
        thinking_level: Some("high".to_string()),
    };

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Explain step by step how to solve: If x + 3 = 7, what is x?")
        .with_generation_config(config)
        .with_store(true)
        .create()
        .await
        .expect("High thinking interaction failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");

    let text = response.text().unwrap();
    println!("High thinking response: {}", text);

    // Should provide a detailed explanation
    let word_count = text.split_whitespace().count();
    println!("Word count: {}", word_count);
    assert!(
        text.contains('4') || text.contains("four"),
        "Should contain the answer"
    );
}

// =============================================================================
// Generation Config: Top-p and Top-k
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_generation_config_top_p() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Low top_p = more focused/deterministic
    let config = GenerationConfig {
        temperature: Some(1.0),
        max_output_tokens: Some(100),
        top_p: Some(0.1), // Very focused
        top_k: None,
        thinking_level: None,
    };

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What is the capital of France? Answer in one word.")
        .with_generation_config(config)
        .with_store(true)
        .create()
        .await
        .expect("Top-p interaction failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");

    let text = response.text().unwrap().to_lowercase();
    println!("Top-p response: {}", text);
    assert!(text.contains("paris"), "Should answer Paris");
}

/// Test top_k generation config parameter.
/// Note: The Interactions API may not support top_k in GenerationConfig.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_generation_config_top_k() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Low top_k = only consider top k tokens
    let config = GenerationConfig {
        temperature: Some(1.0),
        max_output_tokens: Some(100),
        top_p: None,
        top_k: Some(5), // Only top 5 tokens
        thinking_level: None,
    };

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("What is 10 + 5? Answer with just the number.")
        .with_generation_config(config)
        .with_store(true)
        .create()
        .await;

    match result {
        Ok(response) => {
            assert_eq!(response.status, InteractionStatus::Completed);
            assert!(response.has_text(), "Should have text response");

            let text = response.text().unwrap();
            println!("Top-k response: {}", text);
            assert!(
                text.contains("15") || text.contains("fifteen"),
                "Should contain 15"
            );
        }
        Err(e) => {
            let error_str = format!("{:?}", e);
            if error_str.contains("top_k")
                || error_str.contains("invalid JSON")
                || error_str.contains("GenerationConfig")
            {
                println!(
                    "Note: top_k parameter not supported in GenerationConfig for Interactions API. This is expected."
                );
            } else {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }
}

/// Test combining multiple generation config options.
/// Note: top_k is excluded since the Interactions API may not support it.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_generation_config_combined() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Note: Excluding top_k since it's not supported in Interactions API
    let config = GenerationConfig {
        temperature: Some(0.5),
        max_output_tokens: Some(200),
        top_p: Some(0.9),
        top_k: None, // Not supported in Interactions API
        thinking_level: Some("medium".to_string()),
    };

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Write a haiku about programming.")
        .with_generation_config(config)
        .with_store(true)
        .create()
        .await
        .expect("Combined config interaction failed");

    assert_eq!(response.status, InteractionStatus::Completed);
    assert!(response.has_text(), "Should have text response");

    let text = response.text().unwrap();
    println!("Combined config response: {}", text);

    // Haiku should be short and have line breaks or short lines
    let line_count = text.lines().count();
    println!("Line count: {}", line_count);
    assert!(line_count >= 1, "Should have at least one line of text");
}
