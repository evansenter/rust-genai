use crate::types::{CodeExecutionResult, FunctionCall};
use genai_client::models::response::PartResponse;

/// Helper struct to hold processed parts from a response candidate.
#[derive(Debug, Default)]
pub struct ProcessedParts {
    pub(crate) text: Option<String>,
    pub(crate) function_calls: Vec<FunctionCall>,
    pub(crate) code_execution_results: Vec<CodeExecutionResult>,
}

/// Processes a slice of `PartResponse` objects and extracts structured data.
pub fn process_response_parts(parts: &[PartResponse]) -> ProcessedParts {
    let mut collected_text: Option<String> = None;
    let mut collected_function_calls: Vec<FunctionCall> = Vec::new();
    let mut collected_code_execution_results: Vec<CodeExecutionResult> = Vec::new();
    let mut last_executable_code: Option<String> = None;

    for part in parts {
        if let Some(text_part) = &part.text {
            collected_text = collected_text.map_or_else(
                || Some(text_part.clone()),
                |mut existing_text| {
                    existing_text.push_str(text_part);
                    Some(existing_text)
                },
            );
        }

        if let Some(fc_part) = &part.function_call {
            collected_function_calls.push(FunctionCall {
                name: fc_part.name.clone(),
                args: fc_part.args.clone(),
            });
        }

        if let Some(ec_part) = &part.executable_code {
            last_executable_code = Some(ec_part.code.clone());
        }

        if let Some(cer_part) = &part.code_execution_result {
            if let Some(code) = last_executable_code.take() {
                collected_code_execution_results.push(CodeExecutionResult {
                    code,
                    output: cer_part.output.clone(),
                });
            } else {
                log::warn!(
                    "Found codeExecutionResult without preceding executableCode: {:?}",
                    cer_part.output
                );
            }
        }
    }

    ProcessedParts {
        text: collected_text,
        function_calls: collected_function_calls,
        code_execution_results: collected_code_execution_results,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genai_client::models::response::{
        ExecutableCodeResponse, FunctionCallResponse, CodeExecutionResultResponse
    };
    use serde_json::json;

    #[test]
    fn test_process_empty_parts() {
        let parts = vec![];
        let result = process_response_parts(&parts);
        
        assert!(result.text.is_none());
        assert!(result.function_calls.is_empty());
        assert!(result.code_execution_results.is_empty());
    }

    #[test]
    fn test_process_text_parts() {
        let parts = vec![
            PartResponse {
                text: Some("Hello ".to_string()),
                function_call: None,
                executable_code: None,
                code_execution_result: None,
                function_response: None,
            },
            PartResponse {
                text: Some("world!".to_string()),
                function_call: None,
                executable_code: None,
                code_execution_result: None,
                function_response: None,
            },
        ];
        
        let result = process_response_parts(&parts);
        assert_eq!(result.text, Some("Hello world!".to_string()));
    }

    #[test]
    fn test_process_function_calls() {
        let parts = vec![
            PartResponse {
                text: None,
                function_call: Some(FunctionCallResponse {
                    name: "func1".to_string(),
                    args: json!({"arg": "value1"}),
                }),
                executable_code: None,
                code_execution_result: None,
                function_response: None,
            },
            PartResponse {
                text: None,
                function_call: Some(FunctionCallResponse {
                    name: "func2".to_string(),
                    args: json!({"arg": "value2"}),
                }),
                executable_code: None,
                code_execution_result: None,
                function_response: None,
            },
        ];
        
        let result = process_response_parts(&parts);
        assert_eq!(result.function_calls.len(), 2);
        assert_eq!(result.function_calls[0].name, "func1");
        assert_eq!(result.function_calls[1].name, "func2");
    }

    #[test]
    fn test_process_code_execution() {
        let parts = vec![
            PartResponse {
                text: None,
                function_call: None,
                executable_code: Some(ExecutableCodeResponse {
                    language: "python".to_string(),
                    code: "print('hello')".to_string(),
                }),
                code_execution_result: None,
                function_response: None,
            },
            PartResponse {
                text: None,
                function_call: None,
                executable_code: None,
                code_execution_result: Some(CodeExecutionResultResponse {
                    outcome: "SUCCESS".to_string(),
                    output: "hello".to_string(),
                }),
                function_response: None,
            },
        ];
        
        let result = process_response_parts(&parts);
        assert_eq!(result.code_execution_results.len(), 1);
        assert_eq!(result.code_execution_results[0].code, "print('hello')");
        assert_eq!(result.code_execution_results[0].output, "hello");
    }

    #[test]
    fn test_code_execution_result_without_code() {
        // This should trigger the warning log
        let parts = vec![
            PartResponse {
                text: None,
                function_call: None,
                executable_code: None,
                code_execution_result: Some(CodeExecutionResultResponse {
                    outcome: "SUCCESS".to_string(),
                    output: "orphaned output".to_string(),
                }),
                function_response: None,
            },
        ];
        
        let result = process_response_parts(&parts);
        assert!(result.code_execution_results.is_empty());
    }

    #[test]
    fn test_mixed_response_parts() {
        let parts = vec![
            PartResponse {
                text: Some("Here's the result: ".to_string()),
                function_call: None,
                executable_code: None,
                code_execution_result: None,
                function_response: None,
            },
            PartResponse {
                text: None,
                function_call: Some(FunctionCallResponse {
                    name: "calculate".to_string(),
                    args: json!({"x": 5, "y": 3}),
                }),
                executable_code: None,
                code_execution_result: None,
                function_response: None,
            },
            PartResponse {
                text: None,
                function_call: None,
                executable_code: Some(ExecutableCodeResponse {
                    language: "python".to_string(),
                    code: "5 + 3".to_string(),
                }),
                code_execution_result: None,
                function_response: None,
            },
            PartResponse {
                text: None,
                function_call: None,
                executable_code: None,
                code_execution_result: Some(CodeExecutionResultResponse {
                    outcome: "SUCCESS".to_string(),
                    output: "8".to_string(),
                }),
                function_response: None,
            },
        ];
        
        let result = process_response_parts(&parts);
        assert_eq!(result.text, Some("Here's the result: ".to_string()));
        assert_eq!(result.function_calls.len(), 1);
        assert_eq!(result.code_execution_results.len(), 1);
    }
}
