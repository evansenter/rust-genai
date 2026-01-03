//! Core content types for the Interactions API.
//!
//! This module contains `InteractionContent`, the primary enum representing all content
//! types that can appear in API requests and responses, along with its custom serialization
//! and deserialization implementations.

use serde::{Deserialize, Serialize};
use std::fmt;

// =============================================================================
// Annotation Types (for Text Content with Citations)
// =============================================================================

/// An annotation linking a text span to its source.
///
/// Annotations provide citation information for specific portions of generated text,
/// typically appearing when using grounding tools like `GoogleSearch` or `UrlContext`.
///
/// # Byte Indices
///
/// **Important:** The `start_index` and `end_index` fields are **byte positions** (not
/// character indices) in the text content. This matches the UTF-8 byte offsets used
/// by the Gemini API. For multi-byte characters (like emoji or non-ASCII text), you
/// may need to use byte-slicing rather than character indexing.
///
/// # Example
///
/// ```no_run
/// # use genai_client::models::interactions::{InteractionResponse, Annotation};
/// # let response: InteractionResponse = todo!();
/// // Get all annotations from the response
/// for annotation in response.all_annotations() {
///     println!(
///         "Text at bytes {}..{} sourced from: {}",
///         annotation.start_index,
///         annotation.end_index,
///         annotation.source.as_deref().unwrap_or("<no source>")
///     );
/// }
/// ```
///
/// # Extracting Annotated Text
///
/// To extract the annotated substring from the response text:
///
/// ```no_run
/// # use genai_client::models::interactions::{InteractionResponse, Annotation};
/// # let response: InteractionResponse = todo!();
/// # let annotation: &Annotation = todo!();
/// if let Some(text) = response.text() {
///     // Use byte slicing since indices are byte positions
///     let bytes = text.as_bytes();
///     if annotation.end_index <= bytes.len() {
///         if let Ok(span) = std::str::from_utf8(&bytes[annotation.start_index..annotation.end_index]) {
///             println!("Cited text: {}", span);
///         }
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Annotation {
    /// Start of the segment in the text (byte position, inclusive).
    ///
    /// This is a byte offset into the UTF-8 encoded text, not a character index.
    #[serde(default)]
    pub start_index: usize,

    /// End of the segment in the text (byte position, exclusive).
    ///
    /// This is a byte offset into the UTF-8 encoded text, not a character index.
    #[serde(default)]
    pub end_index: usize,

    /// Source attributed for this portion of the text.
    ///
    /// This could be a URL, title, or other identifier depending on the grounding
    /// tool used (e.g., `GoogleSearch` or `UrlContext`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl Annotation {
    /// Creates a new annotation with the given span indices and optional source.
    ///
    /// # Arguments
    ///
    /// * `start_index` - Start of the segment in the text (byte position, inclusive)
    /// * `end_index` - End of the segment in the text (byte position, exclusive)
    /// * `source` - Optional source attribution (URL, title, or identifier)
    ///
    /// # Example
    ///
    /// ```
    /// # use genai_client::models::interactions::Annotation;
    /// let annotation = Annotation::new(0, 10, Some("https://example.com".to_string()));
    /// assert_eq!(annotation.start_index, 0);
    /// assert_eq!(annotation.end_index, 10);
    /// assert!(annotation.has_source());
    /// ```
    #[must_use]
    pub fn new(start_index: usize, end_index: usize, source: Option<String>) -> Self {
        Self {
            start_index,
            end_index,
            source,
        }
    }

    /// Returns the byte length of the annotated span.
    ///
    /// This is equivalent to `end_index - start_index`.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.end_index.saturating_sub(self.start_index)
    }

    /// Returns `true` if this annotation has a source attribution.
    #[must_use]
    pub fn has_source(&self) -> bool {
        self.source.is_some()
    }

    /// Extracts the annotated substring from the given text.
    ///
    /// Returns `None` if the indices are out of bounds or if the byte slice
    /// doesn't form valid UTF-8.
    ///
    /// # Arguments
    ///
    /// * `text` - The full text content to extract from
    ///
    /// # Example
    ///
    /// ```
    /// # use genai_client::models::interactions::Annotation;
    /// let annotation = Annotation {
    ///     start_index: 0,
    ///     end_index: 5,
    ///     source: Some("https://example.com".to_string()),
    /// };
    ///
    /// let text = "Hello, world!";
    /// assert_eq!(annotation.extract_span(text), Some("Hello"));
    /// ```
    #[must_use]
    pub fn extract_span<'a>(&self, text: &'a str) -> Option<&'a str> {
        let bytes = text.as_bytes();
        if self.end_index <= bytes.len() && self.start_index <= self.end_index {
            std::str::from_utf8(&bytes[self.start_index..self.end_index]).ok()
        } else {
            None
        }
    }
}

