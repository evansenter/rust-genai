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
    pub name: String,
    pub description: String,
    pub parameters: FunctionParameters,
}

/// Represents the parameters schema for a function.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FunctionParameters {
    #[serde(rename = "type")]
    pub type_: String,
    pub properties: serde_json::Value,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
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
        let function = FunctionDeclaration {
            name: "get_weather".to_string(),
            description: "Get the current weather in a given location".to_string(),
            parameters: FunctionParameters {
                type_: "object".to_string(),
                properties: serde_json::json!({
                    "location": {
                        "type": "string",
                        "description": "The city and state, e.g. San Francisco, CA"
                    }
                }),
                required: vec!["location".to_string()],
            },
        };

        let json_string = serde_json::to_string(&function).expect("Serialization failed");
        let parsed: FunctionDeclaration =
            serde_json::from_str(&json_string).expect("Deserialization failed");

        assert_eq!(parsed.name, "get_weather");
        assert_eq!(
            parsed.description,
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
