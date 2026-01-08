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
#[derive(Clone)]
pub struct Client {
    pub(crate) api_key: String,
    #[allow(clippy::struct_field_names)]
    pub(crate) http_client: ReqwestClient,
}

// Custom Debug implementation that redacts the API key for security.
// This prevents accidental exposure of credentials in logs, error messages, or debug output.
impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("api_key", &"[REDACTED]")
            .field("http_client", &self.http_client)
            .finish()
    }
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
///     .with_timeout(Duration::from_secs(120))
///     .with_connect_timeout(Duration::from_secs(10))
///     .build()?;
/// # Ok::<(), rust_genai::GenaiError>(())
/// ```
pub struct ClientBuilder {
    api_key: String,
    timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
}

// Custom Debug implementation that redacts the API key for security.
impl std::fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientBuilder")
            .field("api_key", &"[REDACTED]")
            .field("timeout", &self.timeout)
            .field("connect_timeout", &self.connect_timeout)
            .finish()
    }
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
    /// If not set, requests will wait indefinitely (no timeout).
    /// Connection-level timeouts like TCP keepalive may still apply at the OS level.
    ///
    /// # Example
    ///
    /// ```
    /// use rust_genai::Client;
    /// use std::time::Duration;
    ///
    /// let client = Client::builder("api_key".to_string())
    ///     .with_timeout(Duration::from_secs(120))
    ///     .build()?;
    /// # Ok::<(), rust_genai::GenaiError>(())
    /// ```
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the connection timeout.
    ///
    /// This is the maximum time to wait for establishing a connection to the server.
    /// A shorter timeout here can help fail fast if the network is unavailable.
    ///
    /// If not set, the connection phase will wait indefinitely (no timeout).
    ///
    /// # Example
    ///
    /// ```
    /// use rust_genai::Client;
    /// use std::time::Duration;
    ///
    /// let client = Client::builder("api_key".to_string())
    ///     .with_connect_timeout(Duration::from_secs(10))
    ///     .build()?;
    /// # Ok::<(), rust_genai::GenaiError>(())
    /// ```
    #[must_use]
    pub const fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Builds the `Client`.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying HTTP client cannot be constructed. This should only
    /// happen in exceptional circumstances such as TLS backend initialization failures.
    pub fn build(self) -> Result<Client, GenaiError> {
        let mut builder = ReqwestClient::builder();

        if let Some(timeout) = self.timeout {
            builder = builder.timeout(timeout);
        }

        if let Some(connect_timeout) = self.connect_timeout {
            builder = builder.connect_timeout(connect_timeout);
        }

        let http_client = builder
            .build()
            .map_err(|e| GenaiError::ClientBuild(e.to_string()))?;

        Ok(Client {
            api_key: self.api_key,
            http_client,
        })
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
    /// let client = Client::builder("api_key".to_string()).build()?;
    ///
    /// // Simple interaction
    /// let response = client.interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Hello, world!")
    ///     .create()
    ///     .await?;
    ///
    /// // Stateful conversation (requires stored interaction)
    /// let response2 = client.interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("What did I just say?")
    ///     .with_previous_interaction(response.id.as_ref().expect("stored interaction has id"))
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
    /// use rust_genai::{CreateInteractionRequest, InteractionInput};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("your-api-key".to_string());
    ///
    /// let request = CreateInteractionRequest {
    ///     model: Some("gemini-3-flash-preview".to_string()),
    ///     agent: None,
    ///     agent_config: None,
    ///     input: InteractionInput::Text("Hello, world!".to_string()),
    ///     previous_interaction_id: None,
    ///     tools: None,
    ///     response_modalities: None,
    ///     response_format: None,
    ///     response_mime_type: None,
    ///     generation_config: None,
    ///     stream: None,
    ///     background: None,
    ///     store: None,
    ///     system_instruction: None,
    /// };
    ///
    /// let response = client.create_interaction(request).await?;
    /// println!("Interaction ID: {:?}", response.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_interaction(
        &self,
        request: crate::CreateInteractionRequest,
    ) -> Result<crate::InteractionResponse, GenaiError> {
        log::debug!("Creating interaction");
        log_request_body(&request);

        let response = crate::http::interactions::create_interaction(
            &self.http_client,
            &self.api_key,
            request,
        )
        .await?;

        log::debug!("Interaction created: ID={:?}", response.id);

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
    /// use rust_genai::{CreateInteractionRequest, InteractionInput};
    /// use futures_util::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder("api_key".to_string()).build()?;
    /// let request = CreateInteractionRequest {
    ///     model: Some("gemini-3-flash-preview".to_string()),
    ///     agent: None,
    ///     agent_config: None,
    ///     input: InteractionInput::Text("Count to 5".to_string()),
    ///     previous_interaction_id: None,
    ///     tools: None,
    ///     response_modalities: None,
    ///     response_format: None,
    ///     response_mime_type: None,
    ///     generation_config: None,
    ///     stream: Some(true),
    ///     background: None,
    ///     store: None,
    ///     system_instruction: None,
    /// };
    ///
    /// let mut last_event_id = None;
    /// let mut stream = client.create_interaction_stream(request);
    /// while let Some(result) = stream.next().await {
    ///     let event = result?;
    ///     last_event_id = event.event_id.clone();  // Track for resume
    ///     match event.chunk {
    ///         StreamChunk::Delta(delta) => {
    ///             if let Some(text) = delta.text() {
    ///                 print!("{}", text);
    ///             }
    ///         }
    ///         StreamChunk::Complete(response) => {
    ///             println!("\nDone! ID: {:?}", response.id);
    ///         }
    ///         _ => {} // Handle unknown future variants
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_interaction_stream(
        &self,
        request: crate::CreateInteractionRequest,
    ) -> futures_util::stream::BoxStream<'_, Result<crate::StreamEvent, GenaiError>> {
        use futures_util::StreamExt;

        log::debug!("Creating streaming interaction");
        log_request_body(&request);

        let stream = crate::http::interactions::create_interaction_stream(
            &self.http_client,
            &self.api_key,
            request,
        );

        stream
            .map(move |result| {
                result.inspect(|event| {
                    log::debug!(
                        "Received stream event: chunk={:?}, event_id={:?}",
                        event.chunk,
                        event.event_id
                    );
                })
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
    ) -> Result<crate::InteractionResponse, GenaiError> {
        log::debug!("Getting interaction: ID={interaction_id}");

        let response = crate::http::interactions::get_interaction(
            &self.http_client,
            &self.api_key,
            interaction_id,
        )
        .await?;

        log::debug!("Retrieved interaction: status={:?}", response.status);

        Ok(response)
    }

    /// Retrieves an existing interaction by its ID with streaming.
    ///
    /// Returns a stream of events for the interaction. This is useful for:
    /// - Resuming an interrupted stream using `last_event_id`
    /// - Streaming a long-running interaction's progress (e.g., deep research)
    ///
    /// Each event includes an `event_id` that can be used to resume the stream
    /// from that point if the connection is interrupted.
    ///
    /// # Arguments
    ///
    /// * `interaction_id` - The unique identifier of the interaction to stream.
    /// * `last_event_id` - Optional event ID to resume from. Pass the last received
    ///   event's `event_id` to continue from where you left off.
    ///
    /// # Returns
    /// A boxed stream that yields `StreamEvent` items.
    ///
    /// # Example
    /// ```no_run
    /// use rust_genai::{Client, StreamChunk};
    /// use futures_util::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder("api_key".to_string()).build()?;
    /// let interaction_id = "some-interaction-id";
    ///
    /// // Resume a stream from a previous event
    /// let last_event_id = Some("evt_abc123");
    /// let mut stream = client.get_interaction_stream(interaction_id, last_event_id);
    ///
    /// while let Some(result) = stream.next().await {
    ///     let event = result?;
    ///     println!("Event ID: {:?}", event.event_id);
    ///     match event.chunk {
    ///         StreamChunk::Delta(delta) => {
    ///             if let Some(text) = delta.text() {
    ///                 print!("{}", text);
    ///             }
    ///         }
    ///         StreamChunk::Complete(response) => {
    ///             println!("\nDone! Status: {:?}", response.status);
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_interaction_stream<'a>(
        &'a self,
        interaction_id: &'a str,
        last_event_id: Option<&'a str>,
    ) -> futures_util::stream::BoxStream<'a, Result<crate::StreamEvent, GenaiError>> {
        use futures_util::StreamExt;

        log::debug!(
            "Getting interaction stream: ID={}, resume_from={:?}",
            interaction_id,
            last_event_id
        );

        let stream = crate::http::interactions::get_interaction_stream(
            &self.http_client,
            &self.api_key,
            interaction_id,
            last_event_id,
        );

        stream
            .map(move |result| {
                result.inspect(|event| {
                    log::debug!(
                        "Received stream event: chunk={:?}, event_id={:?}",
                        event.chunk,
                        event.event_id
                    );
                })
            })
            .boxed()
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

        crate::http::interactions::delete_interaction(
            &self.http_client,
            &self.api_key,
            interaction_id,
        )
        .await?;

        log::debug!("Interaction deleted successfully");

        Ok(())
    }

    /// Cancels an in-progress background interaction.
    ///
    /// Only applicable to interactions created with `background: true` that are
    /// still in `InProgress` status. Returns the updated interaction with
    /// status `Cancelled`.
    ///
    /// This is useful for:
    /// - Halting long-running agent tasks (e.g., deep-research) when requirements change
    /// - Cost control by stopping interactions consuming significant tokens
    /// - Implementing timeout handling in application logic
    /// - Supporting user-initiated cancellation in UIs
    ///
    /// # Arguments
    ///
    /// * `interaction_id` - The unique identifier of the interaction to cancel.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The interaction doesn't exist
    /// - The interaction is not in a cancellable state (not background or already complete)
    /// - The HTTP request fails
    /// - The API returns an error
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::{Client, InteractionStatus};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("your-api-key".to_string());
    ///
    /// // Start a background agent interaction
    /// let response = client.interaction()
    ///     .with_agent("deep-research-pro-preview-12-2025")
    ///     .with_text("Research AI safety")
    ///     .with_background(true)
    ///     .with_store_enabled()
    ///     .create()
    ///     .await?;
    ///
    /// let interaction_id = response.id.as_ref().expect("stored interaction has id");
    ///
    /// // Later, cancel if still in progress
    /// if response.status == InteractionStatus::InProgress {
    ///     let cancelled = client.cancel_interaction(interaction_id).await?;
    ///     assert_eq!(cancelled.status, InteractionStatus::Cancelled);
    ///     println!("Interaction cancelled");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel_interaction(
        &self,
        interaction_id: &str,
    ) -> Result<crate::InteractionResponse, GenaiError> {
        log::debug!("Cancelling interaction: ID={interaction_id}");

        let response = crate::http::interactions::cancel_interaction(
            &self.http_client,
            &self.api_key,
            interaction_id,
        )
        .await?;

        log::debug!("Interaction cancelled: status={:?}", response.status);

        Ok(response)
    }

    // --- Files API methods ---

    /// Uploads a file from a path to the Files API.
    ///
    /// Files are stored for 48 hours and can be referenced in interactions by their URI.
    /// This is more efficient than inline base64 encoding for large files or files
    /// that will be used across multiple interactions.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to upload
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The MIME type cannot be determined
    /// - The upload fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// // Upload a video file
    /// let file = client.upload_file("video.mp4").await?;
    /// println!("Uploaded: {} -> {}", file.name, file.uri);
    ///
    /// // Use in interaction
    /// let response = client.interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_file(&file)
    ///     .with_text("Describe this video")
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn upload_file(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<crate::FileMetadata, GenaiError> {
        let path = path.as_ref();

        // Read file contents
        let file_data = tokio::fs::read(path).await.map_err(|e| {
            log::warn!("Failed to read file '{}': {}", path.display(), e);
            GenaiError::InvalidInput(format!("Failed to read file '{}': {}", path.display(), e))
        })?;

        // Detect MIME type from extension
        let mime_type = crate::multimodal::detect_mime_type(path).ok_or_else(|| {
            log::warn!(
                "Could not determine MIME type for '{}' - unknown extension",
                path.display()
            );
            GenaiError::InvalidInput(format!(
                "Could not determine MIME type for '{}'. Please use upload_file_with_mime() to specify explicitly.",
                path.display()
            ))
        })?;

        // Use filename as display name
        let display_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());

        log::debug!(
            "Uploading file: path={}, size={} bytes, mime_type={}",
            path.display(),
            file_data.len(),
            mime_type
        );

        crate::http::files::upload_file(
            &self.http_client,
            &self.api_key,
            file_data,
            mime_type,
            display_name.as_deref(),
        )
        .await
    }

    /// Uploads a file with an explicit MIME type.
    ///
    /// Use this when automatic MIME type detection isn't suitable.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to upload
    /// * `mime_type` - MIME type of the file (e.g., "video/mp4")
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// let file = client.upload_file_with_mime("data.bin", "application/octet-stream").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn upload_file_with_mime(
        &self,
        path: impl AsRef<std::path::Path>,
        mime_type: &str,
    ) -> Result<crate::FileMetadata, GenaiError> {
        let path = path.as_ref();

        let file_data = tokio::fs::read(path).await.map_err(|e| {
            log::warn!("Failed to read file '{}': {}", path.display(), e);
            GenaiError::InvalidInput(format!("Failed to read file '{}': {}", path.display(), e))
        })?;

        let display_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());

        log::debug!(
            "Uploading file: path={}, size={} bytes, mime_type={}",
            path.display(),
            file_data.len(),
            mime_type
        );

        crate::http::files::upload_file(
            &self.http_client,
            &self.api_key,
            file_data,
            mime_type,
            display_name.as_deref(),
        )
        .await
    }

    /// Uploads file bytes directly with a specified MIME type.
    ///
    /// Use this when you already have file contents in memory.
    ///
    /// # Arguments
    ///
    /// * `data` - File contents as bytes
    /// * `mime_type` - MIME type of the file
    /// * `display_name` - Optional display name for the file
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// // Upload bytes from memory
    /// let video_bytes = std::fs::read("video.mp4")?;
    /// let file = client.upload_file_bytes(video_bytes, "video/mp4", Some("my-video")).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn upload_file_bytes(
        &self,
        data: Vec<u8>,
        mime_type: &str,
        display_name: Option<&str>,
    ) -> Result<crate::FileMetadata, GenaiError> {
        log::debug!(
            "Uploading file bytes: size={} bytes, mime_type={}, display_name={:?}",
            data.len(),
            mime_type,
            display_name
        );

        crate::http::files::upload_file(
            &self.http_client,
            &self.api_key,
            data,
            mime_type,
            display_name,
        )
        .await
    }

    /// Gets metadata for an uploaded file.
    ///
    /// Use this to check the processing status of a recently uploaded file.
    ///
    /// # Arguments
    ///
    /// * `file_name` - The resource name of the file (e.g., "files/abc123")
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// let file = client.get_file("files/abc123").await?;
    /// if file.is_active() {
    ///     println!("File is ready to use");
    /// } else if file.is_processing() {
    ///     println!("File is still processing...");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_file(&self, file_name: &str) -> Result<crate::FileMetadata, GenaiError> {
        crate::http::files::get_file(&self.http_client, &self.api_key, file_name).await
    }

    /// Lists all uploaded files.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// let response = client.list_files(None, None).await?;
    /// for file in response.files {
    ///     println!("{}: {} ({})", file.name, file.display_name.as_deref().unwrap_or(""), file.mime_type);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_files(
        &self,
        page_size: Option<u32>,
        page_token: Option<&str>,
    ) -> Result<crate::ListFilesResponse, GenaiError> {
        crate::http::files::list_files(&self.http_client, &self.api_key, page_size, page_token)
            .await
    }

    /// Deletes an uploaded file.
    ///
    /// # Arguments
    ///
    /// * `file_name` - The resource name of the file to delete (e.g., "files/abc123")
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// // Upload, use, then delete
    /// let file = client.upload_file("video.mp4").await?;
    /// // ... use in interactions ...
    /// client.delete_file(&file.name).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete_file(&self, file_name: &str) -> Result<(), GenaiError> {
        crate::http::files::delete_file(&self.http_client, &self.api_key, file_name).await
    }

    /// Uploads a file using chunked transfer to minimize memory usage.
    ///
    /// Unlike `upload_file`, this method streams the file from disk in chunks,
    /// never loading the entire file into memory. This is ideal for large files
    /// (500MB-2GB) or memory-constrained environments.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to upload
    ///
    /// # Returns
    ///
    /// Returns a tuple of:
    /// - `FileMetadata`: The uploaded file's metadata
    /// - `ResumableUpload`: A handle that can be used to resume if the upload is interrupted
    ///
    /// # Memory Usage
    ///
    /// This method uses approximately 8MB of memory for buffering, regardless of
    /// the file size. A 2GB file uses the same memory as a 10MB file.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The MIME type cannot be determined
    /// - The upload fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// // Upload a large video file without loading it all into memory
    /// let (file, _upload_handle) = client.upload_file_chunked("large_video.mp4").await?;
    /// println!("Uploaded: {} -> {}", file.name, file.uri);
    ///
    /// // Use in interaction
    /// let response = client.interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_file(&file)
    ///     .with_text("Describe this video")
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn upload_file_chunked(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(crate::FileMetadata, crate::ResumableUpload), GenaiError> {
        let path = path.as_ref();

        // Detect MIME type from extension
        let mime_type = crate::multimodal::detect_mime_type(path).ok_or_else(|| {
            log::warn!(
                "Could not determine MIME type for '{}' - unknown extension",
                path.display()
            );
            GenaiError::InvalidInput(format!(
                "Could not determine MIME type for '{}'. Please use upload_file_chunked_with_mime() to specify explicitly.",
                path.display()
            ))
        })?;

        // Use filename as display name
        let display_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());

        log::debug!(
            "Chunked upload: path={}, mime_type={}",
            path.display(),
            mime_type
        );

        crate::http::files::upload_file_chunked(
            &self.http_client,
            &self.api_key,
            path,
            mime_type,
            display_name.as_deref(),
        )
        .await
    }

    /// Uploads a file using chunked transfer with an explicit MIME type.
    ///
    /// Use this when automatic MIME type detection isn't suitable.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to upload
    /// * `mime_type` - MIME type of the file (e.g., "video/mp4")
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// let (file, _) = client.upload_file_chunked_with_mime(
    ///     "data.bin",
    ///     "application/octet-stream"
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn upload_file_chunked_with_mime(
        &self,
        path: impl AsRef<std::path::Path>,
        mime_type: &str,
    ) -> Result<(crate::FileMetadata, crate::ResumableUpload), GenaiError> {
        let path = path.as_ref();

        let display_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());

        log::debug!(
            "Chunked upload: path={}, mime_type={}",
            path.display(),
            mime_type
        );

        crate::http::files::upload_file_chunked(
            &self.http_client,
            &self.api_key,
            path,
            mime_type,
            display_name.as_deref(),
        )
        .await
    }

    /// Uploads a file using chunked transfer with a custom chunk size.
    ///
    /// This is the same as `upload_file_chunked_with_mime` but allows
    /// specifying the chunk size for streaming. Larger chunks are more
    /// efficient for fast networks, while smaller chunks use less memory.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to upload
    /// * `mime_type` - MIME type of the file
    /// * `chunk_size` - Size of chunks to stream in bytes (default: 8MB)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// // Use 16MB chunks for faster upload on a fast network
    /// let chunk_size = 16 * 1024 * 1024;
    /// let (file, _) = client.upload_file_chunked_with_options(
    ///     "large_video.mp4",
    ///     "video/mp4",
    ///     chunk_size
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn upload_file_chunked_with_options(
        &self,
        path: impl AsRef<std::path::Path>,
        mime_type: &str,
        chunk_size: usize,
    ) -> Result<(crate::FileMetadata, crate::ResumableUpload), GenaiError> {
        let path = path.as_ref();

        let display_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());

        log::debug!(
            "Chunked upload: path={}, mime_type={}, chunk_size={}",
            path.display(),
            mime_type,
            chunk_size
        );

        crate::http::files::upload_file_chunked_with_chunk_size(
            &self.http_client,
            &self.api_key,
            path,
            mime_type,
            display_name.as_deref(),
            chunk_size,
        )
        .await
    }

    /// Waits for a file to finish processing.
    ///
    /// Some files (especially videos) require processing before they can be used.
    /// This method polls the file status until it becomes active or fails.
    ///
    /// # Arguments
    ///
    /// * `file` - The file metadata to wait for
    /// * `poll_interval` - How often to check the status
    /// * `timeout` - Maximum time to wait
    ///
    /// # Returns
    ///
    /// Returns the updated file metadata when processing completes.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file processing fails
    /// - The timeout is exceeded
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    /// use std::time::Duration;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// let file = client.upload_file("large_video.mp4").await?;
    ///
    /// // Wait for processing to complete
    /// let ready_file = client.wait_for_file_ready(
    ///     &file,
    ///     Duration::from_secs(2),
    ///     Duration::from_secs(120)
    /// ).await?;
    ///
    /// println!("File ready: {}", ready_file.uri);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn wait_for_file_ready(
        &self,
        file: &crate::FileMetadata,
        poll_interval: std::time::Duration,
        timeout: std::time::Duration,
    ) -> Result<crate::FileMetadata, GenaiError> {
        use std::time::Instant;

        let start = Instant::now();

        loop {
            let current = self.get_file(&file.name).await?;

            if current.is_active() {
                return Ok(current);
            }

            if current.is_failed() {
                let error_code = current.error.as_ref().and_then(|e| e.code);
                let error_msg = current
                    .error
                    .as_ref()
                    .and_then(|e| e.message.as_deref())
                    .unwrap_or("File processing failed without details");

                log::error!(
                    "File '{}' processing failed: code={:?}, message={}",
                    file.name,
                    error_code,
                    error_msg
                );

                // Use Api error since this is a server-side processing failure
                return Err(GenaiError::Api {
                    status_code: error_code.map_or(500, |c| c as u16),
                    message: format!("File processing failed: {}", error_msg),
                    request_id: None,
                });
            }

            // Log unknown states per Evergreen logging strategy
            if let Some(state) = &current.state
                && state.is_unknown()
            {
                log::warn!(
                    "File '{}' is in unknown state {:?}, continuing to poll. \
                     This may indicate API evolution - consider updating rust-genai.",
                    file.name,
                    state
                );
            }

            if start.elapsed() > timeout {
                // Use Internal error since this is an operational issue, not invalid input
                let state_info = current
                    .state
                    .as_ref()
                    .map(|s| format!("{:?}", s))
                    .unwrap_or_else(|| "unknown".to_string());
                return Err(GenaiError::Internal(format!(
                    "Timeout waiting for file '{}' to be ready (waited {:?}, last state: {}). \
                     The file may still be processing - try again with a longer timeout.",
                    file.name,
                    start.elapsed(),
                    state_info
                )));
            }

            log::debug!(
                "File '{}' still processing, waiting {:?}...",
                file.name,
                poll_interval
            );
            tokio::time::sleep(poll_interval).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder_default() {
        let client = Client::builder("test_key".to_string()).build().unwrap();
        assert_eq!(client.api_key, "test_key");
    }

    #[test]
    fn test_client_builder_with_timeout() {
        let client = Client::builder("test_key".to_string())
            .with_timeout(Duration::from_secs(120))
            .build()
            .unwrap();
        assert_eq!(client.api_key, "test_key");
        // Note: We can't easily inspect the reqwest client's timeout,
        // but this test verifies the builder chain works
    }

    #[test]
    fn test_client_builder_with_connect_timeout() {
        let client = Client::builder("test_key".to_string())
            .with_connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap();
        assert_eq!(client.api_key, "test_key");
    }

    #[test]
    fn test_client_builder_with_both_timeouts() {
        let client = Client::builder("test_key".to_string())
            .with_timeout(Duration::from_secs(120))
            .with_connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap();
        assert_eq!(client.api_key, "test_key");
    }

    #[test]
    fn test_client_new() {
        let client = Client::new("test_key".to_string());
        assert_eq!(client.api_key, "test_key");
    }

    #[test]
    fn test_client_debug_redacts_api_key() {
        let client = Client::new("super_secret_api_key_12345".to_string());
        let debug_output = format!("{:?}", client);

        // API key should NOT appear in debug output
        assert!(
            !debug_output.contains("super_secret_api_key_12345"),
            "API key was exposed in debug output: {}",
            debug_output
        );
        // Should show [REDACTED] instead
        assert!(
            debug_output.contains("[REDACTED]"),
            "Debug output should contain [REDACTED]: {}",
            debug_output
        );
    }

    #[test]
    fn test_client_builder_returns_result() {
        let result = Client::builder("test_key".to_string()).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_client_builder_debug_redacts_api_key() {
        let builder = Client::builder("another_secret_key_67890".to_string())
            .with_timeout(Duration::from_secs(60));
        let debug_output = format!("{:?}", builder);

        // API key should NOT appear in debug output
        assert!(
            !debug_output.contains("another_secret_key_67890"),
            "API key was exposed in builder debug output: {}",
            debug_output
        );
        // Should show [REDACTED] instead
        assert!(
            debug_output.contains("[REDACTED]"),
            "Builder debug output should contain [REDACTED]: {}",
            debug_output
        );
    }

    #[tokio::test]
    async fn test_upload_file_unknown_extension_error() {
        let client = Client::new("test_key".to_string());

        // Create a temp file with an unknown extension
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("data.xyz");
        std::fs::write(&file_path, b"test data").unwrap();

        // upload_file should fail with InvalidInput for unknown MIME type
        let result = client.upload_file(&file_path).await;
        assert!(result.is_err(), "Should fail for unknown extension");

        let err = result.unwrap_err();
        let err_string = err.to_string();
        assert!(
            err_string.contains("Could not determine MIME type"),
            "Error should mention MIME type issue: {}",
            err_string
        );
        assert!(
            err_string.contains("data.xyz"),
            "Error should include filename: {}",
            err_string
        );
    }

    #[tokio::test]
    async fn test_upload_file_nonexistent_file_error() {
        let client = Client::new("test_key".to_string());

        // Try to upload a file that doesn't exist
        let result = client.upload_file("/nonexistent/path/to/file.txt").await;
        assert!(result.is_err(), "Should fail for nonexistent file");

        let err = result.unwrap_err();
        let err_string = err.to_string();
        assert!(
            err_string.contains("Failed to read file"),
            "Error should mention file read failure: {}",
            err_string
        );
    }

    #[tokio::test]
    async fn test_upload_file_bytes_empty_file_error() {
        let client = Client::new("test_key".to_string());

        // Try to upload empty bytes
        let result = client
            .upload_file_bytes(Vec::new(), "text/plain", Some("empty.txt"))
            .await;
        assert!(result.is_err(), "Should fail for empty file");

        let err = result.unwrap_err();
        let err_string = err.to_string();
        assert!(
            err_string.contains("Cannot upload empty file"),
            "Error should mention empty file: {}",
            err_string
        );
    }

    #[tokio::test]
    async fn test_upload_file_bytes_validates_before_network() {
        // This test verifies that validation happens before any network call
        // by using an invalid API key - if we reach the network, we'd get auth error
        let client = Client::new("invalid_key".to_string());

        // Empty file should fail with validation error, not auth error
        let result = client
            .upload_file_bytes(Vec::new(), "text/plain", None)
            .await;
        assert!(result.is_err());
        let err_string = result.unwrap_err().to_string();
        assert!(
            err_string.contains("Cannot upload empty file"),
            "Should fail validation before hitting network: {}",
            err_string
        );
    }
}
