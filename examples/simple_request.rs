use rust_genai::{Client, GenaiError};
use std::env;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Get API Key from environment variable
    let api_key = env::var("GEMINI_API_KEY").map_err(|e| Box::new(e) as Box<dyn Error>)?;

    // Create the client
    let client = Client::new(api_key);

    // 2. Define model and prompt
    let model = "gemini-2.5-flash-preview-05-20"; // Or your preferred model
    let prompt = "Write a short poem about a rusty robot.";

    println!("Sending request to model: {}", model);
    println!("Prompt: {}\n", prompt);

    // 3. Call the method on the client using the builder pattern
    match client
        .with_model(model)
        .with_prompt(prompt)
        .generate()
        .await
    {
        Ok(response) => {
            println!("--- Model Response ---");
            println!("{}", response.text);
            println!("--- End Response ---");
        }
        Err(e) => {
            match &e {
                GenaiError::Api(api_err_msg) => eprintln!("API Error: {}", api_err_msg),
                GenaiError::Http(http_err) => eprintln!("HTTP Error: {}", http_err),
                GenaiError::Json(json_err) => eprintln!("JSON Error: {}", json_err),
                GenaiError::Parse(p_err) => eprintln!("Parse Error: {}", p_err),
                GenaiError::Utf8(u_err) => eprintln!("UTF8 Error: {}", u_err),
                GenaiError::Internal(i_err) => eprintln!("Internal Error: {}", i_err),
            }
            return Err(e.into());
        }
    }

    Ok(())
}
