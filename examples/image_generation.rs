//! Example: Image Generation
//!
//! This example demonstrates how to generate images using Gemini's
//! image generation capabilities.
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

use rust_genai::{Client, GenaiError, InteractionContent, InteractionStatus};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable not set");

    let client = Client::builder(api_key).build();

    println!("=== IMAGE GENERATION EXAMPLE ===\n");

    // ==========================================================================
    // Example 1: Basic Image Generation
    // ==========================================================================
    println!("--- Example 1: Basic Image Generation ---\n");

    // Use the image generation preview model
    let model = "gemini-3-pro-image-preview";
    let prompt = "Generate a simple image of a red circle on a white background.";

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
                // Look for image content in outputs
                let mut image_count = 0;

                for (i, output) in response.outputs.iter().enumerate() {
                    if let InteractionContent::Image {
                        data,
                        mime_type,
                        uri,
                    } = output
                    {
                        image_count += 1;
                        println!("\nImage {} found:", i + 1);

                        if let Some(mime) = mime_type {
                            // Common MIME types: image/png, image/jpeg, image/webp
                            println!("  MIME type: {}", mime);
                        }

                        if let Some(uri) = uri {
                            println!("  URI: {}", uri);
                        }

                        // Show base64 image data info
                        if let Some(base64_data) = data {
                            println!("  Base64 data length: {} chars", base64_data.len());
                            // Approximate decoded size (base64 is ~4/3 of original)
                            let approx_bytes = base64_data.len() * 3 / 4;
                            println!("  Approximate image size: {} bytes", approx_bytes);
                            println!(
                                "  First 50 chars: {}...",
                                &base64_data[..50.min(base64_data.len())]
                            );

                            // To save the image, add the `base64` crate and use:
                            // use base64::Engine;
                            // let bytes = base64::engine::general_purpose::STANDARD.decode(base64_data)?;
                            // std::fs::write("image.png", bytes)?;
                        }
                    }
                }

                if image_count == 0 {
                    println!("\nNo image content in response.");
                    println!("Outputs:");
                    for output in &response.outputs {
                        println!("  {:?}", output);
                    }
                } else {
                    println!("\nGenerated {} image(s) successfully!", image_count);
                }
            }

            // Show token usage
            if let Some(usage) = &response.usage {
                println!("\n--- Token Usage ---");
                if let Some(input) = usage.total_input_tokens {
                    println!("  Input tokens: {}", input);
                }
                if let Some(output) = usage.total_output_tokens {
                    println!("  Output tokens: {}", output);
                }
            }
        }
        Err(e) => {
            handle_image_generation_error(&e);
        }
    }

    // ==========================================================================
    // Example 2: More Detailed Image Prompt
    // ==========================================================================
    println!("\n--- Example 2: Detailed Image Generation ---\n");

    let detailed_prompt = "Generate a watercolor painting of a sunset over calm ocean waves. \
                           The sky should have warm orange and pink tones, reflecting on the water.";

    println!("Prompt: {}\n", detailed_prompt);

    let result = client
        .interaction()
        .with_model(model)
        .with_text(detailed_prompt)
        .with_response_modalities(vec!["IMAGE".to_string()])
        .with_store(true)
        .create()
        .await;

    match result {
        Ok(response) => {
            println!("Status: {:?}", response.status);

            // Count images with actual data (not just metadata)
            let image_count = response
                .outputs
                .iter()
                .filter(|o| matches!(o, InteractionContent::Image { data: Some(_), .. }))
                .count();

            if image_count > 0 {
                println!("Generated {} image(s)!", image_count);
            } else {
                println!("No images with data in response.");
            }
        }
        Err(e) => {
            handle_image_generation_error(&e);
        }
    }

    // ==========================================================================
    // Usage Notes
    // ==========================================================================
    println!("\n--- Usage Notes ---\n");
    println!("Image Generation Tips:");
    println!("  1. Use 'gemini-3-pro-image-preview' or compatible model");
    println!("  2. Set response_modalities to [\"IMAGE\"]");
    println!("  3. Generated images are returned as base64-encoded data");
    println!("  4. Check MIME type to determine image format (PNG, JPEG, WebP)");
    println!("  5. Regional availability may affect access to image models");
    println!("\nTo save generated images, add the `base64` crate and use:");
    println!("  use base64::Engine;");
    println!("  let bytes = base64::engine::general_purpose::STANDARD.decode(&base64_data)?;");
    println!("  std::fs::write(\"image.png\", bytes)?;");
    println!("\nCommon Errors:");
    println!("  - 'model not found': Model not available in your region");
    println!("  - 'not supported': Feature not enabled for your API key");
    println!("  - Empty response: Try a different or simpler prompt");

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
