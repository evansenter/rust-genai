// Shared types used by the Interactions API

use serde::{Deserialize, Serialize};

/// Represents a tool that can be used by the model (Interactions API format).
///
/// Tools in the Interactions API use a flat structure with the tool type and details
/// at the top level, rather than nested in arrays.
///
/// # Forward Compatibility (Evergreen Philosophy)
///
/// This enum is marked `#[non_exhaustive]`, which means:
/// - Match statements must include a wildcard arm (`_ => ...`)
/// - New variants may be added in minor version updates without breaking your code
///
/// When the API returns a tool type that this library doesn't recognize, it will be
/// captured as `Tool::Unknown` rather than causing a deserialization error.
/// This follows the [Evergreen spec](https://github.com/google-deepmind/evergreen-spec)
/// philosophy of graceful degradation.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Tool {
    /// A custom function that the model can call
    Function {
        name: String,
        description: String,
        parameters: FunctionParameters,
    },
    /// Built-in Google Search tool
    GoogleSearch,
    /// Built-in code execution tool
    CodeExecution,
    /// Built-in URL context tool
    UrlContext,
    /// Model Context Protocol (MCP) server
    McpServer { name: String, url: String },
    /// Unknown tool type for forward compatibility.
    ///
    /// This variant captures tool types that the library doesn't recognize yet.
    /// This can happen when Google adds new built-in tools before this library
    /// is updated to support them.
    ///
    /// The `tool_type` field contains the unrecognized type string from the API,
    /// and `data` contains the full JSON object for inspection or debugging.
    Unknown {
        /// The unrecognized tool type name from the API
        tool_type: String,
        /// The full JSON data for this tool, preserved for debugging
        data: serde_json::Value,
    },
}

// Custom Serialize implementation for Tool.
// This handles the Unknown variant by merging tool_type into the data.
impl Serialize for Tool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        match self {
            Self::Function {
                name,
                description,
                parameters,
            } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "function")?;
                map.serialize_entry("name", name)?;
                map.serialize_entry("description", description)?;
                map.serialize_entry("parameters", parameters)?;
                map.end()
            }
            Self::GoogleSearch => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "google_search")?;
                map.end()
            }
            Self::CodeExecution => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "code_execution")?;
                map.end()
            }
            Self::UrlContext => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "url_context")?;
                map.end()
            }
            Self::McpServer { name, url } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "mcp_server")?;
                map.serialize_entry("name", name)?;
                map.serialize_entry("url", url)?;
                map.end()
            }
            Self::Unknown { tool_type, data } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", tool_type)?;
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
                map.end()
            }
        }
    }
}

// Custom Deserialize implementation to handle unknown tool types gracefully.
impl<'de> Deserialize<'de> for Tool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // First, deserialize into a raw JSON value
        let value = serde_json::Value::deserialize(deserializer)?;

        // Helper enum for deserializing known types
        // Note: variant names must match the serialized "type" field values exactly
        #[derive(Deserialize)]
        #[serde(tag = "type")]
        enum KnownTool {
            #[serde(rename = "function")]
            Function {
                name: String,
                description: String,
                parameters: FunctionParameters,
            },
            #[serde(rename = "google_search")]
            GoogleSearch,
            #[serde(rename = "code_execution")]
            CodeExecution,
            #[serde(rename = "url_context")]
            UrlContext,
            #[serde(rename = "mcp_server")]
            McpServer { name: String, url: String },
        }

        // Try to deserialize as a known type
        match serde_json::from_value::<KnownTool>(value.clone()) {
            Ok(known) => Ok(match known {
                KnownTool::Function {
                    name,
                    description,
                    parameters,
                } => Tool::Function {
                    name,
                    description,
                    parameters,
                },
                KnownTool::GoogleSearch => Tool::GoogleSearch,
                KnownTool::CodeExecution => Tool::CodeExecution,
                KnownTool::UrlContext => Tool::UrlContext,
                KnownTool::McpServer { name, url } => Tool::McpServer { name, url },
            }),
            Err(parse_error) => {
                // Unknown type - extract type name and preserve data
                let tool_type = value
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("<missing type>")
                    .to_string();

                // Log the actual parse error for debugging - this helps distinguish
                // between truly unknown types and malformed known types
                log::warn!(
                    "Encountered unknown Tool type '{}'. \
                     Parse error: {}. \
                     This may indicate a new API feature or a malformed response. \
                     The tool will be preserved in the Unknown variant.",
                    tool_type,
                    parse_error
                );

                Ok(Tool::Unknown {
                    tool_type,
                    data: value,
                })
            }
        }
    }
}

