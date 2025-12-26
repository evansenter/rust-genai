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
}
