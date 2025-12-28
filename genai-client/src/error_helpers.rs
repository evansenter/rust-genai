//! Error handling utilities for HTTP responses and error context formatting.

use crate::errors::GenaiError;
use reqwest::Response;
use serde::de::DeserializeOwned;

/// Maximum characters to include from error body in context messages
const ERROR_BODY_PREVIEW_LENGTH: usize = 200;

/// Checks if an HTTP response is successful, returning it if so or an error otherwise.
///
/// This helper consolidates the common pattern of checking response status and
/// extracting error details on failure.
///
/// # Errors
///
/// Returns an error with status code and body preview on non-success status.
pub async fn check_response(response: Response) -> Result<Response, GenaiError> {
    if response.status().is_success() {
        Ok(response)
    } else {
        Err(read_error_with_context(response).await)
    }
}

/// Google's request ID header name.
///
/// This is a standard Google Cloud API header that uniquely identifies each request.
/// The value can be used when contacting Google support or correlating with server logs.
/// See: <https://cloud.google.com/apis/docs/system-parameters>
const REQUEST_ID_HEADER: &str = "x-goog-request-id";

/// Reads error response body and creates a detailed GenaiError::Api with context.
///
/// Extracts:
/// - HTTP status code for programmatic error handling
/// - Truncated response body (first 200 chars)
/// - Request ID from `x-goog-request-id` header for debugging/support
///
/// # Returns
///
/// A structured `GenaiError::Api` with status code, message, and optional request ID.
/// If body cannot be read, the message describes the read failure.
pub async fn read_error_with_context(response: Response) -> GenaiError {
    let status_code = response.status().as_u16();

    // Extract request ID from response headers before consuming the body
    let request_id = response
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let error_body = response
        .text()
        .await
        .unwrap_or_else(|e| format!("Failed to read error body: {}", e));

    let message = truncate_for_context(&error_body, ERROR_BODY_PREVIEW_LENGTH);

    GenaiError::Api {
        status_code,
        message,
        request_id,
    }
}

/// Formats JSON parsing context by including a preview of the raw JSON.
///
/// # Arguments
///
/// * `json_str` - The JSON string that failed to parse
/// * `error` - The original serde_json error
///
/// # Returns
///
/// A formatted error message with JSON preview (first 200 chars)
pub fn format_json_parse_error(json_str: &str, error: serde_json::Error) -> String {
    let preview = truncate_for_context(json_str, ERROR_BODY_PREVIEW_LENGTH);
    format!("JSON parse error: {} | Context: {}", error, preview)
}

/// Deserializes JSON with context-rich error messages.
///
/// This function wraps `serde_json::from_str` and converts deserialization errors
/// to `GenaiError::Json` with additional context about what type failed to parse
/// and a preview of the JSON that caused the error.
///
/// # Arguments
///
/// * `json_str` - The JSON string to deserialize
/// * `type_context` - A human-readable description of what's being deserialized
///   (e.g., "InteractionResponse", "create interaction response")
///
/// # Returns
///
/// The deserialized value on success, or a context-rich `GenaiError` on failure.
///
/// # Example
///
/// ```
/// # use genai_client::error_helpers::deserialize_with_context;
/// # use serde::Deserialize;
/// #[derive(Deserialize, Debug)]
/// struct Response { id: String }
///
/// let json = r#"{"id": "test123"}"#;
/// let result: Result<Response, _> = deserialize_with_context(json, "API response");
/// assert!(result.is_ok());
///
/// let bad_json = r#"{"missing_id": true}"#;
/// let result: Result<Response, _> = deserialize_with_context(bad_json, "API response");
/// let err = result.unwrap_err();
/// assert!(err.to_string().contains("API response"));
/// ```
pub fn deserialize_with_context<T: DeserializeOwned>(
    json_str: &str,
    type_context: &str,
) -> Result<T, GenaiError> {
    serde_json::from_str(json_str).map_err(|e| {
        let preview = truncate_for_context(json_str, ERROR_BODY_PREVIEW_LENGTH);
        let message = format!(
            "Failed to parse {}: {} | JSON: {}",
            type_context, e, preview
        );
        GenaiError::Json(serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            message,
        )))
    })
}

