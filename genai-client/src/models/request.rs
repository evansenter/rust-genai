use serde::Serialize;

// Import shared types used by multiple APIs
use super::shared::{Content, Tool, ToolConfig};

/// Request body for the generateContent API endpoint
#[derive(Clone, Serialize, Debug)]
pub struct GenerateContentRequest {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
    // generationConfig: Option<GenerationConfig>, // Example for future addition
    // safetySettings: Option<Vec<SafetySetting>>, // Example for future addition
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::shared::{FunctionDeclaration, FunctionParameters};
    use serde_json;

    #[test]
    fn test_serialize_generate_content_request() {
        let request = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: Some("Hello".to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: None,
            }],
            system_instruction: None,
            tools: None,
            tool_config: None,
        };

        let json_string = serde_json::to_string(&request).expect("Serialization failed");

        // Example expected JSON - adjust based on actual API needs
        // Note: serde_json doesn't guarantee field order or whitespace
        let expected_json = r#"{"contents":[{"parts":[{"text":"Hello"}]}]}"#;

        // Basic check - parsing back should work and fields should match conceptually
        // For a stricter check, parse both strings into serde_json::Value and compare
        let expected_value: serde_json::Value = serde_json::from_str(expected_json).unwrap();
        let actual_value: serde_json::Value = serde_json::from_str(&json_string).unwrap();

        assert_eq!(actual_value, expected_value);
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
                    },
                    "unit": {
                        "type": "string",
                        "enum": ["celsius", "fahrenheit"],
                        "description": "The temperature unit to use"
                    }
                }),
                required: vec!["location".to_string()],
            },
        };

        let json_string = serde_json::to_string(&function).expect("Serialization failed");
        let expected_json = r#"{"name":"get_weather","description":"Get the current weather in a given location","parameters":{"type":"object","properties":{"location":{"type":"string","description":"The city and state, e.g. San Francisco, CA"},"unit":{"type":"string","enum":["celsius","fahrenheit"],"description":"The temperature unit to use"}},"required":["location"]}}"#;
        let expected_value: serde_json::Value = serde_json::from_str(expected_json).unwrap();
        let actual_value: serde_json::Value = serde_json::from_str(&json_string).unwrap();
        assert_eq!(actual_value, expected_value);
    }
}
