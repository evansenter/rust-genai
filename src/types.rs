use serde_json::Value;

// Note: FunctionCall needs to be defined or visible for GenerateContentResponse
// The order here should be fine as they are in the same module.

/// Represents a function call in the response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionCall {
    /// The name of the function to call.
    pub name: String,
    /// The arguments to pass to the function.
    pub args: Value,
}

/// Represents a successful response from a generate content request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateContentResponse {
    /// The generated text content, if any.
    pub text: Option<String>,
    /// The function call, if any.
    pub function_call: Option<FunctionCall>,
}

/// Represents a function declaration that can be used by the model.
#[derive(Debug, Clone)]
pub struct FunctionDeclaration {
    /// The name of the function.
    pub name: String,
    /// A description of what the function does.
    pub description: String,
    /// The JSON Schema for the function's parameters.
    pub parameters: Value,
    /// The names of required parameters.
    pub required: Vec<String>,
}
