use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct GenerateContentResponse {
    pub candidates: Vec<Candidate>,
}

#[derive(Deserialize, Debug)]
pub struct Candidate {
    pub content: ContentResponse,
}

#[derive(Deserialize, Debug)]
pub struct ContentResponse {
    pub parts: Vec<PartResponse>,
    #[serde(rename = "role")]
    pub role: String,
}

#[derive(Deserialize, Debug)]
pub struct PartResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "functionCall", skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCallResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_response: Option<super::shared::FunctionResponse>,
    #[serde(rename = "executableCode", skip_serializing_if = "Option::is_none")]
    pub executable_code: Option<ExecutableCodeResponse>,
    #[serde(
        rename = "codeExecutionResult",
        skip_serializing_if = "Option::is_none"
    )]
    pub code_execution_result: Option<CodeExecutionResultResponse>,
    /// Thought signature for Gemini 3 reasoning continuity (required for function calling)
    #[serde(rename = "thoughtSignature", skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct FunctionCallResponse {
    pub name: String,
    pub args: serde_json::Value,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExecutableCodeResponse {
    pub language: String,
    pub code: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CodeExecutionResultResponse {
    pub outcome: String,
    pub output: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_deserialize_generate_content_response() {
        // Example JSON mimicking a successful API response
        let response_json = r#"
        {
          "candidates": [
            {
              "content": {
                "parts": [
                  {
                    "text": "This is the generated text."
                  }
                ],
                "role": "model"
              }
            }
          ]
        }
        "#;

        let response: GenerateContentResponse =
            serde_json::from_str(response_json).expect("Deserialization failed");

        assert_eq!(response.candidates.len(), 1);
        let candidate = &response.candidates[0];
        assert_eq!(candidate.content.parts.len(), 1);
        assert_eq!(candidate.content.role, "model");
        let part = &candidate.content.parts[0];
        assert_eq!(part.text.as_deref(), Some("This is the generated text."));
    }

    #[test]
    fn test_deserialize_function_call_response() {
        let response_json = r#"
        {
          "candidates": [
            {
              "content": {
                "parts": [
                  {
                    "functionCall": {
                      "name": "get_weather",
                      "args": {
                        "location": "San Francisco, CA",
                        "unit": "celsius"
                      }
                    }
                  }
                ],
                "role": "model"
              }
            }
          ]
        }
        "#;

        let response: GenerateContentResponse =
            serde_json::from_str(response_json).expect("Deserialization failed");

        assert_eq!(response.candidates.len(), 1);
        let candidate = &response.candidates[0];
        assert_eq!(candidate.content.parts.len(), 1);
        let part = &candidate.content.parts[0];
        assert!(part.text.is_none());
        let function_call = part.function_call.as_ref().expect("Expected function call");
        assert_eq!(function_call.name, "get_weather");
        assert_eq!(
            function_call.args,
            serde_json::json!({
                "location": "San Francisco, CA",
                "unit": "celsius"
            })
        );
    }

    #[test]
    fn test_deserialize_minimal_response() {
        let response_json =
            r#"{"candidates":[{"content":{"parts":[{"text":"Minimal"}],"role":"model"}}]}"#;
        let response: GenerateContentResponse =
            serde_json::from_str(response_json).expect("Minimal deserialization failed");
        assert_eq!(
            response.candidates[0].content.parts[0].text.as_deref(),
            Some("Minimal")
        );
    }

    #[test]
    fn test_deserialize_empty_candidates() {
        let response_json = r#"{"candidates":[]}"#;
        let response: GenerateContentResponse =
            serde_json::from_str(response_json).expect("Empty candidates deserialization failed");
        assert_eq!(response.candidates.len(), 0);
    }

    #[test]
    fn test_deserialize_multiple_candidates() {
        let response_json = r#"
        {
          "candidates": [
            {
              "content": {
                "parts": [{"text": "First candidate"}],
                "role": "model"
              }
            },
            {
              "content": {
                "parts": [{"text": "Second candidate"}],
                "role": "model"
              }
            },
            {
              "content": {
                "parts": [{"text": "Third candidate"}],
                "role": "model"
              }
            }
          ]
        }
        "#;

        let response: GenerateContentResponse = serde_json::from_str(response_json)
            .expect("Multiple candidates deserialization failed");
        assert_eq!(response.candidates.len(), 3);
        assert_eq!(
            response.candidates[0].content.parts[0].text.as_deref(),
            Some("First candidate")
        );
        assert_eq!(
            response.candidates[1].content.parts[0].text.as_deref(),
            Some("Second candidate")
        );
        assert_eq!(
            response.candidates[2].content.parts[0].text.as_deref(),
            Some("Third candidate")
        );
    }

    #[test]
    fn test_deserialize_multiple_parts() {
        let response_json = r#"
        {
          "candidates": [
            {
              "content": {
                "parts": [
                  {"text": "Part 1"},
                  {"text": "Part 2"},
                  {"text": "Part 3"}
                ],
                "role": "model"
              }
            }
          ]
        }
        "#;

        let response: GenerateContentResponse =
            serde_json::from_str(response_json).expect("Multiple parts deserialization failed");
        assert_eq!(response.candidates[0].content.parts.len(), 3);
        assert_eq!(
            response.candidates[0].content.parts[0].text.as_deref(),
            Some("Part 1")
        );
        assert_eq!(
            response.candidates[0].content.parts[1].text.as_deref(),
            Some("Part 2")
        );
        assert_eq!(
            response.candidates[0].content.parts[2].text.as_deref(),
            Some("Part 3")
        );
    }

    #[test]
    fn test_deserialize_code_execution_result() {
        let response_json = r#"
        {
          "candidates": [
            {
              "content": {
                "parts": [
                  {
                    "codeExecutionResult": {
                      "outcome": "OUTCOME_OK",
                      "output": "42"
                    }
                  }
                ],
                "role": "model"
              }
            }
          ]
        }
        "#;

        let response: GenerateContentResponse = serde_json::from_str(response_json)
            .expect("Code execution result deserialization failed");
        assert_eq!(response.candidates.len(), 1);
        let part = &response.candidates[0].content.parts[0];
        assert!(part.text.is_none());
        assert!(part.function_call.is_none());
        let code_result = part
            .code_execution_result
            .as_ref()
            .expect("Expected code execution result");
        assert_eq!(code_result.outcome, "OUTCOME_OK");
        assert_eq!(code_result.output, "42");
    }

    #[test]
    fn test_deserialize_executable_code() {
        let response_json = r#"
        {
          "candidates": [
            {
              "content": {
                "parts": [
                  {
                    "executableCode": {
                      "language": "PYTHON",
                      "code": "print('Hello, World!')"
                    }
                  }
                ],
                "role": "model"
              }
            }
          ]
        }
        "#;

        let response: GenerateContentResponse =
            serde_json::from_str(response_json).expect("Executable code deserialization failed");
        assert_eq!(response.candidates.len(), 1);
        let part = &response.candidates[0].content.parts[0];
        assert!(part.text.is_none());
        let executable_code = part
            .executable_code
            .as_ref()
            .expect("Expected executable code");
        assert_eq!(executable_code.language, "PYTHON");
        assert_eq!(executable_code.code, "print('Hello, World!')");
    }

    #[test]
    fn test_deserialize_mixed_parts() {
        let response_json = r#"
        {
          "candidates": [
            {
              "content": {
                "parts": [
                  {"text": "Here's the code:"},
                  {
                    "executableCode": {
                      "language": "PYTHON",
                      "code": "x = 5 + 3"
                    }
                  },
                  {
                    "codeExecutionResult": {
                      "outcome": "OUTCOME_OK",
                      "output": "8"
                    }
                  },
                  {"text": "The result is 8"}
                ],
                "role": "model"
              }
            }
          ]
        }
        "#;

        let response: GenerateContentResponse =
            serde_json::from_str(response_json).expect("Mixed parts deserialization failed");
        assert_eq!(response.candidates[0].content.parts.len(), 4);
        assert_eq!(
            response.candidates[0].content.parts[0].text.as_deref(),
            Some("Here's the code:")
        );
        assert!(
            response.candidates[0].content.parts[1]
                .executable_code
                .is_some()
        );
        assert!(
            response.candidates[0].content.parts[2]
                .code_execution_result
                .is_some()
        );
        assert_eq!(
            response.candidates[0].content.parts[3].text.as_deref(),
            Some("The result is 8")
        );
    }

    #[test]
    fn test_deserialize_missing_optional_fields() {
        // Test with only required fields present
        let response_json = r#"
        {
          "candidates": [
            {
              "content": {
                "parts": [{}],
                "role": "model"
              }
            }
          ]
        }
        "#;

        let response: GenerateContentResponse = serde_json::from_str(response_json)
            .expect("Missing optional fields deserialization failed");
        assert_eq!(response.candidates.len(), 1);
        let part = &response.candidates[0].content.parts[0];
        assert!(part.text.is_none());
        assert!(part.function_call.is_none());
        assert!(part.function_response.is_none());
        assert!(part.executable_code.is_none());
        assert!(part.code_execution_result.is_none());
    }

    #[test]
    fn test_deserialize_malformed_missing_candidates() {
        let response_json = r#"{}"#;
        let result: Result<GenerateContentResponse, _> = serde_json::from_str(response_json);
        assert!(
            result.is_err(),
            "Should fail when candidates field is missing"
        );
    }

    #[test]
    fn test_deserialize_malformed_missing_content() {
        let response_json = r#"{"candidates":[{}]}"#;
        let result: Result<GenerateContentResponse, _> = serde_json::from_str(response_json);
        assert!(result.is_err(), "Should fail when content field is missing");
    }

    #[test]
    fn test_deserialize_malformed_missing_parts() {
        let response_json = r#"{"candidates":[{"content":{"role":"model"}}]}"#;
        let result: Result<GenerateContentResponse, _> = serde_json::from_str(response_json);
        assert!(result.is_err(), "Should fail when parts field is missing");
    }

    #[test]
    fn test_deserialize_malformed_invalid_json() {
        let response_json = r#"{invalid json"#;
        let result: Result<GenerateContentResponse, _> = serde_json::from_str(response_json);
        assert!(result.is_err(), "Should fail with invalid JSON");
    }

    #[test]
    fn test_deserialize_empty_text() {
        let response_json = r#"
        {
          "candidates": [
            {
              "content": {
                "parts": [{"text": ""}],
                "role": "model"
              }
            }
          ]
        }
        "#;

        let response: GenerateContentResponse =
            serde_json::from_str(response_json).expect("Empty text deserialization failed");
        assert_eq!(
            response.candidates[0].content.parts[0].text.as_deref(),
            Some("")
        );
    }

    #[test]
    fn test_deserialize_unicode_text() {
        let response_json = r#"
        {
          "candidates": [
            {
              "content": {
                "parts": [{"text": "Hello 世界 🌍 مرحبا"}],
                "role": "model"
              }
            }
          ]
        }
        "#;

        let response: GenerateContentResponse =
            serde_json::from_str(response_json).expect("Unicode text deserialization failed");
        assert_eq!(
            response.candidates[0].content.parts[0].text.as_deref(),
            Some("Hello 世界 🌍 مرحبا")
        );
    }
}
