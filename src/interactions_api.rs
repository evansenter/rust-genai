/// Helper functions for constructing model output content (for testing).
///
/// # User Input Content
///
/// For content YOU send to the API, use [`InteractionContent`] methods directly:
/// - [`InteractionContent::new_text()`] for text
/// - [`InteractionContent::new_function_result()`] for function results
/// - [`InteractionContent::new_image_data()`], etc. for multimodal content
///
/// # Model Output Constructors
///
/// This module provides constructors for content the MODEL generates.
/// These are useful for testing and roundtrip verification:
/// - **Code Execution**: `code_execution_call_content`, `code_execution_result_content`
/// - **Google Search**: `google_search_call_content`, `google_search_result_content`
/// - **File Search**: `file_search_result_content`
/// - **URL Context**: `url_context_call_content`, `url_context_result_content`
///
/// In production, access these via response methods (e.g., `response.code_execution_results()`)
/// rather than constructing them directly.
use crate::{
    CodeExecutionLanguage, CodeExecutionOutcome, FileSearchResultItem, GoogleSearchResultItem,
    InteractionContent,
};

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
}
