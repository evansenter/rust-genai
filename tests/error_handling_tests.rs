//! Error handling tests for the rust-genai library.
//!
//! These tests verify that error types, error messages, and retry behavior
//! work correctly. Most tests don't require an API key.
//!
//! # Running Tests
//!
//! ```bash
//! cargo test --test error_handling_tests -- --nocapture
//! ```

mod common;

use common::{is_transient_error, retry_on_transient};
use rust_genai::GenaiError;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

// =============================================================================
// GenaiError Display Tests
// =============================================================================

#[test]
fn test_genai_error_parse_display() {
    // We can't easily create a reqwest::Error, so we test Parse error instead
    let error = GenaiError::Parse("Connection reset".to_string());
    let display = format!("{}", error);
    assert!(display.contains("SSE parsing error"));
}

#[test]
fn test_genai_error_api_429_rate_limit() {
    let error = GenaiError::Api {
        status_code: 429,
        message: "Resource exhausted".to_string(),
        request_id: Some("req-abc123".to_string()),
    };
    let display = format!("{}", error);
    assert!(display.contains("429"));
    assert!(display.contains("Resource exhausted"));

    // Verify we can pattern match on 429 for retry logic
    match &error {
        GenaiError::Api {
            status_code: 429, ..
        } => {
            // This is the pattern for rate limit handling
        }
        _ => panic!("Should match 429 status code"),
    }
}

#[test]
fn test_genai_error_api_503_service_unavailable() {
    let error = GenaiError::Api {
        status_code: 503,
        message: "Service temporarily unavailable".to_string(),
        request_id: None,
    };
    let display = format!("{}", error);
    assert!(display.contains("503"));
    assert!(display.contains("Service temporarily unavailable"));
}

#[test]
fn test_genai_error_api_400_bad_request() {
    let error = GenaiError::Api {
        status_code: 400,
        message: "Invalid model name".to_string(),
        request_id: Some("req-xyz789".to_string()),
    };
    let display = format!("{}", error);
    assert!(display.contains("400"));
    assert!(display.contains("Invalid model name"));
}

#[test]
fn test_genai_error_api_request_id_in_debug() {
    // request_id is available in Debug output (for logging) and pattern matching,
    // but intentionally excluded from Display to keep error messages concise.
    let error = GenaiError::Api {
        status_code: 500,
        message: "Server error".to_string(),
        request_id: Some("req-debug-12345".to_string()),
    };

    // Display is concise - no request_id
    let display = format!("{}", error);
    assert!(display.contains("500"));
    assert!(display.contains("Server error"));

    // Debug includes request_id for logging/diagnostics
    let debug = format!("{:?}", error);
    assert!(
        debug.contains("req-debug-12345"),
        "Debug output should include request_id: {}",
        debug
    );
}

#[test]
fn test_genai_error_malformed_response_patterns() {
    // Test various MalformedResponse scenarios documented in the codebase

    // Missing call_id in function call
    let error1 = GenaiError::MalformedResponse(
        "Function call 'get_weather' is missing required call_id field".to_string(),
    );
    let display1 = format!("{}", error1);
    assert!(display1.contains("Malformed API response"));
    assert!(display1.contains("call_id"));

    // Stream ended without Complete event
    let error2 = GenaiError::MalformedResponse("Stream ended without Complete event".to_string());
    let display2 = format!("{}", error2);
    assert!(display2.contains("Complete event"));
}

#[test]
fn test_genai_error_invalid_input() {
    let error = GenaiError::InvalidInput("Model or agent must be specified".to_string());
    let display = format!("{}", error);
    assert!(display.contains("Invalid input"));
    assert!(display.contains("Model or agent"));
}

#[test]
fn test_genai_error_internal() {
    let error = GenaiError::Internal("Exceeded maximum function call loops".to_string());
    let display = format!("{}", error);
    assert!(display.contains("Internal client error"));
    assert!(display.contains("function call loops"));
}

// =============================================================================
// Transient Error Detection Tests
// =============================================================================

