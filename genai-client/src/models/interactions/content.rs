//! Core content types for the Interactions API.
//!
//! This module contains `InteractionContent`, the primary enum representing all content
//! types that can appear in API requests and responses, along with its custom serialization
//! and deserialization implementations.

use serde::{Deserialize, Serialize};
use std::fmt;

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
///         InteractionContent::Text { text } => println!("Text: {:?}", text),
///         InteractionContent::FunctionCall { name, .. } => println!("Function: {}", name),
///         InteractionContent::Unknown { type_name, .. } => {
///             println!("Unknown content type: {}", type_name);
///         }
///         // Required due to #[non_exhaustive] - handles future variants
///         _ => {}
///     }
/// }
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum InteractionContent {
    /// Text content
    Text { text: Option<String> },
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
    /// The `type_name` field contains the unrecognized type string from the API,
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
    /// if let InteractionContent::Unknown { type_name, data } = content {
    ///     eprintln!("Encountered unknown type '{}': {:?}", type_name, data);
    /// }
    /// ```
    ///
    /// # Serialization Behavior
    ///
    /// Unknown variants can be serialized back to JSON, enabling round-trip in
    /// multi-turn conversations. The serialization follows these rules:
    ///
    /// 1. **Type field**: The `type_name` becomes the `"type"` field in output
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
    ///     type_name: "new_feature".to_string(),
    ///     data: json!({"field1": "value1", "field2": 42}),
    /// };
    /// // Serializes to: {"type": "new_feature", "field1": "value1", "field2": 42}
    /// ```
    ///
    /// ## Example: Duplicate Type Field
    ///
    /// If `data` contains a `"type"` field, it's excluded during serialization
    /// (the `type_name` takes precedence):
    ///
    /// ```
    /// # use genai_client::models::interactions::InteractionContent;
    /// # use serde_json::json;
    /// let content = InteractionContent::Unknown {
    ///     type_name: "my_type".to_string(),
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
    ///     type_name: "array_type".to_string(),
    ///     data: json!([1, 2, 3]),
    /// };
    /// // Serializes to: {"type": "array_type", "data": [1, 2, 3]}
    /// ```
    ///
    /// # Manual Construction
    ///
    /// While Unknown variants are typically created by deserialization, you can
    /// construct them manually for testing or edge cases. Note that:
    ///
    /// - The `type_name` can be any string (including empty, though not recommended)
    /// - The `data` can be any valid JSON value
    /// - For multi-turn conversations, the serialized form must match what the API expects
    Unknown {
        /// The unrecognized type name from the API
        type_name: String,
        /// The full JSON data for this content, preserved for debugging
        data: serde_json::Value,
    },
}

// Custom Serialize implementation for InteractionContent.
// This handles the Unknown variant specially by merging type_name into the data.
impl Serialize for InteractionContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        match self {
            Self::Text { text } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "text")?;
                if let Some(t) = text {
                    map.serialize_entry("text", t)?;
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
            Self::Unknown { type_name, data } => {
                // For Unknown, merge the type_name into the data object
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", type_name)?;
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
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text { text: Some(t) } if !t.is_empty() => Some(t),
            _ => None,
        }
    }

    /// Extract the thought content, if this is a Thought variant with non-empty text.
    ///
    /// Returns `Some` only for `Thought` variants with non-empty text.
    /// Returns `None` for all other variants including `Text`.
    pub fn thought(&self) -> Option<&str> {
        match self {
            Self::Thought { text: Some(t) } if !t.is_empty() => Some(t),
            _ => None,
        }
    }

    /// Check if this is a Text content type.
    pub const fn is_text(&self) -> bool {
        matches!(self, Self::Text { .. })
    }

    /// Check if this is a Thought content type.
    pub const fn is_thought(&self) -> bool {
        matches!(self, Self::Thought { .. })
    }

    /// Check if this is a ThoughtSignature content type.
    pub const fn is_thought_signature(&self) -> bool {
        matches!(self, Self::ThoughtSignature { .. })
    }

    /// Check if this is a FunctionCall content type.
    pub const fn is_function_call(&self) -> bool {
        matches!(self, Self::FunctionCall { .. })
    }

    /// Returns `true` if this is an unknown content type.
    ///
    /// Use this to check for content types that the library doesn't recognize.
    /// See [`InteractionContent::Unknown`] for more details.
    pub const fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }

    /// Check if this is a CodeExecutionCall content type.
    pub const fn is_code_execution_call(&self) -> bool {
        matches!(self, Self::CodeExecutionCall { .. })
    }

    /// Check if this is a CodeExecutionResult content type.
    pub const fn is_code_execution_result(&self) -> bool {
        matches!(self, Self::CodeExecutionResult { .. })
    }

    /// Check if this is a GoogleSearchCall content type.
    pub const fn is_google_search_call(&self) -> bool {
        matches!(self, Self::GoogleSearchCall { .. })
    }

    /// Check if this is a GoogleSearchResult content type.
    pub const fn is_google_search_result(&self) -> bool {
        matches!(self, Self::GoogleSearchResult { .. })
    }

    /// Check if this is a UrlContextCall content type.
    pub const fn is_url_context_call(&self) -> bool {
        matches!(self, Self::UrlContextCall { .. })
    }

    /// Check if this is a UrlContextResult content type.
    pub const fn is_url_context_result(&self) -> bool {
        matches!(self, Self::UrlContextResult { .. })
    }

    /// Returns the type name if this is an unknown content type.
    ///
    /// Returns `None` for known content types.
    pub fn unknown_type(&self) -> Option<&str> {
        match self {
            Self::Unknown { type_name, .. } => Some(type_name),
            _ => None,
        }
    }

    /// Returns the raw JSON data if this is an unknown content type.
    ///
    /// Returns `None` for known content types.
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
                KnownContent::Text { text } => InteractionContent::Text { text },
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
                    let (lang, source) = if let (Some(l), Some(c)) = (language, code) {
                        (l, c)
                    } else if let Some(args) = arguments {
                        // Parse old format: {"language": "PYTHON", "code": "..."}
                        let lang = args
                            .get("language")
                            .and_then(|v| {
                                serde_json::from_value::<CodeExecutionLanguage>(v.clone()).ok()
                            })
                            .unwrap_or(CodeExecutionLanguage::Python);
                        let source = args
                            .get("code")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        (lang, source)
                    } else {
                        log::warn!(
                            "CodeExecutionCall missing both direct fields and arguments for id: {}",
                            id
                        );
                        (CodeExecutionLanguage::Python, String::new())
                    };

                    InteractionContent::CodeExecutionCall {
                        id,
                        language: lang,
                        code: source,
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
            Err(_) => {
                // Unknown type - extract type name and preserve data
                let type_name = value
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("<missing type>")
                    .to_string();

                log::warn!(
                    "Encountered unknown InteractionContent type '{}'. \
                     This may indicate a new API feature not yet supported by this library. \
                     The content will be preserved in the Unknown variant.",
                    type_name
                );

                #[cfg(feature = "strict-unknown")]
                {
                    Err(D::Error::custom(format!(
                        "Unknown InteractionContent type '{}'. \
                         Strict mode is enabled via the 'strict-unknown' feature flag. \
                         Either update the library or disable strict mode.",
                        type_name
                    )))
                }

                #[cfg(not(feature = "strict-unknown"))]
                {
                    Ok(InteractionContent::Unknown {
                        type_name,
                        data: value,
                    })
                }
            }
        }
    }
}