/// Represents a function that can be called by the model.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FunctionDeclaration {
    name: String,
    description: String,
    parameters: FunctionParameters,
}

/// Represents the parameters schema for a function.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FunctionParameters {
    #[serde(rename = "type")]
    type_: String,
    properties: serde_json::Value,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    required: Vec<String>,
}

impl FunctionDeclaration {
    /// Creates a new FunctionDeclaration with the given fields.
    ///
    /// This is primarily intended for internal use by the macro system.
    /// For manual construction, prefer using `FunctionDeclaration::builder()`.
    #[doc(hidden)]
    pub fn new(name: String, description: String, parameters: FunctionParameters) -> Self {
        Self {
            name,
            description,
            parameters,
        }
    }

    /// Creates a builder for ergonomic FunctionDeclaration construction
    pub fn builder(name: impl Into<String>) -> FunctionDeclarationBuilder {
        FunctionDeclarationBuilder::new(name)
    }

    /// Returns the function name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the function description
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns a reference to the function parameters
    pub fn parameters(&self) -> &FunctionParameters {
        &self.parameters
    }

    /// Converts this FunctionDeclaration into a Tool for API requests
    pub fn into_tool(self) -> Tool {
        Tool::Function {
            name: self.name,
            description: self.description,
            parameters: self.parameters,
        }
    }
}

impl FunctionParameters {
    /// Creates a new FunctionParameters with the given fields.
    ///
    /// This is primarily intended for internal use by the macro system.
    /// For manual construction, prefer using `FunctionDeclaration::builder()`.
    #[doc(hidden)]
    pub fn new(type_: String, properties: serde_json::Value, required: Vec<String>) -> Self {
        Self {
            type_,
            properties,
            required,
        }
    }

    /// Returns the parameter type (typically "object")
    pub fn type_(&self) -> &str {
        &self.type_
    }

    /// Returns the properties schema
    pub fn properties(&self) -> &serde_json::Value {
        &self.properties
    }

    /// Returns the list of required parameter names
    pub fn required(&self) -> &[String] {
        &self.required
    }
}

/// Builder for ergonomic FunctionDeclaration creation
#[derive(Debug)]
pub struct FunctionDeclarationBuilder {
    name: String,
    description: String,
    properties: serde_json::Value,
    required: Vec<String>,
}

impl FunctionDeclarationBuilder {
    /// Creates a new builder with the given function name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            properties: serde_json::Value::Object(serde_json::Map::new()),
            required: Vec::new(),
        }
    }

    /// Sets the function description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Adds a parameter to the function schema
    pub fn parameter(mut self, name: &str, schema: serde_json::Value) -> Self {
        if let serde_json::Value::Object(ref mut map) = self.properties {
            map.insert(name.to_string(), schema);
        }
        self
    }

    /// Sets the list of required parameter names
    pub fn required(mut self, required: Vec<String>) -> Self {
        self.required = required;
        self
    }

    /// Builds the FunctionDeclaration
    ///
    /// # Validation
    ///
    /// This method performs validation and logs warnings for:
    /// - Empty or whitespace-only function names
    /// - Required parameters that don't exist in the properties schema
    ///
    /// These conditions may cause API errors but are allowed by the builder
    /// for backwards compatibility.
    pub fn build(self) -> FunctionDeclaration {
        // Validate function name
        if self.name.trim().is_empty() {
            log::warn!(
                "FunctionDeclaration built with empty or whitespace-only name. \
                This will likely be rejected by the API."
            );
        }

        // Validate required parameters exist in properties
        if let serde_json::Value::Object(ref props) = self.properties {
            for req in &self.required {
                if !props.contains_key(req) {
                    log::warn!(
                        "FunctionDeclaration '{}' requires parameter '{}' which is not defined in properties. \
                        This will likely cause API errors.",
                        self.name,
                        req
                    );
                }
            }
        }

        FunctionDeclaration {
            name: self.name,
            description: self.description,
            parameters: FunctionParameters {
                type_: "object".to_string(),
                properties: self.properties,
                required: self.required,
            },
        }
    }
}

