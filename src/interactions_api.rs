/// Helper functions for building Interactions API content
///
/// This module provides ergonomic builders for InteractionContent and InteractionInput,
/// matching the pattern established by content_api.rs for the GenerateContent API.
use genai_client::{InteractionContent, InteractionInput};
use serde_json::Value;

/// Creates a simple text input from a string
///
/// # Example
/// ```
/// use rust_genai::interactions_api::text_input;
///
/// let input = text_input("Hello, how are you?");
/// ```
pub fn text_input(text: impl Into<String>) -> InteractionInput {
    InteractionInput::Text(text.into())
}

/// Creates text content
///
/// # Example
/// ```
/// use rust_genai::interactions_api::text_content;
///
/// let content = text_content("This is a response");
/// ```
pub fn text_content(text: impl Into<String>) -> InteractionContent {
    InteractionContent::Text {
        text: Some(text.into()),
    }
}

/// Creates thought content (internal reasoning visible in agent responses)
///
/// # Example
/// ```
/// use rust_genai::interactions_api::thought_content;
///
/// let thought = thought_content("I need to search for weather data");
/// ```
pub fn thought_content(text: impl Into<String>) -> InteractionContent {
    InteractionContent::Thought {
        text: Some(text.into()),
    }
}

/// Creates a function call content with optional thought signature
///
/// For Gemini 3 models, thought signatures are required for multi-turn function calling.
/// Extract them from the interaction response and pass them here when building conversation history.
///
/// # Example
/// ```
/// use rust_genai::interactions_api::function_call_content_with_signature;
/// use serde_json::json;
///
/// let call = function_call_content_with_signature(
///     "get_weather",
///     json!({"location": "San Francisco"}),
///     Some("encrypted_signature_token".to_string())
/// );
/// ```
pub fn function_call_content_with_signature(
    name: impl Into<String>,
    args: Value,
    thought_signature: Option<String>,
) -> InteractionContent {
    InteractionContent::FunctionCall {
        name: name.into(),
        args,
        thought_signature,
    }
}

/// Creates a function call content (without thought signature)
///
/// For Gemini 3 models, prefer using `function_call_content_with_signature` instead.
///
/// # Example
/// ```
/// use rust_genai::interactions_api::function_call_content;
/// use serde_json::json;
///
/// let call = function_call_content(
///     "get_weather",
///     json!({"location": "San Francisco"})
/// );
/// ```
pub fn function_call_content(name: impl Into<String>, args: Value) -> InteractionContent {
    function_call_content_with_signature(name, args, None)
}

/// Creates a function response content
///
/// # Example
/// ```
/// use rust_genai::interactions_api::function_response_content;
/// use serde_json::json;
///
/// let response = function_response_content(
///     "get_weather",
///     json!({"temperature": "72F", "conditions": "sunny"})
/// );
/// ```
pub fn function_response_content(name: impl Into<String>, response: Value) -> InteractionContent {
    InteractionContent::FunctionResponse {
        name: name.into(),
        response,
    }
}

/// Creates image content from base64-encoded data
///
/// # Example
/// ```
/// use rust_genai::interactions_api::image_data_content;
///
/// let image = image_data_content(
///     "base64encodeddata...",
///     "image/png"
/// );
/// ```
pub fn image_data_content(
    data: impl Into<String>,
    mime_type: impl Into<String>,
) -> InteractionContent {
    InteractionContent::Image {
        data: Some(data.into()),
        uri: None,
        mime_type: Some(mime_type.into()),
    }
}

/// Creates image content from a URI
///
/// # Example
/// ```
/// use rust_genai::interactions_api::image_uri_content;
///
/// let image = image_uri_content(
///     "https://example.com/image.png",
///     Some("image/png".to_string())
/// );
/// ```
pub fn image_uri_content(uri: impl Into<String>, mime_type: Option<String>) -> InteractionContent {
    InteractionContent::Image {
        data: None,
        uri: Some(uri.into()),
        mime_type,
    }
}

/// Creates audio content from base64-encoded data
///
/// # Example
/// ```
/// use rust_genai::interactions_api::audio_data_content;
///
/// let audio = audio_data_content(
///     "base64encodeddata...",
///     "audio/mp3"
/// );
/// ```
pub fn audio_data_content(
    data: impl Into<String>,
    mime_type: impl Into<String>,
) -> InteractionContent {
    InteractionContent::Audio {
        data: Some(data.into()),
        uri: None,
        mime_type: Some(mime_type.into()),
    }
}

