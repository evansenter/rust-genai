use std::time::Instant;

use crate::GenaiError;
use crate::client::Client;
use crate::streaming::{AutoFunctionResult, AutoFunctionStreamChunk, FunctionExecutionResult};

use futures_util::{StreamExt, stream::BoxStream};
use genai_client::{
    self, CreateInteractionRequest, FunctionDeclaration, GenerationConfig, InteractionContent,
    InteractionInput, InteractionResponse, StreamChunk, ThinkingLevel, Tool as InternalTool,
};

/// Default maximum iterations for auto function calling
pub const DEFAULT_MAX_FUNCTION_CALL_LOOPS: usize = 5;

/// Builder for creating interactions with the Gemini Interactions API.
///
/// Provides a fluent interface for constructing interaction requests with models or agents.
///
/// # Examples
///
/// ```no_run
/// # use rust_genai::{Client, StreamChunk};
/// # use futures_util::StreamExt;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::builder("api_key".to_string()).build();
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
/// while let Some(chunk) = stream.next().await {
///     match chunk? {
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
#[derive(Debug)]
pub struct InteractionBuilder<'a> {
    client: &'a Client,
    model: Option<String>,
    agent: Option<String>,
    input: Option<InteractionInput>,
    previous_interaction_id: Option<String>,
    tools: Option<Vec<InternalTool>>,
    response_modalities: Option<Vec<String>>,
    response_format: Option<serde_json::Value>,
    generation_config: Option<GenerationConfig>,
    background: Option<bool>,
    store: Option<bool>,
    system_instruction: Option<InteractionInput>,
    /// Maximum iterations for auto function calling loop
    max_function_call_loops: usize,
}

