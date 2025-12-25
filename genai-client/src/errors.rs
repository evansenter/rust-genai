use thiserror::Error;

// Define an INTERNAL error type for this crate
#[derive(Debug, Error)]
pub enum InternalError {
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("SSE parsing error: {0}")]
    Parse(String),
    #[error("JSON deserialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("UTF-8 decoding error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    /// API error with structured context for debugging.
    ///
    /// Contains the HTTP status code, error message, and optional request ID
    /// for correlation with Google API logs.
    #[error("API error (HTTP {status_code}): {message}")]
    Api {
        /// HTTP status code (e.g., 400, 429, 500)
        status_code: u16,
        /// Error message from the API response body
        message: String,
        /// Request ID from `x-goog-request-id` header, if available
        request_id: Option<String>,
    },
}
