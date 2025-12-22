use rust_genai::{Client, GenaiError};
use std::env;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Get API Key from environment variable
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not found in environment");

    // Create the client
    let client = Client::builder(api_key).build();

    // 2. Define model and prompt
    let model_name = "gemini-3-flash-preview"; // Specify the model directly
    let prompt = "Write a short poem about a rusty robot.";

    println!("Sending request to model: {model_name}");
    println!("Prompt: {prompt}\n");

    // 3. Call the method on the client using the builder pattern
    match client
        .with_model(model_name) // Use with_model
        .with_prompt(prompt)
        .generate()
        .await
    {
        Ok(response) => {
            println!("--- Model Response ---");
            println!("{}", response.text.unwrap_or_default());
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
