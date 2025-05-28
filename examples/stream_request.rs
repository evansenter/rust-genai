use futures_util::{StreamExt, pin_mut};
use rust_genai::{Client, GenaiError};
use std::env;
use std::error::Error;
use std::io::{Write, stdout};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let api_key = env::var("GEMINI_API_KEY").map_err(|e| Box::new(e) as Box<dyn Error>)?;

    let client = Client::new(api_key);
    let model = "gemini-2.5-flash-preview-05-20";
    let prompt =
        "Write a short story about a space explorer discovering a planet made of sentient clouds.";

    println!("Sending streaming request to model: {model}");
    println!("Prompt: {prompt}\n");
    println!("--- Model Response Stream ---");

    let stream = client.with_model(model).with_prompt(prompt).stream();
    pin_mut!(stream);

    let mut error_occurred = false;
    while let Some(result) = stream.next().await {
        match result {
            Ok(response_chunk) => {
                print!("{}", response_chunk.text);
                stdout().flush().unwrap_or_default();
            }
            Err(e) => {
                match &e {
                    GenaiError::Api(api_err_msg) => {
                        eprintln!("\nAPI Error during stream: {api_err_msg}");
                    }
                    GenaiError::Http(http_err) => {
                        eprintln!("\nHTTP Error during stream: {http_err}");
                    }
                    GenaiError::Json(json_err) => {
                        eprintln!("\nJSON Error during stream: {json_err}");
                    }
                    GenaiError::Parse(p_err) => eprintln!("\nParse Error during stream: {p_err}"),
                    GenaiError::Utf8(u_err) => eprintln!("\nUTF8 Error during stream: {u_err}"),
                    GenaiError::Internal(i_err) => {
                        eprintln!("\nInternal Error during stream: {i_err}");
                    }
                }
                error_occurred = true;
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