impl<'a> InteractionBuilder<'a> {
    /// Creates a new interaction builder.
    pub(crate) const fn new(client: &'a Client) -> Self {
        Self {
            client,
            model: None,
            agent: None,
            input: None,
            previous_interaction_id: None,
            tools: None,
            response_modalities: None,
            response_format: None,
            generation_config: None,
            background: None,
            store: None,
            system_instruction: None,
            max_function_call_loops: DEFAULT_MAX_FUNCTION_CALL_LOOPS,
        }
    }

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
    /// let client = Client::builder("api_key".to_string()).build();
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
        }
    }

    /// References a previous interaction for stateful conversations.
    ///
    /// The interaction will have access to the context from the previous interaction.
    pub fn with_previous_interaction(mut self, id: impl Into<String>) -> Self {
        self.previous_interaction_id = Some(id.into());
        self
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
        let tool = function.into_tool();
        self.tools.get_or_insert_with(Vec::new).push(tool);
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
    pub fn with_functions(self, functions: Vec<FunctionDeclaration>) -> Self {
        functions
            .into_iter()
            .fold(self, |builder, func| builder.with_function(func))
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
        self.tools
            .get_or_insert_with(Vec::new)
            .push(InternalTool::GoogleSearch);
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
        self.tools
            .get_or_insert_with(Vec::new)
            .push(InternalTool::CodeExecution);
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
        self.tools
            .get_or_insert_with(Vec::new)
            .push(InternalTool::UrlContext);
        self
    }

    /// Sets response modalities (e.g., ["IMAGE"]).
    pub fn with_response_modalities(mut self, modalities: Vec<String>) -> Self {
        self.response_modalities = Some(modalities);
        self
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
    /// let client = Client::builder("api-key".to_string()).build();
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

    /// Enables background execution mode (agents only).
    pub fn with_background(mut self, background: bool) -> Self {
        self.background = Some(background);
        self
    }

    /// Controls whether interaction data is persisted (default: true).
    pub fn with_store(mut self, store: bool) -> Self {
        self.store = Some(store);
        self
    }

    /// Sets a system instruction for the model.
    pub fn with_system_instruction(mut self, instruction: impl Into<String>) -> Self {
        self.system_instruction = Some(InteractionInput::Text(instruction.into()));
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
    /// let client = Client::builder("api_key".to_string()).build();
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

    /// Creates the interaction and returns the response.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No input was provided
    /// - Neither model nor agent was specified
    /// - The API request fails
    pub async fn create(self) -> Result<InteractionResponse, GenaiError> {
        let client = self.client;
        let request = self.build_request()?;
        client.create_interaction(request).await
    }

    /// Creates a streaming interaction that yields chunks as they arrive.
    ///
    /// Returns a stream of `StreamChunk` items:
    /// - `StreamChunk::Delta`: Incremental content (text or thought)
    /// - `StreamChunk::Complete`: The final complete interaction response
    ///
    /// # Errors
    ///
    /// Returns errors if:
    /// - No input was provided
    /// - Neither model nor agent was specified
    /// - The API request fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use rust_genai::{Client, StreamChunk};
    /// # use futures_util::StreamExt;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder("api_key".to_string()).build();
    ///
    /// let mut stream = client.interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("Count to 5")
    ///     .create_stream();
    ///
    /// while let Some(chunk) = stream.next().await {
    ///     match chunk? {
    ///         StreamChunk::Delta(delta) => {
    ///             if let Some(text) = delta.text() {
    ///                 print!("{}", text);
    ///             }
    ///         }
    ///         StreamChunk::Complete(response) => {
    ///             println!("\nFinal response ID: {}", response.id);
    ///         }
    ///         _ => {} // Handle unknown future variants
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_stream(self) -> BoxStream<'a, Result<StreamChunk, GenaiError>> {
        let client = self.client;
        Box::pin(async_stream::try_stream! {
            let mut request = self.build_request()?;
            request.stream = Some(true);
            let mut stream = client.create_interaction_stream(request);

            while let Some(result) = stream.next().await {
                yield result?;
            }
        })
    }

    /// Creates interaction with automatic function call handling.
    ///
    /// This method implements the auto-function execution loop:
    /// 1. Send initial input to model with available tools
    /// 2. If response contains function calls, execute them
    /// 3. Send function results back to model in new interaction
    /// 4. Repeat until model returns text or max iterations reached
    ///
    /// Functions are auto-discovered from the global registry (via `#[tool]` macro)
    /// or can be explicitly provided via `.with_function()` or `.with_tools()`.
    ///
    /// The loop automatically stops when:
    /// - Model returns text without function calls
    /// - Function calls array is empty
    /// - Maximum iterations is reached (default 5, configurable via `with_max_function_call_loops()`)
    ///
    /// # Thought Signatures
    ///
    /// For Gemini 3 models, thought signatures are required to maintain reasoning context
    /// across function calling turns. This method uses `previous_interaction_id` to link
    /// turns, which allows the server to manage thought signatures automatically.
    ///
    /// See <https://ai.google.dev/gemini-api/docs/thought-signatures> for more details.
    ///
    /// # Example
    /// ```no_run
    /// # use rust_genai::{Client, FunctionDeclaration};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder("api_key".to_string()).build();
    ///
    /// // Functions are auto-discovered from registry
    /// let result = client.interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("What's the weather in Tokyo?")
    ///     .create_with_auto_functions()
    ///     .await?;
    ///
    /// // Access the final response
    /// println!("{}", result.response.text().unwrap_or("No text"));
    ///
    /// // Access execution history
    /// for exec in &result.executions {
    ///     println!("Called {} -> {}", exec.name, exec.result);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Serialization
    ///
    /// Both [`AutoFunctionResult`] and its contained [`InteractionResponse`]
    /// implement `Serialize`, enabling logging, caching, and persistence of complete
    /// execution histories for debugging and evaluation workflows.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No input was provided
    /// - Neither model nor agent was specified
    /// - The API request fails
    /// - Maximum function call loops is exceeded (default 5, configurable via `with_max_function_call_loops()`)
    pub async fn create_with_auto_functions(self) -> Result<AutoFunctionResult, GenaiError> {
        use crate::function_calling::get_global_function_registry;
        use crate::interactions_api::function_result_content;
        use log::{error, warn};
        use serde_json::json;

        let client = self.client;
        let max_loops = self.max_function_call_loops;
        let mut request = self.build_request()?;

        // Track all function executions for the result
        let mut all_executions: Vec<FunctionExecutionResult> = Vec::new();

        // Auto-discover functions from registry if not explicitly provided
        let function_registry = get_global_function_registry();
        if request.tools.is_none() {
            let auto_discovered_declarations = function_registry.all_declarations();
            if !auto_discovered_declarations.is_empty() {
                request.tools = Some(
                    auto_discovered_declarations
                        .into_iter()
                        .map(|decl| decl.into_tool())
                        .collect(),
                );
            }
        }

        // Main auto-function loop (configurable iterations to prevent infinite loops)
        for _loop_count in 0..max_loops {
            let response = client.create_interaction(request.clone()).await?;

            // Extract function calls using convenience method
            let function_calls = response.function_calls();

            // If no function calls, we're done!
            if function_calls.is_empty() {
                return Ok(AutoFunctionResult {
                    response,
                    executions: all_executions,
                });
            }

            // Build function results for next iteration
            let mut function_results = Vec::new();

            for call in function_calls {
                // Validate that we have a call_id (required by API)
                let call_id = call.id.ok_or_else(|| {
                    error!(
                        "Function call '{}' is missing required call_id field.",
                        call.name
                    );
                    GenaiError::InvalidInput(format!(
                        "Function call '{}' is missing required call_id field. \
                         This may indicate an API response format change.",
                        call.name
                    ))
                })?;

                // Execute the function with timing
                let start = Instant::now();
                let result = if let Some(function) = function_registry.get(call.name) {
                    match function.call(call.args.clone()).await {
                        Ok(result) => result,
                        Err(e) => {
                            error!(
                                "Function execution failed: function='{}', error='{}'",
                                call.name, e
                            );
                            json!({ "error": e.to_string() })
                        }
                    }
                } else {
                    warn!(
                        "Function not found in registry: function='{}'. Informing model.",
                        call.name
                    );
                    json!({ "error": format!("Function '{}' is not available or not found.", call.name) })
                };
                let duration = start.elapsed();

                // Track execution for the result
                all_executions.push(FunctionExecutionResult::new(
                    call.name,
                    call_id,
                    result.clone(),
                    duration,
                ));

                // Add function result (only the result, not the call - server has it via previous_interaction_id)
                function_results.push(function_result_content(
                    call.name.to_string(),
                    call_id.to_string(),
                    result,
                ));
            }

            // Create new request with function results
            // The server maintains function call context via previous_interaction_id
            request.previous_interaction_id = Some(response.id);
            request.input = InteractionInput::Content(function_results);
        }

        Err(GenaiError::Internal(format!(
            "Exceeded maximum function call loops ({max_loops}). \
             The model may be stuck in a loop. Check your function implementations, \
             increase the limit using with_max_function_call_loops(), \
             or use manual function calling for more control."
        )))
    }

    /// Creates a streaming interaction with automatic function call handling.
    ///
    /// This method combines the streaming capabilities of `create_stream()` with the
    /// automatic function execution of `create_with_auto_functions()`. It yields
    /// [`AutoFunctionStreamChunk`] events that include:
    ///
    /// - `Delta`: Incremental content from the model (text, thoughts, etc.)
    /// - `ExecutingFunctions`: Notification when function calls are about to execute
    /// - `FunctionResults`: Results from executed functions
    /// - `Complete`: Final response when no more function calls are needed
    ///
    /// The stream automatically handles multiple function calling rounds, streaming
    /// content from each round and executing functions between rounds.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rust_genai::{Client, AutoFunctionStreamChunk, InteractionContent};
    /// # use futures_util::StreamExt;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder("api_key".to_string()).build();
    ///
    /// let mut stream = client.interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("What's the weather in Tokyo?")
    ///     .create_stream_with_auto_functions();
    ///
    /// while let Some(chunk) = stream.next().await {
    ///     match chunk? {
    ///         AutoFunctionStreamChunk::Delta(content) => {
    ///             if let InteractionContent::Text { text: Some(t) } = content {
    ///                 print!("{}", t);
    ///             }
    ///         }
    ///         AutoFunctionStreamChunk::ExecutingFunctions(response) => {
    ///             let names: Vec<_> = response.function_calls().iter().map(|c| c.name).collect();
    ///             println!("[Executing: {:?}]", names);
    ///         }
    ///         AutoFunctionStreamChunk::FunctionResults(results) => {
    ///             println!("[Got {} results]", results.len());
    ///         }
    ///         AutoFunctionStreamChunk::Complete(response) => {
    ///             println!("\n[Complete: {} tokens]", response.usage.as_ref()
    ///                 .and_then(|u| u.total_tokens).unwrap_or(0));
    ///         }
    ///         _ => {} // Handle unknown future variants
    ///     }
    /// }
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
    /// - A function call is missing its required `call_id` field
    /// - Maximum function call loops is exceeded
    pub fn create_stream_with_auto_functions(
        self,
    ) -> BoxStream<'a, Result<AutoFunctionStreamChunk, GenaiError>> {
        use crate::function_calling::get_global_function_registry;
        use crate::interactions_api::function_result_content;
        use log::{error, warn};
        use serde_json::json;

        let client = self.client;
        let max_loops = self.max_function_call_loops;

        Box::pin(async_stream::try_stream! {
            let mut request = self.build_request()?;

            // Auto-discover functions from registry if not explicitly provided
            let function_registry = get_global_function_registry();
            if request.tools.is_none() {
                let auto_discovered_declarations = function_registry.all_declarations();
                if !auto_discovered_declarations.is_empty() {
                    request.tools = Some(
                        auto_discovered_declarations
                            .into_iter()
                            .map(|decl| decl.into_tool())
                            .collect(),
                    );
                }
            }

            // Main auto-function streaming loop
            for _loop_count in 0..max_loops {
                // Enable streaming for this request
                request.stream = Some(true);

                // Stream this iteration's response
                let mut stream = client.create_interaction_stream(request.clone());
                let mut complete_response: Option<InteractionResponse> = None;
                // Accumulate function calls from deltas (streaming API may not include them in Complete)
                let mut accumulated_calls: Vec<(Option<String>, String, serde_json::Value)> = Vec::new();

                while let Some(result) = stream.next().await {
                    match result? {
                        StreamChunk::Delta(delta) => {
                            // Check for function calls in delta
                            if let InteractionContent::FunctionCall { id, name, args, .. } = &delta {
                                accumulated_calls.push((id.clone(), name.clone(), args.clone()));
                            }
                            yield AutoFunctionStreamChunk::Delta(delta);
                        }
                        StreamChunk::Complete(response) => {
                            complete_response = Some(response);
                        }
                        // Ignore unknown chunk types for forward compatibility
                        _ => {}
                    }
                }

                // Get the complete response (should always be present after stream ends)
                let response = complete_response.ok_or_else(|| {
                    GenaiError::Internal(
                        "Stream ended without Complete event".to_string()
                    )
                })?;

                // Check for function calls from two possible sources:
                // 1. response.function_calls(): Populated when the Complete event includes
                //    FunctionCall content items (typical for non-streaming or when the API
                //    batches function calls into the final response)
                // 2. accumulated_calls: Populated from Delta events during streaming when
                //    the API sends FunctionCall content incrementally via deltas
                //
                // We check both because API behavior may vary; prefer Complete response
                // data when available as it represents the finalized state.
                let response_function_calls = response.function_calls();
                let has_function_calls = !response_function_calls.is_empty() || !accumulated_calls.is_empty();

                // If no function calls, we're done!
                if !has_function_calls {
                    yield AutoFunctionStreamChunk::Complete(response);
                    return;
                }

                // Signal that we're executing functions (pass the response for inspection)
                yield AutoFunctionStreamChunk::ExecutingFunctions(response.clone());

                // Determine which function calls to execute.
                // Prefer response.function_calls() if available (finalized data),
                // fall back to accumulated deltas otherwise.
                let calls_to_execute: Vec<(String, String, serde_json::Value)> = if !response_function_calls.is_empty() {
                    let mut calls = Vec::new();
                    for call in &response_function_calls {
                        let call_id = call.id.ok_or_else(|| {
                            error!(
                                "Function call '{}' is missing required call_id field.",
                                call.name
                            );
                            GenaiError::InvalidInput(format!(
                                "Function call '{}' is missing required call_id field. \
                                 This may indicate an API response format change.",
                                call.name
                            ))
                        })?;
                        calls.push((call_id.to_string(), call.name.to_string(), call.args.clone()));
                    }
                    calls
                } else {
                    let mut calls = Vec::new();
                    for (id, name, args) in &accumulated_calls {
                        let call_id = id.as_ref().ok_or_else(|| {
                            error!(
                                "Function call '{}' is missing required call_id field.",
                                name
                            );
                            GenaiError::InvalidInput(format!(
                                "Function call '{}' is missing required call_id field. \
                                 This may indicate an API response format change.",
                                name
                            ))
                        })?;
                        calls.push((call_id.clone(), name.clone(), args.clone()));
                    }
                    calls
                };

                // Build function results for next iteration
                let mut function_results_content = Vec::new();
                let mut execution_results = Vec::new();

                for (call_id, name, args) in &calls_to_execute {
                    // Execute the function with timing
                    let start = Instant::now();
                    let result = if let Some(function) = function_registry.get(name) {
                        match function.call(args.clone()).await {
                            Ok(result) => result,
                            Err(e) => {
                                error!(
                                    "Function execution failed: function='{}', error='{}'",
                                    name, e
                                );
                                json!({ "error": e.to_string() })
                            }
                        }
                    } else {
                        warn!(
                            "Function not found in registry: function='{}'. Informing model.",
                            name
                        );
                        json!({ "error": format!("Function '{}' is not available or not found.", name) })
                    };
                    let duration = start.elapsed();

                    // Track result for yielding
                    execution_results.push(FunctionExecutionResult::new(
                        name.clone(),
                        call_id.clone(),
                        result.clone(),
                        duration,
                    ));

                    // Add function result content for API
                    function_results_content.push(function_result_content(
                        name.clone(),
                        call_id.clone(),
                        result,
                    ));
                }

                // Yield function results
                yield AutoFunctionStreamChunk::FunctionResults(execution_results);

                // Create new request with function results
                request.previous_interaction_id = Some(response.id);
                request.input = InteractionInput::Content(function_results_content);
            }

            // If we get here, we exceeded max loops
            Err(GenaiError::Internal(format!(
                "Exceeded maximum function call loops ({max_loops}). \
                 The model may be stuck in a loop. Check your function implementations, \
                 increase the limit using with_max_function_call_loops(), \
                 or use manual function calling for more control."
            )))?;
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
