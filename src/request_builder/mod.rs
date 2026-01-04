mod auto_functions;

use auto_functions::DEFAULT_MAX_FUNCTION_CALL_LOOPS;

use crate::GenaiError;
use crate::client::Client;
use crate::function_calling::ToolService;
use base64::Engine;
use log::debug;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{StreamExt, stream::BoxStream};
use genai_client::{
    self, CreateInteractionRequest, FunctionCallingMode, FunctionDeclaration, GenerationConfig,
    InteractionContent, InteractionInput, InteractionResponse, StreamEvent, ThinkingLevel,
    ThinkingSummaries, Tool as InternalTool,
};

// ============================================================================
// Typestate Markers for Builder State
// ============================================================================
//
// These marker types enforce at compile time which builder methods are valid
// based on the current configuration.
//
// State Transition Diagram:
//
//                  ┌─────────────────────────────────────┐
//                  │             FirstTurn               │
//                  │    (all methods available)          │
//                  │    • with_system_instruction()      │
//                  │    • with_store_disabled()          │
//                  │    • with_previous_interaction()    │
//                  │    • with_background(true)          │
//                  └──────────────┬──────────────────────┘
//                                 │
//            ┌────────────────────┴────────────────────┐
//            │                                         │
//            ▼                                         ▼
//   ┌─────────────────────┐               ┌─────────────────────┐
//   │       Chained       │               │    StoreDisabled    │
//   │ (via prev_interact) │               │ (via store_disabled)│
//   ├─────────────────────┤               ├─────────────────────┤
//   │ ✗ system_instruction│               │ ✗ prev_interaction  │
//   │ ✗ store_disabled    │               │ ✗ background(true)  │
//   │ ✓ background(true)  │               │ ✗ auto_functions()  │
//   │ ✓ auto_functions()  │               │ ✓ system_instruction│
//   └─────────────────────┘               └─────────────────────┘
//
// Inheritance behavior with `previousInteractionId`:
// - `systemInstruction`: IS inherited (only valid on first turn)
// - `tools`: NOT inherited (must be sent on every turn)
// - conversation history: IS inherited

/// Marker type for the initial builder state.
///
/// All methods are available including:
/// - `with_system_instruction()` - set system instructions
/// - `with_previous_interaction()` - chain to previous (transitions to [`Chained`])
/// - `with_store_disabled()` - disable storage (transitions to [`StoreDisabled`])
/// - `with_background(true)` - enable background execution
#[derive(Debug, Clone, Copy)]
pub struct FirstTurn;

/// Marker type for a builder chained via `with_previous_interaction()`.
///
/// Unavailable methods:
/// - `with_system_instruction()` - system instructions are inherited
/// - `with_store_disabled()` - chained interactions require storage
#[derive(Debug, Clone, Copy)]
pub struct Chained;

/// Marker type for a builder with storage explicitly disabled via `with_store_disabled()`.
///
/// Unavailable methods:
/// - `with_previous_interaction()` - requires storage for chain context
/// - `with_background(true)` - background execution requires storage
/// - `create_with_auto_functions()` - auto-function calling requires storage
/// - `create_stream_with_auto_functions()` - auto-function calling requires storage
#[derive(Debug, Clone, Copy)]
pub struct StoreDisabled;

/// Marker trait for builder states that support auto-function calling.
///
/// This trait is implemented by [`FirstTurn`] and [`Chained`] states.
/// It is NOT implemented by [`StoreDisabled`] because auto-function calling
/// requires stored interactions to maintain conversation context across
/// multiple function execution rounds via `previous_interaction_id`.
///
/// This allows compile-time enforcement that `create_with_auto_functions()`
/// and `create_stream_with_auto_functions()` cannot be called on a builder
/// with storage disabled.
pub trait CanAutoFunction {}
impl CanAutoFunction for FirstTurn {}
impl CanAutoFunction for Chained {}

/// Builder for creating interactions with the Gemini Interactions API.
///
/// Provides a fluent interface for constructing interaction requests with models or agents.
///
/// # Type Parameter
///
/// The `State` parameter tracks whether this builder has been chained to a previous
/// interaction. This enables compile-time enforcement of API constraints:
///
/// - [`FirstTurn`]: Initial state. All methods available including `with_system_instruction()`
///   and `with_store_disabled()`.
/// - [`Chained`]: After calling `with_previous_interaction()`. The `with_system_instruction()`
///   method is not available (system instructions are inherited), and `with_store_disabled()` is
///   not available (chained interactions require storage).
///
/// # Examples
///
/// ```no_run
/// # use rust_genai::{Client, StreamChunk};
/// # use futures_util::StreamExt;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::builder("api_key".to_string()).build()?;
///
/// // Simple interaction with a model
/// let response = client.interaction()
///     .with_model("gemini-3-flash-preview")
///     .with_text("What is the capital of France?")
///     .create()
///     .await?;
///
/// // Streaming interaction
/// let mut stream = client.interaction()
///     .with_model("gemini-3-flash-preview")
///     .with_text("Count to 5")
///     .create_stream();
///
/// while let Some(result) = stream.next().await {
///     let event = result?;
///     // event.event_id can be saved for stream resume support
///     match event.chunk {
///         StreamChunk::Delta(delta) => {
///             if let Some(text) = delta.text() {
///                 print!("{}", text);
///             }
///         }
///         StreamChunk::Complete(response) => {
///             println!("\nDone!");
///         }
///         _ => {} // Handle unknown future variants
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub struct InteractionBuilder<'a, State = FirstTurn> {
    client: &'a Client,
    model: Option<String>,
    agent: Option<String>,
    input: Option<InteractionInput>,
    previous_interaction_id: Option<String>,
    tools: Option<Vec<InternalTool>>,
    response_modalities: Option<Vec<String>>,
    response_format: Option<serde_json::Value>,
    response_mime_type: Option<String>,
    generation_config: Option<GenerationConfig>,
    background: Option<bool>,
    store: Option<bool>,
    system_instruction: Option<InteractionInput>,
    /// Maximum iterations for auto function calling loop
    max_function_call_loops: usize,
    /// Tool service for dependency-injected functions
    tool_service: Option<Arc<dyn ToolService>>,
    /// Optional timeout for the request
    timeout: Option<Duration>,
    /// Phantom data for the state type parameter
    _state: PhantomData<State>,
}

