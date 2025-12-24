use serde::{Deserialize, Serialize};

// Import only Tool from shared types
use super::shared::Tool;

/// Content object for Interactions API - uses flat structure with type field.
///
/// This enum represents all content types that can appear in API requests and responses.
/// It includes an `Unknown` variant for forward compatibility with new API content types.
///
/// # Forward Compatibility
///
/// When the API returns a content type that this library doesn't recognize, it will be
/// captured as `InteractionContent::Unknown` rather than causing a deserialization error.
/// This allows your code to continue working even when Google adds new content types.
///
/// Use [`InteractionResponse::has_unknown`] and [`InteractionResponse::unknown_content`]
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
///         _ => {}
///     }
/// }
/// ```
#[derive(Clone, Debug)]
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
    /// - New API features (e.g., `code_execution_result`, `google_search_result`)
    /// - Beta features not yet supported by this library
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
    /// # Serialization
    ///
    /// Unknown variants can be serialized back to JSON, preserving the original
    /// structure. This enables round-trip in multi-turn conversations.
    Unknown {
        /// The unrecognized type name from the API (e.g., "code_execution_result")
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

/// Input for an interaction - can be a simple string or array of content
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum InteractionInput {
    /// Simple text input
    Text(String),
    /// Array of content objects
    Content(Vec<InteractionContent>),
}

/// Generation configuration for model behavior
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    /// Thinking level: "minimal", "low", "medium", "high"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_level: Option<String>,
}

/// Request body for the Interactions API endpoint
#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateInteractionRequest {
    /// Model name (e.g., "gemini-3-flash-preview") - mutually exclusive with agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Agent name (e.g., "deep-research-pro-preview-12-2025") - mutually exclusive with model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,

    /// The input for this interaction
    pub input: InteractionInput,

    /// Reference to a previous interaction for stateful conversations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_interaction_id: Option<String>,

    /// Tools available for function calling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    /// Response modalities (e.g., ["IMAGE"])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_modalities: Option<Vec<String>>,

    /// JSON schema for structured output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<serde_json::Value>,

    /// Model configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,

    /// Enable streaming responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Background execution mode (agents only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,

    /// Persist interaction data (default: true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,

    /// System instruction for the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<InteractionInput>,
}

/// Status of an interaction
#[derive(Clone, Deserialize, Serialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InteractionStatus {
    Completed,
    InProgress,
    RequiresAction,
    Failed,
    Cancelled,
}

/// Token usage information from the Interactions API
#[derive(Clone, Deserialize, Serialize, Debug, Default, PartialEq)]
#[serde(default)]
pub struct UsageMetadata {
    /// Total number of input tokens (prompt tokens sent to the model)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_input_tokens: Option<i32>,
    /// Total number of output tokens (tokens generated by the model)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_output_tokens: Option<i32>,
    /// Total number of tokens (input + output)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<i32>,
    /// Total number of cached tokens (from context caching, reduces billing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cached_tokens: Option<i32>,
    /// Total number of reasoning tokens (populated for thinking models like gemini-2.0-flash-thinking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_reasoning_tokens: Option<i32>,
    /// Total number of tokens used for tool/function calling overhead
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tool_use_tokens: Option<i32>,
}

impl UsageMetadata {
    /// Returns true if any usage data is present
    pub fn has_data(&self) -> bool {
        self.total_tokens.is_some()
            || self.total_input_tokens.is_some()
            || self.total_output_tokens.is_some()
            || self.total_cached_tokens.is_some()
            || self.total_reasoning_tokens.is_some()
            || self.total_tool_use_tokens.is_some()
    }
}

/// Response from creating or retrieving an interaction
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InteractionResponse {
    /// Unique identifier for this interaction
    pub id: String,

    /// Model name if a model was used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Agent name if an agent was used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,

    /// The input that was provided (array of content objects)
    #[serde(default)]
    pub input: Vec<InteractionContent>,

    /// The outputs generated by the model/agent (array of content objects)
    #[serde(default)]
    pub outputs: Vec<InteractionContent>,

    /// Current status of the interaction
    pub status: InteractionStatus,

    /// Token usage information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageMetadata>,

    /// Tools that were available for this interaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    /// Previous interaction ID if this was a follow-up
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_interaction_id: Option<String>,
}

impl InteractionResponse {
    /// Extract the first text content from outputs
    ///
    /// Returns the first text found in the outputs vector.
    /// Useful for simple queries where you expect a single text response.
    ///
    /// # Example
    /// ```no_run
    /// # use genai_client::models::interactions::InteractionResponse;
    /// # let response: InteractionResponse = todo!();
    /// if let Some(text) = response.text() {
    ///     println!("Response: {}", text);
    /// }
    /// ```
    pub fn text(&self) -> Option<&str> {
        self.outputs.iter().find_map(|content| {
            if let InteractionContent::Text { text: Some(t) } = content {
                Some(t.as_str())
            } else {
                None
            }
        })
    }

