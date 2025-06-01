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
