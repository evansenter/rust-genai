use reqwest::Client;
use std::error::Error;

// Declare the models module
mod models;

// Import the necessary structs from the models module
use models::request::{Content, GenerateContentRequest, Part};
use models::response::GenerateContentResponse;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

pub async fn generate_content(
    api_key: &str,
    model_name: &str,
    prompt_text: &str,
) -> Result<String, Box<dyn Error>> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model_name,
        api_key
    );

    let request_body = GenerateContentRequest {
        contents: vec![Content {
            parts: vec![Part { text: prompt_text.to_string() }],
        }],
    };

    let client = Client::new();
    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("API Error: {}", error_text).into());
    }

    let response_body = response.json::<GenerateContentResponse>().await?;

    if let Some(candidate) = response_body.candidates.get(0) {
        if let Some(part) = candidate.content.parts.get(0) {
            return Ok(part.text.clone());
        }
    }

    Err("No text content found in the response".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_generate_content_integration() {
        let api_key = match env::var("GEMINI_API_KEY") {
            Ok(key) => key,
            Err(_) => {
                println!("Skipping test_generate_content_integration: GEMINI_API_KEY not set.");
                return;
            }
        };

        let model = "gemini-1.5-flash-latest";
        let prompt = "What is the capital of France?";

        let result = generate_content(&api_key, model, prompt).await;

        assert!(result.is_ok(), "generate_content failed: {:?}", result.err());

        let text = result.unwrap();
        assert!(!text.is_empty(), "Generated text is empty");

        println!("test_generate_content_integration response: {}", text);

        assert!(text.to_lowercase().contains("paris"), "Response does not contain expected keyword 'paris'");
    }
}