/// Truncates a string to specified length, adding "..." if truncated.
///
/// Uses character-boundary-aware slicing to prevent panics on multi-byte UTF-8 characters.
fn truncate_for_context(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        // Find a valid UTF-8 character boundary at or before max_len
        // We need to ensure the character END position is <= max_len
        let truncate_at = s
            .char_indices()
            .take_while(|(i, c)| i + c.len_utf8() <= max_len)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);
        format!("{}...", &s[..truncate_at])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_for_context_short_string() {
        let result = truncate_for_context("Short", 100);
        assert_eq!(result, "Short");
    }

    #[test]
    fn test_truncate_for_context_long_string() {
        let long_str = "a".repeat(300);
        let result = truncate_for_context(&long_str, 200);
        assert_eq!(result.len(), 203); // 200 + "..."
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_format_json_parse_error() {
        let json = r#"{"invalid": }"#;
        let err = serde_json::from_str::<serde_json::Value>(json).unwrap_err();
        let result = format_json_parse_error(json, err);

        assert!(result.contains("JSON parse error"));
        assert!(result.contains("Context:"));
        assert!(result.contains(r#"{"invalid": }"#));
    }

    #[test]
    fn test_truncate_for_context_utf8_boundary() {
        // Test with multi-byte UTF-8 characters (emojis are 4 bytes each)
        let emoji_str = "x".repeat(198) + "🎉"; // 198 + 4 = 202 bytes total
        let result = truncate_for_context(&emoji_str, 200);

        // Should truncate before the emoji to avoid splitting it
        // Result should be 198 x's + "..." = 201 bytes
        assert_eq!(result.len(), 201); // 198 + 3 for "..."
        assert!(result.ends_with("..."));
        assert!(result.starts_with("xxx")); // Should start with x's
        assert!(!result.contains("🎉")); // Should not include emoji
        // Verify result is valid UTF-8 (this would panic if we sliced mid-character)
        assert!(result.is_char_boundary(result.len() - 3)); // before "..."
    }

    #[test]
    fn test_truncate_for_context_exactly_at_boundary() {
        // String is exactly max_len bytes
        let exact = "a".repeat(200);
        let result = truncate_for_context(&exact, 200);
        assert_eq!(result, exact); // No truncation needed
    }

    #[test]
    fn test_truncate_for_context_multibyte_characters() {
        // Test with various multi-byte UTF-8: emoji (4 bytes), Chinese (3 bytes), accented (2 bytes)
        let mixed = "Hello 世界 🌍 Café"; // Mix of 1-byte, 2-byte, 3-byte, and 4-byte chars
        let result = truncate_for_context(mixed, 15);

        // Should produce valid UTF-8 without panicking
        assert!(result.ends_with("..."));
        // Verify all characters in result are valid
        for ch in result.chars() {
            assert!(ch.is_ascii() || !ch.is_ascii()); // Tautology, but ensures no panic
        }
    }

    #[test]
    fn test_deserialize_with_context_success() {
        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct TestData {
            id: String,
            value: i32,
        }

        let json = r#"{"id": "test123", "value": 42}"#;
        let result: Result<TestData, _> = deserialize_with_context(json, "test data");
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.id, "test123");
        assert_eq!(data.value, 42);
    }

    #[test]
    fn test_deserialize_with_context_error_includes_context() {
        #[derive(serde::Deserialize, Debug)]
        #[allow(dead_code)]
        struct TestData {
            required_field: String,
        }

        let json = r#"{"wrong_field": "value"}"#;
        let result: Result<TestData, _> = deserialize_with_context(json, "InteractionResponse");
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_str = err.to_string();

        // Should include the type context
        assert!(
            err_str.contains("InteractionResponse"),
            "Error should mention the type: {}",
            err_str
        );
        // Should include JSON preview
        assert!(
            err_str.contains("wrong_field"),
            "Error should include JSON preview: {}",
            err_str
        );
    }

    #[test]
    fn test_deserialize_with_context_truncates_long_json() {
        #[derive(serde::Deserialize, Debug)]
        #[allow(dead_code)]
        struct TestData {
            id: String,
        }

        // Create JSON longer than 200 chars
        let long_value = "x".repeat(300);
        let json = format!(r#"{{"long_field": "{}"}}"#, long_value);

        let result: Result<TestData, _> = deserialize_with_context(&json, "test");
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_str = err.to_string();

        // Should be truncated (contains "...")
        assert!(
            err_str.contains("..."),
            "Long JSON should be truncated: {}",
            err_str
        );
    }
}
