/// Helper functions for building Interactions API content
///
/// This module provides ergonomic builders for InteractionContent and InteractionInput.
///
/// # Thought Signatures
///
/// When using function calling with Gemini 3 models, thought signatures are critical for
/// maintaining reasoning context across multi-turn interactions. Per Google's documentation
/// (<https://ai.google.dev/gemini-api/docs/thought-signatures>):
///
/// - **What they are**: Encrypted representations of the model's internal thought process
/// - **When they appear**: On function calls (first call in each step), and sometimes on final content
/// - **Requirement**: For Gemini 3 models, signatures MUST be echoed back during function calling
///   or you will receive a 400 validation error
///
/// ## Interactions API Handling
///
/// When using `previous_interaction_id` with the Interactions API, thought signatures are
/// managed automatically by the server. You don't need to manually extract and echo them.
///
/// For manual conversation construction (without `previous_interaction_id`), use
/// [`function_call_content_with_signature`] to include the signature when echoing function calls.
use genai_client::{
    CodeExecutionLanguage, CodeExecutionOutcome, InteractionContent, InteractionInput,
};
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

/// Creates a function call content with optional thought signature and call ID
///
/// For Gemini 3 models, thought signatures are required for multi-turn function calling.
/// Extract them from the interaction response and pass them here when building conversation history.
///
/// See <https://ai.google.dev/gemini-api/docs/thought-signatures> for details on thought signatures.
///
/// **Note**: When using `previous_interaction_id`, the server manages signatures automatically.
///
/// # Example
/// ```
/// use rust_genai::interactions_api::function_call_content_with_signature;
/// use serde_json::json;
///
/// let call = function_call_content_with_signature(
///     Some("call_123"),
///     "get_weather",
///     json!({"location": "San Francisco"}),
///     Some("encrypted_signature_token".to_string())
/// );
/// ```
pub fn function_call_content_with_signature(
    id: Option<impl Into<String>>,
    name: impl Into<String>,
    args: Value,
    thought_signature: Option<String>,
) -> InteractionContent {
    let function_name = name.into();

    // Validate that signature is not empty if provided
    if let Some(ref sig) = thought_signature
        && sig.trim().is_empty()
    {
        log::warn!(
            "Empty thought signature provided for function call '{}'. \
             This may cause issues with Gemini 3 multi-turn conversations.",
            function_name
        );
    }

    InteractionContent::FunctionCall {
        id: id.map(|s| s.into()),
        name: function_name,
        args,
        thought_signature,
    }
}

/// Creates a function call content (without thought signature or call ID)
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
    function_call_content_with_signature(None::<String>, name, args, None)
}

