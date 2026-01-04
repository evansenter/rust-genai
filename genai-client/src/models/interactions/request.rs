//! Request types for creating interactions.

use serde::de::{self, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use super::content::InteractionContent;
use crate::models::shared::{FunctionCallingMode, Tool};

/// Input for an interaction - can be a simple string or array of content.
///
/// This enum is marked `#[non_exhaustive]` for forward compatibility.
/// New input types may be added in future versions.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(untagged)]
#[non_exhaustive]
pub enum InteractionInput {
    /// Simple text input
    Text(String),
    /// Array of content objects
    Content(Vec<InteractionContent>),
}

/// Thinking level for chain-of-thought reasoning.
///
/// Controls the depth of reasoning the model performs before generating a response.
/// Higher levels produce more detailed reasoning but consume more tokens.
///
/// This enum is marked `#[non_exhaustive]` for forward compatibility.
/// New thinking levels may be added in future versions.
///
/// # Evergreen Pattern
///
/// Unknown values from the API deserialize into the `Unknown` variant, preserving
/// the original data for debugging and roundtrip serialization.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ThinkingLevel {
    /// Minimal reasoning, fastest responses
    Minimal,
    /// Light reasoning for simple problems
    Low,
    /// Balanced reasoning for moderate complexity
    Medium,
    /// Extensive reasoning for complex problems
    High,
    /// Unknown variant for forward compatibility (Evergreen pattern)
    Unknown {
        /// The unrecognized level type from the API
        level_type: String,
        /// The full JSON data, preserved for debugging and roundtrip serialization
        data: serde_json::Value,
    },
}

impl ThinkingLevel {
    /// Returns true if this is an unknown thinking level.
    #[must_use]
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }

    /// Returns the level type name if this is an unknown thinking level.
    #[must_use]
    pub fn unknown_level_type(&self) -> Option<&str> {
        match self {
            Self::Unknown { level_type, .. } => Some(level_type),
            _ => None,
        }
    }

    /// Returns the preserved data if this is an unknown thinking level.
    #[must_use]
    pub fn unknown_data(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Unknown { data, .. } => Some(data),
            _ => None,
        }
    }
}

impl Serialize for ThinkingLevel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ThinkingLevel::Minimal => serializer.serialize_str("minimal"),
            ThinkingLevel::Low => serializer.serialize_str("low"),
            ThinkingLevel::Medium => serializer.serialize_str("medium"),
            ThinkingLevel::High => serializer.serialize_str("high"),
            ThinkingLevel::Unknown { level_type, data } => {
                // If data is a simple string, serialize just the level_type
                if data.is_string() || data.is_null() {
                    serializer.serialize_str(level_type)
                } else {
                    // For complex data, serialize as an object
                    let mut map = serializer.serialize_map(None)?;
                    map.serialize_entry("level", level_type)?;
                    map.serialize_entry("data", data)?;
                    map.end()
                }
            }
        }
    }
}

impl<'de> Deserialize<'de> for ThinkingLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ThinkingLevelVisitor)
    }
}

struct ThinkingLevelVisitor;

impl<'de> Visitor<'de> for ThinkingLevelVisitor {
    type Value = ThinkingLevel;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a thinking level string or object")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match value {
            "minimal" => Ok(ThinkingLevel::Minimal),
            "low" => Ok(ThinkingLevel::Low),
            "medium" => Ok(ThinkingLevel::Medium),
            "high" => Ok(ThinkingLevel::High),
            other => {
                log::warn!(
                    "Encountered unknown ThinkingLevel '{}' - using Unknown variant (Evergreen)",
                    other
                );
                Ok(ThinkingLevel::Unknown {
                    level_type: other.to_string(),
                    data: serde_json::Value::String(other.to_string()),
                })
            }
        }
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        // For object-based thinking levels (future API compatibility)
        let value: serde_json::Value =
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))?;
        let level_type = value
            .get("level")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        log::warn!(
            "Encountered unknown ThinkingLevel object '{}' - using Unknown variant (Evergreen)",
            level_type
        );
        Ok(ThinkingLevel::Unknown {
            level_type,
            data: value,
        })
    }
}

