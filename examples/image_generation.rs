//! Example: Image Generation
//!
//! This example demonstrates how to generate images using Gemini's
//! image generation capabilities and saves them to disk.
//!
//! # Running
//!
//! ```bash
//! cargo run --example image_generation
//! ```
//!
//! # Prerequisites
//!
//! Set the `GEMINI_API_KEY` environment variable with your API key.
//!
//! # Model Support
//!
//! Image generation requires a model that supports the IMAGE response modality.
//! Currently supported: `gemini-3-pro-image-preview`
//!
//! # Regional Availability
//!
//! Image generation may not be available in all regions. If you receive
//! a "model not found" error, your API key may not have access to the
//! image generation model in your region.

use base64::Engine;
use rust_genai::{Client, GenaiError, InteractionContent, InteractionStatus};
use std::env;
use std::path::PathBuf;

/// Save a base64-encoded image to a file and return the path
fn save_image(
    base64_data: &str,
    mime_type: Option<&str>,
    prefix: &str,
    index: usize,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let extension = match mime_type {
        Some("image/png") => "png",
        Some("image/jpeg") | Some("image/jpg") => "jpg",
        Some("image/webp") => "webp",
        Some("image/gif") => "gif",
        _ => "png", // Default to png
    };

    let filename = format!("{}_{}.{}", prefix, index, extension);
    let path = std::env::temp_dir().join(filename);

    let bytes = base64::engine::general_purpose::STANDARD.decode(base64_data)?;
    std::fs::write(&path, bytes)?;

    Ok(path)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable not set");

    let client = Client::builder(api_key).build();

    println!("=== IMAGE GENERATION EXAMPLE ===\n");

    // ==========================================================================
    // Example 1: Basic Image Generation
    // ==========================================================================
    println!("--- Example 1: Birman Cat ---\n");

    let model = "gemini-3-pro-image-preview";
    let prompt = "A white and orange Birman cat sitting cozily on an electric blanket on a couch.";

    println!("Model: {}", model);
    println!("Prompt: {}\n", prompt);

    let result = client
        .interaction()
        .with_model(model)
        .with_text(prompt)
        .with_response_modalities(vec!["IMAGE".to_string()])
        .with_store(true)
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("Status: {:?}", response.status);

            if response.status == InteractionStatus::Completed {
                let mut image_count = 0;

                for output in response.outputs.iter() {
                    if let InteractionContent::Image {
                        data: Some(base64_data),
                        mime_type,
                        ..
                    } = output
                    {
                        image_count += 1;
                        match save_image(
                            base64_data,
                            mime_type.as_deref(),
                            "birman_cat",
                            image_count,
                        ) {
                            Ok(path) => {
                                println!("\n  Image {} saved!", image_count);
                                println!("  Size: {} bytes", base64_data.len() * 3 / 4);
                                println!("  Path: {}", path.display());
                                println!("  Open: file://{}", path.display());
                            }
                            Err(e) => {
                                eprintln!("\n  Failed to save image {}: {}", image_count, e);
                            }
                        }
                    }
                }

                if image_count == 0 {
                    println!("\nNo image content in response.");
                }
            }

            if let Some(usage) = &response.usage {
                println!(
                    "\n  Tokens: {} in / {} out",
                    usage.total_input_tokens.unwrap_or(0),
                    usage.total_output_tokens.unwrap_or(0)
                );
            }
        }
        Err(e) => {
            handle_image_generation_error(&e);
        }
    }

    // ==========================================================================
    // Example 2: Watercolor Birman with Motorcycle
    // ==========================================================================
    println!("\n--- Example 2: Watercolor Birman with Motorcycle ---\n");

    let prompt = "A watercolor painting of a white and orange Birman cat standing by a Triumph cafe racer motorcycle on a scenic road.";

    println!("Prompt: {}\n", prompt);

    let result = client
        .interaction()
        .with_model(model)
        .with_text(prompt)
        .with_response_modalities(vec!["IMAGE".to_string()])
        .with_store(true)
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("Status: {:?}", response.status);

            if response.status == InteractionStatus::Completed {
                let mut image_count = 0;

                for output in response.outputs.iter() {
                    if let InteractionContent::Image {
                        data: Some(base64_data),
                        mime_type,
                        ..
                    } = output
                    {
                        image_count += 1;
                        match save_image(
                            base64_data,
                            mime_type.as_deref(),
                            "watercolor_birman_motorcycle",
                            image_count,
                        ) {
                            Ok(path) => {
                                println!("\n  Image {} saved!", image_count);
                                println!("  Size: {} bytes", base64_data.len() * 3 / 4);
                                println!("  Path: {}", path.display());
                                println!("  Open: file://{}", path.display());
                            }
                            Err(e) => {
                                eprintln!("\n  Failed to save image {}: {}", image_count, e);
                            }
                        }
                    }
                }

                if image_count == 0 {
                    println!("\nNo image content in response.");
                }
            }

            if let Some(usage) = &response.usage {
                println!(
                    "\n  Tokens: {} in / {} out",
                    usage.total_input_tokens.unwrap_or(0),
                    usage.total_output_tokens.unwrap_or(0)
                );
            }
        }
        Err(e) => {
            handle_image_generation_error(&e);
        }
    }

    println!("\n=== END EXAMPLE ===");

    Ok(())
}

/// Handle common image generation errors with helpful messages
fn handle_image_generation_error(e: &GenaiError) {
    match e {
        GenaiError::Api {
            status_code,
            message,
            ..
        } => {
            eprintln!("API Error (HTTP {}): {}", status_code, message);

            if message.contains("not found") || message.contains("not supported") {
                eprintln!(
                    "\nNote: Image generation requires the gemini-3-pro-image-preview model."
                );
                eprintln!("This model may not be available in all regions or API configurations.");
            }
        }
        GenaiError::Http(http_err) => {
            eprintln!("HTTP Error: {}", http_err);
        }
        GenaiError::Json(json_err) => {
            eprintln!("JSON Error: {}", json_err);
        }
        _ => {
            eprintln!("Error: {}", e);
        }
    }
}