impl<State> std::fmt::Debug for InteractionBuilder<'_, State> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InteractionBuilder")
            .field("model", &self.model)
            .field("agent", &self.agent)
            .field("input", &self.input)
            .field("previous_interaction_id", &self.previous_interaction_id)
            .field("tools", &self.tools)
            .field("response_modalities", &self.response_modalities)
            .field("response_format", &self.response_format)
            .field("response_mime_type", &self.response_mime_type)
            .field("generation_config", &self.generation_config)
            .field("background", &self.background)
            .field("store", &self.store)
            .field("system_instruction", &self.system_instruction)
            .field("max_function_call_loops", &self.max_function_call_loops)
            .field("tool_service", &self.tool_service.as_ref().map(|_| "..."))
            .field("timeout", &self.timeout)
            .finish()
    }
}

// ============================================================================
// Methods available only on FirstTurn (initial state)
// ============================================================================

impl<'a> InteractionBuilder<'a, FirstTurn> {
    /// Creates a new interaction builder.
    pub(crate) fn new(client: &'a Client) -> Self {
        Self {
            client,
            model: None,
            agent: None,
            input: None,
            previous_interaction_id: None,
            tools: None,
            response_modalities: None,
            response_format: None,
            response_mime_type: None,
            generation_config: None,
            background: None,
            store: None,
            system_instruction: None,
            max_function_call_loops: DEFAULT_MAX_FUNCTION_CALL_LOOPS,
            tool_service: None,
            timeout: None,
            _state: PhantomData,
        }
    }

    /// References a previous interaction for stateful conversations.
    ///
    /// The interaction will have access to the context from the previous interaction.
    ///
    /// # State Transition
    ///
    /// This method transitions the builder from [`FirstTurn`] to [`Chained`] state.
    /// After calling this method:
    /// - `with_system_instruction()` is no longer available (system instructions are inherited)
    /// - `with_store_disabled()` is no longer available (chained interactions require storage)
    pub fn with_previous_interaction(
        self,
        id: impl Into<String>,
    ) -> InteractionBuilder<'a, Chained> {
        InteractionBuilder {
            client: self.client,
            model: self.model,
            agent: self.agent,
            input: self.input,
            previous_interaction_id: Some(id.into()),
            tools: self.tools,
            response_modalities: self.response_modalities,
            response_format: self.response_format,
            response_mime_type: self.response_mime_type,
            generation_config: self.generation_config,
            background: self.background,
            store: self.store,
            system_instruction: self.system_instruction,
            max_function_call_loops: self.max_function_call_loops,
            tool_service: self.tool_service,
            timeout: self.timeout,
            _state: PhantomData,
        }
    }

    /// Sets a system instruction for the model.
    ///
    /// # Availability
    ///
    /// This method is only available on [`FirstTurn`] builders. After calling
    /// `with_previous_interaction()`, system instructions are inherited from the
    /// previous interaction and cannot be changed.
    pub fn with_system_instruction(mut self, instruction: impl Into<String>) -> Self {
        self.system_instruction = Some(InteractionInput::Text(instruction.into()));
        self
    }

    /// Explicitly disables storage for this interaction.
    ///
    /// When `store` is `false`, the interaction will not be stored and cannot be
    /// referenced by future interactions via `previousInteractionId`.
    ///
    /// # State Transition
    ///
    /// This method transitions the builder from [`FirstTurn`] to [`StoreDisabled`] state.
    /// After calling this method:
    /// - `with_previous_interaction()` is no longer available (requires storage)
    /// - `with_background(true)` is no longer available (requires storage)
    pub fn with_store_disabled(self) -> InteractionBuilder<'a, StoreDisabled> {
        InteractionBuilder {
            client: self.client,
            model: self.model,
            agent: self.agent,
            input: self.input,
            previous_interaction_id: None, // Explicitly None
            tools: self.tools,
            response_modalities: self.response_modalities,
            response_format: self.response_format,
            response_mime_type: self.response_mime_type,
            generation_config: self.generation_config,
            background: None, // Reset - can't be true with store disabled
            store: Some(false),
            system_instruction: self.system_instruction,
            max_function_call_loops: self.max_function_call_loops,
            tool_service: self.tool_service,
            timeout: self.timeout,
            _state: PhantomData,
        }
    }

    /// Enables background execution for this interaction.
    ///
    /// Background execution allows long-running operations to continue after
    /// the initial API response. Only supported for agents.
    ///
    /// # Availability
    ///
    /// This method is available on [`FirstTurn`] and [`Chained`] builders.
    /// It is NOT available after calling `with_store_disabled()` because
    /// background execution requires storage.
    pub fn with_background(mut self, background: bool) -> Self {
        self.background = Some(background);
        self
    }
}

