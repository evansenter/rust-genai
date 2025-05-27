use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct GenerateContentRequest {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
    // generationConfig: Option<GenerationConfig>, // Example for future addition
    // safetySettings: Option<Vec<SafetySetting>>, // Example for future addition
}

#[derive(Serialize, Debug)]
pub struct Content {
    pub parts: Vec<Part>,
    // role: Option<String>, // Example for future addition
}

#[derive(Serialize, Debug)]
pub struct Part {
    pub text: String,
    // Add other part types later e.g.:
    // pub inline_data: Option<Blob>,
    // pub function_call: Option<FunctionCall>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_serialize_generate_content_request() {
        let request = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: "Hello".to_string(),
                }],
                // role: None, // If role field were added
            }],
            // generationConfig: None, // If config field were added
            // safetySettings: None, // If safety field were added
            system_instruction: None,
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
}
