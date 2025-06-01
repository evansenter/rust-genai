// Declare the models, errors, common, and core modules
pub mod common;
pub mod core;
pub mod errors;
pub mod models;

// Import and selectively re-export the necessary structs from the models module
pub use models::request::Content;
pub use models::request::FunctionCall;
pub use models::request::FunctionCallingConfig;
pub use models::request::FunctionCallingMode;
pub use models::request::FunctionDeclaration;
pub use models::request::FunctionParameters;
pub use models::request::FunctionResponse;
pub use models::request::GenerateContentRequest;
pub use models::request::Part;
pub use models::request::Tool;
pub use models::request::ToolConfig;

pub use models::response::Candidate;
pub use models::response::ContentResponse;
pub use models::response::FunctionCallResponse;
pub use models::response::GenerateContentResponse;
pub use models::response::PartResponse;

// Re-export InternalError from the errors module
pub use errors::InternalError;

// Re-export ApiVersion and construct_url from the common module
pub use common::ApiVersion;
pub use common::construct_url;

// Re-export internal helper functions from the core module
pub use core::generate_content_internal;
pub use core::generate_content_stream_internal;
