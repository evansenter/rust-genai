/// Helper functions for building Interactions API content
///
/// This module provides ergonomic builders for InteractionContent and InteractionInput.
///
/// # Module Organization
///
/// Functions are organized into two categories:
///
/// ## User Input Constructors (re-exported from crate root)
///
/// These are for content YOU send to the API:
/// - **Text & Thought**: `text_content`, `thought_content`
/// - **Function Calling**: `function_call_content`, `function_call_content_with_signature`, `function_result_content`
/// - **Multimodal**: `image_*`, `audio_*`, `video_*`, `document_*`
///
/// ## Model Output Constructors (internal use)
///
/// These represent content the MODEL generates. Access via response methods
/// (e.g., `response.google_search_results()`), not these constructors:
/// - **Code Execution**: `code_execution_*`
/// - **Google Search**: `google_search_*`
/// - **URL Context**: `url_context_*`
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
use crate::{
    CodeExecutionLanguage, CodeExecutionOutcome, FileSearchResultItem, GoogleSearchResultItem,
    InteractionContent, Resolution,
};
use serde_json::Value;

// ============================================================================
// USER INPUT CONSTRUCTORS
// ============================================================================
//
// These functions create content that users send to the API.
// Re-exported from crate root for convenience.

// ----------------------------------------------------------------------------
// Text & Thought
// ----------------------------------------------------------------------------

/// Creates text content.
///
/// **Prefer:** [`InteractionContent::new_text()`] for new code.
///
/// # Example
/// ```
/// use genai_rs::interactions_api::text_content;
///
/// let content = text_content("This is a response");
/// ```
pub fn text_content(text: impl Into<String>) -> InteractionContent {
    InteractionContent::new_text(text)
}

/// Creates thought content (for testing and roundtrip verification only).
///
/// # ⚠️ API Limitation
///
/// **The Gemini API does NOT accept thought blocks in user input.**
/// Attempting to send thoughts returns: `"User turns cannot contain thought blocks."`
///
/// For multi-turn conversations with thinking:
/// - Use `with_previous_interaction(id)` — the server preserves thought context automatically
/// - For stateless mode, only text content can be included in history
///
/// # Valid Uses
///
/// This helper exists for:
/// - Roundtrip serialization testing
/// - Representing API responses as structured data
/// - Property-based testing (proptest)
///
/// # Example
/// ```
/// use genai_rs::interactions_api::thought_content;
///
/// // For testing serialization roundtrip
/// let thought = thought_content("EosFCogFAXLI2...");
/// assert!(thought.is_thought());
/// ```
pub fn thought_content(signature: impl Into<String>) -> InteractionContent {
    InteractionContent::new_thought(signature)
}

// ----------------------------------------------------------------------------
// Function Calling
// ----------------------------------------------------------------------------

/// Creates a function call content with optional thought signature and call ID.
///
/// **Prefer:** [`InteractionContent::new_function_call_with_signature()`] for new code.
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
/// use genai_rs::interactions_api::function_call_content_with_signature;
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
    InteractionContent::new_function_call_with_signature(id, name, args, thought_signature)
}

/// Creates a function call content (without thought signature or call ID).
///
/// **Prefer:** [`InteractionContent::new_function_call()`] for new code.
///
/// For Gemini 3 models, prefer using `function_call_content_with_signature` instead.
///
/// # Example
/// ```
/// use genai_rs::interactions_api::function_call_content;
/// use serde_json::json;
///
/// let call = function_call_content(
///     "get_weather",
///     json!({"location": "San Francisco"})
/// );
/// ```
pub fn function_call_content(name: impl Into<String>, args: Value) -> InteractionContent {
    InteractionContent::new_function_call(name, args)
}

/// Creates a function result content.
///
/// **Prefer:** [`InteractionContent::new_function_result()`] for new code.
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
/// use genai_rs::interactions_api::function_result_content;
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
    InteractionContent::new_function_result(name, call_id, result)
}

// ----------------------------------------------------------------------------
// Multimodal Content (Images, Audio, Video, Documents)
// ----------------------------------------------------------------------------

