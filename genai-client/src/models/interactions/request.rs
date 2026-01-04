//! Request types for creating interactions.

use serde::de::{self, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use super::agent_config::{AgentConfig, ThinkingSummaries};
use super::content::InteractionContent;
use crate::models::shared::{FunctionCallingMode, Tool};

/// Role in a conversation turn.
///
/// Indicates whether the content came from the user or the model.
///
/// This enum is marked `#[non_exhaustive]` for forward compatibility.
/// New roles may be added in future API versions.
///
/// # Evergreen Pattern
///
/// Unknown values from the API deserialize into the `Unknown` variant, preserving
/// the original data for debugging and roundtrip serialization.
///
/// # Example
///
/// ```
/// use genai_client::Role;
///
/// let role = Role::User;
/// assert!(matches!(role, Role::User));
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Role {
    /// Content from the user
    User,
    /// Content from the model
    Model,
    /// Unknown variant for forward compatibility (Evergreen pattern)
    Unknown {
        /// The unrecognized role type from the API
        role_type: String,
        /// The raw JSON value, preserved for debugging and roundtrip
        data: serde_json::Value,
    },
}

impl Role {
    /// Returns true if this is an unknown role.
    #[must_use]
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }

    /// Returns the role type name if this is an unknown role.
    #[must_use]
    pub fn unknown_role_type(&self) -> Option<&str> {
        match self {
            Self::Unknown { role_type, .. } => Some(role_type),
            _ => None,
        }
    }

    /// Returns the preserved data if this is an unknown role.
    #[must_use]
    pub fn unknown_data(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Unknown { data, .. } => Some(data),
            _ => None,
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Model => write!(f, "model"),
            Self::Unknown { role_type, .. } => write!(f, "{}", role_type),
        }
    }
}

impl Serialize for Role {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Role::User => serializer.serialize_str("user"),
            Role::Model => serializer.serialize_str("model"),
            Role::Unknown { role_type, .. } => serializer.serialize_str(role_type),
        }
    }
}

impl<'de> Deserialize<'de> for Role {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "user" => Ok(Role::User),
            "model" => Ok(Role::Model),
            other => {
                log::warn!(
                    "Encountered unknown Role '{}' - using Unknown variant (Evergreen)",
                    other
                );
                Ok(Role::Unknown {
                    role_type: other.to_string(),
                    data: serde_json::Value::String(other.to_string()),
                })
            }
        }
    }
}

/// Content for a conversation turn.
///
/// Can be simple text or an array of content parts for multimodal turns.
///
/// # Example
///
/// ```
/// use genai_client::TurnContent;
///
/// // Simple text
/// let content = TurnContent::Text("Hello!".to_string());
///
/// // From string reference
/// let content: TurnContent = "Hello!".into();
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TurnContent {
    /// Simple text content
    Text(String),
    /// Array of content parts (for multimodal content)
    Parts(Vec<InteractionContent>),
}

impl From<String> for TurnContent {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl From<&str> for TurnContent {
    fn from(s: &str) -> Self {
        Self::Text(s.to_string())
    }
}

impl From<Vec<InteractionContent>> for TurnContent {
    fn from(parts: Vec<InteractionContent>) -> Self {
        Self::Parts(parts)
    }
}

impl TurnContent {
    /// Returns the text content if this is a `Text` variant.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text(t) => Some(t),
            Self::Parts(_) => None,
        }
    }

    /// Returns the content parts if this is a `Parts` variant.
    #[must_use]
    pub fn parts(&self) -> Option<&[InteractionContent]> {
        match self {
            Self::Parts(p) => Some(p),
            Self::Text(_) => None,
        }
    }

    /// Returns `true` if this is text content.
    #[must_use]
    pub const fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    /// Returns `true` if this is parts content.
    #[must_use]
    pub const fn is_parts(&self) -> bool {
        matches!(self, Self::Parts(_))
    }
}

/// A single turn in a multi-turn conversation.
///
/// Represents one message in a conversation, containing the role (who sent it)
/// and the content of the message.
///
/// # Example
///
/// ```
/// use genai_client::{Turn, Role, TurnContent};
///
/// // Create a user turn with text
/// let user_turn = Turn::user("What is 2+2?");
///
/// // Create a model turn with text
/// let model_turn = Turn::model("2+2 equals 4.");
///
/// // Create a turn with explicit role and content
/// let turn = Turn::new(Role::User, "Hello!");
///
/// // Access via getters
/// assert!(matches!(turn.role(), &Role::User));
/// assert_eq!(turn.content().text(), Some("Hello!"));
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Turn {
    role: Role,
    content: TurnContent,
}

impl Turn {
    /// Creates a new turn with the given role and content.
    pub fn new(role: Role, content: impl Into<TurnContent>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    /// Returns a reference to the role of this turn.
    #[must_use]
    pub fn role(&self) -> &Role {
        &self.role
    }

    /// Returns a reference to the content of this turn.
    #[must_use]
    pub fn content(&self) -> &TurnContent {
        &self.content
    }

    /// Creates a user turn with the given content.
    ///
    /// # Example
    ///
    /// ```
    /// use genai_client::Turn;
    ///
    /// let turn = Turn::user("What is the capital of France?");
    /// ```
    pub fn user(content: impl Into<TurnContent>) -> Self {
        Self::new(Role::User, content)
    }

    /// Creates a model turn with the given content.
    ///
    /// # Example
    ///
    /// ```
    /// use genai_client::Turn;
    ///
    /// let turn = Turn::model("The capital of France is Paris.");
    /// ```
    pub fn model(content: impl Into<TurnContent>) -> Self {
        Self::new(Role::Model, content)
    }

    /// Returns `true` if this is a user turn.
    #[must_use]
    pub fn is_user(&self) -> bool {
        *self.role() == Role::User
    }

    /// Returns `true` if this is a model turn.
    #[must_use]
    pub fn is_model(&self) -> bool {
        *self.role() == Role::Model
    }

    /// Returns the text content if this turn contains text.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        self.content().text()
    }
}

/// Input for an interaction - can be a simple string, array of content, or turns.
///
/// This enum is marked `#[non_exhaustive]` for forward compatibility.
/// New input types may be added in future versions.
///
/// # Variants
///
/// - `Text`: Simple text input for single-turn conversations
/// - `Content`: Array of content objects for multimodal input
/// - `Turns`: Array of turns for explicit multi-turn conversations
///
/// # Example
///
/// ```
/// use genai_client::{InteractionInput, Turn};
///
/// // Simple text
/// let input = InteractionInput::Text("Hello!".to_string());
///
/// // Multi-turn conversation
/// let turns = vec![
///     Turn::user("What is 2+2?"),
///     Turn::model("2+2 equals 4."),
///     Turn::user("And what's that times 3?"),
/// ];
/// let input = InteractionInput::Turns(turns);
/// ```
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(untagged)]
#[non_exhaustive]
pub enum InteractionInput {
    /// Simple text input
    Text(String),
    /// Array of content objects
    Content(Vec<InteractionContent>),
    /// Array of turns for multi-turn conversations
    Turns(Vec<Turn>),
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