/// Outcome of a code execution operation.
///
/// This enum represents the result status of code executed via the CodeExecution tool.
/// The API returns these as strings like "OUTCOME_OK", which are deserialized into
/// this enum.
///
/// # Example
///
/// ```no_run
/// # use genai_client::models::interactions::{InteractionResponse, CodeExecutionOutcome};
/// # let response: InteractionResponse = todo!();
/// for result in response.code_execution_results() {
///     match result.outcome {
///         CodeExecutionOutcome::Ok => println!("Success: {}", result.output),
///         CodeExecutionOutcome::Failed => eprintln!("Error: {}", result.output),
///         CodeExecutionOutcome::DeadlineExceeded => eprintln!("Timeout!"),
///         _ => eprintln!("Unknown outcome"),
///     }
/// }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum CodeExecutionOutcome {
    /// Code executed successfully
    #[serde(rename = "OUTCOME_OK")]
    Ok,
    /// Code execution failed (e.g., syntax error, runtime error)
    #[serde(rename = "OUTCOME_FAILED")]
    Failed,
    /// Code execution exceeded the 30-second timeout
    #[serde(rename = "OUTCOME_DEADLINE_EXCEEDED")]
    DeadlineExceeded,
    /// Unrecognized outcome for forward compatibility
    #[serde(other)]
    #[default]
    Unspecified,
}

impl CodeExecutionOutcome {
    /// Returns true if the execution was successful.
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Ok)
    }

    /// Returns true if the execution failed (any error type).
    pub const fn is_error(&self) -> bool {
        !self.is_success()
    }
}

impl fmt::Display for CodeExecutionOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok => write!(f, "OK"),
            Self::Failed => write!(f, "FAILED"),
            Self::DeadlineExceeded => write!(f, "DEADLINE_EXCEEDED"),
            Self::Unspecified => write!(f, "UNSPECIFIED"),
        }
    }
}

/// Programming language for code execution.
///
/// This enum represents the programming language used in code execution requests.
/// Currently only Python is supported by the Gemini API.
///
/// # Example
///
/// ```no_run
/// # use genai_client::models::interactions::{InteractionContent, CodeExecutionLanguage};
/// # let content: InteractionContent = todo!();
/// if let InteractionContent::CodeExecutionCall { language, code, .. } = content {
///     match language {
///         CodeExecutionLanguage::Python => println!("Python code: {}", code),
///         _ => println!("Other language: {}", code),
///     }
/// }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum CodeExecutionLanguage {
    /// Python programming language
    #[default]
    Python,
    /// Unrecognized language for forward compatibility
    #[serde(other)]
    Unspecified,
}

impl fmt::Display for CodeExecutionLanguage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Python => write!(f, "PYTHON"),
            Self::Unspecified => write!(f, "UNSPECIFIED"),
        }
    }
}

