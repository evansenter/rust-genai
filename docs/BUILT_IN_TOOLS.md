# Built-in Tools Guide

Gemini provides several server-side tools that execute automatically without requiring client-side code. This guide covers all built-in tools and when to use each.

## Table of Contents

- [Overview](#overview)
- [Google Search](#google-search)
- [Code Execution](#code-execution)
- [URL Context](#url-context)
- [Computer Use](#computer-use)
- [File Search](#file-search)
- [Combining Tools](#combining-tools)

## Overview

| Tool | Purpose | Execution |
|------|---------|-----------|
| Google Search | Real-time web data | Server-side |
| Code Execution | Run Python code | Server-side sandbox |
| URL Context | Fetch and analyze URLs | Server-side |
| Computer Use | Browser automation | Server-side |
| File Search | Semantic document search | Server-side |

**Key distinction**: These are *server-side* tools executed by Google's infrastructure, unlike *client-side* function calling where your code executes the functions.

## Google Search

Ground responses in real-time web data.

### Basic Usage

```rust,ignore
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("What are the latest Rust 2024 features?")
    .with_google_search()
    .create()
    .await?;

// Access the response text
println!("{}", response.text().unwrap());

// Check if grounded with search
if response.has_google_search_metadata() {
    // Get search queries used
    for query in response.google_search_calls() {
        println!("Searched: {}", query);
    }

    // Get source URLs
    for result in response.google_search_results() {
        println!("Source: {} - {}", result.title, result.url);
    }
}
```

### With Annotations (Citations)

```rust,ignore
if response.has_annotations() {
    let text = response.all_text();
    for annotation in response.all_annotations() {
        if let Some(span) = annotation.extract_span(&text) {
            println!("'{}' sourced from: {:?}", span, annotation.source);
        }
    }
}
```

### Streaming

```rust,ignore
use futures_util::StreamExt;

let mut stream = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Latest AI news?")
    .with_google_search()
    .create_stream();

while let Some(Ok(event)) = stream.next().await {
    if let StreamChunk::Delta(delta) = event.chunk {
        if let Some(text) = delta.text() {
            print!("{}", text);
        }
    }
}
```

**When to use**: Current events, real-time data, fact verification, research tasks.

**Example**: `cargo run --example google_search`

## Code Execution

Execute Python code in a secure sandbox.

### Basic Usage

```rust,ignore
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Calculate the first 20 Fibonacci numbers")
    .with_code_execution()
    .create()
    .await?;

// Check for code execution
if response.has_code_execution() {
    // Get the code that was executed
    for call in response.code_execution_calls() {
        println!("Code:\n{}", call.code);
        println!("Language: {:?}", call.language);
    }

    // Get execution results
    for result in response.code_execution_results() {
        match result.outcome {
            CodeExecutionOutcome::Ok => {
                println!("Output: {}", result.output.as_deref().unwrap_or("(none)"));
            }
            CodeExecutionOutcome::Failed => {
                println!("Error: {}", result.output.as_deref().unwrap_or("unknown"));
            }
            _ => {}
        }
    }
}
```

### Convenience Methods

```rust,ignore
// Get successful output directly
if let Some(output) = response.successful_code_output() {
    println!("Result: {}", output);
}

// Check execution status
if response.has_successful_code() {
    println!("Code ran successfully");
}
```

**When to use**: Mathematical calculations, data processing, algorithm implementation, generating visualizations.

**Limitations**:
- Python only
- Sandboxed environment (no network, limited filesystem)
- Execution timeout limits

**Example**: `cargo run --example code_execution`

## URL Context

Fetch and analyze web pages.

### Basic Usage

```rust,ignore
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Summarize this article")
    .with_url_context(&["https://example.com/article"])
    .create()
    .await?;

// Check URL retrieval status
if let Some(metadata) = response.url_context_metadata() {
    for entry in &metadata.url_metadata {
        println!("URL: {}", entry.url);
        println!("Status: {:?}", entry.status);
    }
}
```

### Multiple URLs

```rust,ignore
let urls = vec![
    "https://example.com/page1",
    "https://example.com/page2",
];

let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Compare these two pages")
    .with_url_context(&urls)
    .create()
    .await?;
```

### Handling Retrieval Errors

```rust,ignore
if let Some(metadata) = response.url_context_metadata() {
    for entry in &metadata.url_metadata {
        match entry.status {
            UrlRetrievalStatus::Success => {
                println!("Retrieved: {}", entry.url);
            }
            UrlRetrievalStatus::Failed => {
                println!("Failed to retrieve: {}", entry.url);
            }
            _ => {}
        }
    }
}
```

**When to use**: Summarizing articles, comparing pages, extracting structured data from websites.

**Limitations**:
- Some sites block automated access
- Large pages may be truncated
- Dynamic content may not render

## Computer Use

Browser automation for web interactions.

### Basic Usage

```rust,ignore
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Go to example.com and describe what you see")
    .with_computer_use()
    .create()
    .await?;
```

### Excluding Actions

```rust,ignore
// Disable specific actions for safety
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Search for Rust tutorials")
    .with_computer_use_excluding(&["click", "type"])  // Read-only mode
    .create()
    .await?;
```

**When to use**: Web scraping, form filling, interactive web tasks.

**Safety considerations**:
- Always review what actions are enabled
- Use exclusions for read-only tasks
- Be cautious with authentication flows

**Example**: `cargo run --example computer_use`

## File Search

Semantic search across uploaded documents.

### Setup: Upload Files First

```rust,ignore
// Upload documents to the Files API
let file1 = client.upload_file("report.pdf").await?;
let file2 = client.upload_file("notes.txt").await?;

// Wait for processing
client.wait_for_file_active(&file1.name, Duration::from_secs(60)).await?;
client.wait_for_file_active(&file2.name, Duration::from_secs(60)).await?;
```

### Basic Search

```rust,ignore
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("What does the report say about Q4 revenue?")
    .with_file_search(&[&file1.name, &file2.name])
    .create()
    .await?;

// Access search results
for result in response.file_search_results() {
    println!("Found in: {}", result.file_name);
    println!("Snippet: {}", result.snippet);
}
```

### With Configuration

```rust,ignore
use genai_rs::FileSearchConfig;

let config = FileSearchConfig {
    max_results: Some(10),
    min_relevance_score: Some(0.7),
};

let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Find all mentions of 'performance optimization'")
    .with_file_search_config(&[&file1.name], config)
    .create()
    .await?;
```

**When to use**: Document Q&A, research across multiple files, finding specific information in large documents.

**Example**: `cargo run --example file_search`

## Combining Tools

Multiple built-in tools can be enabled simultaneously.

### Research Assistant

```rust,ignore
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Research the latest Rust async features and write example code")
    .with_google_search()      // Find current information
    .with_code_execution()     // Write and test code
    .create()
    .await?;
```

### Document Analysis with Web Context

```rust,ignore
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Compare our internal report with public benchmarks")
    .with_file_search(&[&report_file])
    .with_url_context(&["https://benchmarks.example.com"])
    .create()
    .await?;
```

### With Client-Side Functions

Built-in tools can also combine with your own functions:

```rust,ignore
use genai_rs_macros::tool;

#[tool(description = "Get current user's preferences")]
fn get_user_prefs() -> String {
    // Your implementation
    r#"{"theme": "dark", "language": "en"}"#.to_string()
}

let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Personalize search results based on my preferences")
    .with_google_search()
    .with_function::<get_user_prefs>()
    .create_with_auto_functions()
    .await?;
```

## Response Helpers Reference

| Method | Tool | Returns |
|--------|------|---------|
| `has_google_search_metadata()` | Google Search | `bool` |
| `google_search_calls()` | Google Search | `Vec<String>` (queries) |
| `google_search_results()` | Google Search | `Vec<GoogleSearchResultItem>` |
| `has_code_execution()` | Code Execution | `bool` |
| `code_execution_calls()` | Code Execution | `Vec<CodeExecutionCallInfo>` |
| `code_execution_results()` | Code Execution | `Vec<CodeExecutionResultInfo>` |
| `successful_code_output()` | Code Execution | `Option<String>` |
| `url_context_metadata()` | URL Context | `Option<&UrlContextMetadata>` |
| `url_context_calls()` | URL Context | `Vec<UrlContextCallInfo>` |
| `url_context_results()` | URL Context | `Vec<UrlContextResultInfo>` |
| `file_search_results()` | File Search | `Vec<FileSearchResultItem>` |
| `has_annotations()` | Any grounded | `bool` |
| `all_annotations()` | Any grounded | `Iterator<Item = &Annotation>` |

## Examples

| Example | Tools Demonstrated |
|---------|-------------------|
| `google_search` | Google Search with streaming |
| `code_execution` | Python execution and result handling |
| `computer_use` | Browser automation |
| `file_search` | Document search with Files API |
| `deep_research` | Multi-tool research agent |

Run examples with:
```bash
cargo run --example <name>
```
