// Re-export the core functionality from the internal client crate
pub use genai_client::generate_content;
pub use genai_client::generate_content_stream;

// Re-export error type for convenience
pub use genai_client::StreamingError;

// You could also re-export specific types if needed for the public API, e.g.:
// pub use genai_client::{RequestConfig, SafetySetting};

// You can add higher-level wrapper functions or structs here later. 