/// Content object for Interactions API - uses flat structure with type field.
///
/// This enum represents all content types that can appear in API requests and responses.
/// It includes an `Unknown` variant for forward compatibility with new API content types.
///
/// # Forward Compatibility
///
/// This enum is marked `#[non_exhaustive]`, which means:
/// - Match statements must include a wildcard arm (`_ => ...`)
/// - New variants may be added in minor version updates without breaking your code
///
/// When the API returns a content type that this library doesn't recognize, it will be
/// captured as `InteractionContent::Unknown` rather than causing a deserialization error.
/// This allows your code to continue working even when Google adds new content types.
///
/// Use [`super::InteractionResponse::has_unknown`] and [`super::InteractionResponse::unknown_content`]
/// to detect and inspect unknown content.
///
/// # Example
///
/// ```no_run
/// # use genai_client::models::interactions::{InteractionContent, InteractionResponse};
/// # let response: InteractionResponse = todo!();
/// for content in &response.outputs {
///     match content {
///         InteractionContent::Text { text, annotations } => {
///             println!("Text: {:?}", text);
///             if let Some(annots) = annotations {
///                 println!("  {} annotations", annots.len());
///             }
///         }
///         InteractionContent::FunctionCall { name, .. } => println!("Function: {}", name),
///         InteractionContent::Unknown { content_type, .. } => {
///             println!("Unknown content type: {}", content_type);
///         }
///         // Required due to #[non_exhaustive] - handles future variants
///         _ => {}
///     }
/// }
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum InteractionContent {
    /// Text content with optional source annotations.
    ///
    /// Annotations are present when grounding tools like `GoogleSearch` or `UrlContext`
    /// provide citation information linking text spans to their sources.
    Text {
        /// The text content.
        text: Option<String>,
        /// Source annotations for portions of the text.
        ///
        /// Present when the response includes citation information from grounding tools.
        /// Use [`annotations()`](Self::annotations) for convenient access.
        annotations: Option<Vec<Annotation>>,
    },
    /// Thought content (internal reasoning)
    Thought { text: Option<String> },
    /// Thought signature (cryptographic signature for thought verification)
    ///
    /// This variant typically appears only during streaming responses, providing
    /// a cryptographic signature that verifies the authenticity of thought content.
    ThoughtSignature { signature: String },
    /// Image content
    Image {
        data: Option<String>,
        uri: Option<String>,
        mime_type: Option<String>,
    },
    /// Audio content
    Audio {
        data: Option<String>,
        uri: Option<String>,
        mime_type: Option<String>,
    },
    /// Video content
    Video {
        data: Option<String>,
        uri: Option<String>,
        mime_type: Option<String>,
    },
    /// Document content for file-based inputs.
    ///
    /// PDF (`application/pdf`) is the primary supported format with full vision capabilities
    /// for understanding text, images, charts, and tables. Other formats like TXT, Markdown,
    /// HTML, and XML are processed as plain text only, losing visual structure.
    Document {
        data: Option<String>,
        uri: Option<String>,
        mime_type: Option<String>,
    },
    /// Function call (output from model)
    FunctionCall {
        /// Unique identifier for this function call
        id: Option<String>,
        name: String,
        args: serde_json::Value,
        /// Thought signature for Gemini 3 reasoning continuity
        thought_signature: Option<String>,
    },
    /// Function result (input to model with execution result)
    FunctionResult {
        name: String,
        /// The call_id from the FunctionCall being responded to
        call_id: String,
        result: serde_json::Value,
    },
    /// Code execution call (model requesting code execution)
    ///
    /// This variant appears when the model initiates code execution
    /// via the `CodeExecution` built-in tool.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use genai_client::models::interactions::{InteractionContent, CodeExecutionLanguage};
    /// # let content: InteractionContent = todo!();
    /// if let InteractionContent::CodeExecutionCall { id, language, code } = content {
    ///     println!("Executing {:?} code (id: {}): {}", language, id, code);
    /// }
    /// ```
    CodeExecutionCall {
        /// Unique identifier for this code execution call
        id: String,
        /// Programming language (currently only Python is supported)
        language: CodeExecutionLanguage,
        /// Source code to execute
        code: String,
    },
    /// Code execution result (returned after code runs)
    ///
    /// Contains the outcome and output of executed code from the `CodeExecution` tool.
    ///
    /// # Security Note
    ///
    /// When displaying results to end users, check `outcome.is_error()` first. Error
    /// results may contain stack traces or system information that shouldn't be exposed
    /// directly to users without sanitization.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use genai_client::models::interactions::{InteractionContent, CodeExecutionOutcome};
    /// # let content: InteractionContent = todo!();
    /// if let InteractionContent::CodeExecutionResult { outcome, output, .. } = content {
    ///     match outcome {
    ///         CodeExecutionOutcome::Ok => println!("Result: {}", output),
    ///         CodeExecutionOutcome::Failed => eprintln!("Error: {}", output),
    ///         CodeExecutionOutcome::DeadlineExceeded => eprintln!("Timeout!"),
    ///         _ => {}
    ///     }
    /// }
    /// ```
    CodeExecutionResult {
        /// The call_id matching the CodeExecutionCall this result is for
        call_id: String,
        /// Execution outcome (OK, FAILED, DEADLINE_EXCEEDED, etc.)
        outcome: CodeExecutionOutcome,
        /// The output of the code execution (stdout for success, error message for failure)
        output: String,
    },
    /// Google Search call (model requesting a search)
    ///
    /// Appears when the model initiates a Google Search via the `GoogleSearch` tool.
    GoogleSearchCall {
        /// Search query
        query: String,
    },
    /// Google Search result (grounding data from search)
    ///
    /// Contains the results returned by the `GoogleSearch` built-in tool.
    GoogleSearchResult {
        /// Search result data (flexible structure as API evolves)
        results: serde_json::Value,
    },
    /// URL Context call (model requesting URL content)
    ///
    /// Appears when the model requests URL content via the `UrlContext` tool.
    UrlContextCall {
        /// URL to fetch
        url: String,
    },
    /// URL Context result (fetched content from URL)
    ///
    /// Contains the content retrieved by the `UrlContext` built-in tool.
    ///
    /// The `content` field may be `None` if the URL could not be fetched
    /// (e.g., network errors, blocked URLs, timeouts, or access restrictions).
    UrlContextResult {
        /// The URL that was fetched
        url: String,
        /// The fetched content, or `None` if the fetch failed
        content: Option<String>,
    },
    /// Unknown content type for forward compatibility.
    ///
    /// This variant captures content types that the library doesn't recognize yet.
    /// This can happen when Google adds new features to the API before this library
    /// is updated to support them.
    ///
    /// The `content_type` field contains the unrecognized type string from the API,
    /// and `data` contains the full JSON object for inspection or debugging.
    ///
    /// # When This Occurs
    ///
    /// - New API features not yet supported by this library
    /// - Beta features in testing
    /// - Region-specific content types
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use genai_client::models::interactions::InteractionContent;
    /// # let content: InteractionContent = todo!();
    /// if let InteractionContent::Unknown { content_type, data } = content {
    ///     eprintln!("Encountered unknown type '{}': {:?}", content_type, data);
    /// }
    /// ```
    ///
    /// # Serialization Behavior
    ///
    /// Unknown variants can be serialized back to JSON, enabling round-trip in
    /// multi-turn conversations. The serialization follows these rules:
    ///
    /// 1. **Type field**: The `content_type` becomes the `"type"` field in output
    /// 2. **Object data**: If `data` is a JSON object, its fields are flattened
    ///    into the output (excluding any existing `"type"` field to avoid duplicates)
    /// 3. **Non-object data**: If `data` is a non-object value (array, string, etc.),
    ///    it's placed under a `"data"` key
    /// 4. **Null data**: Omitted entirely from the output
    ///
    /// ## Example: Object Data (Common Case)
    ///
    /// ```
    /// # use genai_client::models::interactions::InteractionContent;
    /// # use serde_json::json;
    /// let content = InteractionContent::Unknown {
    ///     content_type: "new_feature".to_string(),
    ///     data: json!({"field1": "value1", "field2": 42}),
    /// };
    /// // Serializes to: {"type": "new_feature", "field1": "value1", "field2": 42}
    /// ```
    ///
    /// ## Example: Duplicate Type Field
    ///
    /// If `data` contains a `"type"` field, it's excluded during serialization
    /// (the `content_type` takes precedence):
    ///
    /// ```
    /// # use genai_client::models::interactions::InteractionContent;
    /// # use serde_json::json;
    /// let content = InteractionContent::Unknown {
    ///     content_type: "my_type".to_string(),
    ///     data: json!({"type": "ignored", "value": 123}),
    /// };
    /// // Serializes to: {"type": "my_type", "value": 123}
    /// // Note: "type": "ignored" is not included
    /// ```
    ///
    /// ## Example: Non-Object Data
    ///
    /// ```
    /// # use genai_client::models::interactions::InteractionContent;
    /// # use serde_json::json;
    /// let content = InteractionContent::Unknown {
    ///     content_type: "array_type".to_string(),
    ///     data: json!([1, 2, 3]),
    /// };
    /// // Serializes to: {"type": "array_type", "data": [1, 2, 3]}
    /// ```
    ///
    /// # Field Ordering
    ///
    /// **Note:** Field ordering is not preserved during round-trip serialization,
    /// but all field **values** are fully preserved. When serializing an `Unknown`
    /// variant, the `"type"` field is always written first, followed by the remaining
    /// fields from `data`. This means the output field order may differ from the
    /// original API response.
    ///
    /// This has **no practical impact** on API compatibility because JSON objects
    /// are inherently unordered per RFC 8259. The Gemini API does not depend on
    /// field ordering.
    ///
    /// If you need to preserve the exact original field ordering (e.g., for logging
    /// or debugging purposes), access the raw `data` field directly via
    /// [`unknown_data()`](Self::unknown_data) instead of re-serializing the variant.
    ///
    /// # Manual Construction
    ///
    /// While Unknown variants are typically created by deserialization, you can
    /// construct them manually for testing or edge cases. Note that:
    ///
    /// - The `content_type` can be any string (including empty, though not recommended)
    /// - The `data` can be any valid JSON value
    /// - For multi-turn conversations, the serialized form must match what the API expects
    Unknown {
        /// The unrecognized type name from the API
        content_type: String,
        /// The full JSON data for this content, preserved for debugging
        data: serde_json::Value,
    },
}