/// Controls whether thinking summaries are included in output.
///
/// When using thinking mode (via `with_thinking_level`), you can control
/// whether the model's reasoning process is summarized in the output.
///
/// This enum is marked `#[non_exhaustive]` for forward compatibility.
/// New summary modes may be added in future versions.
///
/// # Evergreen Pattern
///
/// Unknown values from the API deserialize into the `Unknown` variant, preserving
/// the original data for debugging and roundtrip serialization.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ThinkingSummaries {
    /// Automatically include thinking summaries (default when thinking is enabled)
    Auto,
    /// Do not include thinking summaries
    None,
    /// Unknown variant for forward compatibility (Evergreen pattern)
    Unknown {
        /// The unrecognized summaries type from the API
        summaries_type: String,
        /// The full JSON data, preserved for debugging and roundtrip serialization
        data: serde_json::Value,
    },
}

impl ThinkingSummaries {
    /// Returns true if this is an unknown thinking summaries value.
    #[must_use]
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }

    /// Returns the summaries type name if this is an unknown value.
    #[must_use]
    pub fn unknown_summaries_type(&self) -> Option<&str> {
        match self {
            Self::Unknown { summaries_type, .. } => Some(summaries_type),
            _ => None,
        }
    }

    /// Returns the preserved data if this is an unknown value.
    #[must_use]
    pub fn unknown_data(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Unknown { data, .. } => Some(data),
            _ => None,
        }
    }
}

impl Serialize for ThinkingSummaries {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ThinkingSummaries::Auto => serializer.serialize_str("auto"),
            ThinkingSummaries::None => serializer.serialize_str("none"),
            ThinkingSummaries::Unknown {
                summaries_type,
                data,
            } => {
                // If data is a simple string, serialize just the summaries_type
                if data.is_string() || data.is_null() {
                    serializer.serialize_str(summaries_type)
                } else {
                    // For complex data, serialize as an object
                    let mut map = serializer.serialize_map(None)?;
                    map.serialize_entry("summaries", summaries_type)?;
                    map.serialize_entry("data", data)?;
                    map.end()
                }
            }
        }
    }
}

impl<'de> Deserialize<'de> for ThinkingSummaries {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ThinkingSummariesVisitor)
    }
}

struct ThinkingSummariesVisitor;

impl<'de> Visitor<'de> for ThinkingSummariesVisitor {
    type Value = ThinkingSummaries;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a thinking summaries string or object")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match value {
            "auto" => Ok(ThinkingSummaries::Auto),
            "none" => Ok(ThinkingSummaries::None),
            other => {
                log::warn!(
                    "Encountered unknown ThinkingSummaries '{}' - using Unknown variant (Evergreen)",
                    other
                );
                Ok(ThinkingSummaries::Unknown {
                    summaries_type: other.to_string(),
                    data: serde_json::Value::String(other.to_string()),
                })
            }
        }
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        // For object-based thinking summaries (future API compatibility)
        let value: serde_json::Value =
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))?;
        let summaries_type = value
            .get("summaries")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        log::warn!(
            "Encountered unknown ThinkingSummaries object '{}' - using Unknown variant (Evergreen)",
            summaries_type
        );
        Ok(ThinkingSummaries::Unknown {
            summaries_type,
            data: value,
        })
    }
}

/// Agent-specific configuration for specialized agents.
///
/// When using specialized agents like Deep Research or Dynamic agents,
/// this configuration controls agent-specific behavior.
///
/// # Forward Compatibility (Evergreen Philosophy)
///
/// This enum is marked `#[non_exhaustive]`, which means:
/// - Match statements must include a wildcard arm (`_ => ...`)
/// - New variants may be added in minor version updates without breaking your code
///
/// When the API returns an agent config type that this library doesn't recognize,
/// it will be captured as `AgentConfig::Unknown` rather than causing a
/// deserialization error.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum AgentConfig {
    /// Configuration for Deep Research agent.
    ///
    /// Deep Research agent performs comprehensive research tasks
    /// and can optionally include thinking summaries.
    DeepResearch {
        /// Controls whether thinking summaries are included in output
        thinking_summaries: Option<ThinkingSummaries>,
    },
    /// Configuration for Dynamic agent.
    ///
    /// Dynamic agents adapt their behavior based on the task.
    Dynamic,
    /// Unknown agent configuration for forward compatibility.
    ///
    /// This variant captures agent config types that the library doesn't recognize yet.
    Unknown {
        /// The unrecognized config type name from the API
        config_type: String,
        /// The full JSON data for this config, preserved for debugging
        data: serde_json::Value,
    },
}

