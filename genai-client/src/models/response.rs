use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct GenerateContentResponse {
    pub candidates: Vec<Candidate>,
    // pub prompt_feedback: Option<PromptFeedback>,
}

#[derive(Deserialize, Debug)]
pub struct Candidate {
    pub content: ContentResponse,
    // pub finish_reason: Option<String>,
    // pub safety_ratings: Option<Vec<SafetyRating>>,
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
}