// Custom Serialize implementation for InteractionContent.
// This handles the Unknown variant specially by merging content_type into the data.
impl Serialize for InteractionContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        match self {
            Self::Text { text, annotations } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "text")?;
                if let Some(t) = text {
                    map.serialize_entry("text", t)?;
                }
                if let Some(annots) = annotations
                    && !annots.is_empty()
                {
                    map.serialize_entry("annotations", annots)?;
                }
                map.end()
            }
            Self::Thought { text } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "thought")?;
                if let Some(t) = text {
                    map.serialize_entry("text", t)?;
                }
                map.end()
            }
            Self::ThoughtSignature { signature } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "thought_signature")?;
                map.serialize_entry("signature", signature)?;
                map.end()
            }
            Self::Image {
                data,
                uri,
                mime_type,
            } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "image")?;
                if let Some(d) = data {
                    map.serialize_entry("data", d)?;
                }
                if let Some(u) = uri {
                    map.serialize_entry("uri", u)?;
                }
                if let Some(m) = mime_type {
                    map.serialize_entry("mime_type", m)?;
                }
                map.end()
            }
            Self::Audio {
                data,
                uri,
                mime_type,
            } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "audio")?;
                if let Some(d) = data {
                    map.serialize_entry("data", d)?;
                }
                if let Some(u) = uri {
                    map.serialize_entry("uri", u)?;
                }
                if let Some(m) = mime_type {
                    map.serialize_entry("mime_type", m)?;
                }
                map.end()
            }
            Self::Video {
                data,
                uri,
                mime_type,
            } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "video")?;
                if let Some(d) = data {
                    map.serialize_entry("data", d)?;
                }
                if let Some(u) = uri {
                    map.serialize_entry("uri", u)?;
                }
                if let Some(m) = mime_type {
                    map.serialize_entry("mime_type", m)?;
                }
                map.end()
            }
            Self::Document {
                data,
                uri,
                mime_type,
            } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "document")?;
                if let Some(d) = data {
                    map.serialize_entry("data", d)?;
                }
                if let Some(u) = uri {
                    map.serialize_entry("uri", u)?;
                }
                if let Some(m) = mime_type {
                    map.serialize_entry("mime_type", m)?;
                }
                map.end()
            }
            Self::FunctionCall {
                id,
                name,
                args,
                thought_signature,
            } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "function_call")?;
                if let Some(i) = id {
                    map.serialize_entry("id", i)?;
                }
                map.serialize_entry("name", name)?;
                map.serialize_entry("arguments", args)?;
                if let Some(sig) = thought_signature {
                    map.serialize_entry("thoughtSignature", sig)?;
                }
                map.end()
            }
            Self::FunctionResult {
                name,
                call_id,
                result,
            } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "function_result")?;
                map.serialize_entry("name", name)?;
                map.serialize_entry("call_id", call_id)?;
                map.serialize_entry("result", result)?;
                map.end()
            }
            Self::CodeExecutionCall { id, language, code } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "code_execution_call")?;
                map.serialize_entry("id", id)?;
                map.serialize_entry("language", language)?;
                map.serialize_entry("code", code)?;
                map.end()
            }
            Self::CodeExecutionResult {
                call_id,
                outcome,
                output,
            } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "code_execution_result")?;
                map.serialize_entry("call_id", call_id)?;
                map.serialize_entry("outcome", outcome)?;
                map.serialize_entry("output", output)?;
                map.end()
            }
            Self::GoogleSearchCall { query } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "google_search_call")?;
                map.serialize_entry("query", query)?;
                map.end()
            }
            Self::GoogleSearchResult { results } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "google_search_result")?;
                map.serialize_entry("results", results)?;
                map.end()
            }
            Self::UrlContextCall { url } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "url_context_call")?;
                map.serialize_entry("url", url)?;
                map.end()
            }
            Self::UrlContextResult { url, content } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "url_context_result")?;
                map.serialize_entry("url", url)?;
                if let Some(c) = content {
                    map.serialize_entry("content", c)?;
                }
                map.end()
            }
            Self::Unknown { content_type, data } => {
                // For Unknown, merge the content_type into the data object
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", content_type)?;
                // Flatten the data fields into the map if it's an object
                match data {
                    serde_json::Value::Object(obj) => {
                        for (key, value) in obj {
                            if key != "type" {
                                // Don't duplicate the type field
                                map.serialize_entry(key, value)?;
                            }
                        }
                    }
                    // For non-object data (unlikely but possible), preserve under "data" key
                    other if !other.is_null() => {
                        map.serialize_entry("data", other)?;
                    }
                    _ => {} // Null data is omitted
                }
                map.end()
            }
        }
    }
}