impl AgentConfig {
    /// Check if this is an unknown agent config type.
    #[must_use]
    pub const fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }

    /// Returns the config type name if this is an unknown config.
    ///
    /// Returns `None` for known config types.
    #[must_use]
    pub fn unknown_config_type(&self) -> Option<&str> {
        match self {
            Self::Unknown { config_type, .. } => Some(config_type),
            _ => None,
        }
    }

    /// Returns the raw JSON data if this is an unknown config.
    ///
    /// Returns `None` for known config types.
    #[must_use]
    pub fn unknown_data(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Unknown { data, .. } => Some(data),
            _ => None,
        }
    }
}

impl Serialize for AgentConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;

        match self {
            Self::DeepResearch { thinking_summaries } => {
                map.serialize_entry("type", "deep-research")?;
                if let Some(ts) = thinking_summaries {
                    map.serialize_entry("thinkingSummaries", ts)?;
                }
            }
            Self::Dynamic => {
                map.serialize_entry("type", "dynamic")?;
            }
            Self::Unknown { config_type, data } => {
                map.serialize_entry("type", config_type)?;
                // Flatten the data fields into the map if it's an object
                if let serde_json::Value::Object(obj) = data {
                    for (key, value) in obj {
                        if key != "type" {
                            map.serialize_entry(key, value)?;
                        }
                    }
                } else if !data.is_null() {
                    map.serialize_entry("data", data)?;
                }
            }
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for AgentConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // First, deserialize into a raw JSON value
        let value = serde_json::Value::deserialize(deserializer)?;

        // Helper enum for deserializing known types
        #[derive(Deserialize)]
        #[serde(tag = "type", rename_all = "kebab-case")]
        enum KnownAgentConfig {
            #[serde(rename = "deep-research")]
            DeepResearch {
                #[serde(rename = "thinkingSummaries")]
                thinking_summaries: Option<ThinkingSummaries>,
            },
            #[serde(rename = "dynamic")]
            Dynamic,
        }

        // Try to deserialize as a known type
        match serde_json::from_value::<KnownAgentConfig>(value.clone()) {
            Ok(known) => Ok(match known {
                KnownAgentConfig::DeepResearch { thinking_summaries } => {
                    AgentConfig::DeepResearch { thinking_summaries }
                }
                KnownAgentConfig::Dynamic => AgentConfig::Dynamic,
            }),
            Err(parse_error) => {
                // Unknown type - extract type name and preserve data
                let config_type = value
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("<missing type>")
                    .to_string();

                log::warn!(
                    "Encountered unknown AgentConfig type '{}'. \
                     Parse error: {}. \
                     This may indicate a new API feature or a malformed response. \
                     The config will be preserved in the Unknown variant.",
                    config_type,
                    parse_error
                );

                Ok(AgentConfig::Unknown {
                    config_type,
                    data: value,
                })
            }
        }
    }
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
    /// Thinking level for chain-of-thought reasoning
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_level: Option<ThinkingLevel>,
    /// Seed for deterministic output generation.
    ///
    /// Using the same seed with identical inputs will produce the same output,
    /// useful for testing and debugging.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// Stop sequences that halt generation.
    ///
    /// When the model generates any of these sequences, generation stops immediately.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Controls whether thinking summaries are included in output.
    ///
    /// Use with `thinking_level` to control reasoning output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_summaries: Option<ThinkingSummaries>,
    /// Controls function calling behavior.
    ///
    /// This field determines how the model uses declared tools/functions:
    /// - `Auto` (default): Model decides whether to call functions
    /// - `Any`: Model must call a function
    /// - `None`: Function calling is disabled
    /// - `Validated`: Ensures schema adherence for both function calls and natural language
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<FunctionCallingMode>,
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

    /// Agent-specific configuration (e.g., Deep Research thinking summaries)
    #[serde(rename = "agent_config", skip_serializing_if = "Option::is_none")]
    pub agent_config: Option<AgentConfig>,

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

    /// Response MIME type for structured output.
    ///
    /// Required when using `response_format` with a JSON schema.
    /// Typically "application/json".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_mime_type: Option<String>,

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
