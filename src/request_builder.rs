use crate::GenaiError;
use crate::client::Client;
use crate::types::{FunctionDeclaration, GenerateContentResponse};

use futures_util::StreamExt;
use genai_client::{self};
use serde_json::Value;

use futures_util::stream;
use std::str;

/// Builder for generating content with optional system instructions and function calling.
#[derive(Debug)]
pub struct GenerateContentBuilder<'a> {
    pub(crate) client: &'a Client,
    pub(crate) model_name: &'a str,
    pub(crate) prompt_text: Option<&'a str>,
    pub(crate) system_instruction: Option<&'a str>,
    pub(crate) tools: Option<Vec<genai_client::models::request::Tool>>,
    pub(crate) tool_config: Option<genai_client::models::request::ToolConfig>,
}

impl<'a> GenerateContentBuilder<'a> {
    /// Creates a new builder for generating content.
    pub(crate) const fn new(client: &'a Client, model_name: &'a str) -> Self {
        Self {
            client,
            model_name,
            prompt_text: None,
            system_instruction: None,
            tools: None,
            tool_config: None,
        }
    }

    /// Helper function to convert public `FunctionDeclaration` to internal Tool.
    fn convert_public_fn_decl_to_tool(
        function: FunctionDeclaration,
    ) -> genai_client::models::request::Tool {
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

        let internal_function_parameters = genai_client::models::request::FunctionParameters {
            type_: schema_type,
            properties: schema_properties,
            required: function.required,
        };

        genai_client::models::request::Tool {
            function_declarations: Some(vec![genai_client::models::request::FunctionDeclaration {
                name: function.name,
                description: function.description,
                parameters: internal_function_parameters,
            }]),
            code_execution: None,
        }
    }

    /// Sets the prompt text for the request.
    #[must_use]
    pub const fn with_prompt(mut self, prompt: &'a str) -> Self {
        self.prompt_text = Some(prompt);
        self
    }

    /// Sets the system instruction for the request.
    #[must_use]
    pub const fn with_system_instruction(mut self, instruction: &'a str) -> Self {
        self.system_instruction = Some(instruction);
        self
    }

    /// Adds a function that the model can call.
    #[must_use]
    pub fn with_function(mut self, function: FunctionDeclaration) -> Self {
        let tool = Self::convert_public_fn_decl_to_tool(function);
        self.tools.get_or_insert_with(Vec::new).push(tool);
        self
    }

