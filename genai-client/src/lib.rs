// Declare the models, errors, common, and core modules
pub mod common;
pub mod core;
pub mod errors;
pub mod models;

// Import and selectively re-export the necessary structs from the models module

// Shared types (used by multiple APIs)
pub use models::shared::CodeExecution;
pub use models::shared::Content;
pub use models::shared::FunctionCall;
pub use models::shared::FunctionCallingConfig;
pub use models::shared::FunctionCallingMode;
pub use models::shared::FunctionDeclaration;
pub use models::shared::FunctionParameters;
pub use models::shared::FunctionResponse;
pub use models::shared::Part;
pub use models::shared::Tool;
pub use models::shared::ToolConfig;

// generateContent-specific types
pub use models::request::GenerateContentRequest;

pub use models::response::Candidate;
pub use models::response::ContentResponse;
pub use models::response::FunctionCallResponse;
pub use models::response::GenerateContentResponse;
pub use models::response::PartResponse;

// Interactions API types
pub use models::interactions::CreateInteractionRequest;
pub use models::interactions::GenerationConfig;
pub use models::interactions::InteractionInput;
pub use models::interactions::InteractionResponse;
pub use models::interactions::InteractionStatus;
pub use models::interactions::UsageMetadata;

// Re-export InternalError from the errors module
pub use errors::InternalError;

// Re-export ApiVersion, Endpoint, and URL construction functions from the common module
pub use common::ApiVersion;
pub use common::construct_url;
pub use common::Endpoint;
pub use common::construct_endpoint_url;

// Re-export internal helper functions from the core module
pub use core::generate_content_internal;
pub use core::generate_content_stream_internal;
