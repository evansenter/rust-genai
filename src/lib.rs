// Re-export the core functionality from the internal client crate
pub use genai_client::generate_content;

// You could also re-export specific types if needed for the public API, e.g.:
// pub use genai_client::{RequestConfig, SafetySetting};

// You can add higher-level wrapper functions or structs here later. 