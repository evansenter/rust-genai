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
                // Flatten the data fields into the map
                if let serde_json::Value::Object(obj) = data {
                    for (key, value) in obj {
                        if key != "type" {
                            // Don't duplicate the type field
                            map.serialize_entry(key, value)?;
                        }
                    }
                }
                map.end()
            }
        }
    }
}

impl InteractionContent {
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

/// Token usage information
#[derive(Clone, Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<i32>,
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
        use std::collections::HashSet;

        let mut summary = ContentSummary::default();
        let mut unknown_types_set = HashSet::new();

        for content in &self.outputs {
            match content {
                InteractionContent::Text { .. } => summary.text_count += 1,
                InteractionContent::Thought { .. } => summary.thought_count += 1,
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

        summary.unknown_types = unknown_types_set.into_iter().collect();
        summary.unknown_types.sort();
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

/// Delta content for streaming events.
///
/// Contains incremental content updates during streaming.
/// Used with "content.delta" event types.
///
/// # Forward Compatibility
///
/// Like [`InteractionContent`], this enum includes an `Unknown` variant for
/// delta types that this library doesn't recognize. This commonly occurs
/// with new features like function call streaming.
///
/// # Example
///
/// ```no_run
/// # use genai_client::models::interactions::StreamDelta;
/// # let delta: StreamDelta = todo!();
/// match &delta {
///     StreamDelta::Text { text } => print!("{}", text),
///     StreamDelta::Thought { text } => print!("[thinking: {}]", text),
///     StreamDelta::Unknown { type_name, .. } => {
///         log::debug!("Skipping unknown delta type: {}", type_name);
///     }
///     _ => {}
/// }
/// ```
#[derive(Clone, Debug)]
pub enum StreamDelta {
    /// Text content delta
    Text {
        /// The incremental text content
        text: String,
    },
    /// Thought content delta (internal reasoning)
    Thought {
        /// The incremental thought content
        text: String,
    },
    /// Thought signature (cryptographic signature for thought verification)
    ThoughtSignature {
        /// The signature value
        signature: String,
    },
    /// Unknown delta type for forward compatibility.
    ///
    /// Captures streaming delta types that the library doesn't recognize yet,
    /// such as `function_call` deltas for streaming function calling.
    Unknown {
        /// The unrecognized type name from the API
        type_name: String,
        /// The full JSON data for this delta
        data: serde_json::Value,
    },
}

// Custom Deserialize for StreamDelta to handle unknown delta types gracefully.
impl<'de> Deserialize<'de> for StreamDelta {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[cfg(feature = "strict-unknown")]
        use serde::de::Error as _;

        let value = serde_json::Value::deserialize(deserializer)?;

        #[derive(Deserialize)]
        #[serde(tag = "type", rename_all = "snake_case")]
        enum KnownDelta {
            Text {
                #[serde(default)]
                text: String,
            },
            Thought {
                #[serde(default)]
                text: String,
            },
            ThoughtSignature {
                #[serde(default)]
                signature: String,
            },
        }

        match serde_json::from_value::<KnownDelta>(value.clone()) {
            Ok(known) => Ok(match known {
                KnownDelta::Text { text } => StreamDelta::Text { text },
                KnownDelta::Thought { text } => StreamDelta::Thought { text },
                KnownDelta::ThoughtSignature { signature } => {
                    StreamDelta::ThoughtSignature { signature }
                }
            }),
            Err(_) => {
                let type_name = value
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("<missing type>")
                    .to_string();

                log::warn!(
                    "Encountered unknown StreamDelta type '{}'. \
                     This may indicate a new streaming feature (e.g., function call streaming) \
                     not yet supported by this library.",
                    type_name
                );

                #[cfg(feature = "strict-unknown")]
                {
                    Err(D::Error::custom(format!(
                        "Unknown StreamDelta type '{}'. \
                         Strict mode is enabled via the 'strict-unknown' feature flag.",
                        type_name
                    )))
                }

                #[cfg(not(feature = "strict-unknown"))]
                {
                    Ok(StreamDelta::Unknown {
                        type_name,
                        data: value,
                    })
                }
            }
        }
    }
}

impl StreamDelta {
    /// Extract the text content from this delta, if any.
    ///
    /// Returns `Some` only for non-empty text deltas. Returns `None` for
    /// thought deltas, signatures, unknown types, and empty text.
    pub fn text(&self) -> Option<&str> {
        match self {
            StreamDelta::Text { text } if !text.is_empty() => Some(text),
            _ => None,
        }
    }

    /// Check if this is a text delta.
    pub const fn is_text(&self) -> bool {
        matches!(self, StreamDelta::Text { .. })
    }

    /// Check if this is a thought delta.
    pub const fn is_thought(&self) -> bool {
        matches!(self, StreamDelta::Thought { .. })
    }

    /// Check if this is an unknown delta type.
    ///
    /// Returns `true` for delta types that this library doesn't recognize.
    /// Use this to detect streaming features that aren't yet supported.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use genai_client::models::interactions::StreamDelta;
    /// # let delta: StreamDelta = todo!();
    /// if delta.is_unknown() {
    ///     if let StreamDelta::Unknown { type_name, .. } = &delta {
    ///         log::debug!("Skipping unknown delta: {}", type_name);
    ///     }
    /// }
    /// ```
    pub const fn is_unknown(&self) -> bool {
        matches!(self, StreamDelta::Unknown { .. })
    }

    /// Returns the type name of this delta.
    ///
    /// Useful for logging or debugging, especially with unknown types.
    pub fn type_name(&self) -> &str {
        match self {
            StreamDelta::Text { .. } => "text",
            StreamDelta::Thought { .. } => "thought",
            StreamDelta::ThoughtSignature { .. } => "thought_signature",
            StreamDelta::Unknown { type_name, .. } => type_name,
        }
    }
}

/// A chunk from the streaming API
///
/// During streaming, the API sends different types of events:
/// - `Delta`: Incremental content updates (text or thought)
/// - `Complete`: The final complete interaction response
#[derive(Clone, Debug)]
pub enum StreamChunk {
    /// Incremental content update
    Delta(StreamDelta),
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
    pub delta: Option<StreamDelta>,

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
                "promptTokens": 5,
                "candidatesTokens": 10,
                "totalTokens": 15
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
        assert_eq!(response.usage.unwrap().total_tokens, Some(15));
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
    fn test_deserialize_stream_delta_text() {
        let delta_json = r#"{"type": "text", "text": "Hello world"}"#;
        let delta: StreamDelta = serde_json::from_str(delta_json).expect("Deserialization failed");

        match &delta {
            StreamDelta::Text { text } => {
                assert_eq!(text, "Hello world");
            }
            _ => panic!("Expected Text delta"),
        }

        assert!(delta.is_text());
        assert!(!delta.is_thought());
        assert_eq!(delta.text(), Some("Hello world"));
    }

    #[test]
    fn test_deserialize_stream_delta_thought() {
        let delta_json = r#"{"type": "thought", "text": "I'm thinking..."}"#;
        let delta: StreamDelta = serde_json::from_str(delta_json).expect("Deserialization failed");

        match &delta {
            StreamDelta::Thought { text } => {
                assert_eq!(text, "I'm thinking...");
            }
            _ => panic!("Expected Thought delta"),
        }

        assert!(!delta.is_text());
        assert!(delta.is_thought());
        // text() returns None for thoughts
        assert_eq!(delta.text(), None);
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
    fn test_stream_delta_empty_text_returns_none() {
        let delta = StreamDelta::Text {
            text: String::new(),
        };
        assert_eq!(delta.text(), None);
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
    fn test_deserialize_unknown_stream_delta() {
        // Simulate a function_call delta that's not yet supported
        let unknown_json =
            r#"{"type": "function_call", "name": "get_weather", "arguments": {"city": "Paris"}}"#;

        let delta: StreamDelta =
            serde_json::from_str(unknown_json).expect("Should deserialize as Unknown");

        assert!(delta.is_unknown());
        assert_eq!(delta.type_name(), "function_call");

        match &delta {
            StreamDelta::Unknown { type_name, data } => {
                assert_eq!(type_name, "function_call");
                assert_eq!(data["name"], "get_weather");
                assert_eq!(data["arguments"]["city"], "Paris");
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

        let delta_json = r#"{"type": "text", "text": "Delta text"}"#;
        let delta: StreamDelta = serde_json::from_str(delta_json).unwrap();
        assert!(delta.is_text());
        assert!(!delta.is_unknown());
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
    fn test_stream_delta_type_name() {
        let text = StreamDelta::Text {
            text: "hello".to_string(),
        };
        assert_eq!(text.type_name(), "text");

        let thought = StreamDelta::Thought {
            text: "thinking".to_string(),
        };
        assert_eq!(thought.type_name(), "thought");

        let unknown = StreamDelta::Unknown {
            type_name: "function_call".to_string(),
            data: serde_json::json!({}),
        };
        assert_eq!(unknown.type_name(), "function_call");
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
}
