use crate::GenaiError;
use reqwest::Client as ReqwestClient;
use std::time::Duration;

/// Logs a request body at debug level, preferring JSON format when possible.
fn log_request_body<T: std::fmt::Debug + serde::Serialize>(body: &T) {
    match serde_json::to_string_pretty(body) {
        Ok(json) => log::debug!("Request Body (JSON):\n{json}"),
        Err(_) => log::debug!("Request Body: {body:#?}"),
    }
}

/// The main client for interacting with the Google Generative AI API.
#[derive(Debug, Clone)]
pub struct Client {
    pub(crate) api_key: String,
    #[allow(clippy::struct_field_names)]
    pub(crate) http_client: ReqwestClient,
}

/// Builder for `Client` instances.
///
/// # Example
///
/// ```
/// use rust_genai::Client;
/// use std::time::Duration;
///
/// let client = Client::builder("api_key".to_string())
///     .timeout(Duration::from_secs(120))
///     .connect_timeout(Duration::from_secs(10))
///     .build();
/// ```
#[derive(Debug)]
pub struct ClientBuilder {
    api_key: String,
    timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
}

impl ClientBuilder {
    /// Sets the total request timeout.
    ///
    /// This is the maximum time a request can take from start to finish,
    /// including connection time, sending the request, and receiving the response.
    ///
    /// For LLM requests that may take a long time to generate responses,
    /// consider setting a longer timeout (e.g., 120-300 seconds).
    ///
    /// If not set, uses reqwest's default (no timeout).
    ///
    /// # Example
    ///
    /// ```
    /// use rust_genai::Client;
    /// use std::time::Duration;
    ///
    /// let client = Client::builder("api_key".to_string())
    ///     .timeout(Duration::from_secs(120))
    ///     .build();
    /// ```
    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the connection timeout.
    ///
    /// This is the maximum time to wait for establishing a connection to the server.
    /// A shorter timeout here can help fail fast if the network is unavailable.
    ///
    /// If not set, uses reqwest's default.
    ///
    /// # Example
    ///
    /// ```
    /// use rust_genai::Client;
    /// use std::time::Duration;
    ///
    /// let client = Client::builder("api_key".to_string())
    ///     .connect_timeout(Duration::from_secs(10))
    ///     .build();
    /// ```
    #[must_use]
    pub const fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Builds the `Client`.
    #[must_use]
    pub fn build(self) -> Client {
        let mut builder = ReqwestClient::builder();

        if let Some(timeout) = self.timeout {
            builder = builder.timeout(timeout);
        }

        if let Some(connect_timeout) = self.connect_timeout {
            builder = builder.connect_timeout(connect_timeout);
        }

        // This should never fail with our configuration
        let http_client = builder.build().expect("Failed to build HTTP client");

        Client {
            api_key: self.api_key,
            http_client,
        }
    }
}

impl Client {
    /// Creates a new builder for `Client` instances.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Your Google AI API key.
    #[must_use]
    pub const fn builder(api_key: String) -> ClientBuilder {
        ClientBuilder {
            api_key,
            timeout: None,
            connect_timeout: None,
        }
    }

