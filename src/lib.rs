//! # rust-genai
//!
//! A Rust client library for Google's Generative AI (Gemini) API using the Interactions API.
//!
//! ## Quick Start
//!
//! ```no_run
//! use rust_genai::Client;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), rust_genai::GenaiError> {
//!     let client = Client::new(
//!         std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set")
//!     );
//!
//!     let response = client
//!         .interaction()
//!         .with_model("gemini-3-flash-preview")
//!         .with_text("Hello, Gemini!")
//!         .create()
//!         .await?;
//!
//!     println!("{}", response.text().unwrap_or("No response"));
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! - **Fluent Builder API**: Chain methods for readable request construction
//! - **Streaming**: Real-time response streaming with `create_stream()`
//! - **Function Calling**: Automatic function discovery and execution via macros
//! - **Built-in Tools**: Google Search, Code Execution, URL Context
//! - **Multimodal**: Images, audio, video, and document inputs
//! - **Thinking Mode**: Access model reasoning with configurable levels
//!
//! ## API Stability & Forward Compatibility
//!
//! This library is designed for forward compatibility with evolving APIs:
//!
//! - **`#[non_exhaustive]` enums**: Match statements require wildcard arms (`_ => ...`)
//! - **`Unknown` variants**: Unrecognized API types are captured, not rejected
//! - **Graceful degradation**: New API features won't break existing code
//!
//! When Google adds new features, your code continues to work. Unknown content types
//! and tools are preserved for inspection via helper methods like `has_unknown()`.
//!
//! ## Module Organization
//!
//! - [`Client`]: Main entry point for API interactions
//! - [`InteractionBuilder`]: Fluent builder for configuring requests
//! - [`interactions_api`]: Helper functions for constructing content
//! - [`function_calling`]: Function registration and execution

// =============================================================================
// Internal HTTP Layer (pub(crate))
// =============================================================================
pub(crate) mod http;

// =============================================================================
// Core Type Modules
// =============================================================================

// Error types
pub mod errors;
pub use errors::GenaiError;

// Content types (InteractionContent and related)
pub mod content;
pub use content::{
    Annotation, CodeExecutionLanguage, CodeExecutionOutcome, FileSearchResultItem,
    GoogleSearchResultItem, InteractionContent, Resolution,
};

// Request types (includes agent configuration)
pub mod request;
pub use request::{
    AgentConfig, CreateInteractionRequest, DeepResearchConfig, DynamicConfig, GenerationConfig,
    InteractionInput, Role, ThinkingLevel, ThinkingSummaries, Turn, TurnContent,
};

// Response types
pub mod response;
pub use response::{
    CodeExecutionCallInfo, CodeExecutionResultInfo, ContentSummary, FunctionCallInfo,
    FunctionResultInfo, GroundingChunk, GroundingMetadata, ImageInfo, InteractionResponse,
    InteractionStatus, ModalityTokens, OwnedFunctionCallInfo, UrlContextMetadata,
    UrlContextResultInfo, UrlMetadataEntry, UrlRetrievalStatus, UsageMetadata, WebSource,
};

// Tool types (function declarations, built-in tools)
pub mod tools;
pub use tools::{
    FunctionCallingMode, FunctionDeclaration, FunctionDeclarationBuilder, FunctionParameters, Tool,
};

// Wire streaming types (from API)
pub mod wire_streaming;
pub use wire_streaming::{InteractionStreamEvent, StreamChunk, StreamEvent};

// Files API types
pub use http::files::{
    DEFAULT_CHUNK_SIZE, FileError, FileMetadata, FileState, ListFilesResponse, ResumableUpload,
    VideoMetadata,
};

// =============================================================================
// Client and Builder
// =============================================================================

pub mod client;
pub use client::{Client, ClientBuilder};

pub mod request_builder;
pub use request_builder::{ConversationBuilder, InteractionBuilder};

// =============================================================================
// Function Calling
// =============================================================================

pub mod function_calling;
pub use function_calling::{CallableFunction, FunctionError, ToolService};

// =============================================================================
// Streaming Types for Auto Function Calling
// =============================================================================

pub mod streaming;
pub use streaming::{
    AutoFunctionResult, AutoFunctionResultAccumulator, AutoFunctionStreamChunk,
    AutoFunctionStreamEvent, FunctionExecutionResult,
};

// =============================================================================
// Content Constructor Functions
// =============================================================================
//
// ## Export Strategy
//
// We re-export helper functions that users need to **construct** content to send to the API:
// - Multimodal inputs (images, audio, video) - users build these to send
// - Function results - users send these after executing functions
// - Function calls - needed for multi-turn conversations to echo back the model's call
//
// We do NOT re-export helpers for built-in tool outputs (google_search_*, code_execution_*,
// url_context_*) because those are **generated by the model** and users just read them from
// responses via helper methods like `response.google_search_results()`.
// These are still accessible via `rust_genai::interactions_api::*` if needed.
pub mod interactions_api;
pub use interactions_api::{
    audio_data_content, audio_uri_content, document_data_content, document_uri_content,
    file_uri_content, function_call_content, function_call_content_with_signature,
    function_result_content, image_data_content, image_data_content_with_resolution,
    image_uri_content, image_uri_content_with_resolution, text_content, thought_content,
    video_data_content, video_data_content_with_resolution, video_uri_content,
    video_uri_content_with_resolution,
};

// =============================================================================
// Multimodal File Loading Utilities
// =============================================================================

pub mod multimodal;
pub use multimodal::{
    audio_from_file, audio_from_file_with_mime, detect_mime_type, document_from_file,
    document_from_file_with_mime, image_from_file, image_from_file_with_mime, video_from_file,
    video_from_file_with_mime,
};

// =============================================================================
// Test Modules
// =============================================================================

#[cfg(test)]
mod content_tests;
#[cfg(test)]
mod proptest_tests;
#[cfg(test)]
mod request_tests;
#[cfg(test)]
mod response_tests;
#[cfg(test)]
mod streaming_tests;
