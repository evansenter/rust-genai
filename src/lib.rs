use reqwest::Client as ReqwestClient; // Alias to avoid name clash
use thiserror::Error;

/// Defines errors that can occur when interacting with the GenAI API.
#[derive(Debug, Error)]
pub enum GenaiError {
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("SSE parsing error: {0}")]
    Parse(String),
    #[error("JSON deserialization error: {0}")]
    Json(#[from] serde_json::Error), // Need serde_json dep in root crate now
    #[error("UTF-8 decoding error: {0}")]
    Utf8(#[from] std::str::Utf8Error), // Need std::str::Utf8Error for this
    #[error("API Error returned by Google: {0}")]
    Api(String),
    #[error("Internal client error: {0}")] // Variant to wrap internal errors
    Internal(String),
}

// Implement conversion from internal error to public error
impl From<genai_client::InternalError> for GenaiError {
    fn from(internal_err: genai_client::InternalError) -> Self {
        match internal_err {
            // Directly map variants where possible
            genai_client::InternalError::Http(e) => GenaiError::Http(e),
            genai_client::InternalError::Parse(s) => GenaiError::Parse(s),
            genai_client::InternalError::Json(e) => GenaiError::Json(e),
            genai_client::InternalError::Utf8(e) => GenaiError::Utf8(e),
            genai_client::InternalError::Api(s) => GenaiError::Api(s),
            // Or wrap less specific internal errors if the public enum is different
            // e.g., if InternalError had more variants than GenaiError
        }
    }
}

// Remove re-export of internal response type
// pub use genai_client::GenerateContentResponse;

/// Represents a successful response from a generate content request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateContentResponse {
    /// The generated text content.
    pub text: String,
    // TODO: Add other fields later (e.g., finish_reason, safety_ratings)
}

/// The main client for interacting with the Google Generative AI API.
#[derive(Debug, Clone)] // Add Clone if you want easy cloning
pub struct Client {
    api_key: String,
    // Store a reqwest client for connection pooling and configuration
    http_client: ReqwestClient,
}

impl Client {
    /// Creates a new GenAI client.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Your Google AI API key.
    pub fn new(api_key: String) -> Self {
        Client {
            api_key,
            http_client: ReqwestClient::new(), // Create a default reqwest client
        }
    }

    /// Generates content based on a prompt.
    pub async fn generate_content(
        &self,
        model_name: &str,
        prompt_text: &str,
    ) -> Result<GenerateContentResponse, GenaiError> {
        let internal_response_text = genai_client::generate_content_internal(
            &self.http_client,
            &self.api_key,
            model_name,
            prompt_text,
        )
        .await
        .map_err(Into::<GenaiError>::into)?;

        Ok(GenerateContentResponse {
            text: internal_response_text,
        })
    }

    /// Generates content as a stream based on a prompt.
    pub fn generate_content_stream<'a>(
        &'a self,
        model_name: &'a str,
        prompt_text: &'a str,
    ) -> impl futures_util::Stream<Item = Result<GenerateContentResponse, GenaiError>> + Send + 'a
    {
        use futures_util::TryStreamExt; // Only TryStreamExt is needed here

        genai_client::generate_content_stream_internal(
            &self.http_client,
            &self.api_key,
            model_name,
            prompt_text,
        )
        .map_err(Into::into) // From TryStreamExt
        .and_then(|internal_response| async move {
            // From TryStreamExt
            let text = internal_response
                .candidates
                .first()
                .and_then(|c| c.content.parts.first())
                .map(|p| p.text.clone())
                .unwrap_or_default(); // Handle cases where structure might be missing
            Ok(GenerateContentResponse { text })
        })
    }
}

// Remove old free function re-exports (will add methods to Client instead)
// pub use genai_client::generate_content;
// pub use genai_client::generate_content_stream;

// You can add higher-level wrapper functions or structs here later.

#[cfg(test)]
mod tests {
    use super::*;
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
        bytes.extend_from_slice("valid".as_bytes());
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
            text: "test".to_string(),
        };
        assert_eq!(response.text, "test");
    }
}
