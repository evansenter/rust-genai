//! Metadata types for built-in tools (Google Search grounding, URL context).

use serde::{Deserialize, Serialize};

/// Grounding metadata returned when using the GoogleSearch tool.
///
/// Contains search queries executed by the model and web sources that
/// ground the response in real-time information.
///
/// # Example
///
/// ```no_run
/// # use genai_client::models::interactions::InteractionResponse;
/// # let response: InteractionResponse = todo!();
/// if let Some(metadata) = response.google_search_metadata() {
///     println!("Search queries: {:?}", metadata.web_search_queries);
///     for chunk in &metadata.grounding_chunks {
///         println!("Source: {} - {}", chunk.web.title, chunk.web.uri);
///     }
/// }
/// ```
#[derive(Clone, Deserialize, Serialize, Debug, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct GroundingMetadata {
    /// Search queries that were executed by the model
    pub web_search_queries: Vec<String>,

    /// Web sources referenced in the response
    pub grounding_chunks: Vec<GroundingChunk>,
}

/// A web source referenced in grounding.
#[derive(Clone, Deserialize, Serialize, Debug, Default, PartialEq)]
pub struct GroundingChunk {
    /// Web resource information
    #[serde(default)]
    pub web: WebSource,
}

/// Web source details (URI, title, and domain).
#[derive(Clone, Deserialize, Serialize, Debug, Default, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct WebSource {
    /// URI of the web page
    pub uri: String,
    /// Title of the source
    pub title: String,
    /// Domain of the web page (e.g., "wikipedia.org")
    pub domain: String,
}

/// Metadata returned when using the UrlContext tool.
///
/// Contains retrieval status for each URL that was processed.
/// This is useful for verification and debugging URL fetches.
///
/// # Example
///
/// ```no_run
/// # use genai_client::models::interactions::InteractionResponse;
/// # let response: InteractionResponse = todo!();
/// if let Some(metadata) = response.url_context_metadata() {
///     for entry in &metadata.url_metadata {
///         println!("URL: {} - Status: {:?}", entry.retrieved_url, entry.url_retrieval_status);
///     }
/// }
/// ```
#[derive(Clone, Deserialize, Serialize, Debug, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct UrlContextMetadata {
    /// Metadata for each URL that was processed
    pub url_metadata: Vec<UrlMetadataEntry>,
}

/// Retrieval status for a single URL processed by the UrlContext tool.
#[derive(Clone, Deserialize, Serialize, Debug, Default, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct UrlMetadataEntry {
    /// The URL that was retrieved
    pub retrieved_url: String,
    /// Status of the retrieval attempt
    pub url_retrieval_status: UrlRetrievalStatus,
}

/// Status of a URL retrieval attempt.
#[derive(Clone, Deserialize, Serialize, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UrlRetrievalStatus {
    /// Status not specified
    #[default]
    UrlRetrievalStatusUnspecified,
    /// URL content was successfully retrieved
    UrlRetrievalStatusSuccess,
    /// URL failed safety/content moderation checks
    UrlRetrievalStatusUnsafe,
    /// URL retrieval failed for other reasons
    UrlRetrievalStatusError,
}