    /// Creates a new `GenAI` client.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Your Google AI API key.
    #[must_use]
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            http_client: ReqwestClient::new(),
        }
    }

    // --- Interactions API methods ---

    /// Creates a builder for constructing an interaction request.
    ///
    /// This provides a fluent interface for building interactions with models or agents.
    /// Use this method for a more ergonomic API compared to manually constructing
    /// `CreateInteractionRequest`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use rust_genai::Client;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder("api_key".to_string()).build();
    ///
    /// // Simple interaction
    /// let response = client.interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Hello, world!")
    ///     .create()
    ///     .await?;
    ///
    /// // Stateful conversation
    /// let response2 = client.interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("What did I just say?")
    ///     .with_previous_interaction(&response.id)
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn interaction(&self) -> crate::request_builder::InteractionBuilder<'_> {
        crate::request_builder::InteractionBuilder::new(self)
    }

    /// Creates a new interaction using the Gemini Interactions API.
    ///
    /// The Interactions API provides a unified interface for working with models and agents,
    /// with built-in support for stateful conversations, function calling, and long-running tasks.
    ///
    /// # Arguments
    ///
    /// * `request` - The interaction request with model/agent, input, and optional configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The HTTP request fails
    /// - Response parsing fails
    /// - The API returns an error
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    /// use genai_client::{CreateInteractionRequest, InteractionInput};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("your-api-key".to_string());
    ///
    /// let request = CreateInteractionRequest {
    ///     model: Some("gemini-3-flash-preview".to_string()),
    ///     agent: None,
    ///     input: InteractionInput::Text("Hello, world!".to_string()),
    ///     previous_interaction_id: None,
    ///     tools: None,
    ///     response_modalities: None,
    ///     response_format: None,
    ///     generation_config: None,
    ///     stream: None,
    ///     background: None,
    ///     store: None,
    ///     system_instruction: None,
    /// };
    ///
    /// let response = client.create_interaction(request).await?;
    /// println!("Interaction ID: {}", response.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_interaction(
        &self,
        request: genai_client::CreateInteractionRequest,
    ) -> Result<genai_client::InteractionResponse, GenaiError> {
        log::debug!("Creating interaction");
        log_request_body(&request);

        let response =
            genai_client::create_interaction(&self.http_client, &self.api_key, request).await?;

        log::debug!("Interaction created: ID={}", response.id);

        Ok(response)
    }

    /// Creates a new interaction with streaming responses.
    ///
    /// Returns a stream of `StreamChunk` items as they arrive from the server.
    /// Each chunk can be either:
    /// - `StreamChunk::Delta`: Incremental content (text or thought)
    /// - `StreamChunk::Complete`: The final complete interaction response
    ///
    /// # Arguments
    ///
    /// * `request` - The interaction request with streaming enabled.
    ///
    /// # Returns
    /// A boxed stream that yields `StreamChunk` items.
    ///
    /// # Example
    /// ```no_run
    /// use rust_genai::{Client, StreamChunk};
    /// use genai_client::{CreateInteractionRequest, InteractionInput};
    /// use futures_util::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder("api_key".to_string()).build();
    /// let request = CreateInteractionRequest {
    ///     model: Some("gemini-3-flash-preview".to_string()),
    ///     agent: None,
    ///     input: InteractionInput::Text("Count to 5".to_string()),
    ///     previous_interaction_id: None,
    ///     tools: None,
    ///     response_modalities: None,
    ///     response_format: None,
    ///     generation_config: None,
    ///     stream: Some(true),
    ///     background: None,
    ///     store: None,
    ///     system_instruction: None,
    /// };
    ///
    /// let mut stream = client.create_interaction_stream(request);
    /// while let Some(chunk) = stream.next().await {
    ///     match chunk? {
    ///         StreamChunk::Delta(delta) => {
    ///             if let Some(text) = delta.text() {
    ///                 print!("{}", text);
    ///             }
    ///         }
    ///         StreamChunk::Complete(response) => {
    ///             println!("\nDone! ID: {}", response.id);
    ///         }
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_interaction_stream(
        &self,
        request: genai_client::CreateInteractionRequest,
    ) -> futures_util::stream::BoxStream<'_, Result<genai_client::StreamChunk, GenaiError>> {
        use futures_util::StreamExt;

        log::debug!("Creating streaming interaction");
        log_request_body(&request);

        let stream =
            genai_client::create_interaction_stream(&self.http_client, &self.api_key, request);

        stream
            .map(move |result| {
                result
                    .inspect(|chunk| {
                        log::debug!("Received stream chunk: {:?}", chunk);
                    })
                    .map_err(GenaiError::from)
            })
            .boxed()
    }

    /// Retrieves an existing interaction by its ID.
    ///
    /// Useful for checking the status of long-running interactions or agents,
    /// or for retrieving the full conversation history.
    ///
    /// # Arguments
    ///
    /// * `interaction_id` - The unique identifier of the interaction to retrieve.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The HTTP request fails
    /// - Response parsing fails
    /// - The API returns an error
    pub async fn get_interaction(
        &self,
        interaction_id: &str,
    ) -> Result<genai_client::InteractionResponse, GenaiError> {
        log::debug!("Getting interaction: ID={interaction_id}");

        let response =
            genai_client::get_interaction(&self.http_client, &self.api_key, interaction_id).await?;

        log::debug!("Retrieved interaction: status={:?}", response.status);

        Ok(response)
    }

    /// Deletes an interaction by its ID.
    ///
    /// Removes the interaction from the server, freeing up storage and making it
    /// unavailable for future reference via `previous_interaction_id`.
    ///
    /// # Arguments
    ///
    /// * `interaction_id` - The unique identifier of the interaction to delete.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The HTTP request fails
    /// - The API returns an error
    pub async fn delete_interaction(&self, interaction_id: &str) -> Result<(), GenaiError> {
        log::debug!("Deleting interaction: ID={interaction_id}");

        genai_client::delete_interaction(&self.http_client, &self.api_key, interaction_id).await?;

        log::debug!("Interaction deleted successfully");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder_default() {
        let client = Client::builder("test_key".to_string()).build();
        assert_eq!(client.api_key, "test_key");
    }

    #[test]
    fn test_client_builder_with_timeout() {
        let client = Client::builder("test_key".to_string())
            .timeout(Duration::from_secs(120))
            .build();
        assert_eq!(client.api_key, "test_key");
        // Note: We can't easily inspect the reqwest client's timeout,
        // but this test verifies the builder chain works
    }

    #[test]
    fn test_client_builder_with_connect_timeout() {
        let client = Client::builder("test_key".to_string())
            .connect_timeout(Duration::from_secs(10))
            .build();
        assert_eq!(client.api_key, "test_key");
    }

    #[test]
    fn test_client_builder_with_both_timeouts() {
        let client = Client::builder("test_key".to_string())
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(10))
            .build();
        assert_eq!(client.api_key, "test_key");
    }

    #[test]
    fn test_client_new() {
        let client = Client::new("test_key".to_string());
        assert_eq!(client.api_key, "test_key");
    }
}