/// Creates audio content from a URI
///
/// # Example
/// ```
/// use rust_genai::interactions_api::audio_uri_content;
///
/// let audio = audio_uri_content(
///     "https://example.com/audio.mp3",
///     Some("audio/mp3".to_string())
/// );
/// ```
pub fn audio_uri_content(uri: impl Into<String>, mime_type: Option<String>) -> InteractionContent {
    InteractionContent::Audio {
        data: None,
        uri: Some(uri.into()),
        mime_type,
    }
}

/// Creates video content from base64-encoded data
///
/// # Example
/// ```
/// use rust_genai::interactions_api::video_data_content;
///
/// let video = video_data_content(
///     "base64encodeddata...",
///     "video/mp4"
/// );
/// ```
pub fn video_data_content(
    data: impl Into<String>,
    mime_type: impl Into<String>,
) -> InteractionContent {
    InteractionContent::Video {
        data: Some(data.into()),
        uri: None,
        mime_type: Some(mime_type.into()),
    }
}

/// Creates video content from a URI
///
/// # Example
/// ```
/// use rust_genai::interactions_api::video_uri_content;
///
/// let video = video_uri_content(
///     "https://example.com/video.mp4",
///     Some("video/mp4".to_string())
/// );
/// ```
pub fn video_uri_content(uri: impl Into<String>, mime_type: Option<String>) -> InteractionContent {
    InteractionContent::Video {
        data: None,
        uri: Some(uri.into()),
        mime_type,
    }
}

/// Builds a complete interaction input from multiple content items
///
/// # Example
/// ```
/// use rust_genai::interactions_api::{build_interaction_input, text_content, image_uri_content};
///
/// let input = build_interaction_input(vec![
///     text_content("What's in this image?"),
///     image_uri_content("https://example.com/photo.jpg", None),
/// ]);
/// ```
pub fn build_interaction_input(contents: Vec<InteractionContent>) -> InteractionInput {
    InteractionInput::Content(contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_text_input() {
        let input = text_input("Hello");
        match input {
            InteractionInput::Text(text) => assert_eq!(text, "Hello"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_text_content() {
        let content = text_content("Hello");
        match content {
            InteractionContent::Text { text } => assert_eq!(text, Some("Hello".to_string())),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_thought_content() {
        let content = thought_content("Thinking...");
        match content {
            InteractionContent::Thought { text } => {
                assert_eq!(text, Some("Thinking...".to_string()))
            }
            _ => panic!("Expected Thought variant"),
        }
    }

    #[test]
    fn test_function_call_content() {
        let content = function_call_content("test", json!({"key": "value"}));
        match content {
            InteractionContent::FunctionCall { name, args, .. } => {
                assert_eq!(name, "test");
                assert_eq!(args, json!({"key": "value"}));
            }
            _ => panic!("Expected FunctionCall variant"),
        }
    }

    #[test]
    fn test_function_response_content() {
        let content = function_response_content("test", json!({"result": "ok"}));
        match content {
            InteractionContent::FunctionResponse { name, response } => {
                assert_eq!(name, "test");
                assert_eq!(response, json!({"result": "ok"}));
            }
            _ => panic!("Expected FunctionResponse variant"),
        }
    }

    #[test]
    fn test_image_data_content() {
        let content = image_data_content("data123", "image/png");
        match content {
            InteractionContent::Image {
                data,
                uri,
                mime_type,
            } => {
                assert_eq!(data, Some("data123".to_string()));
                assert_eq!(uri, None);
                assert_eq!(mime_type, Some("image/png".to_string()));
            }
            _ => panic!("Expected Image variant"),
        }
    }

    #[test]
    fn test_image_uri_content() {
        let content = image_uri_content("http://example.com/img.png", Some("image/png".into()));
        match content {
            InteractionContent::Image {
                data,
                uri,
                mime_type,
            } => {
                assert_eq!(data, None);
                assert_eq!(uri, Some("http://example.com/img.png".to_string()));
                assert_eq!(mime_type, Some("image/png".to_string()));
            }
            _ => panic!("Expected Image variant"),
        }
    }

    #[test]
    fn test_build_interaction_input() {
        let contents = vec![text_content("Hello"), text_content("World")];
        let input = build_interaction_input(contents);
        match input {
            InteractionInput::Content(vec) => assert_eq!(vec.len(), 2),
            _ => panic!("Expected Content variant"),
        }
    }
}
