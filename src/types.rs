use serde_json::Value;

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
    /// Opaque thought signatures from Gemini 3 models.
    ///
    /// These encrypted tokens represent the model's internal reasoning state and must be
    /// passed back unchanged in multi-turn function calling conversations. Without them,
    /// Gemini 3 returns 400 errors when receiving function call responses.
    ///
    /// Extract from responses using this field and pass to
    /// `model_function_calls_request_with_signatures()` when building conversation history.
    ///
    /// # Example
    /// ```rust,ignore
    /// let response = client.with_model("gemini-3-flash-preview")
    ///     .with_prompt("What's the weather?")
    ///     .with_function(weather_fn)
    ///     .generate()
    ///     .await?;
    ///
    /// // Extract signatures
    /// let signatures = response.thought_signatures.clone();
    ///
    /// // Pass them back with function responses
    /// let contents = vec![
    ///     user_text(prompt),
    ///     model_function_calls_request_with_signatures(calls, signatures),
    ///     user_tool_response("weather", result),
    /// ];
    /// ```
    pub thought_signatures: Option<Vec<String>>,
}

// NOTE: FunctionDeclaration has been moved to genai_client and is re-exported from the root crate.
// This provides a unified type system without duplication.
