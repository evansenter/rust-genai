//! Agent configuration types for specialized agents.

use serde::de::{self, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// Thinking summaries configuration for agent output.
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
        // Note: GenerationConfig uses lowercase ("auto"/"none")
        // For AgentConfig, use to_agent_config_value() instead
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

impl ThinkingSummaries {
    /// Convert to the agent_config wire format (THINKING_SUMMARIES_*).
    ///
    /// AgentConfig uses a different wire format than GenerationConfig:
    /// - GenerationConfig: lowercase ("auto", "none")
    /// - AgentConfig: SCREAMING_CASE ("THINKING_SUMMARIES_AUTO", "THINKING_SUMMARIES_NONE")
    #[must_use]
    pub fn to_agent_config_value(&self) -> serde_json::Value {
        match self {
            ThinkingSummaries::Auto => {
                serde_json::Value::String("THINKING_SUMMARIES_AUTO".to_string())
            }
            ThinkingSummaries::None => {
                serde_json::Value::String("THINKING_SUMMARIES_NONE".to_string())
            }
            ThinkingSummaries::Unknown { summaries_type, .. } => {
                // For unknown values, preserve the original format
                serde_json::Value::String(summaries_type.clone())
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
            // Wire format is THINKING_SUMMARIES_*, but also accept lowercase for flexibility
            "THINKING_SUMMARIES_AUTO" | "auto" => Ok(ThinkingSummaries::Auto),
            "THINKING_SUMMARIES_NONE" | "none" => Ok(ThinkingSummaries::None),
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
/// This is a thin wrapper around JSON that provides full forward compatibility.
/// Use typed config structs like [`DeepResearchConfig`] for compile-time guidance,
/// or construct directly from JSON for unknown/future agent types.
///
/// # Usage
///
/// ## Typed configs (recommended for known agents)
/// ```
/// use rust_genai::{AgentConfig, DeepResearchConfig, ThinkingSummaries};
///
/// let config: AgentConfig = DeepResearchConfig::new()
///     .with_thinking_summaries(ThinkingSummaries::Auto)
///     .into();
/// ```
///
/// ## Raw JSON (for unknown/future agents)
/// ```
/// use rust_genai::AgentConfig;
///
/// let config = AgentConfig::from_value(serde_json::json!({
///     "type": "future-agent",
///     "newOption": true
/// }));
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentConfig(serde_json::Value);

impl AgentConfig {
    /// Create an agent config from a raw JSON value.
    ///
    /// Use this for unknown or future agent types that don't have typed config structs.
    #[must_use]
    pub fn from_value(value: serde_json::Value) -> Self {
        Self(value)
    }

    /// Access the underlying JSON value.
    #[must_use]
    pub fn as_value(&self) -> &serde_json::Value {
        &self.0
    }

    /// Get the agent config type (e.g., "deep-research", "dynamic").
    #[must_use]
    pub fn config_type(&self) -> Option<&str> {
        self.0.get("type").and_then(|v| v.as_str())
    }
}

/// Configuration for Deep Research agent.
///
/// Deep Research agent performs comprehensive research tasks
/// and can optionally include thinking summaries.
///
/// # Example
///
/// ```
/// use rust_genai::{AgentConfig, DeepResearchConfig, ThinkingSummaries};
///
/// let config: AgentConfig = DeepResearchConfig::new()
///     .with_thinking_summaries(ThinkingSummaries::Auto)
///     .into();
/// ```
#[derive(Clone, Debug, Default)]
pub struct DeepResearchConfig {
    thinking_summaries: Option<ThinkingSummaries>,
}

impl DeepResearchConfig {
    /// Create a new Deep Research configuration with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set thinking summaries mode.
    ///
    /// Controls whether the agent's reasoning process is summarized in output.
    #[must_use]
    pub fn with_thinking_summaries(mut self, summaries: ThinkingSummaries) -> Self {
        self.thinking_summaries = Some(summaries);
        self
    }
}

impl From<DeepResearchConfig> for AgentConfig {
    fn from(config: DeepResearchConfig) -> Self {
        let mut map = serde_json::Map::new();
        map.insert(
            "type".into(),
            serde_json::Value::String("deep-research".into()),
        );
        if let Some(ts) = config.thinking_summaries {
            // Use agent_config format (THINKING_SUMMARIES_*), not generation_config format (auto/none)
            map.insert("thinkingSummaries".into(), ts.to_agent_config_value());
        }
        AgentConfig(serde_json::Value::Object(map))
    }
}

/// Configuration for Dynamic agent.
///
/// Dynamic agents adapt their behavior based on the task.
/// Currently has no configurable options.
///
/// # Example
///
/// ```
/// use rust_genai::{AgentConfig, DynamicConfig};
///
/// let config: AgentConfig = DynamicConfig::new().into();
/// ```
#[derive(Clone, Debug, Default)]
pub struct DynamicConfig;

impl DynamicConfig {
    /// Create a new Dynamic agent configuration.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl From<DynamicConfig> for AgentConfig {
    fn from(_: DynamicConfig) -> Self {
        AgentConfig(serde_json::json!({"type": "dynamic"}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thinking_summaries_serialization() {
        // GenerationConfig wire format uses lowercase
        assert_eq!(
            serde_json::to_string(&ThinkingSummaries::Auto).unwrap(),
            "\"auto\""
        );

        assert_eq!(
            serde_json::to_string(&ThinkingSummaries::None).unwrap(),
            "\"none\""
        );
    }

    #[test]
    fn test_thinking_summaries_agent_config_format() {
        // AgentConfig uses THINKING_SUMMARIES_* format via to_agent_config_value()
        assert_eq!(
            ThinkingSummaries::Auto.to_agent_config_value(),
            serde_json::Value::String("THINKING_SUMMARIES_AUTO".to_string())
        );

        assert_eq!(
            ThinkingSummaries::None.to_agent_config_value(),
            serde_json::Value::String("THINKING_SUMMARIES_NONE".to_string())
        );
    }

    #[test]
    fn test_thinking_summaries_deserialization() {
        // Test wire format (THINKING_SUMMARIES_*)
        assert_eq!(
            serde_json::from_str::<ThinkingSummaries>("\"THINKING_SUMMARIES_AUTO\"").unwrap(),
            ThinkingSummaries::Auto
        );
        assert_eq!(
            serde_json::from_str::<ThinkingSummaries>("\"THINKING_SUMMARIES_NONE\"").unwrap(),
            ThinkingSummaries::None
        );

        // Also accept lowercase for flexibility
        assert_eq!(
            serde_json::from_str::<ThinkingSummaries>("\"auto\"").unwrap(),
            ThinkingSummaries::Auto
        );
        assert_eq!(
            serde_json::from_str::<ThinkingSummaries>("\"none\"").unwrap(),
            ThinkingSummaries::None
        );
    }

    #[test]
    fn test_thinking_summaries_unknown_roundtrip() {
        let unknown: ThinkingSummaries = serde_json::from_str("\"future_variant\"").unwrap();
        assert!(unknown.is_unknown());
        assert_eq!(unknown.unknown_summaries_type(), Some("future_variant"));

        // Roundtrip preserves the unknown value
        let json = serde_json::to_string(&unknown).unwrap();
        assert_eq!(json, "\"future_variant\"");
    }

    #[test]
    fn test_deep_research_config_serialization() {
        let config: AgentConfig = DeepResearchConfig::new()
            .with_thinking_summaries(ThinkingSummaries::Auto)
            .into();

        let json = serde_json::to_string(&config).expect("Serialization failed");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["type"], "deep-research");
        assert_eq!(value["thinkingSummaries"], "THINKING_SUMMARIES_AUTO");
    }

    #[test]
    fn test_deep_research_config_without_thinking_summaries() {
        let config: AgentConfig = DeepResearchConfig::new().into();

        let json = serde_json::to_string(&config).expect("Serialization failed");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["type"], "deep-research");
        assert!(value.get("thinkingSummaries").is_none());
    }

    #[test]
    fn test_dynamic_config_serialization() {
        let config: AgentConfig = DynamicConfig::new().into();

        let json = serde_json::to_string(&config).expect("Serialization failed");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["type"], "dynamic");
    }

    #[test]
    fn test_agent_config_from_raw_json() {
        let config = AgentConfig::from_value(serde_json::json!({
            "type": "custom-agent",
            "option1": true,
            "option2": "value"
        }));

        assert_eq!(config.config_type(), Some("custom-agent"));
        assert_eq!(config.as_value()["option1"], true);
    }

    #[test]
    fn test_agent_config_roundtrip() {
        let config: AgentConfig = DeepResearchConfig::new()
            .with_thinking_summaries(ThinkingSummaries::Auto)
            .into();

        let json = serde_json::to_string(&config).expect("Serialization failed");
        let parsed: AgentConfig = serde_json::from_str(&json).expect("Deserialization failed");

        assert_eq!(config, parsed);
    }
}
