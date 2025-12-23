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
            thought_signature: None,
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
            thought_signature: None,
        }],
        role: Some("model".to_string()),
    }
}

/// Creates a model content block representing a function call.
///
/// # Arguments
///
/// * `name` - The name of the function
/// * `args` - The function arguments
/// * `thought_signature` - Optional thought signature from Gemini 3 (required for multi-turn function calling)
#[must_use]
pub fn model_function_call_with_signature(
    name: String,
    args: Value,
    thought_signature: Option<String>,
) -> Content {
    Content {
        parts: vec![Part {
            text: None,
            function_call: Some(FunctionCall { name, args }),
            function_response: None,
            thought_signature,
        }],
        role: Some("model".to_string()),
    }
}

/// Creates a model content block representing a function call (without thought signature).
///
/// For Gemini 3 function calling, prefer using `model_function_call_with_signature` instead.
#[must_use]
pub fn model_function_call(name: String, args: Value) -> Content {
    model_function_call_with_signature(name, args, None)
}

/// Creates a model content block representing a list of function call requests from the model.
///
/// This is typically used to record the model's request in the conversation history.
/// For Gemini 3, thought signatures are required - extract them from the response's
/// `thought_signatures` field and pass them here.
///
/// # Arguments
///
/// * `calls` - Function calls from the model
/// * `thought_signatures` - Optional thought signatures from Gemini 3 (one per function call)
#[must_use]
pub fn model_function_calls_request_with_signatures(
    calls: Vec<FunctionCall>,
    thought_signatures: Option<Vec<String>>,
) -> Content {
    let signatures = thought_signatures.unwrap_or_default();

    // Log signature count mismatches to help users debug issues
    if !signatures.is_empty() && signatures.len() != calls.len() {
        log::debug!(
            "Thought signature count ({}) doesn't match function call count ({}). \
             Extra calls will have no signature.",
            signatures.len(),
            calls.len()
        );
    }

    Content {
        parts: calls
            .into_iter()
            .enumerate()
            .map(|(i, fc)| Part {
                text: None,
                function_call: Some(fc),
                function_response: None,
                thought_signature: signatures.get(i).cloned(),
            })
            .collect(),
        role: Some("model".to_string()),
    }
}

/// Creates a model content block representing a list of function call requests from the model (without thought signatures).
///
/// For Gemini 3 function calling, prefer using `model_function_calls_request_with_signatures` instead.
#[must_use]
pub fn model_function_calls_request(calls: Vec<FunctionCall>) -> Content {
    model_function_calls_request_with_signatures(calls, None)
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
            thought_signature: None,
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