// ============================================================================
// Methods available on Chained builders
// ============================================================================

impl<'a> InteractionBuilder<'a, Chained> {
    /// Enables background execution for this interaction.
    ///
    /// Background execution allows long-running operations to continue after
    /// the initial API response. Only supported for agents.
    ///
    /// # Availability
    ///
    /// This method is available on [`FirstTurn`] and [`Chained`] builders.
    /// It is NOT available after calling `with_store_disabled()` because
    /// background execution requires storage.
    pub fn with_background(mut self, background: bool) -> Self {
        self.background = Some(background);
        self
    }
}

// ============================================================================
// Methods available on all builder states
// ============================================================================

impl<'a, State: Send + 'a> InteractionBuilder<'a, State> {
    /// Sets the model to use for this interaction (e.g., "gemini-3-flash-preview").
    ///
    /// Note: Mutually exclusive with `with_agent()`.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Sets the agent to use for this interaction (e.g., "deep-research-pro-preview-12-2025").
    ///
    /// Note: Mutually exclusive with `with_model()`.
    pub fn with_agent(mut self, agent: impl Into<String>) -> Self {
        self.agent = Some(agent.into());
        self
    }

    /// Sets the input for this interaction from an `InteractionInput`.
    ///
    /// For simple text input, prefer `with_text()`.
    pub fn with_input(mut self, input: InteractionInput) -> Self {
        self.input = Some(input);
        self
    }

    /// Sets a simple text input for this interaction.
    ///
    /// This is a convenience method that creates an `InteractionInput::Text`.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.input = Some(InteractionInput::Text(text.into()));
        self
    }

    /// Sets the input from a vector of content objects.
    ///
    /// This is useful for building multi-part inputs or for sending function results.
    ///
    /// # Example
    /// ```no_run
    /// # use rust_genai::{Client, function_result_content};
    /// # use serde_json::json;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder("api_key".to_string()).build()?;
    ///
    /// let result = function_result_content("my_func", "call_123", json!({"data": "result"}));
    ///
    /// let response = client.interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_content(vec![result])
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_content(mut self, content: Vec<InteractionContent>) -> Self {
        self.input = Some(InteractionInput::Content(content));
        self
    }

    // =========================================================================
    // Multimodal Content Addition Methods
    // =========================================================================
    //
    // These `add_*` methods allow fluent construction of multimodal content.
    // Unlike `with_text` and `with_content` which REPLACE the input,
    // these methods ACCUMULATE content items.

    /// Adds an image from a file to the content.
    ///
    /// Reads the file, encodes it as base64, and auto-detects the MIME type
    /// from the file extension.
    ///
    /// This method accumulates content - it can be called multiple times to add
    /// multiple images, and works alongside `with_text()`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Compare these two images")
    ///     .add_image_file("photo1.jpg").await?
    ///     .add_image_file("photo2.jpg").await?
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add_image_file(
        mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, GenaiError> {
        let content = crate::multimodal::image_from_file(path).await?;
        self.add_content_item(content);
        Ok(self)
    }

    /// Adds an image from base64-encoded data to the content.
    ///
    /// This method accumulates content - it can be called multiple times.
    pub fn add_image_data(mut self, data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        let content = crate::interactions_api::image_data_content(data, mime_type);
        self.add_content_item(content);
        self
    }

    /// Adds an image from raw bytes to the content.
    ///
    /// The bytes are automatically base64-encoded. This is useful when you have
    /// image data in memory (e.g., downloaded from a URL or generated programmatically).
    ///
    /// This method accumulates content - it can be called multiple times.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// // Read image bytes from file or network
    /// let image_bytes = std::fs::read("photo.png")?;
    ///
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Describe this image")
    ///     .add_image_bytes(&image_bytes, "image/png")
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_image_bytes(self, data: &[u8], mime_type: impl Into<String>) -> Self {
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        self.add_image_data(encoded, mime_type)
    }

    /// Adds an image from a URI to the content.
    ///
    /// This method accumulates content - it can be called multiple times.
    pub fn add_image_uri(mut self, uri: impl Into<String>, mime_type: impl Into<String>) -> Self {
        let content = crate::interactions_api::image_uri_content(uri, mime_type);
        self.add_content_item(content);
        self
    }

    /// Adds an audio file to the content.
    ///
    /// Reads the file, encodes it as base64, and auto-detects the MIME type.
    pub async fn add_audio_file(
        mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, GenaiError> {
        let content = crate::multimodal::audio_from_file(path).await?;
        self.add_content_item(content);
        Ok(self)
    }

    /// Adds audio from base64-encoded data to the content.
    pub fn add_audio_data(mut self, data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        let content = crate::interactions_api::audio_data_content(data, mime_type);
        self.add_content_item(content);
        self
    }

    /// Adds audio from raw bytes to the content.
    ///
    /// The bytes are automatically base64-encoded. This is useful when you have
    /// audio data in memory.
    ///
    /// This method accumulates content - it can be called multiple times.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    /// let audio_bytes = std::fs::read("recording.mp3")?;
    ///
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Transcribe this audio")
    ///     .add_audio_bytes(&audio_bytes, "audio/mp3")
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_audio_bytes(self, data: &[u8], mime_type: impl Into<String>) -> Self {
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        self.add_audio_data(encoded, mime_type)
    }

    /// Adds audio from a URI to the content.
    pub fn add_audio_uri(mut self, uri: impl Into<String>, mime_type: impl Into<String>) -> Self {
        let content = crate::interactions_api::audio_uri_content(uri, mime_type);
        self.add_content_item(content);
        self
    }

    /// Adds a video file to the content.
    ///
    /// Reads the file, encodes it as base64, and auto-detects the MIME type.
    pub async fn add_video_file(
        mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, GenaiError> {
        let content = crate::multimodal::video_from_file(path).await?;
        self.add_content_item(content);
        Ok(self)
    }

    /// Adds video from base64-encoded data to the content.
    pub fn add_video_data(mut self, data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        let content = crate::interactions_api::video_data_content(data, mime_type);
        self.add_content_item(content);
        self
    }

    /// Adds video from raw bytes to the content.
    ///
    /// The bytes are automatically base64-encoded. This is useful when you have
    /// video data in memory.
    ///
    /// This method accumulates content - it can be called multiple times.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    /// let video_bytes = std::fs::read("clip.mp4")?;
    ///
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Describe what happens in this video")
    ///     .add_video_bytes(&video_bytes, "video/mp4")
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_video_bytes(self, data: &[u8], mime_type: impl Into<String>) -> Self {
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        self.add_video_data(encoded, mime_type)
    }

    /// Adds video from a URI to the content.
    pub fn add_video_uri(mut self, uri: impl Into<String>, mime_type: impl Into<String>) -> Self {
        let content = crate::interactions_api::video_uri_content(uri, mime_type);
        self.add_content_item(content);
        self
    }

    /// Adds a document file (e.g., PDF) to the content.
    ///
    /// Reads the file, encodes it as base64, and auto-detects the MIME type.
    pub async fn add_document_file(
        mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, GenaiError> {
        let content = crate::multimodal::document_from_file(path).await?;
        self.add_content_item(content);
        Ok(self)
    }

    /// Adds a document from base64-encoded data to the content.
    pub fn add_document_data(
        mut self,
        data: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        let content = crate::interactions_api::document_data_content(data, mime_type);
        self.add_content_item(content);
        self
    }

    /// Adds a document from raw bytes to the content.
    ///
    /// The bytes are automatically base64-encoded. This is useful when you have
    /// document data in memory (e.g., a PDF generated programmatically).
    ///
    /// This method accumulates content - it can be called multiple times.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    /// let pdf_bytes = std::fs::read("document.pdf")?;
    ///
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Summarize this document")
    ///     .add_document_bytes(&pdf_bytes, "application/pdf")
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_document_bytes(self, data: &[u8], mime_type: impl Into<String>) -> Self {
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        self.add_document_data(encoded, mime_type)
    }

    /// Adds a document from a URI to the content.
    pub fn add_document_uri(
        mut self,
        uri: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        let content = crate::interactions_api::document_uri_content(uri, mime_type);
        self.add_content_item(content);
        self
    }

    /// Adds a file from the Files API to the content.
    ///
    /// Use this to include files uploaded via `client.upload_file()`. The file
    /// is referenced by its URI, which is more efficient than sending the file
    /// data inline for large files or files used across multiple interactions.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// // Upload a file once
    /// let file = client.upload_file("video.mp4").await?;
    ///
    /// // Use in interaction
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Describe this video")
    ///     .with_file(&file)
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_file(mut self, file: &genai_client::FileMetadata) -> Self {
        let content = crate::interactions_api::file_uri_content(file);
        self.add_content_item(content);
        self
    }

    /// Adds a file from the Files API using just the URI and MIME type.
    ///
    /// Use this when you have the file URI and MIME type but not the full
    /// `FileMetadata` struct. The content type is inferred from the MIME type.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Describe this video")
    ///     .with_file_uri(
    ///         "https://generativelanguage.googleapis.com/v1beta/files/abc123",
    ///         "video/mp4"
    ///     )
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_file_uri(mut self, uri: impl Into<String>, mime_type: impl Into<String>) -> Self {
        let content =
            crate::interactions_api::content_from_uri_and_mime(uri.into(), mime_type.into());
        self.add_content_item(content);
        self
    }

    /// Internal helper to add a content item, converting input type if needed.
    ///
    /// - If input is `None`: creates a new `Content` variant with the item
    /// - If input is `Text`: converts to `Content` with the text as first item, then adds the new item
    /// - If input is `Content`: appends the item to the existing vec
    fn add_content_item(&mut self, item: InteractionContent) {
        match &mut self.input {
            None => {
                self.input = Some(InteractionInput::Content(vec![item]));
            }
            Some(InteractionInput::Text(text)) => {
                let text_item = crate::interactions_api::text_content(std::mem::take(text));
                self.input = Some(InteractionInput::Content(vec![text_item, item]));
            }
            Some(InteractionInput::Content(contents)) => {
                contents.push(item);
            }
            // Required by #[non_exhaustive] but unreachable: InteractionInput uses
            // #[serde(untagged)] so only Text/Content can exist at runtime.
            Some(_) => {
                unreachable!("InteractionInput is untagged; only Text/Content variants exist")
            }
        }
    }

    /// Internal helper to add a tool to the tools list.
    fn add_tool(&mut self, tool: InternalTool) {
        self.tools.get_or_insert_with(Vec::new).push(tool);
    }

    /// Adds tools for function calling.
    pub fn with_tools(mut self, tools: Vec<InternalTool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Adds a single function declaration to the request.
    ///
    /// This method can be called multiple times to add several functions.
    /// Each function is converted into a [`crate::Tool`] and added to the request.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::{Client, FunctionDeclaration};
    /// use serde_json::json;
    ///
    /// let client = Client::new("api-key".to_string());
    ///
    /// let func = FunctionDeclaration::builder("get_temperature")
    ///     .description("Get the temperature for a location")
    ///     .parameter("location", json!({"type": "string"}))
    ///     .required(vec!["location".to_string()])
    ///     .build();
    ///
    /// let builder = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("What's the temperature in Paris?")
    ///     .with_function(func);
    /// ```
    pub fn with_function(mut self, function: FunctionDeclaration) -> Self {
        self.add_tool(function.into_tool());
        self
    }

    /// Adds multiple function declarations to the request at once.
    ///
    /// This is a convenience method equivalent to calling [`with_function`] multiple times.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::{Client, FunctionDeclaration};
    ///
    /// let client = Client::new("api-key".to_string());
    ///
    /// let functions = vec![
    ///     FunctionDeclaration::builder("get_weather").build(),
    ///     FunctionDeclaration::builder("get_time").build(),
    /// ];
    ///
    /// let builder = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("What's the weather and time?")
    ///     .with_functions(functions);
    /// ```
    ///
    /// [`with_function`]: InteractionBuilder::with_function
    pub fn with_functions(mut self, functions: Vec<FunctionDeclaration>) -> Self {
        for func in functions {
            self.add_tool(func.into_tool());
        }
        self
    }

    /// Sets a tool service for dependency-injected functions.
    ///
    /// Use this when your tool functions need access to shared state like
    /// database connections, API clients, or configuration. The service
    /// provides callable functions that can access the service's internal state.
    ///
    /// Tools from the service are used in addition to any auto-discovered
    /// tools from the global registry (via `#[tool]` macro).
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rust_genai::{Client, ToolService, CallableFunction};
    /// use std::sync::Arc;
    ///
    /// struct MyService { db: Database }
    ///
    /// impl ToolService for MyService {
    ///     fn tools(&self) -> Vec<Arc<dyn CallableFunction>> {
    ///         vec![Arc::new(QueryTool { db: self.db.clone() })]
    ///     }
    /// }
    ///
    /// let service = Arc::new(MyService { db: Database::new() });
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_tool_service(service)
    ///     .with_text("Query the database for users")
    ///     .create_with_auto_functions()
    ///     .await?;
    /// ```
    pub fn with_tool_service(mut self, service: Arc<dyn ToolService>) -> Self {
        self.tool_service = Some(service);
        self
    }

    /// Enables Google Search grounding for this interaction.
    ///
    /// This adds the built-in `GoogleSearch` tool which allows the model to
    /// search the web and ground its responses in real-time information.
    /// Grounding metadata will be available in the response via
    /// [`InteractionResponse::google_search_metadata`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Who won the 2024 World Series?")
    ///     .with_google_search()
    ///     .create()
    ///     .await?;
    ///
    /// // Access grounding metadata
    /// if let Some(metadata) = response.google_search_metadata() {
    ///     println!("Search queries: {:?}", metadata.web_search_queries);
    ///     for chunk in &metadata.grounding_chunks {
    ///         println!("Source: {}", chunk.web.uri);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`InteractionResponse::google_search_metadata`]: crate::InteractionResponse::google_search_metadata
    pub fn with_google_search(mut self) -> Self {
        self.add_tool(InternalTool::GoogleSearch);
        self
    }

    /// Enables code execution for this interaction.
    ///
    /// This adds the built-in `CodeExecution` tool which allows the model to
    /// write and execute Python code to help answer questions. The code runs
    /// in a sandboxed environment on Google's servers.
    ///
    /// # Security Considerations
    ///
    /// Code execution runs in a sandboxed environment with the following
    /// limitations:
    /// - Maximum execution time: 30 seconds
    /// - No network access
    /// - Limited file I/O capabilities
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Calculate the factorial of 50")
    ///     .with_code_execution()
    ///     .create()
    ///     .await?;
    ///
    /// // Access code execution results
    /// for result in response.code_execution_results() {
    ///     if result.outcome.is_success() {
    ///         println!("Code output: {}", result.output);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_code_execution(mut self) -> Self {
        self.add_tool(InternalTool::CodeExecution);
        self
    }

    /// Enables URL context fetching for this interaction.
    ///
    /// This adds the built-in `UrlContext` tool which allows the model to
    /// fetch and analyze content from URLs provided in the prompt.
    /// URL context metadata will be available in the response via
    /// [`InteractionResponse::url_context_metadata`].
    ///
    /// # Limitations
    ///
    /// - Maximum 20 URLs per request
    /// - Maximum 34MB content size per URL
    /// - Unsupported: paywalled content, YouTube, Google Workspace files, video/audio
    /// - Retrieved content counts toward input token usage
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Summarize the content from https://example.com")
    ///     .with_url_context()
    ///     .create()
    ///     .await?;
    ///
    /// // Access URL context metadata
    /// if let Some(metadata) = response.url_context_metadata() {
    ///     for entry in &metadata.url_metadata {
    ///         println!("URL: {} - Status: {:?}",
    ///             entry.retrieved_url,
    ///             entry.url_retrieval_status);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`InteractionResponse::url_context_metadata`]: crate::InteractionResponse::url_context_metadata
    pub fn with_url_context(mut self) -> Self {
        self.add_tool(InternalTool::UrlContext);
        self
    }

    /// Adds an MCP (Model Context Protocol) server as a tool.
    ///
    /// MCP servers provide a standardized way to expose external tools and
    /// capabilities to the model. When configured, the model can call tools
    /// exposed by the MCP server to access external data, services, or actions.
    ///
    /// This method can be called multiple times to add multiple MCP servers.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("What files are in my project?")
    ///     .with_mcp_server("filesystem", "https://mcp.example.com/fs")
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Multiple Servers
    ///
    /// ```no_run
    /// use rust_genai::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Search the database and format the results")
    ///     .with_mcp_server("database", "https://mcp.example.com/db")
    ///     .with_mcp_server("formatter", "https://mcp.example.com/fmt")
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_mcp_server(mut self, name: impl Into<String>, url: impl Into<String>) -> Self {
        self.add_tool(InternalTool::McpServer {
            name: name.into(),
            url: url.into(),
        });
        self
    }

    /// Sets response modalities (e.g., ["IMAGE"]).
    pub fn with_response_modalities(mut self, modalities: Vec<String>) -> Self {
        self.response_modalities = Some(modalities);
        self
    }

    /// Configures the request to return image output.
    ///
    /// This is a convenience method equivalent to:
    /// ```ignore
    /// .with_response_modalities(vec!["IMAGE".to_string()])
    /// ```
    ///
    /// Use this when you want the model to generate images. Requires a model
    /// that supports image generation (e.g., `gemini-3-pro-image-preview`).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::{Client, InteractionResponseExt};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-pro-image-preview")
    ///     .with_text("A cute cat playing with yarn")
    ///     .with_image_output()
    ///     .create()
    ///     .await?;
    ///
    /// // Extract generated image
    /// if let Some(bytes) = response.first_image_bytes()? {
    ///     std::fs::write("cat.png", &bytes)?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_image_output(self) -> Self {
        self.with_response_modalities(vec!["IMAGE".to_string()])
    }

    /// Sets a JSON schema to enforce structured output from the model.
    ///
    /// When you provide a JSON schema, the model will return responses that
    /// conform exactly to your schema structure. This is useful for:
    /// - Extracting structured data from text
    /// - Building reliable data pipelines
    /// - Ensuring consistent API responses
    ///
    /// The schema should be a standard JSON Schema object with `type`, `properties`,
    /// and optionally `required` fields.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    /// use serde_json::json;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// let schema = json!({
    ///     "type": "object",
    ///     "properties": {
    ///         "name": {"type": "string"},
    ///         "age": {"type": "integer"},
    ///         "hobbies": {
    ///             "type": "array",
    ///             "items": {"type": "string"}
    ///         }
    ///     },
    ///     "required": ["name", "age"]
    /// });
    ///
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Generate info for someone named Alice who is 30 and likes hiking")
    ///     .with_response_format(schema)
    ///     .create()
    ///     .await?;
    ///
    /// // Response is guaranteed to be valid JSON matching the schema
    /// let text = response.text().unwrap();
    /// let data: serde_json::Value = serde_json::from_str(text)?;
    /// println!("Name: {}", data["name"]);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Combining with Tools
    ///
    /// Structured output can be combined with built-in tools like Google Search
    /// or URL Context to get structured data from real-time sources:
    ///
    /// ```no_run
    /// # use rust_genai::Client;
    /// # use serde_json::json;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Client::new("api-key".to_string());
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("What is the current weather in Tokyo?")
    ///     .with_google_search()
    ///     .with_response_format(json!({
    ///         "type": "object",
    ///         "properties": {
    ///             "temperature": {"type": "string"},
    ///             "conditions": {"type": "string"}
    ///         },
    ///         "required": ["temperature", "conditions"]
    ///     }))
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_response_format(mut self, format: serde_json::Value) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Sets generation configuration (temperature, max tokens, etc.).
    pub fn with_generation_config(mut self, config: GenerationConfig) -> Self {
        self.generation_config = Some(config);
        self
    }

    /// Sets the thinking level for reasoning/chain-of-thought output.
    ///
    /// Higher levels produce more detailed reasoning but consume more tokens.
    /// When thinking is enabled, the model's reasoning process is exposed
    /// in the response as `Thought` content. Use `response.usage.total_reasoning_tokens`
    /// to track reasoning token costs.
    ///
    /// # Example
    /// ```no_run
    /// # use rust_genai::{Client, ThinkingLevel};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder("api-key".to_string()).build()?;
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Solve this step by step: 15 * 23")
    ///     .with_thinking_level(ThinkingLevel::Medium)
    ///     .create()
    ///     .await?;
    ///
    /// if response.has_thoughts() {
    ///     for thought in response.thoughts() {
    ///         println!("Reasoning: {}", thought);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_thinking_level(mut self, level: ThinkingLevel) -> Self {
        let config = self
            .generation_config
            .get_or_insert_with(GenerationConfig::default);
        config.thinking_level = Some(level);
        self
    }

    /// Controls whether thinking summaries are included in output.
    ///
    /// When using `with_thinking_level()`, summaries of the model's reasoning
    /// process can be included alongside thought signatures. Use `Auto` to
    /// include summaries, or `None` to exclude them.
    ///
    /// # Example
    /// ```no_run
    /// # use rust_genai::{Client, ThinkingLevel, ThinkingSummaries};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder("api-key".to_string()).build()?;
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Solve this step by step: 15 * 23")
    ///     .with_thinking_level(ThinkingLevel::Medium)
    ///     .with_thinking_summaries(ThinkingSummaries::Auto)
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_thinking_summaries(mut self, summaries: ThinkingSummaries) -> Self {
        let config = self
            .generation_config
            .get_or_insert_with(GenerationConfig::default);
        config.thinking_summaries = Some(summaries);
        self
    }

    /// Sets a seed for deterministic output generation.
    ///
    /// Using the same seed with identical inputs will produce the same output,
    /// useful for testing and debugging. The exact same seed, model, and input
    /// should produce reproducible results.
    ///
    /// # Example
    /// ```no_run
    /// # use rust_genai::Client;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder("api-key".to_string()).build()?;
    ///
    /// // Two requests with the same seed should produce the same output
    /// let response1 = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Generate a random number")
    ///     .with_seed(42)
    ///     .create()
    ///     .await?;
    ///
    /// let response2 = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Generate a random number")
    ///     .with_seed(42)
    ///     .create()
    ///     .await?;
    ///
    /// // response1.text() should equal response2.text()
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_seed(mut self, seed: i64) -> Self {
        let config = self
            .generation_config
            .get_or_insert_with(GenerationConfig::default);
        config.seed = Some(seed);
        self
    }

    /// Sets stop sequences that halt generation.
    ///
    /// When the model generates any of these sequences, generation stops
    /// immediately. Useful for controlling output boundaries in chat applications
    /// or structured generation.
    ///
    /// # Example
    /// ```no_run
    /// # use rust_genai::Client;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder("api-key".to_string()).build()?;
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Write a story")
    ///     .with_stop_sequences(vec!["THE END".to_string(), "---".to_string()])
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_stop_sequences(mut self, sequences: Vec<String>) -> Self {
        let config = self
            .generation_config
            .get_or_insert_with(GenerationConfig::default);
        config.stop_sequences = Some(sequences);
        self
    }

    /// Sets the function calling mode.
    ///
    /// Controls how the model uses function calling capabilities.
    ///
    /// # Modes
    ///
    /// - `Auto` (default): Model decides whether to call functions or respond naturally
    /// - `Any`: Model must call a function; guarantees schema adherence for calls
    /// - `None`: Prohibits function calling entirely
    /// - `Validated` (Preview): Ensures either function calls OR natural language adhere to schema
    ///
    /// # Example
    /// ```no_run
    /// # use rust_genai::{Client, FunctionCallingMode};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder("api-key".to_string()).build()?;
    ///
    /// // Force the model to use a function
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Get weather in Tokyo")
    ///     .with_function_calling_mode(FunctionCallingMode::Any)
    ///     .create()
    ///     .await?;
    ///
    /// // Use VALIDATED mode for guaranteed schema adherence
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Get weather in Tokyo")
    ///     .with_function_calling_mode(FunctionCallingMode::Validated)
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_function_calling_mode(mut self, mode: FunctionCallingMode) -> Self {
        let config = self
            .generation_config
            .get_or_insert_with(GenerationConfig::default);
        config.tool_choice = Some(mode);
        self
    }

    /// Sets the response MIME type for structured output.
    ///
    /// Required when using `with_response_format()` with a JSON schema.
    /// Typically "application/json" for structured JSON output.
    ///
    /// # Example
    /// ```no_run
    /// # use rust_genai::Client;
    /// # use serde_json::json;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder("api-key".to_string()).build()?;
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Generate user data")
    ///     .with_response_mime_type("application/json")
    ///     .with_response_format(json!({
    ///         "type": "object",
    ///         "properties": {
    ///             "name": {"type": "string"},
    ///             "age": {"type": "integer"}
    ///         }
    ///     }))
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_response_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.response_mime_type = Some(mime_type.into());
        self
    }

    /// Explicitly enables storage for this interaction.
    ///
    /// Storage is enabled by default, so this method is typically only needed
    /// to be explicit about intent or to re-enable after conditional logic.
    ///
    /// When storage is enabled:
    /// - The response will include an `id` field
    /// - The interaction can be retrieved later with `get_interaction()`
    /// - The interaction can be referenced via `with_previous_interaction()` in follow-up requests
    /// - Auto-function calling (`create_with_auto_functions()`) will work
    ///
    /// # See Also
    ///
    /// Use `with_store_disabled()` (only available on [`FirstTurn`] builders) to disable storage.
    pub fn with_store_enabled(mut self) -> Self {
        self.store = Some(true);
        self
    }

    /// Sets the maximum number of function call loops for `create_with_auto_functions()`.
    ///
    /// Default is 5. Increase for complex multi-step function calling scenarios,
    /// or decrease to fail faster if the model is stuck in a loop.
    ///
    /// # Example
    /// ```no_run
    /// # use rust_genai::Client;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder("api_key".to_string()).build()?;
    ///
    /// let response = client.interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Complex multi-step task")
    ///     .with_max_function_call_loops(10)  // Allow up to 10 iterations
    ///     .create_with_auto_functions()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_max_function_call_loops(mut self, max_loops: usize) -> Self {
        if max_loops == 0 {
            log::warn!(
                "max_function_call_loops set to 0 - auto function calling will immediately fail \
                 if the model returns any function calls. Consider using create() instead of \
                 create_with_auto_functions() if you don't want automatic function execution."
            );
        }
        self.max_function_call_loops = max_loops;
        self
    }

    /// Sets a timeout for the request.
    ///
    /// If the request takes longer than the specified duration, it will be
    /// cancelled and return [`GenaiError::Timeout`].
    ///
    /// # Behavior by Method
    ///
    /// | Method | Timeout Applies To |
    /// |--------|-------------------|
    /// | `create()` | Entire request |
    /// | `create_stream()` | Per-chunk (inter-chunk timeout) |
    /// | `create_with_auto_functions()` | Per-API-call (each round) |
    /// | `create_stream_with_auto_functions()` | Per-chunk (each streaming round) |
    ///
    /// For auto-function methods, function execution time is **not** counted against
    /// the timeout. For a total timeout including function execution, wrap the call
    /// in `tokio::time::timeout()`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::Client;
    /// use std::time::Duration;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("What is the meaning of life?")
    ///     .with_timeout(Duration::from_secs(30))
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`GenaiError::Timeout`]: crate::GenaiError::Timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Creates the interaction and returns the response.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No input was provided
    /// - Neither model nor agent was specified
    /// - The API request fails
    /// - The request times out (if `with_timeout()` was set)
    pub async fn create(self) -> Result<InteractionResponse, GenaiError> {
        let client = self.client;
        let timeout = self.timeout;
        let request = self.build_request()?;

        let future = client.create_interaction(request);

        match timeout {
            Some(duration) => tokio::time::timeout(duration, future).await.map_err(|_| {
                debug!("Request timed out after {:?}", duration);
                GenaiError::Timeout(duration)
            })?,
            None => future.await,
        }
    }

    /// Creates a streaming interaction that yields chunks as they arrive.
    ///
    /// Returns a stream of `StreamChunk` items:
    /// - `StreamChunk::Delta`: Incremental content (text or thought)
    /// - `StreamChunk::Complete`: The final complete interaction response
    ///
    /// # Timeout Behavior
    ///
    /// If `with_timeout()` was set, the timeout applies **per-chunk**, not to
    /// the total stream duration. Each `stream.next().await` call must complete
    /// within the timeout, or a [`GenaiError::Timeout`] error is yielded.
    ///
    /// This is useful for detecting stalled connections (e.g., model stops
    /// responding mid-stream), but does **not** limit the total time to
    /// complete the stream. For a total timeout, wrap the stream consumption
    /// in `tokio::time::timeout()`:
    ///
    /// ```no_run
    /// # use rust_genai::Client;
    /// # use futures_util::StreamExt;
    /// # use std::time::Duration;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Client::new("api_key".to_string());
    /// let mut stream = client.interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Write a story")
    ///     .create_stream();
    ///
    /// // Total timeout for entire stream consumption
    /// tokio::time::timeout(Duration::from_secs(60), async {
    ///     while let Some(chunk) = stream.next().await {
    ///         // process chunk...
    ///     }
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns errors if:
    /// - No input was provided
    /// - Neither model nor agent was specified
    /// - The API request fails
    /// - A chunk doesn't arrive within the timeout (if set)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use rust_genai::{Client, StreamChunk, StreamEvent};
    /// # use futures_util::StreamExt;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder("api_key".to_string()).build()?;
    ///
    /// let mut stream = client.interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Count to 5")
    ///     .create_stream();
    ///
    /// while let Some(event) = stream.next().await {
    ///     let event = event?;
    ///     // event.event_id can be saved for stream resumption
    ///     match &event.chunk {
    ///         StreamChunk::Delta(delta) => {
    ///             if let Some(text) = delta.text() {
    ///                 print!("{}", text);
    ///             }
    ///         }
    ///         StreamChunk::Complete(response) => {
    ///             println!("\nFinal response ID: {:?}", response.id);
    ///         }
    ///         _ => {} // Handle unknown future variants
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`GenaiError::Timeout`]: crate::GenaiError::Timeout
    pub fn create_stream(self) -> BoxStream<'a, Result<StreamEvent, GenaiError>> {
        let client = self.client;
        let timeout = self.timeout;
        Box::pin(async_stream::try_stream! {
            let mut request = self.build_request()?;
            request.stream = Some(true);
            let mut stream = client.create_interaction_stream(request);

            loop {
                let next_chunk = stream.next();
                let result = match timeout {
                    Some(duration) => {
                        match tokio::time::timeout(duration, next_chunk).await {
                            Ok(Some(result)) => Some(result),
                            Ok(None) => None,
                            Err(_) => {
                                debug!("Stream chunk timed out after {:?}", duration);
                                Err(GenaiError::Timeout(duration))?;
                                unreachable!()
                            }
                        }
                    }
                    None => next_chunk.await,
                };

                match result {
                    Some(Ok(event)) => yield event,
                    Some(Err(e)) => Err(e)?,
                    None => break,
                }
            }
        })
    }

    /// Builds the `CreateInteractionRequest` from the builder state.
    ///
    /// This method is primarily for testing validation logic. In normal usage,
    /// call `.create()` or `.create_stream()` instead, which call this internally.
    #[doc(hidden)]
    pub fn build_request(self) -> Result<CreateInteractionRequest, GenaiError> {
        // Validate that we have input
        let input = self.input.ok_or_else(|| {
            GenaiError::InvalidInput("Input is required for interaction".to_string())
        })?;

        // Validate that we have either model or agent (but not both)
        match (&self.model, &self.agent) {
            (None, None) => {
                return Err(GenaiError::InvalidInput(
                    "Either model or agent must be specified".to_string(),
                ));
            }
            (Some(model), Some(agent)) => {
                return Err(GenaiError::InvalidInput(format!(
                    "Cannot specify both model ('{}') and agent ('{}') - use one or the other",
                    model, agent
                )));
            }
            _ => {} // Valid: exactly one is set
        }

        Ok(CreateInteractionRequest {
            model: self.model,
            agent: self.agent,
            input,
            previous_interaction_id: self.previous_interaction_id,
            tools: self.tools,
            response_modalities: self.response_modalities,
            response_format: self.response_format,
            response_mime_type: self.response_mime_type,
            generation_config: self.generation_config,
            stream: None, // Set by create() vs create_stream()
            background: self.background,
            store: self.store,
            system_instruction: self.system_instruction,
        })
    }
}

#[cfg(test)]
mod tests;
