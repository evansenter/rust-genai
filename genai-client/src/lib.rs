// Declare the models, errors, common, interactions, and sse_parser modules
pub mod common;
pub mod error_helpers;
pub mod errors;
pub mod interactions;
pub mod models;
pub mod sse_parser;

// Import and selectively re-export the necessary structs from the models module

// Shared types for the Interactions API
pub use models::shared::FunctionCall;
pub use models::shared::FunctionCallingConfig;
pub use models::shared::FunctionCallingMode;
pub use models::shared::FunctionDeclaration;
pub use models::shared::FunctionDeclarationBuilder;
pub use models::shared::FunctionParameters;
pub use models::shared::Tool;
pub use models::shared::ToolConfig;

// Interactions API types
pub use models::interactions::ContentSummary;
pub use models::interactions::CreateInteractionRequest;
pub use models::interactions::GenerationConfig;
pub use models::interactions::GroundingChunk;
pub use models::interactions::GroundingMetadata;
pub use models::interactions::InteractionContent;
pub use models::interactions::InteractionInput;
pub use models::interactions::InteractionResponse;
pub use models::interactions::InteractionStatus;
pub use models::interactions::StreamChunk;
pub use models::interactions::UsageMetadata;
pub use models::interactions::WebSource;

// Re-export InternalError from the errors module
pub use errors::InternalError;

// Re-export ApiVersion, Endpoint, and URL construction functions from the common module
pub use common::ApiVersion;
pub use common::Endpoint;
pub use common::construct_endpoint_url;

// Re-export Interactions API functions from the interactions module
pub use interactions::create_interaction;
pub use interactions::create_interaction_stream;
pub use interactions::delete_interaction;
pub use interactions::get_interaction;