#[test]
fn test_is_transient_error_spanner_utf8() {
    // This is the known transient error pattern from issue #60
    let error = GenaiError::Api {
        status_code: 500,
        message: "Spanner UTF-8 encoding error in backend".to_string(),
        request_id: None,
    };
    assert!(
        is_transient_error(&error),
        "Spanner UTF-8 error should be transient"
    );
}

#[test]
fn test_is_transient_error_case_insensitive() {
    // Test case insensitivity
    let error = GenaiError::Api {
        status_code: 500,
        message: "SPANNER UTF-8 error".to_string(),
        request_id: None,
    };
    assert!(
        is_transient_error(&error),
        "Spanner detection should be case insensitive"
    );
}

#[test]
fn test_is_transient_error_requires_both_keywords() {
    // Only "spanner" is not enough
    let error1 = GenaiError::Api {
        status_code: 500,
        message: "Spanner database error".to_string(),
        request_id: None,
    };
    assert!(
        !is_transient_error(&error1),
        "Just 'spanner' should not be transient"
    );

    // Only "utf-8" is not enough
    let error2 = GenaiError::Api {
        status_code: 500,
        message: "UTF-8 encoding error".to_string(),
        request_id: None,
    };
    assert!(
        !is_transient_error(&error2),
        "Just 'utf-8' should not be transient"
    );
}

#[test]
fn test_is_transient_error_non_api_errors() {
    // Non-API errors are never transient
    let parse_error = GenaiError::Parse("Invalid SSE".to_string());
    assert!(
        !is_transient_error(&parse_error),
        "Parse errors are not transient"
    );

    let internal_error = GenaiError::Internal("Max loops".to_string());
    assert!(
        !is_transient_error(&internal_error),
        "Internal errors are not transient"
    );

    let invalid_input = GenaiError::InvalidInput("Missing model".to_string());
    assert!(
        !is_transient_error(&invalid_input),
        "Invalid input errors are not transient"
    );

    let malformed = GenaiError::MalformedResponse("Missing call_id".to_string());
    assert!(
        !is_transient_error(&malformed),
        "MalformedResponse errors are not transient"
    );
}

#[test]
fn test_is_transient_error_regular_500() {
    // Regular 500 errors without spanner/utf-8 are not transient
    let error = GenaiError::Api {
        status_code: 500,
        message: "Internal server error".to_string(),
        request_id: None,
    };
    assert!(
        !is_transient_error(&error),
        "Generic 500 should not be transient"
    );
}

// =============================================================================
// Retry Logic Tests
// =============================================================================

