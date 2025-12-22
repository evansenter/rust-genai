use serde::{Deserialize, Serialize};

// Import only Tool from shared types
use super::shared::Tool;

/// Content object for Interactions API - uses flat structure with type field
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InteractionContent {
    /// Text content
    Text {
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
    },
    /// Thought content (internal reasoning)
    Thought {
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
    },
    /// Image content
    Image {
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        uri: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
    /// Audio content
    Audio {
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        uri: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
    /// Video content
    Video {
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        uri: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
    /// Function call
    FunctionCall {
        name: String,
        args: serde_json::Value,
    },
    /// Function response
    FunctionResponse {
        name: String,
        response: serde_json::Value,
    },
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
        let content_json =
            r#"{"type": "function_call", "name": "get_weather", "args": {"location": "Paris"}}"#;

        let content: InteractionContent =
            serde_json::from_str(content_json).expect("Deserialization failed");

        match content {
            InteractionContent::FunctionCall { name, args } => {
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
}
