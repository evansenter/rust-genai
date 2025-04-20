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
    #[serde(rename = "role")] // Map JSON field "role" to Rust field "_role"
    pub _role: String,
}

#[derive(Deserialize, Debug)]
pub struct PartResponse {
    pub text: String,
    // Add other part types later
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
              // "finishReason": "STOP",
              // "safetyRatings": []
            }
          ]
          // "promptFeedback": { ... }
        }
        "#;

        let response: GenerateContentResponse =
            serde_json::from_str(response_json).expect("Deserialization failed");

        assert_eq!(response.candidates.len(), 1);
        let candidate = &response.candidates[0];
        assert_eq!(candidate.content.parts.len(), 1);
        assert_eq!(candidate.content._role, "model"); // Check the renamed field
        let part = &candidate.content.parts[0];
        assert_eq!(part.text, "This is the generated text.");
    }

    #[test]
    fn test_deserialize_minimal_response() {
        // Test with absolute minimum valid fields
        let response_json =
            r#"{"candidates":[{"content":{"parts":[{"text":"Minimal"}],"role":"model"}}]}"#;
        let response: GenerateContentResponse =
            serde_json::from_str(response_json).expect("Minimal deserialization failed");
        assert_eq!(response.candidates[0].content.parts[0].text, "Minimal");
    }

    // Add more tests here for variations: multiple candidates/parts, presence of optional fields, etc.
}