/// Creates a function result content (preferred for new code)
///
/// This is the correct way to send function execution results back to the Interactions API.
/// The call_id must match the id from the FunctionCall you're responding to.
///
/// # Panics
///
/// Will log a warning if call_id is empty or whitespace-only, as this may cause
/// API errors when the server tries to match the result to a function call.
///
/// # Example
/// ```
/// use rust_genai::interactions_api::function_result_content;
/// use serde_json::json;
///
/// let result = function_result_content(
///     "get_weather",
///     "call_abc123",
///     json!({"temperature": "72F", "conditions": "sunny"})
/// );
/// ```
pub fn function_result_content(
    name: impl Into<String>,
    call_id: impl Into<String>,
    result: Value,
) -> InteractionContent {
    let function_name = name.into();
    let call_id_str = call_id.into();

    // Validate call_id is not empty
    if call_id_str.trim().is_empty() {
        log::warn!(
            "Empty call_id provided for function result '{}'. \
             This may cause the API to fail to match the result to its function call.",
            function_name
        );
    }

    InteractionContent::FunctionResult {
        name: function_name,
        call_id: call_id_str,
        result,
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
/// # Arguments
///
/// * `uri` - The URI of the image
/// * `mime_type` - The MIME type (required by the API for URI-based content)
///
/// # Example
/// ```
/// use rust_genai::interactions_api::image_uri_content;
///
/// let image = image_uri_content(
///     "https://example.com/image.png",
///     "image/png"
/// );
/// ```
pub fn image_uri_content(
    uri: impl Into<String>,
    mime_type: impl Into<String>,
) -> InteractionContent {
    InteractionContent::Image {
        data: None,
        uri: Some(uri.into()),
        mime_type: Some(mime_type.into()),
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
/// # Arguments
///
/// * `uri` - The URI of the audio file
/// * `mime_type` - The MIME type (required by the API for URI-based content)
///
/// # Example
/// ```
/// use rust_genai::interactions_api::audio_uri_content;
///
/// let audio = audio_uri_content(
///     "https://example.com/audio.mp3",
///     "audio/mp3"
/// );
/// ```
pub fn audio_uri_content(
    uri: impl Into<String>,
    mime_type: impl Into<String>,
) -> InteractionContent {
    InteractionContent::Audio {
        data: None,
        uri: Some(uri.into()),
        mime_type: Some(mime_type.into()),
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
/// # Arguments
///
/// * `uri` - The URI of the video file
/// * `mime_type` - The MIME type (required by the API for URI-based content)
///
/// # Example
/// ```
/// use rust_genai::interactions_api::video_uri_content;
///
/// let video = video_uri_content(
///     "https://example.com/video.mp4",
///     "video/mp4"
/// );
/// ```
pub fn video_uri_content(
    uri: impl Into<String>,
    mime_type: impl Into<String>,
) -> InteractionContent {
    InteractionContent::Video {
        data: None,
        uri: Some(uri.into()),
        mime_type: Some(mime_type.into()),
    }
}

/// Creates document content from base64-encoded data
///
/// Use this for PDF files and other document formats.
///
/// # Example
/// ```
/// use rust_genai::interactions_api::document_data_content;
///
/// let document = document_data_content(
///     "base64encodeddata...",
///     "application/pdf"
/// );
/// ```
pub fn document_data_content(
    data: impl Into<String>,
    mime_type: impl Into<String>,
) -> InteractionContent {
    InteractionContent::Document {
        data: Some(data.into()),
        uri: None,
        mime_type: Some(mime_type.into()),
    }
}

/// Creates document content from a URI
///
/// Use this for PDF files and other document formats accessible via URI.
///
/// # Arguments
///
/// * `uri` - The URI of the document
/// * `mime_type` - The MIME type (required by the API for URI-based content)
///
/// # Example
/// ```
/// use rust_genai::interactions_api::document_uri_content;
///
/// let document = document_uri_content(
///     "https://example.com/document.pdf",
///     "application/pdf"
/// );
/// ```
pub fn document_uri_content(
    uri: impl Into<String>,
    mime_type: impl Into<String>,
) -> InteractionContent {
    InteractionContent::Document {
        data: None,
        uri: Some(uri.into()),
        mime_type: Some(mime_type.into()),
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
///     image_uri_content("https://example.com/photo.jpg", "image/jpeg"),
/// ]);
/// ```
pub fn build_interaction_input(contents: Vec<InteractionContent>) -> InteractionInput {
    InteractionInput::Content(contents)
}

/// Creates code execution call content
///
/// This variant appears when the model initiates code execution
/// via the `CodeExecution` built-in tool.
///
/// # Example
/// ```
/// use rust_genai::interactions_api::code_execution_call_content;
/// use rust_genai::CodeExecutionLanguage;
///
/// let call = code_execution_call_content("call_123", CodeExecutionLanguage::Python, "print('Hello, World!')");
/// ```
pub fn code_execution_call_content(
    id: impl Into<String>,
    language: CodeExecutionLanguage,
    code: impl Into<String>,
) -> InteractionContent {
    InteractionContent::CodeExecutionCall {
        id: id.into(),
        language,
        code: code.into(),
    }
}

/// Creates code execution result content
///
/// Contains the outcome and output of executed code from the `CodeExecution` tool.
///
/// # Example
/// ```
/// use rust_genai::interactions_api::code_execution_result_content;
/// use rust_genai::CodeExecutionOutcome;
///
/// let result = code_execution_result_content("call_123", CodeExecutionOutcome::Ok, "42");
/// ```
pub fn code_execution_result_content(
    call_id: impl Into<String>,
    outcome: CodeExecutionOutcome,
    output: impl Into<String>,
) -> InteractionContent {
    InteractionContent::CodeExecutionResult {
        call_id: call_id.into(),
        outcome,
        output: output.into(),
    }
}

/// Creates a successful code execution result (convenience helper)
///
/// Shorthand for creating an `OUTCOME_OK` result.
///
/// # Example
/// ```
/// use rust_genai::interactions_api::code_execution_success;
///
/// let result = code_execution_success("call_123", "42\n");
/// ```
pub fn code_execution_success(
    call_id: impl Into<String>,
    output: impl Into<String>,
) -> InteractionContent {
    code_execution_result_content(call_id, CodeExecutionOutcome::Ok, output)
}

/// Creates a failed code execution result (convenience helper)
///
/// Shorthand for creating an `OUTCOME_FAILED` result.
///
/// # Example
/// ```
/// use rust_genai::interactions_api::code_execution_error;
///
/// let result = code_execution_error("call_123", "NameError: name 'x' is not defined");
/// ```
pub fn code_execution_error(
    call_id: impl Into<String>,
    error_output: impl Into<String>,
) -> InteractionContent {
    code_execution_result_content(call_id, CodeExecutionOutcome::Failed, error_output)
}

/// Creates Google Search call content
///
/// Appears when the model initiates a Google Search via the `GoogleSearch` tool.
///
/// # Example
/// ```
/// use rust_genai::interactions_api::google_search_call_content;
///
/// let search = google_search_call_content("Rust programming language");
/// ```
pub fn google_search_call_content(query: impl Into<String>) -> InteractionContent {
    InteractionContent::GoogleSearchCall {
        query: query.into(),
    }
}

/// Creates Google Search result content
///
/// Contains the results returned by the `GoogleSearch` built-in tool.
///
/// # Example
/// ```
/// use rust_genai::interactions_api::google_search_result_content;
/// use serde_json::json;
///
/// let results = google_search_result_content(json!({
///     "results": [{"title": "Rust", "url": "https://rust-lang.org"}]
/// }));
/// ```
pub fn google_search_result_content(results: serde_json::Value) -> InteractionContent {
    InteractionContent::GoogleSearchResult { results }
}

/// Creates URL context call content
///
/// Appears when the model requests URL content via the `UrlContext` tool.
///
/// # Example
/// ```
/// use rust_genai::interactions_api::url_context_call_content;
///
/// let fetch = url_context_call_content("https://example.com");
/// ```
pub fn url_context_call_content(url: impl Into<String>) -> InteractionContent {
    InteractionContent::UrlContextCall { url: url.into() }
}

/// Creates URL context result content
///
/// Contains the content retrieved by the `UrlContext` built-in tool.
///
/// # Example
/// ```
/// use rust_genai::interactions_api::url_context_result_content;
///
/// let result = url_context_result_content(
///     "https://example.com",
///     Some("<html>...</html>".to_string())
/// );
/// ```
pub fn url_context_result_content(
    url: impl Into<String>,
    content: Option<String>,
) -> InteractionContent {
    InteractionContent::UrlContextResult {
        url: url.into(),
        content,
    }
}

/// Creates a successful URL context result (convenience helper)
///
/// Shorthand for creating a result where the URL content was successfully fetched.
///
/// # Example
/// ```
/// use rust_genai::interactions_api::url_context_success;
///
/// let result = url_context_success("https://example.com", "<html>...</html>");
/// ```
pub fn url_context_success(
    url: impl Into<String>,
    content: impl Into<String>,
) -> InteractionContent {
    url_context_result_content(url, Some(content.into()))
}

/// Creates a failed URL context result (convenience helper)
///
/// Shorthand for creating a result where the URL content could not be fetched
/// (e.g., network errors, blocked URLs, timeouts, or access restrictions).
///
/// # Example
/// ```
/// use rust_genai::interactions_api::url_context_failure;
///
/// let result = url_context_failure("https://example.com/blocked");
/// ```
pub fn url_context_failure(url: impl Into<String>) -> InteractionContent {
    url_context_result_content(url, None)
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
    fn test_function_result_content() {
        let content = function_result_content("test", "call_123", json!({"result": "ok"}));
        match content {
            InteractionContent::FunctionResult {
                name,
                call_id,
                result,
            } => {
                assert_eq!(name, "test");
                assert_eq!(call_id, "call_123");
                assert_eq!(result, json!({"result": "ok"}));
            }
            _ => panic!("Expected FunctionResult variant"),
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
        let content = image_uri_content("http://example.com/img.png", "image/png");
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

    #[test]
    fn test_function_call_content_with_signature() {
        let content = function_call_content_with_signature(
            Some("call_abc"),
            "get_weather",
            json!({"city": "Tokyo"}),
            Some("sig_xyz".to_string()),
        );
        match content {
            InteractionContent::FunctionCall {
                id,
                name,
                args,
                thought_signature,
            } => {
                assert_eq!(id, Some("call_abc".to_string()));
                assert_eq!(name, "get_weather");
                assert_eq!(args, json!({"city": "Tokyo"}));
                assert_eq!(thought_signature, Some("sig_xyz".to_string()));
            }
            _ => panic!("Expected FunctionCall variant"),
        }
    }

    #[test]
    fn test_function_call_content_without_signature() {
        let content =
            function_call_content_with_signature(None::<String>, "test_fn", json!({}), None);
        match content {
            InteractionContent::FunctionCall {
                id,
                thought_signature,
                ..
            } => {
                assert_eq!(id, None);
                assert_eq!(thought_signature, None);
            }
            _ => panic!("Expected FunctionCall variant"),
        }
    }

    #[test]
    fn test_audio_data_content() {
        let content = audio_data_content("audio_base64_data", "audio/mp3");
        match content {
            InteractionContent::Audio {
                data,
                uri,
                mime_type,
            } => {
                assert_eq!(data, Some("audio_base64_data".to_string()));
                assert_eq!(uri, None);
                assert_eq!(mime_type, Some("audio/mp3".to_string()));
            }
            _ => panic!("Expected Audio variant"),
        }
    }

    #[test]
    fn test_audio_uri_content() {
        let content = audio_uri_content("https://example.com/audio.mp3", "audio/mp3");
        match content {
            InteractionContent::Audio {
                data,
                uri,
                mime_type,
            } => {
                assert_eq!(data, None);
                assert_eq!(uri, Some("https://example.com/audio.mp3".to_string()));
                assert_eq!(mime_type, Some("audio/mp3".to_string()));
            }
            _ => panic!("Expected Audio variant"),
        }
    }

    #[test]
    fn test_video_data_content() {
        let content = video_data_content("video_base64_data", "video/mp4");
        match content {
            InteractionContent::Video {
                data,
                uri,
                mime_type,
            } => {
                assert_eq!(data, Some("video_base64_data".to_string()));
                assert_eq!(uri, None);
                assert_eq!(mime_type, Some("video/mp4".to_string()));
            }
            _ => panic!("Expected Video variant"),
        }
    }

    #[test]
    fn test_video_uri_content() {
        let content = video_uri_content("https://example.com/video.mp4", "video/mp4");
        match content {
            InteractionContent::Video {
                data,
                uri,
                mime_type,
            } => {
                assert_eq!(data, None);
                assert_eq!(uri, Some("https://example.com/video.mp4".to_string()));
                assert_eq!(mime_type, Some("video/mp4".to_string()));
            }
            _ => panic!("Expected Video variant"),
        }
    }

    #[test]
    fn test_document_data_content() {
        let content = document_data_content("pdf_base64_data", "application/pdf");
        match content {
            InteractionContent::Document {
                data,
                uri,
                mime_type,
            } => {
                assert_eq!(data, Some("pdf_base64_data".to_string()));
                assert_eq!(uri, None);
                assert_eq!(mime_type, Some("application/pdf".to_string()));
            }
            _ => panic!("Expected Document variant"),
        }
    }

    #[test]
    fn test_document_uri_content() {
        let content = document_uri_content("https://example.com/doc.pdf", "application/pdf");
        match content {
            InteractionContent::Document {
                data,
                uri,
                mime_type,
            } => {
                assert_eq!(data, None);
                assert_eq!(uri, Some("https://example.com/doc.pdf".to_string()));
                assert_eq!(mime_type, Some("application/pdf".to_string()));
            }
            _ => panic!("Expected Document variant"),
        }
    }

    #[test]
    fn test_code_execution_call_content() {
        let content =
            code_execution_call_content("call_123", CodeExecutionLanguage::Python, "print(42)");
        match content {
            InteractionContent::CodeExecutionCall { id, language, code } => {
                assert_eq!(id, "call_123");
                assert_eq!(language, CodeExecutionLanguage::Python);
                assert_eq!(code, "print(42)");
            }
            _ => panic!("Expected CodeExecutionCall variant"),
        }
    }

    #[test]
    fn test_code_execution_result_content() {
        let content = code_execution_result_content("call_123", CodeExecutionOutcome::Ok, "42\n");
        match content {
            InteractionContent::CodeExecutionResult {
                call_id,
                outcome,
                output,
            } => {
                assert_eq!(call_id, "call_123");
                assert_eq!(outcome, CodeExecutionOutcome::Ok);
                assert_eq!(output, "42\n");
            }
            _ => panic!("Expected CodeExecutionResult variant"),
        }
    }

    #[test]
    fn test_code_execution_success() {
        let content = code_execution_success("call_456", "Hello World");
        match content {
            InteractionContent::CodeExecutionResult {
                outcome, output, ..
            } => {
                assert_eq!(outcome, CodeExecutionOutcome::Ok);
                assert_eq!(output, "Hello World");
            }
            _ => panic!("Expected CodeExecutionResult variant"),
        }
    }

    #[test]
    fn test_code_execution_error() {
        let content = code_execution_error("call_789", "NameError: x not defined");
        match content {
            InteractionContent::CodeExecutionResult {
                outcome, output, ..
            } => {
                assert_eq!(outcome, CodeExecutionOutcome::Failed);
                assert!(output.contains("NameError"));
            }
            _ => panic!("Expected CodeExecutionResult variant"),
        }
    }

    #[test]
    fn test_google_search_call_content() {
        let content = google_search_call_content("Rust programming");
        match content {
            InteractionContent::GoogleSearchCall { query } => {
                assert_eq!(query, "Rust programming");
            }
            _ => panic!("Expected GoogleSearchCall variant"),
        }
    }

    #[test]
    fn test_google_search_result_content() {
        let results = json!({"items": [{"title": "Rust"}]});
        let content = google_search_result_content(results.clone());
        match content {
            InteractionContent::GoogleSearchResult { results: r } => {
                assert_eq!(r, results);
            }
            _ => panic!("Expected GoogleSearchResult variant"),
        }
    }

    #[test]
    fn test_url_context_call_content() {
        let content = url_context_call_content("https://docs.rs");
        match content {
            InteractionContent::UrlContextCall { url } => {
                assert_eq!(url, "https://docs.rs");
            }
            _ => panic!("Expected UrlContextCall variant"),
        }
    }

    #[test]
    fn test_url_context_result_content() {
        let content = url_context_result_content(
            "https://example.com",
            Some("<html>test</html>".to_string()),
        );
        match content {
            InteractionContent::UrlContextResult { url, content: c } => {
                assert_eq!(url, "https://example.com");
                assert_eq!(c, Some("<html>test</html>".to_string()));
            }
            _ => panic!("Expected UrlContextResult variant"),
        }
    }

    #[test]
    fn test_url_context_success() {
        let content = url_context_success("https://example.com", "<html>...</html>");
        match content {
            InteractionContent::UrlContextResult { url, content: c } => {
                assert_eq!(url, "https://example.com");
                assert_eq!(c, Some("<html>...</html>".to_string()));
            }
            _ => panic!("Expected UrlContextResult variant"),
        }
    }

    #[test]
    fn test_url_context_failure() {
        let content = url_context_failure("https://blocked.com");
        match content {
            InteractionContent::UrlContextResult { url, content: c } => {
                assert_eq!(url, "https://blocked.com");
                assert_eq!(c, None);
            }
            _ => panic!("Expected UrlContextResult variant"),
        }
    }
}