    /// Extract all text contents concatenated
    ///
    /// Combines all text outputs into a single string.
    /// Useful when the model returns multiple text chunks.
    ///
    /// # Example
    /// ```no_run
    /// # use genai_client::models::interactions::InteractionResponse;
    /// # let response: InteractionResponse = todo!();
    /// let full_text = response.all_text();
    /// println!("Complete response: {}", full_text);
    /// ```
    pub fn all_text(&self) -> String {
        self.outputs
            .iter()
            .filter_map(|content| {
                if let InteractionContent::Text { text: Some(t) } = content {
                    Some(t.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Extract function calls from outputs
    ///
    /// Returns a vector of (call_id, function_name, arguments, thought_signature) tuples.
    /// The call_id should be used when sending function results back to the model.
    ///
    /// # Example
    /// ```no_run
    /// # use genai_client::models::interactions::InteractionResponse;
    /// # let response: InteractionResponse = todo!();
    /// for (call_id, name, args, signature) in response.function_calls() {
    ///     println!("Call ID: {:?}, Function: {} with args: {}", call_id, name, args);
    /// }
    /// ```
    pub fn function_calls(&self) -> Vec<(Option<&str>, &str, &serde_json::Value, Option<&str>)> {
        self.outputs
            .iter()
            .filter_map(|content| {
                if let InteractionContent::FunctionCall {
                    id,
                    name,
                    args,
                    thought_signature,
                } = content
                {
                    Some((
                        id.as_ref().map(|s| s.as_str()),
                        name.as_str(),
                        args,
                        thought_signature.as_ref().map(|s| s.as_str()),
                    ))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if response contains text
    ///
    /// Returns true if any output contains text content.
    pub fn has_text(&self) -> bool {
        self.outputs
            .iter()
            .any(|c| matches!(c, InteractionContent::Text { text: Some(_) }))
    }

    /// Check if response contains function calls
    ///
    /// Returns true if any output contains a function call.
    pub fn has_function_calls(&self) -> bool {
        self.outputs
            .iter()
            .any(|c| matches!(c, InteractionContent::FunctionCall { .. }))
    }

    /// Check if response contains thoughts (internal reasoning)
    ///
    /// Returns true if any output contains thought content.
    pub fn has_thoughts(&self) -> bool {
        self.outputs
            .iter()
            .any(|c| matches!(c, InteractionContent::Thought { text: Some(_) }))
    }

    /// Check if response contains unknown content types.
    ///
    /// Returns `true` if any output contains an [`InteractionContent::Unknown`] variant.
    /// This indicates the API returned content types that this library version doesn't
    /// recognize.
    ///
    /// # When to Use
    ///
    /// Call this after receiving a response to detect if you might be missing content:
    ///
    /// ```no_run
    /// # use genai_client::models::interactions::InteractionResponse;
    /// # let response: InteractionResponse = todo!();
    /// if response.has_unknown() {
    ///     eprintln!("Warning: Response contains unknown content types");
    ///     for (type_name, data) in response.unknown_content() {
    ///         eprintln!("  - {}: {:?}", type_name, data);
    ///     }
    /// }
    /// ```
    pub fn has_unknown(&self) -> bool {
        self.outputs
            .iter()
            .any(|c| matches!(c, InteractionContent::Unknown { .. }))
    }

    /// Get all unknown content as (type_name, data) tuples.
    ///
    /// Returns a vector of references to the type names and JSON data for all
    /// [`InteractionContent::Unknown`] variants in the outputs.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use genai_client::models::interactions::InteractionResponse;
    /// # let response: InteractionResponse = todo!();
    /// for (type_name, data) in response.unknown_content() {
    ///     println!("Unknown type '{}': {}", type_name, data);
    /// }
    /// ```
    pub fn unknown_content(&self) -> Vec<(&str, &serde_json::Value)> {
        self.outputs
            .iter()
            .filter_map(|content| {
                if let InteractionContent::Unknown { type_name, data } = content {
                    Some((type_name.as_str(), data))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get a summary of content types present in outputs.
    ///
    /// Returns a [`ContentSummary`] with counts for each content type.
    /// Useful for debugging, logging, or detecting unexpected content.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use genai_client::models::interactions::InteractionResponse;
    /// # let response: InteractionResponse = todo!();
    /// let summary = response.content_summary();
    /// println!("Response has {} text outputs", summary.text_count);
    /// if summary.unknown_count > 0 {
    ///     println!("Warning: {} unknown types: {:?}",
    ///         summary.unknown_count, summary.unknown_types);
    /// }
    /// ```
    pub fn content_summary(&self) -> ContentSummary {
        use std::collections::BTreeSet;

        let mut summary = ContentSummary::default();
        let mut unknown_types_set = BTreeSet::new();

        for content in &self.outputs {
            match content {
                InteractionContent::Text { .. } => summary.text_count += 1,
                InteractionContent::Thought { .. } => summary.thought_count += 1,
                InteractionContent::ThoughtSignature { .. } => {
                    // ThoughtSignature typically only appears during streaming,
                    // not in final outputs. Count with thoughts if present.
                    summary.thought_count += 1
                }
                InteractionContent::Image { .. } => summary.image_count += 1,
                InteractionContent::Audio { .. } => summary.audio_count += 1,
                InteractionContent::Video { .. } => summary.video_count += 1,
                InteractionContent::FunctionCall { .. } => summary.function_call_count += 1,
                InteractionContent::FunctionResult { .. } => summary.function_result_count += 1,
                InteractionContent::Unknown { type_name, .. } => {
                    summary.unknown_count += 1;
                    unknown_types_set.insert(type_name.clone());
                }
            }
        }

        // BTreeSet maintains sorted order, so no need to sort
        summary.unknown_types = unknown_types_set.into_iter().collect();
        summary
    }
}

/// Summary of content types present in an interaction response.
///
/// Returned by [`InteractionResponse::content_summary`]. Provides a quick overview
/// of what content types are present, including any unknown types.
///
/// # Example
///
/// ```no_run
/// # use genai_client::models::interactions::InteractionResponse;
/// # let response: InteractionResponse = todo!();
/// let summary = response.content_summary();
///
/// // Check for unexpected content
/// if summary.unknown_count > 0 {
///     log::warn!(
///         "Response contains {} unknown content types: {:?}",
///         summary.unknown_count,
///         summary.unknown_types
///     );
/// }
///
/// // Log content breakdown
/// log::debug!(
///     "Content: {} text, {} thoughts, {} function calls",
///     summary.text_count,
///     summary.thought_count,
///     summary.function_call_count
/// );
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContentSummary {
    /// Number of text content items
    pub text_count: usize,
    /// Number of thought content items
    pub thought_count: usize,
    /// Number of image content items
    pub image_count: usize,
    /// Number of audio content items
    pub audio_count: usize,
    /// Number of video content items
    pub video_count: usize,
    /// Number of function call content items
    pub function_call_count: usize,
    /// Number of function result content items
    pub function_result_count: usize,
    /// Number of unknown content items
    pub unknown_count: usize,
    /// List of unique unknown type names encountered (sorted alphabetically)
    pub unknown_types: Vec<String>,
}

/// A chunk from the streaming API
///
/// During streaming, the API sends different types of events:
/// - `Delta`: Incremental content updates (text, thought, function_call, etc.)
/// - `Complete`: The final complete interaction response
#[derive(Clone, Debug)]
pub enum StreamChunk {
    /// Incremental content update
    Delta(InteractionContent),
    /// Complete interaction response (final event)
    Complete(InteractionResponse),
}

/// Wrapper for SSE streaming events from the Interactions API
///
/// The API returns different event types during streaming:
/// - `content.delta`: Contains incremental content in the `delta` field
/// - `interaction.complete`: Contains the full interaction in the `interaction` field
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct InteractionStreamEvent {
    /// Event type (e.g., "content.delta", "interaction.complete")
    pub event_type: String,

    /// The full interaction data (present in "interaction.complete" events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction: Option<InteractionResponse>,

    /// Incremental content delta (present in "content.delta" events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<InteractionContent>,

    /// Interaction ID (present in various events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction_id: Option<String>,

    /// Status (present in status update events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<InteractionStatus>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_create_interaction_request_with_model() {
        let request = CreateInteractionRequest {
            model: Some("gemini-3-flash-preview".to_string()),
            agent: None,
            input: InteractionInput::Text("Hello, world!".to_string()),
            previous_interaction_id: None,
            tools: None,
            response_modalities: None,
            response_format: None,
            generation_config: None,
            stream: None,
            background: None,
            store: None,
            system_instruction: None,
        };

        let json = serde_json::to_string(&request).expect("Serialization failed");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["model"], "gemini-3-flash-preview");
        assert_eq!(value["input"], "Hello, world!");
        assert!(value.get("agent").is_none());
    }

    #[test]
    fn test_serialize_interaction_content() {
        let content = InteractionContent::Text {
            text: Some("Hello".to_string()),
        };

        let json = serde_json::to_string(&content).expect("Serialization failed");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["type"], "text");
        assert_eq!(value["text"], "Hello");
    }

    #[test]
    fn test_deserialize_interaction_response_completed() {
        let response_json = r#"{
            "id": "interaction_123",
            "model": "gemini-3-flash-preview",
            "input": [{"type": "text", "text": "Hello"}],
            "outputs": [{"type": "text", "text": "Hi there!"}],
            "status": "completed",
            "usage": {
                "total_input_tokens": 5,
                "total_output_tokens": 10,
                "total_tokens": 15
            }
        }"#;

        let response: InteractionResponse =
            serde_json::from_str(response_json).expect("Deserialization failed");

        assert_eq!(response.id, "interaction_123");
        assert_eq!(response.model.as_deref(), Some("gemini-3-flash-preview"));
        assert_eq!(response.status, InteractionStatus::Completed);
        assert_eq!(response.input.len(), 1);
        assert_eq!(response.outputs.len(), 1);
        assert!(response.usage.is_some());
        let usage = response.usage.unwrap();
        assert_eq!(usage.total_input_tokens, Some(5));
        assert_eq!(usage.total_output_tokens, Some(10));
        assert_eq!(usage.total_tokens, Some(15));
    }

    #[test]
    fn test_deserialize_usage_metadata_partial() {
        // Test that partial usage responses deserialize correctly with #[serde(default)]
        let partial_json = r#"{"total_tokens": 42}"#;
        let usage: UsageMetadata =
            serde_json::from_str(partial_json).expect("Deserialization failed");

        assert_eq!(usage.total_tokens, Some(42));
        assert_eq!(usage.total_input_tokens, None);
        assert_eq!(usage.total_output_tokens, None);
        assert_eq!(usage.total_cached_tokens, None);
        assert_eq!(usage.total_reasoning_tokens, None);
        assert_eq!(usage.total_tool_use_tokens, None);
    }

    #[test]
    fn test_deserialize_usage_metadata_empty() {
        // Test that empty usage object deserializes to defaults
        let empty_json = r#"{}"#;
        let usage: UsageMetadata =
            serde_json::from_str(empty_json).expect("Deserialization failed");

        assert_eq!(usage.total_tokens, None);
        assert_eq!(usage.total_input_tokens, None);
        assert_eq!(usage.total_output_tokens, None);
    }

    #[test]
    fn test_usage_metadata_has_data() {
        // Empty usage has no data
        let empty = UsageMetadata::default();
        assert!(!empty.has_data());

        // Usage with only total_tokens
        let with_total = UsageMetadata {
            total_tokens: Some(100),
            ..Default::default()
        };
        assert!(with_total.has_data());

        // Usage with only cached tokens
        let with_cached = UsageMetadata {
            total_cached_tokens: Some(50),
            ..Default::default()
        };
        assert!(with_cached.has_data());
    }

    #[test]
    fn test_deserialize_function_call_content() {
        let content_json = r#"{"type": "function_call", "name": "get_weather", "arguments": {"location": "Paris"}}"#;

        let content: InteractionContent =
            serde_json::from_str(content_json).expect("Deserialization failed");

        match content {
            InteractionContent::FunctionCall { name, args, .. } => {
                assert_eq!(name, "get_weather");
                assert_eq!(args["location"], "Paris");
            }
            _ => panic!("Expected FunctionCall variant"),
        }
    }

    #[test]
    fn test_generation_config_serialization() {
        let config = GenerationConfig {
            temperature: Some(0.7),
            max_output_tokens: Some(500),
            top_p: Some(0.9),
            top_k: Some(40),
            thinking_level: Some("medium".to_string()),
        };

        let json = serde_json::to_string(&config).expect("Serialization failed");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["temperature"], 0.7);
        assert_eq!(value["maxOutputTokens"], 500);
        assert_eq!(value["thinkingLevel"], "medium");
    }

    #[test]
    fn test_interaction_response_text() {
        let response = InteractionResponse {
            id: "test_id".to_string(),
            model: Some("gemini-3-flash".to_string()),
            agent: None,
            input: vec![],
            outputs: vec![
                InteractionContent::Text {
                    text: Some("Hello".to_string()),
                },
                InteractionContent::Text {
                    text: Some("World".to_string()),
                },
            ],
            status: InteractionStatus::Completed,
            usage: None,
            tools: None,
            previous_interaction_id: None,
        };

        assert_eq!(response.text(), Some("Hello"));
        assert_eq!(response.all_text(), "HelloWorld");
        assert!(response.has_text());
        assert!(!response.has_function_calls());
    }

    #[test]
    fn test_interaction_response_function_calls() {
        let response = InteractionResponse {
            id: "test_id".to_string(),
            model: Some("gemini-3-flash".to_string()),
            agent: None,
            input: vec![],
            outputs: vec![
                InteractionContent::FunctionCall {
                    id: Some("call_001".to_string()),
                    name: "get_weather".to_string(),
                    args: serde_json::json!({"location": "Paris"}),
                    thought_signature: Some("sig123".to_string()),
                },
                InteractionContent::FunctionCall {
                    id: Some("call_002".to_string()),
                    name: "get_time".to_string(),
                    args: serde_json::json!({"timezone": "UTC"}),
                    thought_signature: None,
                },
            ],
            status: InteractionStatus::Completed,
            usage: None,
            tools: None,
            previous_interaction_id: None,
        };

        let calls = response.function_calls();
        assert_eq!(calls.len(), 2);
        // Tuple is (call_id, name, args, signature)
        assert_eq!(calls[0].0, Some("call_001")); // call_id at index 0
        assert_eq!(calls[0].1, "get_weather"); // name at index 1
        assert_eq!(calls[0].2["location"], "Paris"); // args at index 2
        assert_eq!(calls[0].3, Some("sig123")); // signature at index 3
        assert_eq!(calls[1].0, Some("call_002")); // call_id at index 0
        assert_eq!(calls[1].1, "get_time"); // name at index 1
        assert_eq!(calls[1].3, None); // signature at index 3
        assert!(response.has_function_calls());
        assert!(!response.has_text());
    }

    #[test]
    fn test_interaction_response_mixed_content() {
        let response = InteractionResponse {
            id: "test_id".to_string(),
            model: Some("gemini-3-flash".to_string()),
            agent: None,
            input: vec![],
            outputs: vec![
                InteractionContent::Text {
                    text: Some("Let me check".to_string()),
                },
                InteractionContent::FunctionCall {
                    id: Some("call_mixed".to_string()),
                    name: "check_status".to_string(),
                    args: serde_json::json!({}),
                    thought_signature: None,
                },
                InteractionContent::Text {
                    text: Some("Done!".to_string()),
                },
            ],
            status: InteractionStatus::Completed,
            usage: None,
            tools: None,
            previous_interaction_id: None,
        };

        assert_eq!(response.text(), Some("Let me check"));
        assert_eq!(response.all_text(), "Let me checkDone!");
        assert_eq!(response.function_calls().len(), 1);
        assert!(response.has_text());
        assert!(response.has_function_calls());
    }

    #[test]
    fn test_interaction_response_empty_outputs() {
        let response = InteractionResponse {
            id: "test_id".to_string(),
            model: Some("gemini-3-flash".to_string()),
            agent: None,
            input: vec![],
            outputs: vec![],
            status: InteractionStatus::Completed,
            usage: None,
            tools: None,
            previous_interaction_id: None,
        };

        assert_eq!(response.text(), None);
        assert_eq!(response.all_text(), "");
        assert_eq!(response.function_calls().len(), 0);
        assert!(!response.has_text());
        assert!(!response.has_function_calls());
    }

    // --- Streaming Event Tests ---

    #[test]
    fn test_deserialize_streaming_text_content() {
        // Streaming deltas now use InteractionContent directly
        let delta_json = r#"{"type": "text", "text": "Hello world"}"#;
        let delta: InteractionContent =
            serde_json::from_str(delta_json).expect("Deserialization failed");

        match &delta {
            InteractionContent::Text { text } => {
                assert_eq!(text.as_deref(), Some("Hello world"));
            }
            _ => panic!("Expected Text content"),
        }

        assert!(delta.is_text());
        assert!(!delta.is_thought());
        assert_eq!(delta.text(), Some("Hello world"));
    }

    #[test]
    fn test_deserialize_streaming_thought_content() {
        let delta_json = r#"{"type": "thought", "text": "I'm thinking..."}"#;
        let delta: InteractionContent =
            serde_json::from_str(delta_json).expect("Deserialization failed");

        match &delta {
            InteractionContent::Thought { text } => {
                assert_eq!(text.as_deref(), Some("I'm thinking..."));
            }
            _ => panic!("Expected Thought content"),
        }

        assert!(!delta.is_text());
        assert!(delta.is_thought());
        // text() returns None for thoughts (only returns text for Text variant)
        assert_eq!(delta.text(), None);
    }

    #[test]
    fn test_deserialize_streaming_function_call() {
        // Function calls can now be streamed - this was issue #27
        let delta_json =
            r#"{"type": "function_call", "name": "get_weather", "arguments": {"city": "Paris"}}"#;
        let delta: InteractionContent =
            serde_json::from_str(delta_json).expect("Deserialization failed");

        match &delta {
            InteractionContent::FunctionCall { name, args, .. } => {
                assert_eq!(name, "get_weather");
                assert_eq!(args["city"], "Paris");
            }
            _ => panic!("Expected FunctionCall content"),
        }

        assert!(delta.is_function_call());
        assert!(!delta.is_unknown()); // function_call is now a KNOWN type!
    }

    #[test]
    fn test_deserialize_streaming_thought_signature() {
        let delta_json = r#"{"type": "thought_signature", "signature": "abc123"}"#;
        let delta: InteractionContent =
            serde_json::from_str(delta_json).expect("Deserialization failed");

        match &delta {
            InteractionContent::ThoughtSignature { signature } => {
                assert_eq!(signature, "abc123");
            }
            _ => panic!("Expected ThoughtSignature content"),
        }

        assert!(delta.is_thought_signature());
    }

    #[test]
    fn test_deserialize_content_delta_event() {
        let event_json = r#"{
            "event_type": "content.delta",
            "interaction_id": "test_123",
            "delta": {"type": "text", "text": "Hello"}
        }"#;

        let event: InteractionStreamEvent =
            serde_json::from_str(event_json).expect("Deserialization failed");

        assert_eq!(event.event_type, "content.delta");
        assert_eq!(event.interaction_id.as_deref(), Some("test_123"));
        assert!(event.delta.is_some());
        assert!(event.interaction.is_none());

        let delta = event.delta.unwrap();
        assert!(delta.is_text());
        assert_eq!(delta.text(), Some("Hello"));
    }

    #[test]
    fn test_deserialize_interaction_complete_event() {
        let event_json = r#"{
            "event_type": "interaction.complete",
            "interaction": {
                "id": "interaction_456",
                "model": "gemini-3-flash-preview",
                "input": [{"type": "text", "text": "Count to 3"}],
                "outputs": [{"type": "text", "text": "1, 2, 3"}],
                "status": "completed"
            }
        }"#;

        let event: InteractionStreamEvent =
            serde_json::from_str(event_json).expect("Deserialization failed");

        assert_eq!(event.event_type, "interaction.complete");
        assert!(event.interaction.is_some());
        assert!(event.delta.is_none());

        let interaction = event.interaction.unwrap();
        assert_eq!(interaction.id, "interaction_456");
        assert_eq!(interaction.text(), Some("1, 2, 3"));
    }

    #[test]
    fn test_content_empty_text_returns_none() {
        let content = InteractionContent::Text {
            text: Some(String::new()),
        };
        assert_eq!(content.text(), None);

        let content_none = InteractionContent::Text { text: None };
        assert_eq!(content_none.text(), None);
    }

    // --- Unknown Variant Tests ---

    #[test]
    fn test_deserialize_unknown_interaction_content() {
        // Simulate a new API content type that this library doesn't know about
        let unknown_json =
            r#"{"type": "code_execution_result", "outcome": "success", "output": "42"}"#;

        let content: InteractionContent =
            serde_json::from_str(unknown_json).expect("Should deserialize as Unknown");

        match &content {
            InteractionContent::Unknown { type_name, data } => {
                assert_eq!(type_name, "code_execution_result");
                assert_eq!(data["outcome"], "success");
                assert_eq!(data["output"], "42");
            }
            _ => panic!("Expected Unknown variant, got {:?}", content),
        }

        assert!(content.is_unknown());
        assert_eq!(content.unknown_type(), Some("code_execution_result"));
        assert!(content.unknown_data().is_some());
    }

    #[test]
    fn test_deserialize_unknown_streaming_content() {
        // Simulate a new streaming content type that this library doesn't know about
        let unknown_json = r#"{"type": "new_feature_delta", "data": "some_value"}"#;

        let content: InteractionContent =
            serde_json::from_str(unknown_json).expect("Should deserialize as Unknown");

        assert!(content.is_unknown());
        assert_eq!(content.unknown_type(), Some("new_feature_delta"));

        match &content {
            InteractionContent::Unknown { type_name, data } => {
                assert_eq!(type_name, "new_feature_delta");
                assert_eq!(data["data"], "some_value");
            }
            _ => panic!("Expected Unknown variant"),
        }
    }

    #[test]
    fn test_known_types_still_work() {
        // Ensure adding Unknown doesn't break known types
        let text_json = r#"{"type": "text", "text": "Hello"}"#;
        let content: InteractionContent = serde_json::from_str(text_json).unwrap();
        assert!(matches!(content, InteractionContent::Text { .. }));
        assert!(!content.is_unknown());

        let thought_json = r#"{"type": "thought", "text": "Thinking..."}"#;
        let content: InteractionContent = serde_json::from_str(thought_json).unwrap();
        assert!(matches!(content, InteractionContent::Thought { .. }));
        assert!(!content.is_unknown());

        let signature_json = r#"{"type": "thought_signature", "signature": "sig123"}"#;
        let content: InteractionContent = serde_json::from_str(signature_json).unwrap();
        assert!(matches!(
            content,
            InteractionContent::ThoughtSignature { .. }
        ));
        assert!(!content.is_unknown());

        let function_json = r#"{"type": "function_call", "name": "test", "arguments": {}}"#;
        let content: InteractionContent = serde_json::from_str(function_json).unwrap();
        assert!(matches!(content, InteractionContent::FunctionCall { .. }));
        assert!(!content.is_unknown());
    }

    #[test]
    fn test_interaction_response_has_unknown() {
        let response = InteractionResponse {
            id: "test_id".to_string(),
            model: Some("gemini-3-flash".to_string()),
            agent: None,
            input: vec![],
            outputs: vec![
                InteractionContent::Text {
                    text: Some("Here's the result:".to_string()),
                },
                InteractionContent::Unknown {
                    type_name: "code_execution_result".to_string(),
                    data: serde_json::json!({
                        "type": "code_execution_result",
                        "outcome": "success",
                        "output": "42"
                    }),
                },
            ],
            status: InteractionStatus::Completed,
            usage: None,
            tools: None,
            previous_interaction_id: None,
        };

        assert!(response.has_unknown());
        assert!(response.has_text());

        let unknowns = response.unknown_content();
        assert_eq!(unknowns.len(), 1);
        assert_eq!(unknowns[0].0, "code_execution_result");
        assert_eq!(unknowns[0].1["outcome"], "success");
    }

    #[test]
    fn test_interaction_response_no_unknown() {
        let response = InteractionResponse {
            id: "test_id".to_string(),
            model: Some("gemini-3-flash".to_string()),
            agent: None,
            input: vec![],
            outputs: vec![InteractionContent::Text {
                text: Some("Normal response".to_string()),
            }],
            status: InteractionStatus::Completed,
            usage: None,
            tools: None,
            previous_interaction_id: None,
        };

        assert!(!response.has_unknown());
        assert!(response.unknown_content().is_empty());
    }

    #[test]
    fn test_content_summary() {
        let response = InteractionResponse {
            id: "test_id".to_string(),
            model: Some("gemini-3-flash".to_string()),
            agent: None,
            input: vec![],
            outputs: vec![
                InteractionContent::Text {
                    text: Some("Text 1".to_string()),
                },
                InteractionContent::Text {
                    text: Some("Text 2".to_string()),
                },
                InteractionContent::Thought {
                    text: Some("Thinking".to_string()),
                },
                InteractionContent::FunctionCall {
                    id: Some("call_1".to_string()),
                    name: "test_fn".to_string(),
                    args: serde_json::json!({}),
                    thought_signature: None,
                },
                InteractionContent::Unknown {
                    type_name: "type_a".to_string(),
                    data: serde_json::json!({"type": "type_a"}),
                },
                InteractionContent::Unknown {
                    type_name: "type_b".to_string(),
                    data: serde_json::json!({"type": "type_b"}),
                },
                InteractionContent::Unknown {
                    type_name: "type_a".to_string(), // Duplicate type
                    data: serde_json::json!({"type": "type_a", "extra": true}),
                },
            ],
            status: InteractionStatus::Completed,
            usage: None,
            tools: None,
            previous_interaction_id: None,
        };

        let summary = response.content_summary();

        assert_eq!(summary.text_count, 2);
        assert_eq!(summary.thought_count, 1);
        assert_eq!(summary.function_call_count, 1);
        assert_eq!(summary.unknown_count, 3);
        // Unknown types should be deduplicated and sorted
        assert_eq!(summary.unknown_types.len(), 2);
        assert_eq!(summary.unknown_types, vec!["type_a", "type_b"]);
    }

    #[test]
    fn test_content_summary_empty() {
        let response = InteractionResponse {
            id: "test_id".to_string(),
            model: Some("gemini-3-flash".to_string()),
            agent: None,
            input: vec![],
            outputs: vec![],
            status: InteractionStatus::Completed,
            usage: None,
            tools: None,
            previous_interaction_id: None,
        };

        let summary = response.content_summary();

        assert_eq!(summary.text_count, 0);
        assert_eq!(summary.unknown_count, 0);
        assert!(summary.unknown_types.is_empty());
    }

    #[test]
    fn test_serialize_unknown_content_roundtrip() {
        // Create an Unknown content (simulating what we'd receive from API)
        let unknown = InteractionContent::Unknown {
            type_name: "code_execution_result".to_string(),
            data: serde_json::json!({
                "outcome": "success",
                "output": "42"
            }),
        };

        // Serialize it
        let json = serde_json::to_string(&unknown).expect("Serialization should work");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Verify the structure: type field + flattened data
        assert_eq!(value["type"], "code_execution_result");
        assert_eq!(value["outcome"], "success");
        assert_eq!(value["output"], "42");
    }

    #[test]
    fn test_deserialize_response_with_unknown_in_outputs() {
        // Test deserializing a full response that contains unknown content
        let response_json = r#"{
            "id": "interaction_789",
            "model": "gemini-3-flash-preview",
            "input": [{"type": "text", "text": "Execute some code"}],
            "outputs": [
                {"type": "text", "text": "Here's the result:"},
                {"type": "executable_code", "language": "python", "code": "print(42)"},
                {"type": "code_execution_result", "outcome": "success", "output": "42"}
            ],
            "status": "completed"
        }"#;

        let response: InteractionResponse =
            serde_json::from_str(response_json).expect("Should deserialize with unknown types");

        assert_eq!(response.id, "interaction_789");
        assert_eq!(response.outputs.len(), 3);
        assert!(response.has_text());
        assert!(response.has_unknown());

        let summary = response.content_summary();
        assert_eq!(summary.text_count, 1);
        assert_eq!(summary.unknown_count, 2);
        assert!(
            summary
                .unknown_types
                .contains(&"executable_code".to_string())
        );
        assert!(
            summary
                .unknown_types
                .contains(&"code_execution_result".to_string())
        );
    }

    #[test]
    fn test_serialize_known_variant_with_none_fields() {
        // Test that known variants with None fields serialize correctly (omit None fields)
        let text = InteractionContent::Text { text: None };
        let json = serde_json::to_string(&text).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "text");
        assert!(value.get("text").is_none());

        let image = InteractionContent::Image {
            data: Some("base64data".to_string()),
            uri: None,
            mime_type: None,
        };
        let json = serde_json::to_string(&image).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "image");
        assert_eq!(value["data"], "base64data");
        assert!(value.get("uri").is_none());
        assert!(value.get("mime_type").is_none());

        let fc = InteractionContent::FunctionCall {
            id: None,
            name: "test_fn".to_string(),
            args: serde_json::json!({"arg": "value"}),
            thought_signature: None,
        };
        let json = serde_json::to_string(&fc).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "function_call");
        assert_eq!(value["name"], "test_fn");
        assert!(value.get("id").is_none());
        assert!(value.get("thoughtSignature").is_none());
    }

    #[test]
    fn test_serialize_unknown_with_non_object_data() {
        // Test that Unknown with non-object data (array, string, number) is preserved
        let unknown_array = InteractionContent::Unknown {
            type_name: "weird_type".to_string(),
            data: serde_json::json!([1, 2, 3]),
        };
        let json = serde_json::to_string(&unknown_array).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "weird_type");
        assert_eq!(value["data"], serde_json::json!([1, 2, 3]));

        let unknown_string = InteractionContent::Unknown {
            type_name: "string_type".to_string(),
            data: serde_json::json!("just a string"),
        };
        let json = serde_json::to_string(&unknown_string).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "string_type");
        assert_eq!(value["data"], "just a string");

        let unknown_null = InteractionContent::Unknown {
            type_name: "null_type".to_string(),
            data: serde_json::Value::Null,
        };
        let json = serde_json::to_string(&unknown_null).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "null_type");
        // Null data should be omitted
        assert!(value.get("data").is_none());
    }

    #[test]
    fn test_deserialize_unknown_with_missing_type() {
        // Edge case: JSON object without a type field
        let malformed_json = r#"{"foo": "bar", "baz": 42}"#;
        let content: InteractionContent = serde_json::from_str(malformed_json).unwrap();
        match content {
            InteractionContent::Unknown { type_name, data } => {
                assert_eq!(type_name, "<missing type>");
                assert_eq!(data["foo"], "bar");
                assert_eq!(data["baz"], 42);
            }
            _ => panic!("Expected Unknown variant"),
        }
    }

    #[test]
    fn test_deserialize_unknown_with_null_type() {
        // Edge case: JSON object with null type field
        let null_type_json = r#"{"type": null, "content": "test"}"#;
        let content: InteractionContent = serde_json::from_str(null_type_json).unwrap();
        match content {
            InteractionContent::Unknown { type_name, data } => {
                assert_eq!(type_name, "<missing type>");
                assert_eq!(data["content"], "test");
            }
            _ => panic!("Expected Unknown variant"),
        }
    }
}
