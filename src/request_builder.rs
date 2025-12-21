use crate::GenaiError;
use crate::client::Client;
use crate::content_api::{model_function_calls_request, model_text, user_text, user_tool_response};
use crate::function_calling::get_global_function_registry;
use crate::types::{
    FunctionCall as PublicFunctionCall, FunctionDeclaration, GenerateContentResponse,
};

use futures_util::StreamExt;
use genai_client::{
    self,
    models::request::GenerateContentRequest as InternalGenerateContentRequest,
    Content as InternalContent,
    FunctionCall as InternalFunctionCall,
    Part as InternalPart,
    Tool as InternalTool,
    ToolConfig as InternalToolConfig,
};
use serde_json::{Value, json};

const MAX_FUNCTION_CALL_LOOPS: usize = 5; // Max iterations for auto function calling

/// Builder for generating content with optional system instructions and function calling.
#[derive(Debug)]
pub struct GenerateContentBuilder<'a> {
    pub(crate) client: &'a Client,
    pub(crate) model_name: &'a str,
    pub(crate) prompt_text: Option<&'a str>, // Used for single-turn generation
    pub(crate) initial_contents: Option<Vec<InternalContent>>, // Used for multi-turn/auto-functions
    pub(crate) system_instruction: Option<&'a str>,
    pub(crate) tools: Option<Vec<InternalTool>>,
    pub(crate) tool_config: Option<InternalToolConfig>,
}

// Helper to convert public FunctionCall to internal FunctionCall
fn to_internal_function_call(public_fc: &PublicFunctionCall) -> InternalFunctionCall {
    InternalFunctionCall {
        name: public_fc.name.clone(),
        args: public_fc.args.clone(), // Assuming serde_json::Value is Clone
    }
}

fn to_internal_function_calls(public_fcs: &[PublicFunctionCall]) -> Vec<InternalFunctionCall> {
    public_fcs.iter().map(to_internal_function_call).collect()
}

impl<'a> GenerateContentBuilder<'a> {
    /// Creates a new builder for generating content.
    pub(crate) const fn new(client: &'a Client, model_name: &'a str) -> Self {
        Self {
            client,
            model_name,
            prompt_text: None,
            initial_contents: None,
            system_instruction: None,
            tools: None,
            tool_config: None,
        }
    }

    /// Helper function to convert public `FunctionDeclaration` to internal Tool.
    fn convert_public_fn_decl_to_tool(function: FunctionDeclaration) -> InternalTool {
        let schema_properties = function
            .parameters
            .as_ref()
            .and_then(|p| p.get("properties"))
            .cloned()
            .unwrap_or(Value::Null);
        let schema_type = function
            .parameters
            .as_ref()
            .and_then(|p| p.get("type"))
            .and_then(Value::as_str)
            .unwrap_or("object")
            .to_string();

        let internal_function_parameters = genai_client::FunctionParameters {
            type_: schema_type,
            properties: schema_properties,
            required: function.required,
        };

        InternalTool {
            function_declarations: Some(vec![genai_client::FunctionDeclaration {
                name: function.name,
                description: function.description,
                parameters: internal_function_parameters,
            }]),
            code_execution: None,
        }
    }

    /// Sets the prompt text for the request.
    /// Note: If using `generate_with_auto_functions`, prefer `with_initial_user_text` or `with_contents`.
    #[must_use]
    pub const fn with_prompt(mut self, prompt: &'a str) -> Self {
        self.prompt_text = Some(prompt);
        self
    }

    /// Sets the initial user text for a conversation that might involve automatic function calls.
    /// This will be the first user message in the conversation history.
    #[must_use]
    pub fn with_initial_user_text(mut self, user_text_prompt: &'a str) -> Self {
        self.initial_contents = Some(vec![user_text(user_text_prompt.to_string())]);
        self
    }

    /// Sets the initial conversation contents. Use this if you need to start
    /// with a more complex history than a single user prompt.
    #[must_use]
    pub fn with_contents(mut self, contents: Vec<InternalContent>) -> Self {
        self.initial_contents = Some(contents);
        self
    }

    /// Sets the system instruction for the request.
    #[must_use]
    pub const fn with_system_instruction(mut self, instruction: &'a str) -> Self {
        self.system_instruction = Some(instruction);
        self
    }

    /// Adds a function that the model can call.
    /// Note: For `generate_with_auto_functions`, functions are auto-discovered.
    /// Explicitly adding functions here might be used if auto-discovery is bypassed or supplemented.
    #[must_use]
    pub fn with_function(mut self, function: FunctionDeclaration) -> Self {
        let tool = Self::convert_public_fn_decl_to_tool(function);
        self.tools.get_or_insert_with(Vec::new).push(tool);
        self
    }

    /// Enables the code execution tool for the model.
    #[must_use]
    pub fn with_code_execution(mut self) -> Self {
        self.tools.get_or_insert_with(Vec::new).push(InternalTool {
            function_declarations: None,
            code_execution: Some(genai_client::CodeExecution::default()),
        });
        self
    }

