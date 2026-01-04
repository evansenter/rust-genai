//! # genai-client
//!
//! Internal HTTP client and JSON models for the Gemini Interactions API.
//!
//! This crate provides the low-level building blocks used by `rust-genai`.
//! Most users should use `rust-genai` directly rather than this crate.
//!
//! ## Crate Organization
//!
//! - [`models`]: JSON types for API requests/responses
//! - [`interactions`]: HTTP client functions for the Interactions API
//! - [`errors`]: Error types with structured API error information
//!
//! ## Forward Compatibility
//!
//! Types in this crate follow the Evergreen philosophy:
//! - Enums like [`InteractionContent`] and [`Tool`] include `Unknown` variants
//! - These capture unrecognized API types without deserialization failures
//! - Use `#[non_exhaustive]` to ensure match statements handle future variants

// Declare modules (common, error_helpers, loud_wire, sse_parser are crate-internal)
pub(crate) mod common;
pub(crate) mod error_helpers;
pub mod errors;
pub mod files;
pub mod interactions;
pub(crate) mod loud_wire;
pub mod models;
pub(crate) mod sse_parser;

// Import and selectively re-export the necessary structs from the models module

// Shared types for the Interactions API
pub use models::shared::FunctionCallingMode;
pub use models::shared::FunctionDeclaration;
pub use models::shared::FunctionDeclarationBuilder;
pub use models::shared::FunctionParameters;
pub use models::shared::Tool;

// Interactions API types
pub use models::interactions::AgentConfig;
pub use models::interactions::Annotation;
pub use models::interactions::CodeExecutionCallInfo;
pub use models::interactions::CodeExecutionLanguage;
pub use models::interactions::CodeExecutionOutcome;
pub use models::interactions::CodeExecutionResultInfo;
pub use models::interactions::ContentSummary;
pub use models::interactions::CreateInteractionRequest;
pub use models::interactions::DeepResearchConfig;
pub use models::interactions::DynamicConfig;
pub use models::interactions::FunctionCallInfo;
pub use models::interactions::FunctionResultInfo;
pub use models::interactions::GenerationConfig;
pub use models::interactions::GoogleSearchResultItem;
pub use models::interactions::GroundingChunk;
pub use models::interactions::GroundingMetadata;
pub use models::interactions::InteractionContent;
pub use models::interactions::InteractionInput;
pub use models::interactions::InteractionResponse;
pub use models::interactions::InteractionStatus;
pub use models::interactions::ModalityTokens;
pub use models::interactions::OwnedFunctionCallInfo;
pub use models::interactions::StreamChunk;
pub use models::interactions::StreamEvent;
pub use models::interactions::ThinkingLevel;
pub use models::interactions::ThinkingSummaries;
pub use models::interactions::UrlContextMetadata;
pub use models::interactions::UrlContextResultInfo;
pub use models::interactions::UrlMetadataEntry;
pub use models::interactions::UrlRetrievalStatus;
pub use models::interactions::UsageMetadata;
pub use models::interactions::WebSource;

// Re-export GenaiError from the errors module
pub use errors::GenaiError;

// Re-export Files API types and functions
pub use files::{
    DEFAULT_CHUNK_SIZE, FileError, FileMetadata, FileState, ListFilesResponse, ResumableUpload,
    VideoMetadata, delete_file, get_file, list_files, upload_file, upload_file_chunked,
    upload_file_chunked_with_chunk_size,
};

// Re-export Interactions API functions from the interactions module
pub use interactions::cancel_interaction;
pub use interactions::create_interaction;
pub use interactions::create_interaction_stream;
pub use interactions::delete_interaction;
pub use interactions::get_interaction;
pub use interactions::get_interaction_stream;
