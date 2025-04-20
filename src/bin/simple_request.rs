use rust_genai::generate_content;
use std::env;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Get API Key from environment variable
    let api_key = env::var("GEMINI_API_KEY")
        .map_err(|_| "Error: GEMINI_API_KEY environment variable not set.")?;

    // 2. Define model and prompt
    let model = "gemini-1.5-flash-latest"; // Or your preferred model
    let prompt = "Write a short poem about a rusty robot.";

    println!("Sending request to model: {}", model);
    println!("Prompt: {}\n", prompt);

    // 3. Call the generate_content function
    match generate_content(&api_key, model, prompt).await {
        Ok(response_text) => {
            println!("--- Model Response ---");
            println!("{}", response_text);
            println!("--- End Response ---");
        }
        Err(e) => {
            eprintln!("Error generating content: {}", e);
            // Return the error to indicate failure
            return Err(e);
        }
    }

    Ok(())
} 