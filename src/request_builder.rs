use crate::GenaiError;
use crate::builder_traits::HasToolsField;
use crate::client::Client;

use futures_util::{StreamExt, stream::BoxStream};
use genai_client::{
    self, CreateInteractionRequest, GenerationConfig, InteractionContent, InteractionInput,
    InteractionResponse, Tool as InternalTool,
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
/// # use rust_genai::Client;
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
/// // Streaming interaction with tools
/// let mut stream = client.interaction()
///     .with_model("gemini-3-flash-preview")
///     .with_text("Calculate 2 + 2")
///     .with_tools(vec![/* tools */])
///     .create_stream();
///
/// while let Some(chunk) = stream.next().await {
///     println!("{:?}", chunk?);
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

    /// Sets response modalities (e.g., ["IMAGE"]).
    pub fn with_response_modalities(mut self, modalities: Vec<String>) -> Self {
        self.response_modalities = Some(modalities);
        self
    }

    /// Sets a JSON schema for structured output.
    pub fn with_response_format(mut self, format: serde_json::Value) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Sets generation configuration (temperature, max tokens, etc.).
    pub fn with_generation_config(mut self, config: GenerationConfig) -> Self {
        self.generation_config = Some(config);
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
    /// # use rust_genai::Client;
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
    ///     let response = chunk?;
    ///     println!("{:?}", response);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_stream(self) -> BoxStream<'a, Result<InteractionResponse, GenaiError>> {
        let client = self.client;
        Box::pin(async_stream::try_stream! {
            let request = self.build_request()?;
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
    /// Functions are auto-discovered from the global registry (via `#[generate_function_declaration]` macro)
    /// or can be explicitly provided via `.with_function()` or `.with_tools()`.
    ///
    /// The loop automatically stops when:
    /// - Model returns text without function calls
    /// - Function calls array is empty
    /// - Maximum iterations is reached (default 5, configurable via `with_max_function_call_loops()`)
    ///
    /// # Example
    /// ```no_run
    /// # use rust_genai::{Client, FunctionDeclaration};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder("api_key".to_string()).build();
    ///
    /// // Functions are auto-discovered from registry
    /// let response = client.interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("What's the weather in Tokyo?")
    ///     .create_with_auto_functions()
    ///     .await?;
    ///
    /// println!("{}", response.text().unwrap_or("No text"));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No input was provided
    /// - Neither model nor agent was specified
    /// - The API request fails
    /// - Maximum function call loops is exceeded (default 5, configurable via `with_max_function_call_loops()`)
    pub async fn create_with_auto_functions(self) -> Result<InteractionResponse, GenaiError> {
        use crate::function_calling::get_global_function_registry;
        use crate::interactions_api::function_result_content;
        use log::{error, warn};
        use serde_json::json;

        let client = self.client;
        let max_loops = self.max_function_call_loops;
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

        // Main auto-function loop (configurable iterations to prevent infinite loops)
        for _loop_count in 0..max_loops {
            let response = client.create_interaction(request.clone()).await?;

            // Extract function calls using convenience method
            let function_calls = response.function_calls();

            // If no function calls, we're done!
            if function_calls.is_empty() {
                return Ok(response);
            }

            // Build function results for next iteration
            let mut function_results = Vec::new();

            for (call_id, name, args, _thought_signature) in function_calls {
                // Validate that we have a call_id (required by API)
                let call_id = match call_id {
                    Some(id) => id,
                    None => {
                        warn!(
                            "Function call '{}' missing call_id. Using fallback value.",
                            name
                        );
                        "unknown"
                    }
                };

                // Execute the function
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

                // Add function result (only the result, not the call - server has it via previous_interaction_id)
                function_results.push(function_result_content(
                    name.to_string(),
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
             increase the limit with with_max_function_call_loops(), \
             or use manual function calling for more control."
        )))
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

        // Validate that we have either model or agent
        if self.model.is_none() && self.agent.is_none() {
            return Err(GenaiError::InvalidInput(
                "Either model or agent must be specified".to_string(),
            ));
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

// Implement trait for function calling support
impl HasToolsField for InteractionBuilder<'_> {
    fn get_tools_mut(&mut self) -> &mut Option<Vec<InternalTool>> {
        &mut self.tools
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder_traits::WithFunctionCalling;
    use crate::{Client, FunctionDeclaration};
    use genai_client::Tool;
    use serde_json::json;

    fn create_test_client() -> Client {
        Client::builder("test-api-key".to_string()).build()
    }

    #[test]
    fn test_function_declaration_builder() {
        let func_decl = FunctionDeclaration::builder("my_func")
            .description("Does something")
            .parameter("arg1", json!({"type": "string"}))
            .required(vec!["arg1".to_string()])
            .build();

        assert_eq!(func_decl.name(), "my_func");
        assert_eq!(func_decl.description(), "Does something");
        assert_eq!(func_decl.parameters().type_(), "object");
        assert_eq!(
            func_decl
                .parameters()
                .properties()
                .get("arg1")
                .unwrap()
                .get("type")
                .unwrap()
                .as_str(),
            Some("string")
        );
        assert_eq!(func_decl.parameters().required(), vec!["arg1".to_string()]);
    }

    #[test]
    fn test_function_declaration_into_tool() {
        let func_decl = FunctionDeclaration::builder("test")
            .description("Test function")
            .build();

        let tool = func_decl.into_tool();
        match tool {
            Tool::Function { name, .. } => {
                assert_eq!(name, "test");
            }
            _ => panic!("Expected Tool::Function variant"),
        }
    }

    // --- InteractionBuilder Tests ---

    #[test]
    fn test_interaction_builder_with_model() {
        let client = create_test_client();
        let builder = client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_text("Hello");

        assert_eq!(builder.model.as_deref(), Some("gemini-3-flash-preview"));
        assert!(builder.agent.is_none());
        assert!(matches!(
            builder.input,
            Some(genai_client::InteractionInput::Text(_))
        ));
    }

    #[test]
    fn test_interaction_builder_with_agent() {
        let client = create_test_client();
        let builder = client
            .interaction()
            .with_agent("deep-research-pro")
            .with_text("Research topic");

        assert!(builder.model.is_none());
        assert_eq!(builder.agent.as_deref(), Some("deep-research-pro"));
    }

    #[test]
    fn test_interaction_builder_with_previous_interaction() {
        let client = create_test_client();
        let builder = client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_text("Follow-up question")
            .with_previous_interaction("interaction_123");

        assert_eq!(
            builder.previous_interaction_id.as_deref(),
            Some("interaction_123")
        );
    }

    #[test]
    fn test_interaction_builder_with_system_instruction() {
        let client = create_test_client();
        let builder = client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_text("Hello")
            .with_system_instruction("You are a helpful assistant");

        assert!(matches!(
            builder.system_instruction,
            Some(genai_client::InteractionInput::Text(_))
        ));
    }

    #[test]
    fn test_interaction_builder_with_generation_config() {
        let client = create_test_client();
        let config = genai_client::GenerationConfig {
            temperature: Some(0.7),
            max_output_tokens: Some(1000),
            top_p: Some(0.9),
            top_k: Some(40),
            thinking_level: Some("medium".to_string()),
        };

        let builder = client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_text("Hello")
            .with_generation_config(config.clone());

        assert!(builder.generation_config.is_some());
        assert_eq!(
            builder.generation_config.as_ref().unwrap().temperature,
            Some(0.7)
        );
    }

    #[test]
    fn test_interaction_builder_with_function() {
        let client = create_test_client();
        let func = FunctionDeclaration::builder("test_func")
            .description("Test function")
            .parameter("location", json!({"type": "string"}))
            .required(vec!["location".to_string()])
            .build();

        let builder = client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_text("Call a function")
            .with_function(func);

        assert!(builder.tools.is_some());
        assert_eq!(builder.tools.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_interaction_builder_with_background() {
        let client = create_test_client();
        let builder = client
            .interaction()
            .with_agent("deep-research-pro")
            .with_text("Long running task")
            .with_background(true);

        assert_eq!(builder.background, Some(true));
    }

    #[test]
    fn test_interaction_builder_with_store() {
        let client = create_test_client();
        let builder = client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_text("Temporary interaction")
            .with_store(false);

        assert_eq!(builder.store, Some(false));
    }

    #[test]
    fn test_interaction_builder_build_request_success() {
        let client = create_test_client();
        let builder = client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_text("Hello");

        let result = builder.build_request();
        assert!(result.is_ok());

        let request = result.unwrap();
        assert_eq!(request.model.as_deref(), Some("gemini-3-flash-preview"));
        assert!(matches!(
            request.input,
            genai_client::InteractionInput::Text(_)
        ));
    }

    #[test]
    fn test_interaction_builder_build_request_missing_input() {
        let client = create_test_client();
        let builder = client.interaction().with_model("gemini-3-flash-preview");

        let result = builder.build_request();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::GenaiError::InvalidInput(_)
        ));
    }

    #[test]
    fn test_interaction_builder_build_request_missing_model_and_agent() {
        let client = create_test_client();
        let builder = client.interaction().with_text("Hello");

        let result = builder.build_request();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::GenaiError::InvalidInput(_)
        ));
    }

    #[test]
    fn test_interaction_builder_with_response_modalities() {
        let client = create_test_client();
        let builder = client
            .interaction()
            .with_model("gemini-3-flash-preview")
            .with_text("Generate an image")
            .with_response_modalities(vec!["IMAGE".to_string()]);

        assert_eq!(
            builder.response_modalities.as_ref().unwrap(),
            &vec!["IMAGE".to_string()]
        );
    }
}