    #[must_use]
    pub fn with_functions(mut self, functions: Vec<FunctionDeclaration>) -> Self {
        let tools_vec = functions
            .into_iter()
            .map(Self::convert_public_fn_decl_to_tool)
            .collect::<Vec<_>>();

        if !tools_vec.is_empty() {
            self.tools.get_or_insert_with(Vec::new).extend(tools_vec);
        }
        self
    }

    /// Sets the tool configuration for the request.
    #[must_use]
    pub fn with_tool_config(mut self, config: InternalToolConfig) -> Self {
        self.tool_config = Some(config);
        self
    }

    /// Generates content based on the configured parameters (single turn, no automatic function execution loop).
    ///
    /// # Errors
    /// Returns an error if:
    /// - The HTTP request fails
    /// - Response parsing fails
    /// - The API returns an error
    /// - Neither prompt text nor initial contents are provided
    pub async fn generate(self) -> Result<GenerateContentResponse, GenaiError> {
        let current_contents = if let Some(initial_contents) = self.initial_contents {
            initial_contents
        } else if let Some(prompt_text) = self.prompt_text {
            vec![user_text(prompt_text.to_string())]
        } else {
            return Err(GenaiError::Internal(
                "Prompt text or initial contents are required for content generation".to_string(),
            ));
        };

        let request_body = InternalGenerateContentRequest {
            contents: current_contents,
            system_instruction: self.system_instruction.map(|text| InternalContent {
                parts: vec![InternalPart {
                    text: Some(text.to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: Some("system".to_string()),
            }),
            tools: self.tools,
            tool_config: self.tool_config,
        };

        self.client
            .generate_from_request(self.model_name, request_body)
            .await
    }

    /// Generates content with automatic function call handling.
    ///
    /// This method will:
    /// 1. Send the initial prompt/contents to the model.
    /// 2. If the model requests function calls, these functions (auto-discovered via macros)
    ///    will be executed locally.
    /// 3. The function results will be sent back to the model.
    /// 4. This process repeats until the model responds with text or a loop limit is reached.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails, response parsing fails, the API returns an error,
    /// or if a function execution fails critically.
    pub async fn generate_with_auto_functions(self) -> Result<GenerateContentResponse, GenaiError> {
        let mut conversation_history = self
            .initial_contents
            .or_else(|| self.prompt_text.map(|pt| vec![user_text(pt.to_string())]))
            .ok_or_else(|| {
                GenaiError::Internal(
                    "Initial prompt or contents are required for automatic function calling."
                        .to_string(),
                )
            })?;

        let function_registry = get_global_function_registry();
        let mut tools_to_send = self.tools.clone();

        if tools_to_send.is_none() {
            let auto_discovered_declarations = function_registry.all_declarations();
            if !auto_discovered_declarations.is_empty() {
                tools_to_send = Some(
                    auto_discovered_declarations
                        .into_iter()
                        .map(Self::convert_public_fn_decl_to_tool)
                        .collect(),
                );
            }
        }

        for _loop_count in 0..MAX_FUNCTION_CALL_LOOPS {
            let request_body = InternalGenerateContentRequest {
                contents: conversation_history.clone(),
                system_instruction: self.system_instruction.map(|text| InternalContent {
                    parts: vec![InternalPart {
                        text: Some(text.to_string()),
                        function_call: None,
                        function_response: None,
                    }],
                    role: Some("system".to_string()),
                }),
                tools: tools_to_send.clone(),
                tool_config: self.tool_config.clone(),
            };

            let response = self
                .client
                .generate_from_request(self.model_name, request_body)
                .await?;

            if let Some(text_part) = &response.text {
                if response.function_calls.is_none() || !text_part.trim().is_empty() {
                    conversation_history.push(model_text(text_part.clone()));
                }
            }

            if let Some(public_function_calls) = &response.function_calls {
                if public_function_calls.is_empty() {
                    return Ok(response);
                }
                let internal_fcs = to_internal_function_calls(public_function_calls);
                conversation_history.push(model_function_calls_request(internal_fcs));

                for call_to_execute in public_function_calls {
                    if let Some(function_to_call) = function_registry.get(&call_to_execute.name) {
                        match function_to_call.call(call_to_execute.args.clone()).await {
                            Ok(result) => {
                                conversation_history
                                    .push(user_tool_response(call_to_execute.name.clone(), result));
                            }
                            Err(e) => {
                                eprintln!(
                                    "Error executing function '{}': {}",
                                    call_to_execute.name, e
                                );
                                let error_response_val = json!({ "error": e.to_string() });
                                conversation_history.push(user_tool_response(
                                    call_to_execute.name.clone(),
                                    error_response_val,
                                ));
                            }
                        }
                    } else {
                        eprintln!(
                            "Function '{}' not found in registry. Informing model.",
                            call_to_execute.name
                        );
                        let error_response_val = json!({ "error": format!("Function '{}' is not available or not found.", call_to_execute.name) });
                        conversation_history.push(user_tool_response(
                            call_to_execute.name.clone(),
                            error_response_val,
                        ));
                    }
                }
            } else {
                return Ok(response);
            }
        }

        Err(GenaiError::Internal(format!(
            "Exceeded maximum function call loops ({MAX_FUNCTION_CALL_LOOPS}). Returning last known conversation state."
        )))
    }

    /// Generates content as a stream based on the configured parameters.
    /// Note: Automatic function calling is not supported for streaming responses directly in this builder.
    /// You would typically get a function call, execute it, then start a new stream request with the result.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Prompt text is not provided (streaming requires a prompt)
    pub fn generate_stream(
        self,
    ) -> Result<impl StreamExt<Item = Result<GenerateContentResponse, GenaiError>>, GenaiError>
    {
        let prompt_text = self.prompt_text.ok_or_else(|| {
            GenaiError::Internal(
                "Prompt text is required for streaming content generation".to_string(),
            )
        })?;

        let request_body = InternalGenerateContentRequest {
            contents: vec![InternalContent {
                parts: vec![InternalPart {
                    text: Some(prompt_text.to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: None,
            }],
            system_instruction: self.system_instruction.map(|text| InternalContent {
                parts: vec![InternalPart {
                    text: Some(text.to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: Some("system".to_string()),
            }),
            tools: self.tools,
            tool_config: self.tool_config,
        };

        Ok(self
            .client
            .stream_from_request(self.model_name, request_body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Client;
    use crate::types::FunctionDeclaration as PublicFunctionDeclaration;
    use serde_json::json;

    fn create_test_client() -> Client {
        Client::builder("test-api-key".to_string()).build()
    }

    #[test]
    fn test_builder_with_prompt() {
        let client = create_test_client();
        let builder = GenerateContentBuilder::new(&client, "test-model").with_prompt("Hello");
        assert_eq!(builder.prompt_text, Some("Hello"));
    }

    #[test]
    fn test_builder_with_initial_user_text() {
        let client = create_test_client();
        let builder =
            GenerateContentBuilder::new(&client, "test-model").with_initial_user_text("Hi there");
        assert!(builder.initial_contents.is_some());
        let contents = builder.initial_contents.unwrap();
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].parts[0].text.as_deref(), Some("Hi there"));
        assert_eq!(contents[0].role.as_deref(), Some("user"));
    }

    #[test]
    fn test_builder_with_system_instruction() {
        let client = create_test_client();
        let builder = GenerateContentBuilder::new(&client, "test-model")
            .with_system_instruction("Be concise");
        assert_eq!(builder.system_instruction, Some("Be concise"));
    }

    #[test]
    fn test_convert_public_fn_decl_to_tool_basic() {
        let func_decl = PublicFunctionDeclaration {
            name: "my_func".to_string(),
            description: "Does something".to_string(),
            parameters: Some(
                json!({ "type": "object", "properties": { "arg1": { "type": "string"}}}),
            ),
            required: vec!["arg1".to_string()],
        };
        let tool = GenerateContentBuilder::convert_public_fn_decl_to_tool(func_decl);
        assert!(tool.function_declarations.is_some());
        let decls = tool.function_declarations.unwrap();
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "my_func");
        assert_eq!(
            decls[0]
                .parameters
                .properties
                .get("arg1")
                .unwrap()
                .get("type")
                .unwrap()
                .as_str(),
            Some("string")
        );
    }

    #[test]
    fn test_convert_function_declaration_empty_params() {
        let func_decl = PublicFunctionDeclaration {
            name: "test".to_string(),
            description: "Test".to_string(),
            parameters: None,
            required: vec![],
        };
        let tool = GenerateContentBuilder::convert_public_fn_decl_to_tool(func_decl);
        let internal_func = &tool.function_declarations.as_ref().unwrap()[0];
        assert_eq!(internal_func.parameters.type_, "object");
        assert_eq!(internal_func.parameters.properties, Value::Null);
    }

    #[test]
    fn test_convert_function_declaration_missing_type() {
        let func_decl = PublicFunctionDeclaration {
            name: "test".to_string(),
            description: "Test".to_string(),
            parameters: Some(json!({
                "properties": {
                    "param1": {"type": "string"}
                }
            })),
            required: vec![],
        };

        let tool = GenerateContentBuilder::convert_public_fn_decl_to_tool(func_decl);
        let internal_func = &tool.function_declarations.unwrap()[0];
        assert_eq!(internal_func.parameters.type_, "object");
    }

    #[test]
    fn test_with_function() {
        let client = create_test_client();
        let func = PublicFunctionDeclaration {
            name: "test".to_string(),
            description: "Test".to_string(),
            parameters: None,
            required: vec![],
        };

        let builder = GenerateContentBuilder::new(&client, "test-model").with_function(func);

        assert!(builder.tools.is_some());
        assert_eq!(builder.tools.as_ref().unwrap().len(), 1);
    }
}
