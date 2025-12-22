// Shared types used by multiple API endpoints (generateContent, interactions, etc.)

use serde::{Deserialize, Serialize};

/// Represents a message in a conversation with role and content parts.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Content {
    pub parts: Vec<Part>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// Represents a part of content (text, function call, function response, etc.)
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_response: Option<FunctionResponse>,
    // Add other part types later e.g.:
    // pub inline_data: Option<Blob>,
}

/// Represents a tool that can be used by the model.
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Tool {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_declarations: Option<Vec<FunctionDeclaration>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_execution: Option<CodeExecution>,
}

/// Represents the code execution tool.
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct CodeExecution {
    // No fields, as per API documentation for the basic CodeExecution tool.
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
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
        Tool {
            function_declarations: Some(vec![self]),
            code_execution: None,
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

/// Represents a function call made by the model.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

/// Represents the response to a function call.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FunctionResponse {
    pub name: String,
    pub response: serde_json::Value,
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
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
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
    fn test_serialize_content() {
        let content = Content {
            parts: vec![Part {
                text: Some("Hello".to_string()),
                function_call: None,
                function_response: None,
            }],
            role: Some("user".to_string()),
        };

        let json = serde_json::to_string(&content).expect("Serialization failed");
        let parsed: Content = serde_json::from_str(&json).expect("Deserialization failed");

        assert_eq!(parsed.role.as_deref(), Some("user"));
        assert_eq!(parsed.parts.len(), 1);
        assert_eq!(parsed.parts[0].text.as_deref(), Some("Hello"));
    }

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
}
