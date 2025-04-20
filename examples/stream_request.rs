use rust_genai::{generate_content_stream};
use std::env;
use futures_util::{StreamExt, pin_mut}; // Import pin_mut
use std::io::{stdout, Write}; // For flushing output

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> { // Keep Box<dyn Error> for main
    // 1. Get API Key from environment variable
    let api_key = env::var("GEMINI_API_KEY")
        .map_err(|_| "Error: GEMINI_API_KEY environment variable not set.")?;

    // 2. Define model and prompt
    let model = "gemini-1.5-flash-latest"; // Or your preferred model
    let prompt = "Write a short story about a space explorer discovering a planet made of sentient clouds.";

    println!("Sending streaming request to model: {}", model);
    println!("Prompt: {}\n", prompt);
    println!("--- Model Response Stream ---");

    // 3. Call the generate_content_stream function
    let stream = generate_content_stream(&api_key, model, prompt);
    // Pin the stream to the stack before iterating
    pin_mut!(stream);

    // 4. Process the stream
    let mut full_response_text = String::new();
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
                eprintln!("\nError receiving stream chunk: {}", e);
                // Decide whether to break or continue on error
                // For simplicity, we'll print the error and continue collecting text
                // In a real app, might want to return Err(Box::new(e))?
            }
        }
    }

    println!("\n--- End Response Stream ---");

    // Optional: Print the fully collected text
    // println!("\n--- Full Response Text ---\n{}", full_response_text);
    // println!("--- End Full Response ---");

    Ok(())
} 