impl InteractionContent {
    /// Extract the text content, if this is a Text variant with non-empty text.
    ///
    /// Returns `Some` only for `Text` variants with non-empty text.
    /// Returns `None` for all other variants including `Thought`.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text { text: Some(t), .. } if !t.is_empty() => Some(t),
            _ => None,
        }
    }

    /// Returns annotations if this is Text content with annotations.
    ///
    /// Returns `Some` with a slice of annotations only for `Text` variants that
    /// have non-empty annotations. Returns `None` for all other variants.
    ///
    /// Annotations are typically present when using grounding tools like
    /// `GoogleSearch` or `UrlContext`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use genai_client::models::interactions::{InteractionContent, Annotation};
    /// # let content: InteractionContent = todo!();
    /// if let Some(annotations) = content.annotations() {
    ///     for annotation in annotations {
    ///         println!("Source: {:?} for bytes {}..{}",
    ///             annotation.source,
    ///             annotation.start_index,
    ///             annotation.end_index);
    ///     }
    /// }
    /// ```
    #[must_use]
    pub fn annotations(&self) -> Option<&[Annotation]> {
        match self {
            Self::Text {
                annotations: Some(annots),
                ..
            } if !annots.is_empty() => Some(annots),
            _ => None,
        }
    }

    /// Extract the thought content, if this is a Thought variant with non-empty text.
    ///
    /// Returns `Some` only for `Thought` variants with non-empty text.
    /// Returns `None` for all other variants including `Text`.
    #[must_use]
    pub fn thought(&self) -> Option<&str> {
        match self {
            Self::Thought { text: Some(t) } if !t.is_empty() => Some(t),
            _ => None,
        }
    }

    /// Check if this is a Text content type.
    #[must_use]
    pub const fn is_text(&self) -> bool {
        matches!(self, Self::Text { .. })
    }

    /// Check if this is a Thought content type.
    #[must_use]
    pub const fn is_thought(&self) -> bool {
        matches!(self, Self::Thought { .. })
    }

    /// Check if this is a ThoughtSignature content type.
    #[must_use]
    pub const fn is_thought_signature(&self) -> bool {
        matches!(self, Self::ThoughtSignature { .. })
    }

    /// Check if this is a FunctionCall content type.
    #[must_use]
    pub const fn is_function_call(&self) -> bool {
        matches!(self, Self::FunctionCall { .. })
    }

    /// Returns `true` if this is an unknown content type.
    ///
    /// Use this to check for content types that the library doesn't recognize.
    /// See [`InteractionContent::Unknown`] for more details.
    #[must_use]
    pub const fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }

    /// Check if this is a CodeExecutionCall content type.
    #[must_use]
    pub const fn is_code_execution_call(&self) -> bool {
        matches!(self, Self::CodeExecutionCall { .. })
    }

    /// Check if this is a CodeExecutionResult content type.
    #[must_use]
    pub const fn is_code_execution_result(&self) -> bool {
        matches!(self, Self::CodeExecutionResult { .. })
    }

    /// Check if this is a GoogleSearchCall content type.
    #[must_use]
    pub const fn is_google_search_call(&self) -> bool {
        matches!(self, Self::GoogleSearchCall { .. })
    }

    /// Check if this is a GoogleSearchResult content type.
    #[must_use]
    pub const fn is_google_search_result(&self) -> bool {
        matches!(self, Self::GoogleSearchResult { .. })
    }

    /// Check if this is a UrlContextCall content type.
    #[must_use]
    pub const fn is_url_context_call(&self) -> bool {
        matches!(self, Self::UrlContextCall { .. })
    }

    /// Check if this is a UrlContextResult content type.
    #[must_use]
    pub const fn is_url_context_result(&self) -> bool {
        matches!(self, Self::UrlContextResult { .. })
    }

    /// Returns the content type name if this is an unknown content type.
    ///
    /// Returns `None` for known content types.
    #[must_use]
    pub fn unknown_content_type(&self) -> Option<&str> {
        match self {
            Self::Unknown { content_type, .. } => Some(content_type),
            _ => None,
        }
    }

    /// Returns the raw JSON data if this is an unknown content type.
    ///
    /// Returns `None` for known content types.
    #[must_use]
    pub fn unknown_data(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Unknown { data, .. } => Some(data),
            _ => None,
        }
    }
}