#[tokio::test]
async fn test_retry_on_transient_success_first_try() {
    let call_count = Arc::new(AtomicU32::new(0));
    let count = call_count.clone();

    let result = retry_on_transient(3, || {
        let count = count.clone();
        async move {
            count.fetch_add(1, Ordering::SeqCst);
            Ok::<_, GenaiError>("success".to_string())
        }
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");
    assert_eq!(
        call_count.load(Ordering::SeqCst),
        1,
        "Should only call once on success"
    );
}

#[tokio::test]
async fn test_retry_on_transient_non_transient_error_no_retry() {
    let call_count = Arc::new(AtomicU32::new(0));
    let count = call_count.clone();

    let result = retry_on_transient(3, || {
        let count = count.clone();
        async move {
            count.fetch_add(1, Ordering::SeqCst);
            Err::<String, _>(GenaiError::InvalidInput("bad input".to_string()))
        }
    })
    .await;

    assert!(result.is_err());
    assert_eq!(
        call_count.load(Ordering::SeqCst),
        1,
        "Should not retry non-transient errors"
    );
}

#[tokio::test]
async fn test_retry_on_transient_success_after_retry() {
    let call_count = Arc::new(AtomicU32::new(0));
    let count = call_count.clone();

    let result = retry_on_transient(3, || {
        let count = count.clone();
        async move {
            let attempt = count.fetch_add(1, Ordering::SeqCst);
            if attempt == 0 {
                // First attempt fails with transient error
                Err(GenaiError::Api {
                    status_code: 500,
                    message: "Spanner UTF-8 error".to_string(),
                    request_id: None,
                })
            } else {
                // Second attempt succeeds
                Ok("recovered".to_string())
            }
        }
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "recovered");
    assert_eq!(
        call_count.load(Ordering::SeqCst),
        2,
        "Should retry once and then succeed"
    );
}

#[tokio::test]
async fn test_retry_on_transient_exhausted_retries() {
    let call_count = Arc::new(AtomicU32::new(0));
    let count = call_count.clone();

    let result = retry_on_transient(2, || {
        let count = count.clone();
        async move {
            count.fetch_add(1, Ordering::SeqCst);
            Err::<String, _>(GenaiError::Api {
                status_code: 500,
                message: "Spanner UTF-8 error".to_string(),
                request_id: None,
            })
        }
    })
    .await;

    assert!(result.is_err());
    // max_retries=2 means: initial attempt + 2 retries = 3 total attempts
    assert_eq!(
        call_count.load(Ordering::SeqCst),
        3,
        "Should attempt initial + max_retries times"
    );
}

#[tokio::test]
async fn test_retry_on_transient_zero_retries() {
    let call_count = Arc::new(AtomicU32::new(0));
    let count = call_count.clone();

    let result = retry_on_transient(0, || {
        let count = count.clone();
        async move {
            count.fetch_add(1, Ordering::SeqCst);
            Err::<String, _>(GenaiError::Api {
                status_code: 500,
                message: "Spanner UTF-8 error".to_string(),
                request_id: None,
            })
        }
    })
    .await;

    assert!(result.is_err());
    assert_eq!(
        call_count.load(Ordering::SeqCst),
        1,
        "max_retries=0 should only run once"
    );
}

// =============================================================================
// Error Matching Pattern Tests
// =============================================================================

#[test]
fn test_error_matching_for_retry_logic() {
    // Test the pattern users would use for implementing their own retry logic
    let errors = vec![
        (
            GenaiError::Api {
                status_code: 429,
                message: "Rate limited".to_string(),
                request_id: None,
            },
            "rate_limit",
        ),
        (
            GenaiError::Api {
                status_code: 500,
                message: "Server error".to_string(),
                request_id: None,
            },
            "server_error",
        ),
        (
            GenaiError::Api {
                status_code: 503,
                message: "Unavailable".to_string(),
                request_id: None,
            },
            "unavailable",
        ),
    ];

    for (error, expected_category) in errors {
        let category = match &error {
            GenaiError::Api {
                status_code: 429, ..
            } => "rate_limit",
            GenaiError::Api {
                status_code: 500, ..
            } => "server_error",
            GenaiError::Api {
                status_code: 503, ..
            } => "unavailable",
            _ => "other",
        };
        assert_eq!(
            category, expected_category,
            "Error matching should work for {:?}",
            error
        );
    }
}

#[test]
fn test_error_request_id_extraction() {
    let error = GenaiError::Api {
        status_code: 500,
        message: "Error".to_string(),
        request_id: Some("req-abc123".to_string()),
    };

    // Test pattern for extracting request_id for logging/debugging
    if let GenaiError::Api {
        request_id: Some(id),
        ..
    } = &error
    {
        assert_eq!(id, "req-abc123");
    } else {
        panic!("Should extract request_id");
    }
}

// =============================================================================
// Client Configuration Error Tests
// =============================================================================

#[test]
fn test_client_with_empty_api_key() {
    // Empty API key should still create a client (validation happens at request time)
    let client = rust_genai::Client::builder("".to_string()).build();
    // Client creation succeeds - the API will return 401 when used
    assert!(
        client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_text("test")
            .build_request()
            .is_ok()
    );
}

#[test]
fn test_interaction_builder_missing_model() {
    let client = rust_genai::Client::builder("fake-key".to_string()).build();

    // Building a request without a model should fail
    let result = client.interaction().with_text("test").build_request();

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("model") || err.contains("agent"),
        "Error should mention missing model: {}",
        err
    );
}

#[test]
fn test_interaction_builder_missing_content() {
    let client = rust_genai::Client::builder("fake-key".to_string()).build();

    // Building a request without content should fail
    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .build_request();

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("content") || err.contains("input"),
        "Error should mention missing content: {}",
        err
    );
}
