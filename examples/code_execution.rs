//! Example: Code Execution with Gemini
//!
//! This example demonstrates how to use Gemini's built-in code execution
//! capability to run Python code in a sandboxed environment.
//!
//! Run with: cargo run --example code_execution

use rust_genai::{Client, CodeExecutionOutcome, GenaiError};
use std::env;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Get API Key from environment variable
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not found in environment");

    // Create the client
    let client = Client::builder(api_key).build();

    // 2. Create an interaction with code execution enabled
    let model_name = "gemini-3-flash-preview";
    let prompt = "Calculate the first 20 prime numbers using Python. \
                  Show both the code and explain the results.";

    println!("Creating interaction with model: {model_name}");
    println!("Prompt: {prompt}\n");

    // 3. Send the interaction request with code execution enabled
    match client
        .interaction()
        .with_model(model_name)
        .with_text(prompt)
        .with_code_execution() // Enable Python code execution
        .with_store(true)
        .create()
        .await
    {
        Ok(response) => {
            println!("--- Code Execution Response ---");
            println!("Interaction ID: {}", response.id);
            println!("Status: {:?}\n", response.status);

            // 4. Display the model's text response
            if let Some(text) = response.text() {
                println!("Model Explanation:\n{text}\n");
            }

            // 5. Display the executed code
            println!("--- Executed Code ---");
            for (language, code) in response.executable_code() {
                println!("Language: {language}");
                println!("```python\n{code}\n```\n");
            }

            // 6. Display the code execution results with typed outcomes
            println!("--- Execution Results ---");
            for (outcome, output) in response.code_execution_results() {
                match outcome {
                    CodeExecutionOutcome::Ok => {
                        println!("Status: SUCCESS");
                        println!("Output:\n{output}");
                    }
                    CodeExecutionOutcome::Failed => {
                        println!("Status: FAILED");
                        println!("Error:\n{output}");
                    }
                    CodeExecutionOutcome::DeadlineExceeded => {
                        println!("Status: TIMEOUT");
                        println!("The code execution exceeded the 30-second limit.");
                    }
                    _ => {
                        println!("Status: UNKNOWN");
                        println!("Output:\n{output}");
                    }
                }
            }

            // 7. Use convenience helper to get the first successful output
            if let Some(output) = response.successful_code_output() {
                println!("\n--- Quick Result ---");
                println!("First successful output: {output}");
            }

            // 8. Show content summary
            let summary = response.content_summary();
            println!("\n--- Content Summary ---");
            println!("  Text blocks: {}", summary.text_count);
            println!(
                "  Code execution calls: {}",
                summary.code_execution_call_count
            );
            println!(
                "  Code execution results: {}",
                summary.code_execution_result_count
            );

            if let Some(usage) = response.usage {
                println!("\n--- Token Usage ---");
                if let Some(input) = usage.total_input_tokens {
                    println!("  Input tokens: {input}");
                }
                if let Some(output) = usage.total_output_tokens {
                    println!("  Output tokens: {output}");
                }
            }
        }
        Err(e) => {
            match &e {
                GenaiError::Api(api_err_msg) => {
                    eprintln!("API Error: {api_err_msg}");
                    if api_err_msg.contains("not supported") {
                        eprintln!("Note: Code execution may not be available in all regions.");
                    }
                }
                GenaiError::Http(http_err) => eprintln!("HTTP Error: {http_err}"),
                GenaiError::Json(json_err) => eprintln!("JSON Error: {json_err}"),
                _ => eprintln!("Error: {e}"),
            }
            return Err(e.into());
        }
    }

    println!("\n--- End Response ---");
    Ok(())
}
