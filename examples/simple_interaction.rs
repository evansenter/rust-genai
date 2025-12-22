use rust_genai::{Client, GenaiError};
use std::env;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Get API Key from environment variable
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not found in environment");

    // Create the client
    let client = Client::builder(api_key).build();

    // 2. Create an interaction using the builder pattern
    let model_name = "gemini-3-flash-preview";
    let prompt = "Explain the concept of recursion in programming in one paragraph.";

    println!("Creating interaction with model: {model_name}");
    println!("Prompt: {prompt}\n");

    // 3. Send the interaction request using the fluent builder API
    match client
        .interaction()
        .with_model(model_name)
        .with_text(prompt)
        .with_store(true) // Store for potential follow-up
        .create()
        .await
    {
        Ok(response) => {
            println!("--- Interaction Response ---");
            println!("Interaction ID: {}", response.id);
            println!("Status: {:?}", response.status);

            if !response.outputs.is_empty() {
                println!("\nModel Output:");
                for output in &response.outputs {
                    match output {
                        rust_genai::InteractionContent::Text { text: Some(t) } => {
                            println!("{t}");
                        }
                        rust_genai::InteractionContent::Thought { text: Some(t) } => {
                            println!("[Thought] {t}");
                        }
                        _ => {}
                    }
                }
            }

            if let Some(usage) = response.usage {
                println!("\nToken Usage:");
                if let Some(total) = usage.total_tokens {
                    println!("  Total tokens: {total}");
                }
            }
            println!("--- End Response ---");
        }
        Err(e) => {
            match &e {
                GenaiError::Api(api_err_msg) => eprintln!("API Error: {api_err_msg}"),
                GenaiError::Http(http_err) => eprintln!("HTTP Error: {http_err}"),
                GenaiError::Json(json_err) => eprintln!("JSON Error: {json_err}"),
                GenaiError::Parse(p_err) => eprintln!("Parse Error: {p_err}"),
                GenaiError::Utf8(u_err) => eprintln!("UTF8 Error: {u_err}"),
                GenaiError::Internal(i_err) => eprintln!("Internal Error: {i_err}"),
                GenaiError::InvalidInput(input_err) => eprintln!("Invalid Input: {input_err}"),
            }
            return Err(e.into());
        }
    }

    Ok(())
}
