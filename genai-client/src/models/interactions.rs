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
    /// Function call (output from model)
    FunctionCall {
        /// Unique identifier for this function call
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        name: String,
        #[serde(rename = "arguments")]
        args: serde_json::Value,
        /// Thought signature for Gemini 3 reasoning continuity
        #[serde(rename = "thoughtSignature", skip_serializing_if = "Option::is_none")]
        thought_signature: Option<String>,
    },
    /// Function result (input to model with execution result)
    FunctionResult {
        name: String,
        /// The call_id from the FunctionCall being responded to
        call_id: String,
        result: serde_json::Value,
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
}

/// Delta content for streaming events
///
/// Contains incremental content updates during streaming.
/// Used with "content.delta" event types.
#[derive(Clone, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamDelta {
    /// Text content delta
    Text {
        #[serde(default)]
        text: String,
    },
    /// Thought content delta (internal reasoning)
    Thought {
        #[serde(default)]
        text: String,
    },
}

impl StreamDelta {
    /// Extract the text content from this delta, if any
    pub fn text(&self) -> Option<&str> {
        match self {
            StreamDelta::Text { text } if !text.is_empty() => Some(text),
            _ => None,
        }
    }

    /// Check if this is a text delta
    pub const fn is_text(&self) -> bool {
        matches!(self, StreamDelta::Text { .. })
    }

    /// Check if this is a thought delta
    pub const fn is_thought(&self) -> bool {
        matches!(self, StreamDelta::Thought { .. })
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
#[serde(rename_all = "camelCase")]
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
            "eventType": "content.delta",
            "interactionId": "test_123",
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
            "eventType": "interaction.complete",
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
}
