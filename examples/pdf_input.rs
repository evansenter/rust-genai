//! Example: PDF Document Input
//!
//! This example demonstrates how to send PDF documents to the Gemini API
//! for analysis, extraction, and question answering.
//!
//! # Running
//!
//! ```bash
//! cargo run --example pdf_input
//! ```
//!
//! # Prerequisites
//!
//! Set the `GEMINI_API_KEY` environment variable with your API key.
//!
//! # PDF Support
//!
//! Gemini models can process PDF documents natively, understanding:
//! - Text content (both native text and OCR from scanned documents)
//! - Images, diagrams, charts, and tables
//! - Document structure and layout
//!
//! PDFs can be up to 1000 pages and are tokenized at approximately
//! 258 tokens per page. Since each page is treated as an image, costs follow
//! Gemini's image pricing (see <https://ai.google.dev/gemini-api/docs/document-processing>).

use futures_util::StreamExt;
use rust_genai::{Client, InteractionInput, StreamChunk, document_data_content, text_content};
use std::env;
use std::io::{Write, stdout};

// A minimal PDF document containing "Hello World" text for demonstration
// In real applications, you would read a PDF file and base64 encode it
const SAMPLE_PDF_BASE64: &str = "JVBERi0xLjQKMSAwIG9iago8PCAvVHlwZSAvQ2F0YWxvZyAvUGFnZXMgMiAwIFIgPj4KZW5kb2JqCjIgMCBvYmoKPDwgL1R5cGUgL1BhZ2VzIC9LaWRzIFszIDAgUl0gL0NvdW50IDEgPj4KZW5kb2JqCjMgMCBvYmoKPDwgL1R5cGUgL1BhZ2UgL1BhcmVudCAyIDAgUiAvTWVkaWFCb3ggWzAgMCA3MiA3Ml0gL0NvbnRlbnRzIDQgMCBSIC9SZXNvdXJjZXMgPDwgPj4gPj4KZW5kb2JqCjQgMCBvYmoKPDwgL0xlbmd0aCA0NCA+PgpzdHJlYW0KQlQgL0YxIDEyIFRmIDEwIDUwIFRkIChIZWxsbyBXb3JsZCkgVGogRVQKZW5kc3RyZWFtCmVuZG9iagp4cmVmCjAgNQowMDAwMDAwMDAwIDY1NTM1IGYgCjAwMDAwMDAwMDkgMDAwMDAgbiAKMDAwMDAwMDA1OCAwMDAwMCBuIAowMDAwMDAwMTE1IDAwMDAwIG4gCjAwMDAwMDAyMjQgMDAwMDAgbiAKdHJhaWxlcgo8PCAvU2l6ZSA1IC9Sb290IDEgMCBSID4+CnN0YXJ0eHJlZgozMjAKJSVFT0Y=";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable not set");

    let client = Client::builder(api_key).build();

    println!("=== PDF DOCUMENT INPUT EXAMPLE ===\n");

    // ==========================================================================
    // Example 1: Basic PDF Analysis
    // ==========================================================================
    println!("--- Example 1: Basic PDF Analysis ---\n");

    // Build content with text prompt and PDF document
    // In a real application, you would read the PDF file and base64 encode it:
    //
    //   use base64::Engine;
    //   let pdf_bytes = std::fs::read("document.pdf")?;
    //   let pdf_base64 = base64::engine::general_purpose::STANDARD.encode(&pdf_bytes);
    //
    let contents = vec![
        text_content("What text content does this PDF document contain?"),
        document_data_content(SAMPLE_PDF_BASE64, "application/pdf"),
    ];

    println!("Sending PDF to Gemini for analysis...\n");

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(InteractionInput::Content(contents))
        .with_store(true)
        .create()
        .await?;

    println!("Status: {:?}\n", response.status);

    if let Some(text) = response.text() {
        println!("Analysis:\n{}\n", text);
    }

    // ==========================================================================
    // Example 2: PDF with Follow-up Questions
    // ==========================================================================
    println!("--- Example 2: Follow-up Questions ---\n");

    // Use stateful conversation to ask follow-up questions about the PDF
    let follow_up = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_previous_interaction(&response.id)
        .with_text("What format is this document? Is it a valid PDF structure?")
        .with_store(true)
        .create()
        .await?;

    if let Some(text) = follow_up.text() {
        println!("Follow-up Response:\n{}\n", text);
    }

    // ==========================================================================
    // Example 3: PDF with Streaming Response
    // ==========================================================================
    println!("--- Example 3: Streaming PDF Analysis ---\n");

    let stream_contents = vec![
        text_content("Describe the structure of this PDF document in detail."),
        document_data_content(SAMPLE_PDF_BASE64, "application/pdf"),
    ];

    print!("Streaming Response: ");
    // Flush to ensure the prompt appears before streaming starts (stdout is line-buffered)
    stdout().flush()?;

    let mut stream = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input(InteractionInput::Content(stream_contents))
        .create_stream();

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => match chunk {
                StreamChunk::Delta(content) => {
                    if let Some(text) = content.text() {
                        print!("{}", text);
                        // Flush each chunk immediately for real-time streaming effect
                        stdout().flush()?;
                    }
                }
                StreamChunk::Complete(response) => {
                    println!("\n");
                    if let Some(usage) = response.usage {
                        if let Some(input) = usage.total_input_tokens {
                            println!("Input tokens: {}", input);
                        }
                        if let Some(output) = usage.total_output_tokens {
                            println!("Output tokens: {}", output);
                        }
                    }
                }
                _ => {} // Handle unknown variants
            },
            Err(e) => {
                eprintln!("\nStream error: {}", e);
                break;
            }
        }
    }

    // ==========================================================================
    // Usage Notes
    // ==========================================================================
    println!("\n--- Usage Notes ---\n");
    println!("PDF Document Input Tips:");
    println!("  1. Use base64 encoding for inline PDF data");
    println!("  2. Set mime_type to 'application/pdf'");
    println!("  3. PDFs up to 1000 pages are supported");
    println!("  4. Each page costs approximately 258 tokens");
    println!("  5. Native text extraction works for most PDFs");
    println!("  6. OCR is applied automatically to scanned pages");
    println!("\nTo encode a PDF file:");
    println!("  use base64::Engine;");
    println!("  let bytes = std::fs::read(\"doc.pdf\")?;");
    println!("  let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);");

    println!("\n=== END EXAMPLE ===");

    Ok(())
}
