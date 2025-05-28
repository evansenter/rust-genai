use futures_util::StreamExt;
use reqwest::Client as ReqwestClient; // Alias to avoid name clash
use thiserror::Error; // Add this import for boxed()

/// Defines errors that can occur when interacting with the `GenAI` API.
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

/// Builder for generating content with optional system instructions.
#[derive(Debug)]
pub struct GenerateContentBuilder<'a> {
    client: &'a Client,
    model_name: &'a str,
    prompt_text: Option<&'a str>,
    system_instruction: Option<&'a str>,
}

impl<'a> GenerateContentBuilder<'a> {
    /// Creates a new builder for generating content.
    fn new(client: &'a Client, model_name: &'a str) -> Self {
        Self {
            client,
            model_name,
            prompt_text: None,
            system_instruction: None,
        }
    }

    /// Sets the prompt text for the request.
    #[must_use]
    pub fn with_prompt(mut self, prompt: &'a str) -> Self {
        self.prompt_text = Some(prompt);
        self
    }

    /// Sets the system instruction for the request.
    #[must_use]
    pub fn with_system_instruction(mut self, instruction: &'a str) -> Self {
        self.system_instruction = Some(instruction);
        self
    }

    /// Generates content based on the configured parameters.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails, response parsing fails, or the API returns an error.
    pub async fn generate(self) -> Result<GenerateContentResponse, GenaiError> {
        let prompt_text = self.prompt_text.ok_or_else(|| {
            GenaiError::Internal("Prompt text is required for content generation".to_string())
        })?;

        let internal_response_text = genai_client::generate_content_internal(
            &self.client.http_client,
            &self.client.api_key,
            self.model_name,
            prompt_text,
            self.system_instruction,
        )
        .await
        .map_err(Into::<GenaiError>::into)?;

        Ok(GenerateContentResponse {
            text: internal_response_text,
        })
    }

    /// Generates content as a stream based on the configured parameters.
    pub fn stream(
        self,
    ) -> impl futures_util::Stream<Item = Result<GenerateContentResponse, GenaiError>> + Send + 'a
    {
        use futures_util::TryStreamExt;

        let Some(prompt_text) = self.prompt_text else {
            return futures_util::stream::once(async move {
                Err(GenaiError::Internal(
                    "Prompt text is required for content generation".to_string(),
                ))
            })
            .boxed();
        };

        genai_client::generate_content_stream_internal(
            &self.client.http_client,
            &self.client.api_key,
            self.model_name,
            prompt_text,
            self.system_instruction,
        )
        .map_err(Into::into)
        .and_then(|internal_response| async move {
            let text = internal_response
                .candidates
                .first()
                .and_then(|c| c.content.parts.first())
                .map(|p| p.text.clone())
                .unwrap_or_default();
            Ok(GenerateContentResponse { text })
        })
        .boxed()
    }
}

impl Client {
    /// Creates a new `GenAI` client.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Your Google AI API key.
    #[must_use]
    pub fn new(api_key: String) -> Self {
        Client {
            api_key,
            http_client: ReqwestClient::new(),
        }
    }

    /// Starts building a content generation request.
    ///
    /// # Arguments
    ///
    /// * `model_name` - The name of the model to use (e.g., "gemini-1.5-flash-latest")
    #[must_use]
    pub fn with_model<'a>(&'a self, model_name: &'a str) -> GenerateContentBuilder<'a> {
        GenerateContentBuilder::new(self, model_name)
    }
}

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
