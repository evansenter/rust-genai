use serde_json::Value;
use serde::{Deserialize, Serialize};

/// Represents a function call in the response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionCall {
    /// The name of the function to call.
    pub name: String,
    /// The arguments to pass to the function.
    pub args: Value,
}

/// Represents the result of a code execution, including the executed code and its output.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CodeExecutionResult {
    /// The code that was executed.
    pub code: String,
    /// The output from the code execution.
    pub output: String,
}

/// Represents a successful response from a generate content request.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GenerateContentResponse {
    /// The generated text content, if any.
    pub text: Option<String>,
    /// Function calls, if any, requested by the model.
    pub function_calls: Option<Vec<FunctionCall>>,
    /// The results of any code executions performed by the model.
    pub code_execution_results: Option<Vec<CodeExecutionResult>>,
}

/// Represents a function declaration that can be used by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    /// The name of the function.
    pub name: String,
    /// A description of what the function does.
    pub description: String,
    /// The JSON Schema for the function's parameters.
    pub parameters: Option<Value>,
    /// The names of required parameters.
    pub required: Vec<String>,
}

impl FunctionDeclaration {
    /// Converts this public `FunctionDeclaration` to the internal `genai_client::Tool` format.
    #[must_use]
    pub fn to_tool(self) -> genai_client::Tool {
        let properties = self.parameters.as_ref().and_then(|p| p.get("properties")).cloned();
        let required_from_params = self.parameters.as_ref().and_then(|p| p.get("required")).cloned();

        let internal_fd = genai_client::FunctionDeclaration {
            name: self.name,
            description: self.description,
            parameters: genai_client::FunctionParameters {
                type_: "object".to_string(),
                properties: properties.unwrap_or(Value::Null),
                required: if let Some(Value::Array(arr)) = required_from_params {
                    arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
                } else {
                    self.required
                },
            },
        };
        genai_client::Tool {
            function_declarations: Some(vec![internal_fd]),
            code_execution: None,
        }
    }
}