    /// Enables the code execution tool for the model.
    #[must_use]
    pub fn with_code_execution(mut self) -> Self {
        self.tools
            .get_or_insert_with(Vec::new)
            .push(genai_client::models::request::Tool {
                function_declarations: None,
                code_execution: Some(genai_client::models::request::CodeExecution::default()),
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
    pub fn with_tool_config(mut self, config: genai_client::models::request::ToolConfig) -> Self {
        self.tool_config = Some(config);
        self
    }

    /// Generates content based on the configured parameters.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails, response parsing fails, or the API returns an error.
    // TODO: consider also having a generate_parts that returns 1+ Parts directly.
    pub async fn generate(self) -> Result<GenerateContentResponse, GenaiError> {
        let prompt_text = self.prompt_text.ok_or_else(|| {
            GenaiError::Internal("Prompt text is required for content generation".to_string())
        })?;

        let request_body = genai_client::models::request::GenerateContentRequest {
            contents: vec![genai_client::models::request::Content {
                parts: vec![genai_client::models::request::Part {
                    text: Some(prompt_text.to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: None,
            }],
            system_instruction: self.system_instruction.map(|text| {
                genai_client::models::request::Content {
                    parts: vec![genai_client::models::request::Part {
                        text: Some(text.to_string()),
                        function_call: None,
                        function_response: None,
                    }],
                    role: None,
                }
            }),
            tools: self.tools,
            tool_config: self.tool_config,
        };

        self.client
            .generate_from_request(self.model_name, request_body)
            .await
    }

    /// Generates content as a stream based on the configured parameters.
    pub fn stream(
        self,
    ) -> impl stream::Stream<Item = Result<GenerateContentResponse, GenaiError>> + Send + 'a {
        let Some(prompt_text) = self.prompt_text else {
            return stream::once(async move {
                Err(GenaiError::Internal(
                    "Prompt text is required for content generation".to_string(),
                ))
            })
            .boxed();
        };

        let request_body = genai_client::models::request::GenerateContentRequest {
            contents: vec![genai_client::models::request::Content {
                parts: vec![genai_client::models::request::Part {
                    text: Some(prompt_text.to_string()),
                    function_call: None,
                    function_response: None,
                }],
                role: None,
            }],
            system_instruction: self.system_instruction.map(|text| {
                genai_client::models::request::Content {
                    parts: vec![genai_client::models::request::Part {
                        text: Some(text.to_string()),
                        function_call: None,
                        function_response: None,
                    }],
                    role: None,
                }
            }),
            tools: self.tools,
            tool_config: self.tool_config,
        };

        self.client
            .stream_from_request(self.model_name, request_body)
            .boxed() // Ensure the return type matches Client::stream_from_request
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Mock client for testing
    fn create_test_client() -> Client {
        Client::new("test-key".to_string(), None)
    }

    #[test]
    fn test_builder_initialization() {
        let client = create_test_client();
        let builder = GenerateContentBuilder::new(&client, "test-model");
        
        assert_eq!(builder.model_name, "test-model");
        assert!(builder.prompt_text.is_none());
        assert!(builder.system_instruction.is_none());
        assert!(builder.tools.is_none());
        assert!(builder.tool_config.is_none());
    }

    #[test]
    fn test_builder_with_prompt() {
        let client = create_test_client();
        let builder = GenerateContentBuilder::new(&client, "test-model")
            .with_prompt("Hello world");
        
        assert_eq!(builder.prompt_text, Some("Hello world"));
    }

    #[test]
    fn test_builder_with_system_instruction() {
        let client = create_test_client();
        let builder = GenerateContentBuilder::new(&client, "test-model")
            .with_system_instruction("Be helpful");
        
        assert_eq!(builder.system_instruction, Some("Be helpful"));
    }

    #[test]
    fn test_convert_function_declaration_basic() {
        let func_decl = FunctionDeclaration {
            name: "test_func".to_string(),
            description: "Test function".to_string(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "param1": {"type": "string"}
                }
            })),
            required: vec!["param1".to_string()],
        };
        
        let tool = GenerateContentBuilder::convert_public_fn_decl_to_tool(func_decl);
        
        assert!(tool.function_declarations.is_some());
        let func_decls = tool.function_declarations.unwrap();
        assert_eq!(func_decls.len(), 1);
        
        let internal_func = &func_decls[0];
        assert_eq!(internal_func.name, "test_func");
        assert_eq!(internal_func.description, "Test function");
        assert_eq!(internal_func.parameters.type_, "object");
        assert_eq!(internal_func.parameters.required, vec!["param1"]);
    }

    #[test]
    fn test_convert_function_declaration_no_parameters() {
        let func_decl = FunctionDeclaration {
            name: "no_params".to_string(),
            description: "No parameters".to_string(),
            parameters: None,
            required: vec![],
        };
        
        let tool = GenerateContentBuilder::convert_public_fn_decl_to_tool(func_decl);
        
        let func_decls = tool.function_declarations.unwrap();
        let internal_func = &func_decls[0];
        
        assert_eq!(internal_func.parameters.type_, "object");
        assert!(internal_func.parameters.properties.is_null());
        assert!(internal_func.parameters.required.is_empty());
    }

    #[test]
    fn test_convert_function_declaration_missing_type() {
        let func_decl = FunctionDeclaration {
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
        
        // Should default to "object" when type is missing
        assert_eq!(internal_func.parameters.type_, "object");
    }

    #[test]
    fn test_with_function() {
        let client = create_test_client();
        let func = FunctionDeclaration {
            name: "test".to_string(),
            description: "Test".to_string(),
            parameters: None,
            required: vec![],
        };
        
        let builder = GenerateContentBuilder::new(&client, "test-model")
            .with_function(func);
        
        assert!(builder.tools.is_some());
        assert_eq!(builder.tools.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_with_multiple_functions() {
        let client = create_test_client();
        let func1 = FunctionDeclaration {
            name: "func1".to_string(),
            description: "Function 1".to_string(),
            parameters: None,
            required: vec![],
        };
        let func2 = FunctionDeclaration {
            name: "func2".to_string(),
            description: "Function 2".to_string(),
            parameters: None,
            required: vec![],
        };
        
        let builder = GenerateContentBuilder::new(&client, "test-model")
            .with_function(func1)
            .with_function(func2);
        
        assert_eq!(builder.tools.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_with_functions_vector() {
        let client = create_test_client();
        let functions = vec![
            FunctionDeclaration {
                name: "func1".to_string(),
                description: "Function 1".to_string(),
                parameters: None,
                required: vec![],
            },
            FunctionDeclaration {
                name: "func2".to_string(),
                description: "Function 2".to_string(),
                parameters: None,
                required: vec![],
            },
        ];
        
        let builder = GenerateContentBuilder::new(&client, "test-model")
            .with_functions(functions);
        
        assert_eq!(builder.tools.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_with_code_execution() {
        let client = create_test_client();
        let builder = GenerateContentBuilder::new(&client, "test-model")
            .with_code_execution();
        
        assert!(builder.tools.is_some());
        let tools = builder.tools.as_ref().unwrap();
        assert_eq!(tools.len(), 1);
        assert!(tools[0].code_execution.is_some());
        assert!(tools[0].function_declarations.is_none());
    }

    #[test]
    fn test_builder_chaining() {
        let client = create_test_client();
        let func = FunctionDeclaration {
            name: "test".to_string(),
            description: "Test".to_string(),
            parameters: None,
            required: vec![],
        };
        
        let builder = GenerateContentBuilder::new(&client, "test-model")
            .with_prompt("Hello")
            .with_system_instruction("Be helpful")
            .with_function(func)
            .with_code_execution();
        
        assert_eq!(builder.prompt_text, Some("Hello"));
        assert_eq!(builder.system_instruction, Some("Be helpful"));
        assert_eq!(builder.tools.as_ref().unwrap().len(), 2); // 1 function + 1 code execution
    }
}
