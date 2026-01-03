//! Types for the Interactions API.
//!
//! This module provides all types needed for working with the Gemini Interactions API,
//! including request/response structures, content types, and streaming support.

mod content;
mod metadata;
mod request;
mod response;
mod streaming;

#[cfg(test)]
mod content_tests;
#[cfg(test)]
mod request_tests;
#[cfg(test)]
mod response_tests;
#[cfg(test)]
mod streaming_tests;

#[cfg(test)]
mod proptest_tests;

// Re-export all public types at module root for backwards compatibility
pub use content::{
    Annotation, CodeExecutionLanguage, CodeExecutionOutcome, GoogleSearchResultItem,
    InteractionContent,
};
pub use metadata::{
    GroundingChunk, GroundingMetadata, UrlContextMetadata, UrlMetadataEntry, UrlRetrievalStatus,
    WebSource,
};
pub use request::{
    CreateInteractionRequest, GenerationConfig, InteractionInput, ThinkingLevel, ThinkingSummaries,
};
pub use response::{
    CodeExecutionCallInfo, CodeExecutionResultInfo, ContentSummary, FunctionCallInfo,
    FunctionResultInfo, InteractionResponse, InteractionStatus, ModalityTokens,
    OwnedFunctionCallInfo, UrlContextResultInfo, UsageMetadata,
};
pub use streaming::{InteractionStreamEvent, StreamChunk};