/// Creates image content from base64-encoded data.
///
/// **Prefer:** [`InteractionContent::new_image_data()`] for new code.
///
/// # Example
/// ```
/// use genai_rs::interactions_api::image_data_content;
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
    InteractionContent::new_image_data(data, mime_type)
}

/// Creates image content from base64-encoded data with specified resolution.
///
/// **Prefer:** [`InteractionContent::new_image_data_with_resolution()`] for new code.
///
/// # Resolution Trade-offs
///
/// | Level | Token Cost | Detail |
/// |-------|-----------|--------|
/// | Low | Lowest | Basic shapes and colors |
/// | Medium | Moderate | Standard detail |
/// | High | Higher | Fine details visible |
/// | UltraHigh | Highest | Maximum fidelity |
///
/// # Example
/// ```
/// use genai_rs::interactions_api::image_data_content_with_resolution;
/// use genai_rs::Resolution;
///
/// let image = image_data_content_with_resolution(
///     "base64encodeddata...",
///     "image/png",
///     Resolution::High
/// );
/// ```
pub fn image_data_content_with_resolution(
    data: impl Into<String>,
    mime_type: impl Into<String>,
    resolution: Resolution,
) -> InteractionContent {
    InteractionContent::new_image_data_with_resolution(data, mime_type, resolution)
}

/// Creates image content from a URI.
///
/// **Prefer:** [`InteractionContent::new_image_uri()`] for new code.
///
/// # Arguments
///
/// * `uri` - The URI of the image
/// * `mime_type` - The MIME type (required by the API for URI-based content)
///
/// # Example
/// ```
/// use genai_rs::interactions_api::image_uri_content;
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
    InteractionContent::new_image_uri(uri, mime_type)
}

/// Creates image content from a URI with specified resolution.
///
/// **Prefer:** [`InteractionContent::new_image_uri_with_resolution()`] for new code.
///
/// # Arguments
///
/// * `uri` - The URI of the image
/// * `mime_type` - The MIME type (required by the API for URI-based content)
/// * `resolution` - Resolution level for processing
///
/// # Example
/// ```
/// use genai_rs::interactions_api::image_uri_content_with_resolution;
/// use genai_rs::Resolution;
///
/// let image = image_uri_content_with_resolution(
///     "https://example.com/image.png",
///     "image/png",
///     Resolution::Low  // Use low resolution to reduce token cost
/// );
/// ```
pub fn image_uri_content_with_resolution(
    uri: impl Into<String>,
    mime_type: impl Into<String>,
    resolution: Resolution,
) -> InteractionContent {
    InteractionContent::new_image_uri_with_resolution(uri, mime_type, resolution)
}

/// Creates audio content from base64-encoded data.
///
/// **Prefer:** [`InteractionContent::new_audio_data()`] for new code.
///
/// # Example
/// ```
/// use genai_rs::interactions_api::audio_data_content;
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
    InteractionContent::new_audio_data(data, mime_type)
}

/// Creates audio content from a URI.
///
/// **Prefer:** [`InteractionContent::new_audio_uri()`] for new code.
///
/// # Arguments
///
/// * `uri` - The URI of the audio file
/// * `mime_type` - The MIME type (required by the API for URI-based content)
///
/// # Example
/// ```
/// use genai_rs::interactions_api::audio_uri_content;
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
    InteractionContent::new_audio_uri(uri, mime_type)
}

/// Creates video content from base64-encoded data.
///
/// **Prefer:** [`InteractionContent::new_video_data()`] for new code.
///
/// # Example
/// ```
/// use genai_rs::interactions_api::video_data_content;
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
    InteractionContent::new_video_data(data, mime_type)
}

/// Creates video content from base64-encoded data with specified resolution.
///
/// **Prefer:** [`InteractionContent::new_video_data_with_resolution()`] for new code.
///
/// # Resolution Trade-offs
///
/// | Level | Token Cost | Detail |
/// |-------|-----------|--------|
/// | Low | Lowest | Basic shapes and colors |
/// | Medium | Moderate | Standard detail |
/// | High | Higher | Fine details visible |
/// | UltraHigh | Highest | Maximum fidelity |
///
/// # Example
/// ```
/// use genai_rs::interactions_api::video_data_content_with_resolution;
/// use genai_rs::Resolution;
///
/// let video = video_data_content_with_resolution(
///     "base64encodeddata...",
///     "video/mp4",
///     Resolution::Low  // Use low resolution to reduce token cost for long videos
/// );
/// ```
pub fn video_data_content_with_resolution(
    data: impl Into<String>,
    mime_type: impl Into<String>,
    resolution: Resolution,
) -> InteractionContent {
    InteractionContent::new_video_data_with_resolution(data, mime_type, resolution)
}

