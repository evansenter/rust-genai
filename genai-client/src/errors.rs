use thiserror::Error;

/// Defines errors that can occur when interacting with the GenAI API.
///
/// # Example: Handling API Errors
///
/// ```ignore
/// match client.interaction().create().await {
///     Err(GenaiError::Api { status_code: 429, request_id, .. }) => {
///         log::warn!("Rate limited, request_id: {:?}", request_id);
///         // Retry with backoff
///     }
///     Err(GenaiError::Api { status_code, message, request_id }) => {
///         log::error!("API error {}: {} (request: {:?})", status_code, message, request_id);
///     }
///     // ...
/// }
/// ```
#[derive(Debug, Error)]
pub enum GenaiError {
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("SSE parsing error: {0}")]
    Parse(String),
    #[error("JSON deserialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("UTF-8 decoding error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    /// API error with structured context for debugging and automated handling.
    ///
    /// Contains the HTTP status code (for retry logic), error message, and
    /// optional request ID (for correlation with Google API logs/support).
    #[error("API error (HTTP {status_code}): {message}")]
    Api {
        /// HTTP status code (e.g., 400, 429, 500)
        status_code: u16,
        /// Error message from the API response body
        message: String,
        /// Request ID from `x-goog-request-id` header, if available
        request_id: Option<String>,
    },
    #[error("Internal client error: {0}")]
    Internal(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    /// API returned a successful response but with unexpected or invalid content.
    ///
    /// This indicates the API response didn't match the expected schema,
    /// possibly due to API evolution or an undocumented response format.
    /// Unlike `InvalidInput` (user's fault), this represents an issue with
    /// the API response itself.
    #[error("Malformed API response: {0}")]
    MalformedResponse(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genai_error_parse_display() {
        let error = GenaiError::Parse("Invalid SSE format".to_string());
        let display = format!("{}", error);
        assert!(display.contains("SSE parsing error"));
        assert!(display.contains("Invalid SSE format"));
    }

    #[test]
    fn test_genai_error_api_display() {
        let error = GenaiError::Api {
            status_code: 429,
            message: "Rate limited".to_string(),
            request_id: Some("req-123".to_string()),
        };
        let display = format!("{}", error);
        assert!(display.contains("429"));
        assert!(display.contains("Rate limited"));
    }

    #[test]
    fn test_genai_error_api_without_request_id() {
        let error = GenaiError::Api {
            status_code: 500,
            message: "Internal error".to_string(),
            request_id: None,
        };
        let display = format!("{}", error);
        assert!(display.contains("500"));
        assert!(display.contains("Internal error"));
    }

    #[test]
    fn test_genai_error_internal_display() {
        let error = GenaiError::Internal("Max function loops exceeded".to_string());
        let display = format!("{}", error);
        assert!(display.contains("Internal client error"));
        assert!(display.contains("Max function loops exceeded"));
    }

    #[test]
    fn test_genai_error_invalid_input_display() {
        let error = GenaiError::InvalidInput("Missing model or agent".to_string());
        let display = format!("{}", error);
        assert!(display.contains("Invalid input"));
        assert!(display.contains("Missing model or agent"));
    }

    #[test]
    fn test_genai_error_json_from() {
        let json_str = "not valid json";
        let json_err = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let genai_err: GenaiError = json_err.into();
        let display = format!("{}", genai_err);
        assert!(display.contains("JSON deserialization error"));
    }

    #[test]
    fn test_genai_error_utf8_from() {
        // Create an invalid UTF-8 byte sequence
        let bytes = vec![0xff, 0xfe];
        let utf8_err = std::str::from_utf8(&bytes).unwrap_err();
        let genai_err: GenaiError = utf8_err.into();
        let display = format!("{}", genai_err);
        assert!(display.contains("UTF-8 decoding error"));
    }

    #[test]
    fn test_genai_error_debug_format() {
        let error = GenaiError::Api {
            status_code: 400,
            message: "Bad request".to_string(),
            request_id: Some("req-456".to_string()),
        };
        let debug = format!("{:?}", error);
        assert!(debug.contains("Api"));
        assert!(debug.contains("400"));
        assert!(debug.contains("req-456"));
    }

    #[test]
    fn test_genai_error_api_status_codes() {
        // Test common HTTP status codes
        let status_codes = [
            (400, "Bad Request"),
            (401, "Unauthorized"),
            (403, "Forbidden"),
            (404, "Not Found"),
            (429, "Too Many Requests"),
            (500, "Internal Server Error"),
            (503, "Service Unavailable"),
        ];

        for (code, message) in status_codes {
            let error = GenaiError::Api {
                status_code: code,
                message: message.to_string(),
                request_id: None,
            };
            let display = format!("{}", error);
            assert!(
                display.contains(&code.to_string()),
                "Expected {} in display: {}",
                code,
                display
            );
        }
    }

    #[test]
    fn test_genai_error_api_with_empty_message() {
        // Some APIs might return empty error messages
        let error = GenaiError::Api {
            status_code: 500,
            message: "".to_string(),
            request_id: None,
        };
        let display = format!("{}", error);
        assert!(display.contains("500"));
        // Should still display properly even with empty message
        assert!(display.contains("API error"));
    }

    #[test]
    fn test_genai_error_malformed_response_display() {
        let error = GenaiError::MalformedResponse(
            "Function call 'get_weather' is missing required call_id field".to_string(),
        );
        let display = format!("{}", error);
        assert!(display.contains("Malformed API response"));
        assert!(display.contains("call_id"));
    }

    #[test]
    fn test_genai_error_malformed_response_stream() {
        let error =
            GenaiError::MalformedResponse("Stream ended without Complete event".to_string());
        let display = format!("{}", error);
        assert!(display.contains("Malformed API response"));
        assert!(display.contains("Complete event"));
    }
}
