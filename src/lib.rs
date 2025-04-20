use reqwest::Client as ReqwestClient; // Alias to avoid name clash

// Re-export the unified error type
pub use genai_client::GenaiError;
// Re-export the response type needed for streaming result
pub use genai_client::GenerateContentResponse;

// Define the main client struct
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
    ) -> Result<String, GenaiError> {
        // Use GenaiError
        // Call the internal helper function
        genai_client::generate_content_internal(
            &self.http_client,
            &self.api_key,
            model_name,
            prompt_text,
        )
        .await
    }

    /// Generates content as a stream based on a prompt.
    pub fn generate_content_stream<'a>(
        &'a self,
        model_name: &'a str,
        prompt_text: &'a str,
    ) -> impl futures_util::Stream<Item = Result<GenerateContentResponse, GenaiError>> + Send + 'a
    {
        // Call the internal helper function
        genai_client::generate_content_stream_internal(
            &self.http_client,
            &self.api_key,
            model_name,
            prompt_text,
        )
    }
}

// Remove old free function re-exports (will add methods to Client instead)
// pub use genai_client::generate_content;
// pub use genai_client::generate_content_stream;

// You can add higher-level wrapper functions or structs here later.
