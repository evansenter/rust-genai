use genai_client::models::request::GenerateContentRequest as InternalGenerateContentRequest;
use genai_client::{Content, FunctionCall, FunctionResponse, Part, Tool};
use serde_json::Value;

/// Creates a user content block from a text string.
#[must_use]
pub fn user_text(text: String) -> Content {
    Content {
        parts: vec![Part {
            text: Some(text),
            function_call: None,
            function_response: None,
        }],
        role: Some("user".to_string()),
    }
}

/// Creates a model content block from a text string.
#[must_use]
pub fn model_text(text: String) -> Content {
    Content {
        parts: vec![Part {
            text: Some(text),
            function_call: None,
            function_response: None,
        }],
        role: Some("model".to_string()),
    }
}

/// Creates a model content block representing a function call.
#[must_use]
pub fn model_function_call(name: String, args: Value) -> Content {
    Content {
        parts: vec![Part {
            text: None,
            function_call: Some(FunctionCall { name, args }),
            function_response: None,
        }],
        role: Some("model".to_string()),
    }
}

/// Creates a model content block representing a list of function call requests from the model.
/// This is typically used to record the model's request in the conversation history.
#[must_use]
pub fn model_function_calls_request(calls: Vec<FunctionCall>) -> Content {
    Content {
        parts: calls
            .into_iter()
            .map(|fc| Part {
                text: None,
                function_call: Some(fc),
                function_response: None,
            })
            .collect(),
        role: Some("model".to_string()),
    }
}

/// Creates a user content block representing the response from a function/tool execution.
#[must_use]
pub fn user_tool_response(name: String, response_data: Value) -> Content {
    let api_compliant_response_data = if response_data.is_object() {
        response_data
    } else {
        serde_json::json!({ "result": response_data })
    };
    Content {
        parts: vec![Part {
            text: None,
            function_call: None,
            function_response: Some(FunctionResponse {
                name,
                response: api_compliant_response_data,
            }),
        }],
        role: Some("user".to_string()),
    }
}

/// Builds a `GenerateContentRequest` with the given contents and optional tools.
/// System instruction and tool configuration are defaulted to `None`.
#[must_use]
pub const fn build_content_request(
    contents: Vec<Content>,
    tools: Option<Vec<Tool>>,
) -> InternalGenerateContentRequest {
    InternalGenerateContentRequest {
        contents,
        tools,
        system_instruction: None,
        tool_config: None,
    }
}