/// Creates video content from a URI.
///
/// **Prefer:** [`InteractionContent::new_video_uri()`] for new code.
///
/// # Arguments
///
/// * `uri` - The URI of the video file
/// * `mime_type` - The MIME type (required by the API for URI-based content)
///
/// # Example
/// ```
/// use genai_rs::interactions_api::video_uri_content;
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
    InteractionContent::new_video_uri(uri, mime_type)
}

/// Creates video content from a URI with specified resolution.
///
/// **Prefer:** [`InteractionContent::new_video_uri_with_resolution()`] for new code.
///
/// # Arguments
///
/// * `uri` - The URI of the video file
/// * `mime_type` - The MIME type (required by the API for URI-based content)
/// * `resolution` - Resolution level for processing
///
/// # Example
/// ```
/// use genai_rs::interactions_api::video_uri_content_with_resolution;
/// use genai_rs::Resolution;
///
/// let video = video_uri_content_with_resolution(
///     "https://example.com/video.mp4",
///     "video/mp4",
///     Resolution::Medium
/// );
/// ```
pub fn video_uri_content_with_resolution(
    uri: impl Into<String>,
    mime_type: impl Into<String>,
    resolution: Resolution,
) -> InteractionContent {
    InteractionContent::new_video_uri_with_resolution(uri, mime_type, resolution)
}

/// Creates document content from base64-encoded data.
///
/// **Prefer:** [`InteractionContent::new_document_data()`] for new code.
///
/// Use this for PDF files and other document formats.
///
/// # Example
/// ```
/// use genai_rs::interactions_api::document_data_content;
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
    InteractionContent::new_document_data(data, mime_type)
}

/// Creates document content from a URI.
///
/// **Prefer:** [`InteractionContent::new_document_uri()`] for new code.
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
/// use genai_rs::interactions_api::document_uri_content;
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
    InteractionContent::new_document_uri(uri, mime_type)
}

/// Creates file content from a Files API URI.
///
/// **Prefer:** [`InteractionContent::from_file()`] for new code.
///
/// Use this to reference files uploaded via the Files API. The content type
/// is inferred from the file's MIME type (image, audio, video, or document).
///
/// # Arguments
///
/// * `file` - The uploaded file metadata from the Files API
///
/// # Example
///
/// ```no_run
/// use genai_rs::Client;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::new("api-key".to_string());
///
/// let file = client.upload_file("video.mp4").await?;
/// let content = genai_rs::file_uri_content(&file);
///
/// let response = client.interaction()
///     .with_model("gemini-3-flash-preview")
///     .with_content(vec![
///         genai_rs::text_content("Describe this video"),
///         content,
///     ])
///     .create()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub fn file_uri_content(file: &crate::FileMetadata) -> InteractionContent {
    InteractionContent::from_file(file)
}

/// Creates content from a URI and MIME type.
///
/// **Prefer:** [`InteractionContent::from_uri_and_mime()`] for new code.
///
/// This is the shared implementation used by [`file_uri_content`] and
/// [`crate::InteractionBuilder::with_file_uri`]. The content type is inferred
/// from the MIME type:
///
/// - `image/*` → [`InteractionContent::Image`]
/// - `audio/*` → [`InteractionContent::Audio`]
/// - `video/*` → [`InteractionContent::Video`]
/// - Other MIME types (including `application/*`, `text/*`) → [`InteractionContent::Document`]
///
/// # Arguments
///
/// * `uri` - The file URI (typically from the Files API)
/// * `mime_type` - The MIME type of the file
pub fn content_from_uri_and_mime(uri: String, mime_type: String) -> InteractionContent {
    InteractionContent::from_uri_and_mime(uri, mime_type)
}

