use thiserror::Error;

pub use genai_client::ApiVersion;

pub mod types;
pub use types::{CodeExecutionResult, FunctionCall, FunctionDeclaration, GenerateContentResponse};

pub mod content_api;
pub use content_api::{
    build_content_request, model_function_call, model_function_calls_request, model_text, user_text, user_tool_response,
};

pub mod client;
pub use client::{Client, ClientBuilder};

pub mod request_builder;
pub use request_builder::GenerateContentBuilder;

pub(crate) mod internal;

pub mod function_calling;
// Re-export public types from function_calling module
pub use function_calling::{CallableFunction, FunctionError};

/// Defines errors that can occur when interacting with the `GenAI` API.
#[derive(Debug, Error)]
pub enum GenaiError {
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("SSE parsing error: {0}")]
    Parse(String),
    #[error("JSON deserialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("UTF-8 decoding error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("API Error returned by Google: {0}")]
    Api(String),
    #[error("Internal client error: {0}")]
    Internal(String),
}

// Implement conversion from internal error to public error
impl From<genai_client::InternalError> for GenaiError {
    fn from(internal_err: genai_client::InternalError) -> Self {
        match internal_err {
            genai_client::InternalError::Http(e) => Self::Http(e),
            genai_client::InternalError::Parse(s) => Self::Parse(s),
            genai_client::InternalError::Json(e) => Self::Json(e),
            genai_client::InternalError::Utf8(e) => Self::Utf8(e),
            genai_client::InternalError::Api(s) => Self::Api(s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*; // This will bring in Client, GenerateContentResponse, etc.
    use genai_client::InternalError;

    #[test]
    fn test_internal_error_to_genai_error_conversion() {
        // Test Parse variant
        let internal_parse = InternalError::Parse("parse error".to_string());
        let public_parse: GenaiError = internal_parse.into();
        assert!(matches!(public_parse, GenaiError::Parse(s) if s == "parse error"));

        // Test Http variant - we'll skip this test since creating a reqwest::Error is complex
        // and the #[from] attribute is well-tested in the reqwest crate itself
        // If we need to test this in the future, we can use a mock HTTP client

        // Test Json variant
        let invalid_json = "{invalid json";
        let json_error = serde_json::from_str::<serde_json::Value>(invalid_json).unwrap_err();
        let internal_json = InternalError::Json(json_error);
        let public_json: GenaiError = internal_json.into();
        assert!(matches!(public_json, GenaiError::Json(_)));

        // Test Utf8 variant - using a dynamic approach to create invalid UTF-8
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"valid");
        bytes.push(0xFF); // Add an invalid byte
        let utf8_error = std::str::from_utf8(&bytes).unwrap_err();
        let internal_utf8 = InternalError::Utf8(utf8_error);
        let public_utf8: GenaiError = internal_utf8.into();
        assert!(matches!(public_utf8, GenaiError::Utf8(_)));

        // Test Api variant
        let internal_api = InternalError::Api("api error".to_string());
        let public_api: GenaiError = internal_api.into();
        assert!(matches!(public_api, GenaiError::Api(s) if s == "api error"));
    }

    #[test]
    fn test_public_response_struct() {
        let response = GenerateContentResponse {
            text: Some("test".to_string()),
            function_calls: None,
            code_execution_results: None,
        };
        assert_eq!(response.text.as_deref(), Some("test"));
        assert!(response.function_calls.is_none());

        let fc = FunctionCall {
            name: "test_function".to_string(),
            args: serde_json::json!({ "arg": "value" }),
        };
        let response = GenerateContentResponse {
            text: None,
            function_calls: Some(vec![fc]),
            code_execution_results: None,
        };
        assert!(response.text.is_none());
        assert_eq!(
            response
                .function_calls
                .as_ref()
                .unwrap()
                .first()
                .unwrap()
                .name,
            "test_function"
        );
        assert_eq!(
            response
                .function_calls
                .as_ref()
                .unwrap()
                .first()
                .unwrap()
                .args,
            serde_json::json!({ "arg": "value" })
        );
    }
}
