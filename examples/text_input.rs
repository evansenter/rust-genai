//! Example: Text Document Input
//!
//! This example demonstrates how to send text-based documents (TXT, Markdown,
//! JSON, CSV, HTML, XML) to the Gemini API for analysis.
//!
//! # Running
//!
//! ```bash
//! cargo run --example text_input
//! ```
//!
//! # Prerequisites
//!
//! Set the `GEMINI_API_KEY` environment variable with your API key.
//!
//! # Supported Formats
//!
//! The `document_from_file()` function supports these text formats:
//!
//! | Extension | MIME Type |
//! |-----------|-----------|
//! | `.txt` | `text/plain` |
//! | `.md` | `text/markdown` |
//! | `.json` | `application/json` |
//! | `.csv` | `text/csv` |
//! | `.html` | `text/html` |
//! | `.xml` | `application/xml` |

use rust_genai::{Client, document_data_content, document_from_file, text_content};
use std::env;

// Sample JSON data for demonstration
const SAMPLE_JSON: &str = r#"{
  "users": [
    {"name": "Alice", "age": 30, "role": "engineer"},
    {"name": "Bob", "age": 25, "role": "designer"},
    {"name": "Carol", "age": 35, "role": "manager"}
  ],
  "metadata": {
    "version": "1.0",
    "generated": "2024-01-15"
  }
}"#;

// Sample CSV data for demonstration
const SAMPLE_CSV: &str = r#"name,age,role,department
Alice,30,engineer,R&D
Bob,25,designer,Product
Carol,35,manager,Operations
David,28,analyst,Finance"#;

// Sample Markdown for demonstration
const SAMPLE_MARKDOWN: &str = r#"# Project Status Report

## Summary
The project is on track for Q1 delivery.

## Key Metrics
- **Completion**: 75%
- **Budget**: Under by 10%
- **Team Size**: 8 members

## Next Steps
1. Complete integration testing
2. Finalize documentation
3. Prepare deployment scripts
"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable not set");

    let client = Client::builder(api_key).build();

    println!("=== TEXT DOCUMENT INPUT EXAMPLE ===\n");

    // ==========================================================================
    // Example 1: JSON Analysis
    // ==========================================================================
    println!("--- Example 1: JSON Analysis ---\n");

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_content(vec![
            text_content("Analyze this JSON data. How many users are there and what roles exist?"),
            document_data_content(SAMPLE_JSON, "application/json"),
        ])
        .create()
        .await?;

    if let Some(text) = response.text() {
        println!("JSON Analysis:\n{}\n", text);
    }

    // ==========================================================================
    // Example 2: CSV Data Extraction
    // ==========================================================================
    println!("--- Example 2: CSV Data Extraction ---\n");

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_content(vec![
            text_content("Parse this CSV and calculate the average age of all employees."),
            document_data_content(SAMPLE_CSV, "text/csv"),
        ])
        .create()
        .await?;

    if let Some(text) = response.text() {
        println!("CSV Analysis:\n{}\n", text);
    }

    // ==========================================================================
    // Example 3: Markdown Summarization (inline data)
    // ==========================================================================
    println!("--- Example 3: Markdown Summarization (inline) ---\n");

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_content(vec![
            text_content("Summarize this markdown document in one sentence."),
            document_data_content(SAMPLE_MARKDOWN, "text/markdown"),
        ])
        .create()
        .await?;

    if let Some(text) = response.text() {
        println!("Markdown Summary:\n{}\n", text);
    }

    // ==========================================================================
    // Example 4: File-based Loading with document_from_file()
    // ==========================================================================
    println!("--- Example 4: File-based Loading ---\n");

    // Load a markdown file from disk - MIME type is auto-detected from extension
    let readme_content = document_from_file("README.md").await?;

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_content(vec![
            text_content("What is the main purpose of this project based on the README?"),
            readme_content,
        ])
        .create()
        .await?;

    if let Some(text) = response.text() {
        println!("README Analysis:\n{}\n", text);
    }

    // ==========================================================================
    // Example 5: Builder Pattern with add_document_file()
    // ==========================================================================
    println!("--- Example 5: Builder Pattern ---\n");

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("List the main sections in this markdown file.")
        .add_document_file("CHANGELOG.md")
        .await?
        .create()
        .await?;

    if let Some(text) = response.text() {
        println!("CHANGELOG Sections:\n{}\n", text);
    }

    // ==========================================================================
    // Usage Notes
    // ==========================================================================
    println!("--- Usage Notes ---\n");
    println!("Text Document Input Tips:");
    println!("  1. Use document_from_file() for automatic file loading:");
    println!("     let doc = document_from_file(\"data.json\").await?;");
    println!();
    println!("  2. Use add_document_file() with the builder pattern:");
    println!("     client.interaction()");
    println!("         .add_document_file(\"README.md\").await?");
    println!("         .with_text(\"Summarize this\")");
    println!("         .create().await?;");
    println!();
    println!("  3. For inline data, use document_data_content():");
    println!("     document_data_content(json_string, \"application/json\")");
    println!();
    println!("Supported formats: .txt, .md, .json, .csv, .html, .xml");

    println!("\n=== END EXAMPLE ===");

    Ok(())
}