// ============================================================================
// MODEL OUTPUT CONSTRUCTORS
// ============================================================================
//
// These functions create content that represents MODEL-generated outputs.
// NOT re-exported from crate root - access via response methods instead
// (e.g., response.google_search_results(), response.code_execution_results()).
//
// Available via genai_rs::interactions_api::* if direct construction is needed.

// ----------------------------------------------------------------------------
// Code Execution (built-in tool output)
// ----------------------------------------------------------------------------

/// Creates code execution call content
///
/// This variant appears when the model initiates code execution
/// via the `CodeExecution` built-in tool.
///
/// # Example
/// ```
/// use genai_rs::interactions_api::code_execution_call_content;
/// use genai_rs::CodeExecutionLanguage;
///
/// let call = code_execution_call_content("call_123", CodeExecutionLanguage::Python, "print('Hello, World!')");
/// ```
pub fn code_execution_call_content(
    id: impl Into<String>,
    language: CodeExecutionLanguage,
    code: impl Into<String>,
) -> InteractionContent {
    InteractionContent::CodeExecutionCall {
        id: Some(id.into()),
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
/// use genai_rs::interactions_api::code_execution_result_content;
/// use genai_rs::CodeExecutionOutcome;
///
/// let result = code_execution_result_content("call_123", CodeExecutionOutcome::Ok, "42");
/// ```
pub fn code_execution_result_content(
    call_id: impl Into<String>,
    outcome: CodeExecutionOutcome,
    output: impl Into<String>,
) -> InteractionContent {
    InteractionContent::CodeExecutionResult {
        call_id: Some(call_id.into()),
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
/// use genai_rs::interactions_api::code_execution_success;
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
/// use genai_rs::interactions_api::code_execution_error;
///
/// let result = code_execution_error("call_123", "NameError: name 'x' is not defined");
/// ```
pub fn code_execution_error(
    call_id: impl Into<String>,
    error_output: impl Into<String>,
) -> InteractionContent {
    code_execution_result_content(call_id, CodeExecutionOutcome::Failed, error_output)
}

// ----------------------------------------------------------------------------
// Google Search (built-in tool output)
// ----------------------------------------------------------------------------

/// Creates Google Search call content
///
/// Appears when the model initiates a Google Search via the `GoogleSearch` tool.
///
/// # Example
/// ```
/// use genai_rs::interactions_api::google_search_call_content;
///
/// let search = google_search_call_content("call-123", vec!["Rust programming language"]);
/// ```
pub fn google_search_call_content(
    id: impl Into<String>,
    queries: Vec<impl Into<String>>,
) -> InteractionContent {
    InteractionContent::GoogleSearchCall {
        id: id.into(),
        queries: queries.into_iter().map(|q| q.into()).collect(),
    }
}

/// Creates Google Search result content
///
/// Contains the results returned by the `GoogleSearch` built-in tool.
///
/// # Example
/// ```
/// use genai_rs::interactions_api::google_search_result_content;
/// use genai_rs::GoogleSearchResultItem;
///
/// let results = google_search_result_content("call-123", vec![
///     GoogleSearchResultItem::new("Rust", "https://rust-lang.org"),
/// ]);
/// ```
pub fn google_search_result_content(
    call_id: impl Into<String>,
    result: Vec<GoogleSearchResultItem>,
) -> InteractionContent {
    InteractionContent::GoogleSearchResult {
        call_id: call_id.into(),
        result,
    }
}

// ----------------------------------------------------------------------------
// File Search (built-in tool output)
// ----------------------------------------------------------------------------

/// Creates file search result content
///
/// Returned when the model retrieves documents from file search stores.
///
/// # Example
/// ```
/// use genai_rs::interactions_api::file_search_result_content;
/// use genai_rs::FileSearchResultItem;
///
/// let results = file_search_result_content("call-123", vec![
///     FileSearchResultItem {
///         title: "Document".into(),
///         text: "Content".into(),
///         store: "store-1".into(),
///     },
/// ]);
/// ```
pub fn file_search_result_content(
    call_id: impl Into<String>,
    result: Vec<FileSearchResultItem>,
) -> InteractionContent {
    InteractionContent::FileSearchResult {
        call_id: call_id.into(),
        result,
    }
}

// ----------------------------------------------------------------------------
// URL Context (built-in tool output)
// ----------------------------------------------------------------------------

/// Creates URL context call content
///
/// Appears when the model requests URL content via the `UrlContext` tool.
///
/// # Example
/// ```
/// use genai_rs::interactions_api::url_context_call_content;
///
/// let fetch = url_context_call_content("ctx_123", vec!["https://example.com"]);
/// ```
pub fn url_context_call_content(
    id: impl Into<String>,
    urls: impl IntoIterator<Item = impl Into<String>>,
) -> InteractionContent {
    InteractionContent::UrlContextCall {
        id: id.into(),
        urls: urls.into_iter().map(Into::into).collect(),
    }
}

/// Creates URL context result content
///
/// Contains the results retrieved by the `UrlContext` built-in tool.
///
/// # Example
/// ```
/// use genai_rs::interactions_api::url_context_result_content;
/// use genai_rs::UrlContextResultItem;
///
/// let result = url_context_result_content(
///     "ctx_123",
///     vec![UrlContextResultItem::new("https://example.com", "success")]
/// );
/// ```
pub fn url_context_result_content(
    call_id: impl Into<String>,
    result: Vec<crate::UrlContextResultItem>,
) -> InteractionContent {
    InteractionContent::UrlContextResult {
        call_id: call_id.into(),
        result,
    }
}

/// Creates a successful URL context result for a single URL (convenience helper)
///
/// Shorthand for creating a result where a single URL was successfully fetched.
///
/// # Example
/// ```
/// use genai_rs::interactions_api::url_context_success;
///
/// let result = url_context_success("ctx_123", "https://example.com");
/// ```
pub fn url_context_success(
    call_id: impl Into<String>,
    url: impl Into<String>,
) -> InteractionContent {
    url_context_result_content(
        call_id,
        vec![crate::UrlContextResultItem::new(url, "success")],
    )
}

/// Creates a failed URL context result for a single URL (convenience helper)
///
/// Shorthand for creating a result where a single URL fetch failed
/// (e.g., network errors, blocked URLs, timeouts, or access restrictions).
///
/// # Example
/// ```
/// use genai_rs::interactions_api::url_context_failure;
///
/// let result = url_context_failure("ctx_123", "https://example.com/blocked");
/// ```
pub fn url_context_failure(
    call_id: impl Into<String>,
    url: impl Into<String>,
) -> InteractionContent {
    url_context_result_content(
        call_id,
        vec![crate::UrlContextResultItem::new(url, "error")],
    )
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_text_content() {
        let content = text_content("Hello");
        match content {
            InteractionContent::Text { text, .. } => assert_eq!(text, Some("Hello".to_string())),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_thought_content() {
        // Thought content now contains a signature, not text
        let content = thought_content("EosFCogFAXLI2...");
        match content {
            InteractionContent::Thought { signature } => {
                assert_eq!(signature, Some("EosFCogFAXLI2...".to_string()))
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
                is_error,
            } => {
                assert_eq!(name, Some("test".to_string()));
                assert_eq!(call_id, "call_123");
                assert_eq!(result, json!({"result": "ok"}));
                assert_eq!(is_error, None);
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
                resolution,
            } => {
                assert_eq!(data, Some("data123".to_string()));
                assert_eq!(uri, None);
                assert_eq!(mime_type, Some("image/png".to_string()));
                assert_eq!(resolution, None);
            }
            _ => panic!("Expected Image variant"),
        }
    }

    #[test]
    fn test_image_data_content_with_resolution() {
        let content = image_data_content_with_resolution("data123", "image/png", Resolution::High);
        match content {
            InteractionContent::Image {
                data,
                uri,
                mime_type,
                resolution,
            } => {
                assert_eq!(data, Some("data123".to_string()));
                assert_eq!(uri, None);
                assert_eq!(mime_type, Some("image/png".to_string()));
                assert_eq!(resolution, Some(Resolution::High));
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
                resolution,
            } => {
                assert_eq!(data, None);
                assert_eq!(uri, Some("http://example.com/img.png".to_string()));
                assert_eq!(mime_type, Some("image/png".to_string()));
                assert_eq!(resolution, None);
            }
            _ => panic!("Expected Image variant"),
        }
    }

    #[test]
    fn test_image_uri_content_with_resolution() {
        let content = image_uri_content_with_resolution(
            "http://example.com/img.png",
            "image/png",
            Resolution::Low,
        );
        match content {
            InteractionContent::Image {
                data,
                uri,
                mime_type,
                resolution,
            } => {
                assert_eq!(data, None);
                assert_eq!(uri, Some("http://example.com/img.png".to_string()));
                assert_eq!(mime_type, Some("image/png".to_string()));
                assert_eq!(resolution, Some(Resolution::Low));
            }
            _ => panic!("Expected Image variant"),
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
                resolution,
            } => {
                assert_eq!(data, Some("video_base64_data".to_string()));
                assert_eq!(uri, None);
                assert_eq!(mime_type, Some("video/mp4".to_string()));
                assert_eq!(resolution, None);
            }
            _ => panic!("Expected Video variant"),
        }
    }

    #[test]
    fn test_video_data_content_with_resolution() {
        let content =
            video_data_content_with_resolution("video_base64_data", "video/mp4", Resolution::Low);
        match content {
            InteractionContent::Video {
                data,
                uri,
                mime_type,
                resolution,
            } => {
                assert_eq!(data, Some("video_base64_data".to_string()));
                assert_eq!(uri, None);
                assert_eq!(mime_type, Some("video/mp4".to_string()));
                assert_eq!(resolution, Some(Resolution::Low));
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
                resolution,
            } => {
                assert_eq!(data, None);
                assert_eq!(uri, Some("https://example.com/video.mp4".to_string()));
                assert_eq!(mime_type, Some("video/mp4".to_string()));
                assert_eq!(resolution, None);
            }
            _ => panic!("Expected Video variant"),
        }
    }

    #[test]
    fn test_video_uri_content_with_resolution() {
        let content = video_uri_content_with_resolution(
            "https://example.com/video.mp4",
            "video/mp4",
            Resolution::UltraHigh,
        );
        match content {
            InteractionContent::Video {
                data,
                uri,
                mime_type,
                resolution,
            } => {
                assert_eq!(data, None);
                assert_eq!(uri, Some("https://example.com/video.mp4".to_string()));
                assert_eq!(mime_type, Some("video/mp4".to_string()));
                assert_eq!(resolution, Some(Resolution::UltraHigh));
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
                assert_eq!(id, Some("call_123".to_string()));
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
                assert_eq!(call_id, Some("call_123".to_string()));
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
        let content =
            google_search_call_content("call123", vec!["Rust programming", "latest version"]);
        match content {
            InteractionContent::GoogleSearchCall { id, queries } => {
                assert_eq!(id, "call123");
                assert_eq!(queries, vec!["Rust programming", "latest version"]);
            }
            _ => panic!("Expected GoogleSearchCall variant"),
        }
    }

    #[test]
    fn test_google_search_result_content() {
        use crate::GoogleSearchResultItem;
        let result = vec![GoogleSearchResultItem::new("Rust", "https://rust-lang.org")];
        let content = google_search_result_content("call123", result.clone());
        match content {
            InteractionContent::GoogleSearchResult { call_id, result: r } => {
                assert_eq!(call_id, "call123");
                assert_eq!(r.len(), 1);
                assert_eq!(r[0].title, "Rust");
                assert_eq!(r[0].url, "https://rust-lang.org");
            }
            _ => panic!("Expected GoogleSearchResult variant"),
        }
    }

    #[test]
    fn test_url_context_call_content() {
        let content =
            url_context_call_content("ctx_123", vec!["https://docs.rs", "https://crates.io"]);
        match content {
            InteractionContent::UrlContextCall { id, urls } => {
                assert_eq!(id, "ctx_123");
                assert_eq!(urls.len(), 2);
                assert_eq!(urls[0], "https://docs.rs");
                assert_eq!(urls[1], "https://crates.io");
            }
            _ => panic!("Expected UrlContextCall variant"),
        }
    }

    #[test]
    fn test_url_context_result_content() {
        let content = url_context_result_content(
            "ctx_123",
            vec![crate::UrlContextResultItem::new(
                "https://example.com",
                "success",
            )],
        );
        match content {
            InteractionContent::UrlContextResult { call_id, result } => {
                assert_eq!(call_id, "ctx_123");
                assert_eq!(result.len(), 1);
                assert_eq!(result[0].url, "https://example.com");
                assert!(result[0].is_success());
            }
            _ => panic!("Expected UrlContextResult variant"),
        }
    }

    #[test]
    fn test_url_context_success() {
        let content = url_context_success("ctx_123", "https://example.com");
        match content {
            InteractionContent::UrlContextResult { call_id, result } => {
                assert_eq!(call_id, "ctx_123");
                assert_eq!(result.len(), 1);
                assert_eq!(result[0].url, "https://example.com");
                assert!(result[0].is_success());
            }
            _ => panic!("Expected UrlContextResult variant"),
        }
    }

    #[test]
    fn test_url_context_failure() {
        let content = url_context_failure("ctx_123", "https://blocked.com");
        match content {
            InteractionContent::UrlContextResult { call_id, result } => {
                assert_eq!(call_id, "ctx_123");
                assert_eq!(result.len(), 1);
                assert_eq!(result[0].url, "https://blocked.com");
                assert!(result[0].is_error());
            }
            _ => panic!("Expected UrlContextResult variant"),
        }
    }

    #[test]
    fn test_content_from_uri_and_mime_image() {
        let content =
            content_from_uri_and_mime("files/abc123".to_string(), "image/png".to_string());
        match content {
            InteractionContent::Image { uri, mime_type, .. } => {
                assert_eq!(uri, Some("files/abc123".to_string()));
                assert_eq!(mime_type, Some("image/png".to_string()));
            }
            _ => panic!("Expected Image variant for image/* MIME type"),
        }
    }

    #[test]
    fn test_content_from_uri_and_mime_audio() {
        let content =
            content_from_uri_and_mime("files/audio456".to_string(), "audio/mp3".to_string());
        match content {
            InteractionContent::Audio { uri, mime_type, .. } => {
                assert_eq!(uri, Some("files/audio456".to_string()));
                assert_eq!(mime_type, Some("audio/mp3".to_string()));
            }
            _ => panic!("Expected Audio variant for audio/* MIME type"),
        }
    }

    #[test]
    fn test_content_from_uri_and_mime_video() {
        let content =
            content_from_uri_and_mime("files/video789".to_string(), "video/mp4".to_string());
        match content {
            InteractionContent::Video { uri, mime_type, .. } => {
                assert_eq!(uri, Some("files/video789".to_string()));
                assert_eq!(mime_type, Some("video/mp4".to_string()));
            }
            _ => panic!("Expected Video variant for video/* MIME type"),
        }
    }

    #[test]
    fn test_content_from_uri_and_mime_document_pdf() {
        let content =
            content_from_uri_and_mime("files/doc123".to_string(), "application/pdf".to_string());
        match content {
            InteractionContent::Document { uri, mime_type, .. } => {
                assert_eq!(uri, Some("files/doc123".to_string()));
                assert_eq!(mime_type, Some("application/pdf".to_string()));
            }
            _ => panic!("Expected Document variant for application/pdf"),
        }
    }

    #[test]
    fn test_content_from_uri_and_mime_text_routes_to_document() {
        // text/* MIME types should route to Document variant
        let content =
            content_from_uri_and_mime("files/text123".to_string(), "text/plain".to_string());
        match content {
            InteractionContent::Document { uri, mime_type, .. } => {
                assert_eq!(uri, Some("files/text123".to_string()));
                assert_eq!(mime_type, Some("text/plain".to_string()));
            }
            _ => panic!("Expected Document variant for text/plain"),
        }

        // text/markdown should also route to Document
        let content =
            content_from_uri_and_mime("files/md456".to_string(), "text/markdown".to_string());
        match content {
            InteractionContent::Document { uri, mime_type, .. } => {
                assert_eq!(uri, Some("files/md456".to_string()));
                assert_eq!(mime_type, Some("text/markdown".to_string()));
            }
            _ => panic!("Expected Document variant for text/markdown"),
        }
    }
}
