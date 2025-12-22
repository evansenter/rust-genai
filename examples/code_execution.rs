use rust_genai::{GenaiError, client::Client, types::GenerateContentResponse};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");
    let client = Client::builder(api_key).debug().build();
    let model_name = "gemini-3-flash-preview";
    let prompt_text = "What is 7 + 5? Use code execution if you want.";

    println!("Sending request to model: {model_name}");
    println!("Prompt: {prompt_text}\n");

    // Send request with code execution enabled
    let response: Result<GenerateContentResponse, GenaiError> = client
        .with_model(model_name)
        .with_prompt(prompt_text)
        .with_code_execution() // Enable code execution tool
        .generate()
        .await;

    match response {
        Ok(res) => {
            println!("--- Model Response ---");
            if let Some(ref text) = res.text {
                println!("Text response: {text}");
            }

            if let Some(ref results) = res.code_execution_results {
                println!("\nCode Execution Results received:");
                for (i, exec_result) in results.iter().enumerate() {
                    println!("  Result {}:", i + 1);
                    println!("    Code:   {}", exec_result.code);
                    println!("    Output: {}", exec_result.output);
                }
            }

            if let Some(ref fcs) = res.function_calls {
                if !fcs.is_empty() {
                    println!("\nUnexpected function calls received (showing first):");
                    println!("  Name: {}", fcs[0].name);
                    println!("  Args: {}", fcs[0].args);
                }
            }

            if res.text.is_none()
                && res.code_execution_results.is_none()
                && res.function_calls.as_ref().is_none_or(Vec::is_empty)
            {
                println!(
                    "Model did not return text, code execution results, or any function calls."
                );
            }
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
            // It's often useful to return the original error for further handling if needed
            // For an example, just printing and exiting might be okay, but for a library, you'd return it.
            // return Err(e.into()); // Uncomment if you want to propagate the error
        }
    }

    println!("--- End of Interaction ---");

    Ok(())
}
