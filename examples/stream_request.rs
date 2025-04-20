use futures_util::{StreamExt, pin_mut}; // Import pin_mut
use rust_genai::{Client, GenaiError}; // Removed GenerateContentResponse import
use std::env;
use std::error::Error;
use std::io::{Write, stdout}; // For flushing output // Import Error trait

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Keep Box<dyn Error>
    // 1. Get API Key from environment variable
    let api_key = env::var("GEMINI_API_KEY").map_err(|e| Box::new(e) as Box<dyn Error>)?;

    // Create the client
    let client = Client::new(api_key);

    // 2. Define model and prompt
    let model = "gemini-1.5-flash-latest"; // Or your preferred model
    let prompt =
        "Write a short story about a space explorer discovering a planet made of sentient clouds.";

    println!("Sending streaming request to model: {}", model);
    println!("Prompt: {}\n", prompt);
    println!("--- Model Response Stream ---");

    // 3. Call the stream method on the client
    let stream = client.generate_content_stream(model, prompt);
    // Pin the stream to the stack before iterating
    pin_mut!(stream);

    // 4. Process the stream
    let mut full_response_text = String::new();
    let mut error_occurred = false;
    while let Some(result) = stream.next().await {
        match result {
            Ok(response_chunk) => {
                // Extract text from the first part of the first candidate
                if let Some(candidate) = response_chunk.candidates.get(0) {
                    if let Some(part) = candidate.content.parts.get(0) {
                        print!("{}", part.text);
                        // Flush stdout to ensure text appears immediately
                        stdout().flush().unwrap_or_default();
                        full_response_text.push_str(&part.text);
                    }
                }
            }
            Err(e) => {
                // Log the specific stream error
                match &e {
                    GenaiError::Api(api_err_msg) => {
                        eprintln!("\nAPI Error during stream: {}", api_err_msg)
                    }
                    GenaiError::Http(http_err) => {
                        eprintln!("\nHTTP Error during stream: {}", http_err)
                    }
                    GenaiError::Json(json_err) => {
                        eprintln!("\nJSON Error during stream: {}", json_err)
                    }
                    GenaiError::Parse(p_err) => eprintln!("\nParse Error during stream: {}", p_err),
                    GenaiError::Utf8(u_err) => eprintln!("\nUTF8 Error during stream: {}", u_err),
                }
                error_occurred = true;
                // Stop processing the stream on error
                break;
            }
        }
    }

    println!("\n--- End Response Stream ---");

    if error_occurred {
        Err("Stream processing failed due to an error.".into())
    } else {
        Ok(())
    }
}