// Custom Deserialize implementation to handle unknown content types gracefully.
//
// This tries to deserialize known types first, and falls back to Unknown for
// unrecognized types. This provides forward compatibility when Google adds
// new content types to the API.
impl<'de> Deserialize<'de> for InteractionContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[cfg(feature = "strict-unknown")]
        use serde::de::Error as _;

        // First, deserialize into a raw JSON value
        let value = serde_json::Value::deserialize(deserializer)?;

        // Helper enum for deserializing known types
        #[derive(Deserialize)]
        #[serde(tag = "type", rename_all = "snake_case")]
        enum KnownContent {
            Text {
                text: Option<String>,
                #[serde(default)]
                annotations: Option<Vec<Annotation>>,
            },
            Thought {
                text: Option<String>,
            },
            ThoughtSignature {
                #[serde(default)]
                signature: String,
            },
            Image {
                data: Option<String>,
                uri: Option<String>,
                mime_type: Option<String>,
            },
            Audio {
                data: Option<String>,
                uri: Option<String>,
                mime_type: Option<String>,
            },
            Video {
                data: Option<String>,
                uri: Option<String>,
                mime_type: Option<String>,
            },
            Document {
                data: Option<String>,
                uri: Option<String>,
                mime_type: Option<String>,
            },
            FunctionCall {
                id: Option<String>,
                name: String,
                #[serde(rename = "arguments")]
                args: serde_json::Value,
                #[serde(rename = "thoughtSignature")]
                thought_signature: Option<String>,
            },
            FunctionResult {
                name: String,
                call_id: String,
                result: serde_json::Value,
            },
            CodeExecutionCall {
                id: String,
                // API returns language/code in the arguments object
                #[serde(default)]
                language: Option<CodeExecutionLanguage>,
                #[serde(default)]
                code: Option<String>,
                // Fallback for old API format
                #[serde(default)]
                arguments: Option<serde_json::Value>,
            },
            CodeExecutionResult {
                call_id: String,
                // New typed outcome
                #[serde(default)]
                outcome: Option<CodeExecutionOutcome>,
                // New output field
                #[serde(default)]
                output: Option<String>,
                // Old API format fallback
                #[serde(default)]
                is_error: Option<bool>,
                #[serde(default)]
                result: Option<String>,
            },
            GoogleSearchCall {
                query: String,
            },
            GoogleSearchResult {
                results: serde_json::Value,
            },
            UrlContextCall {
                url: String,
            },
            UrlContextResult {
                url: String,
                content: Option<String>,
            },
        }

        // Try to deserialize as a known type
        match serde_json::from_value::<KnownContent>(value.clone()) {
            Ok(known) => Ok(match known {
                KnownContent::Text { text, annotations } => {
                    InteractionContent::Text { text, annotations }
                }
                KnownContent::Thought { text } => InteractionContent::Thought { text },
                KnownContent::ThoughtSignature { signature } => {
                    InteractionContent::ThoughtSignature { signature }
                }
                KnownContent::Image {
                    data,
                    uri,
                    mime_type,
                } => InteractionContent::Image {
                    data,
                    uri,
                    mime_type,
                },
                KnownContent::Audio {
                    data,
                    uri,
                    mime_type,
                } => InteractionContent::Audio {
                    data,
                    uri,
                    mime_type,
                },
                KnownContent::Video {
                    data,
                    uri,
                    mime_type,
                } => InteractionContent::Video {
                    data,
                    uri,
                    mime_type,
                },
                KnownContent::Document {
                    data,
                    uri,
                    mime_type,
                } => InteractionContent::Document {
                    data,
                    uri,
                    mime_type,
                },
                KnownContent::FunctionCall {
                    id,
                    name,
                    args,
                    thought_signature,
                } => InteractionContent::FunctionCall {
                    id,
                    name,
                    args,
                    thought_signature,
                },
                KnownContent::FunctionResult {
                    name,
                    call_id,
                    result,
                } => InteractionContent::FunctionResult {
                    name,
                    call_id,
                    result,
                },
                KnownContent::CodeExecutionCall {
                    id,
                    language,
                    code,
                    arguments,
                } => {
                    // Prefer direct fields, fall back to parsing arguments
                    if let (Some(lang), Some(source)) = (language, code) {
                        InteractionContent::CodeExecutionCall {
                            id,
                            language: lang,
                            code: source,
                        }
                    } else if let Some(args) = arguments {
                        // Parse old format: {"language": "PYTHON", "code": "..."}
                        // Code is required - if missing, treat as Unknown per Evergreen philosophy
                        let source = match args.get("code").and_then(|v| v.as_str()) {
                            Some(code) => code.to_string(),
                            None => {
                                log::warn!(
                                    "CodeExecutionCall arguments missing required 'code' field for id: {}. \
                                     Treating as Unknown variant to preserve data for debugging.",
                                    id
                                );
                                return Ok(InteractionContent::Unknown {
                                    content_type: "code_execution_call".to_string(),
                                    data: value.clone(),
                                });
                            }
                        };

                        // Language defaults to Python if missing or malformed (most common case)
                        let lang = match args.get("language") {
                            Some(lang_value) => {
                                match serde_json::from_value::<CodeExecutionLanguage>(
                                    lang_value.clone(),
                                ) {
                                    Ok(lang) => lang,
                                    Err(e) => {
                                        log::warn!(
                                            "CodeExecutionCall has invalid language value for id: {}, \
                                             defaulting to Python. Parse error: {}",
                                            id,
                                            e
                                        );
                                        CodeExecutionLanguage::Python
                                    }
                                }
                            }
                            None => CodeExecutionLanguage::Python,
                        };

                        InteractionContent::CodeExecutionCall {
                            id,
                            language: lang,
                            code: source,
                        }
                    } else {
                        // Malformed CodeExecutionCall - treat as Unknown to preserve data
                        // per Evergreen philosophy (see CLAUDE.md). This avoids silently
                        // degrading to an empty code string which could cause subtle bugs.
                        log::warn!(
                            "CodeExecutionCall missing both direct fields and arguments for id: {}. \
                             Treating as Unknown variant to preserve data for debugging.",
                            id
                        );
                        InteractionContent::Unknown {
                            content_type: "code_execution_call".to_string(),
                            data: value.clone(),
                        }
                    }
                }
                KnownContent::CodeExecutionResult {
                    call_id,
                    outcome,
                    output,
                    is_error,
                    result,
                } => {
                    // Prefer new fields, fall back to old fields
                    let exec_outcome = outcome.unwrap_or(
                        // Convert old is_error boolean to outcome
                        match is_error {
                            Some(true) => CodeExecutionOutcome::Failed,
                            Some(false) => CodeExecutionOutcome::Ok,
                            None => CodeExecutionOutcome::Unspecified,
                        },
                    );

                    let exec_output = output.or(result).unwrap_or_default();

                    InteractionContent::CodeExecutionResult {
                        call_id,
                        outcome: exec_outcome,
                        output: exec_output,
                    }
                }
                KnownContent::GoogleSearchCall { query } => {
                    InteractionContent::GoogleSearchCall { query }
                }
                KnownContent::GoogleSearchResult { results } => {
                    InteractionContent::GoogleSearchResult { results }
                }
                KnownContent::UrlContextCall { url } => InteractionContent::UrlContextCall { url },
                KnownContent::UrlContextResult { url, content } => {
                    InteractionContent::UrlContextResult { url, content }
                }
            }),
            Err(parse_error) => {
                // Unknown type - extract type name and preserve data
                let content_type = value
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("<missing type>")
                    .to_string();

                // Log the actual parse error for debugging - this helps distinguish
                // between truly unknown types and malformed known types
                log::warn!(
                    "Encountered unknown InteractionContent type '{}'. \
                     Parse error: {}. \
                     This may indicate a new API feature or a malformed response. \
                     The content will be preserved in the Unknown variant.",
                    content_type,
                    parse_error
                );

                #[cfg(feature = "strict-unknown")]
                {
                    Err(D::Error::custom(format!(
                        "Unknown InteractionContent type '{}'. \
                         Strict mode is enabled via the 'strict-unknown' feature flag. \
                         Either update the library or disable strict mode.",
                        content_type
                    )))
                }

                #[cfg(not(feature = "strict-unknown"))]
                {
                    Ok(InteractionContent::Unknown {
                        content_type,
                        data: value,
                    })
                }
            }
        }
    }
}