/// Represents tool configuration for function calling.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ToolConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_calling_config: Option<FunctionCallingConfig>,
}

/// Configuration for how the model should use function calling.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FunctionCallingConfig {
    #[serde(rename = "mode")]
    pub mode: FunctionCallingMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_function_names: Option<Vec<String>>,
}

/// Modes for function calling behavior.
///
/// This enum is marked `#[non_exhaustive]` for forward compatibility.
/// New modes may be added in future versions.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum FunctionCallingMode {
    Auto,
    Any,
    None,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_serialize_function_declaration() {
        let function = FunctionDeclaration::builder("get_weather")
            .description("Get the current weather in a given location")
            .parameter(
                "location",
                serde_json::json!({
                    "type": "string",
                    "description": "The city and state, e.g. San Francisco, CA"
                }),
            )
            .required(vec!["location".to_string()])
            .build();

        let json_string = serde_json::to_string(&function).expect("Serialization failed");
        let parsed: FunctionDeclaration =
            serde_json::from_str(&json_string).expect("Deserialization failed");

        assert_eq!(parsed.name(), "get_weather");
        assert_eq!(
            parsed.description(),
            "Get the current weather in a given location"
        );
    }

    #[test]
    fn test_function_calling_mode_serialization() {
        let mode = FunctionCallingMode::Auto;
        let json = serde_json::to_string(&mode).expect("Serialization failed");
        assert_eq!(json, "\"AUTO\"");

        let parsed: FunctionCallingMode =
            serde_json::from_str(&json).expect("Deserialization failed");
        assert!(matches!(parsed, FunctionCallingMode::Auto));
    }

    #[test]
    fn test_tool_google_search_roundtrip() {
        let tool = Tool::GoogleSearch;
        let json = serde_json::to_string(&tool).expect("Serialization failed");
        assert!(json.contains("\"type\":\"google_search\""));

        let parsed: Tool = serde_json::from_str(&json).expect("Deserialization failed");
        assert!(matches!(parsed, Tool::GoogleSearch));
    }

    #[test]
    fn test_tool_function_roundtrip() {
        let tool = Tool::Function {
            name: "get_weather".to_string(),
            description: "Get weather".to_string(),
            parameters: FunctionParameters::new(
                "object".to_string(),
                serde_json::json!({}),
                vec![],
            ),
        };
        let json = serde_json::to_string(&tool).expect("Serialization failed");
        let parsed: Tool = serde_json::from_str(&json).expect("Deserialization failed");

        match parsed {
            Tool::Function { name, .. } => assert_eq!(name, "get_weather"),
            other => panic!("Expected Function variant, got {:?}", other),
        }
    }

    #[test]
    fn test_tool_unknown_deserialization() {
        // Simulate an unknown tool type from the API
        let json = r#"{"type": "future_tool", "some_field": "value", "number": 42}"#;
        let parsed: Tool = serde_json::from_str(json).expect("Deserialization failed");

        match parsed {
            Tool::Unknown { tool_type, data } => {
                assert_eq!(tool_type, "future_tool");
                assert_eq!(data.get("some_field").unwrap(), "value");
                assert_eq!(data.get("number").unwrap(), 42);
            }
            _ => panic!("Expected Unknown variant"),
        }
    }

    #[test]
    fn test_tool_unknown_roundtrip() {
        let tool = Tool::Unknown {
            tool_type: "new_tool".to_string(),
            data: serde_json::json!({"type": "new_tool", "config": {"enabled": true}}),
        };
        let json = serde_json::to_string(&tool).expect("Serialization failed");

        // Should contain the type and config, but not duplicate "type"
        assert!(json.contains("\"type\":\"new_tool\""));
        assert!(json.contains("\"config\""));

        let parsed: Tool = serde_json::from_str(&json).expect("Deserialization failed");
        match parsed {
            Tool::Unknown { tool_type, .. } => assert_eq!(tool_type, "new_tool"),
            _ => panic!("Expected Unknown variant"),
        }
    }
}
