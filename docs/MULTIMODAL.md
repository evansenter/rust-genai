# Multimodal Content Guide

This guide covers working with images, audio, video, and documents in `genai-rs`.

## Table of Contents

- [Overview](#overview)
- [Images](#images)
- [Audio](#audio)
- [Video](#video)
- [Documents](#documents)
- [Files API](#files-api)
- [Resolution Control](#resolution-control)
- [Content Constructors](#content-constructors)

## Overview

Gemini supports multimodal inputs through three methods:

| Method | Best For | Size Limit |
|--------|----------|------------|
| Inline base64 | Small files (<20MB) | ~20MB |
| URI reference | Files API uploads | Large files |
| Builder helpers | Ergonomic file loading | Varies |

## Images

### Method 1: Builder Pattern (Recommended)

```rust,ignore
// From file path (async, reads and encodes)
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Describe this image")
    .add_image_file("photo.jpg")
    .await?
    .create()
    .await?;

// From base64 data (sync)
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("What's in this image?")
    .add_image_data(base64_string, "image/png")
    .create()
    .await?;

// From bytes (sync, auto-encodes)
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Analyze this")
    .add_image_bytes(&image_bytes, "image/jpeg")
    .create()
    .await?;

// From URI (Files API or public URL)
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Describe the uploaded image")
    .add_image_uri(&file_metadata.uri, "image/png")
    .create()
    .await?;
```

### Method 2: Content Constructors

```rust,ignore
use genai_rs::{text_content, image_data_content};

// Build content vector manually
let contents = vec![
    text_content("Compare these images:"),
    image_data_content(base64_image1, "image/png"),
    image_data_content(base64_image2, "image/png"),
];

let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_content(contents)
    .create()
    .await?;
```

### Method 3: File Helpers

```rust,ignore
use genai_rs::image_from_file;

// Load and encode from filesystem
let content = image_from_file("photo.jpg").await?;

let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("What do you see?")
    .with_content(vec![content])
    .create()
    .await?;
```

### Supported Image Formats

| Format | MIME Type |
|--------|-----------|
| PNG | `image/png` |
| JPEG | `image/jpeg` |
| GIF | `image/gif` |
| WebP | `image/webp` |

## Audio

### Input

```rust,ignore
// From file
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Transcribe this audio")
    .add_audio_file("recording.mp3")
    .await?
    .create()
    .await?;

// From base64
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("What's being said?")
    .add_audio_data(base64_audio, "audio/mp3")
    .create()
    .await?;
```

### Output (Text-to-Speech)

```rust,ignore
let response = client
    .interaction()
    .with_model("gemini-2.5-pro-preview-tts")  // TTS-specific model
    .with_text("Hello, welcome to genai-rs!")
    .with_audio_output()
    .with_voice("Kore")  // Optional voice selection
    .create()
    .await?;

// Get audio data
if let Some(audio) = response.first_audio() {
    let bytes = audio.bytes()?;
    std::fs::write("output.wav", &bytes)?;
    println!("Saved audio: {} bytes", bytes.len());
}

// Iterate multiple audio outputs
for (i, audio) in response.audios().enumerate() {
    let bytes = audio.bytes()?;
    let filename = format!("audio_{}.{}", i, audio.extension());
    std::fs::write(&filename, bytes)?;
}
```

### Supported Audio Formats

| Format | MIME Type |
|--------|-----------|
| MP3 | `audio/mp3` or `audio/mpeg` |
| WAV | `audio/wav` |
| FLAC | `audio/flac` |
| OGG | `audio/ogg` |

## Video

```rust,ignore
// From file
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Describe what happens in this video")
    .add_video_file("clip.mp4")
    .await?
    .create()
    .await?;

// From base64
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Summarize this video")
    .add_video_data(base64_video, "video/mp4")
    .create()
    .await?;

// From Files API URI (for large videos)
let file = client.upload_file("large_video.mp4").await?;
client.wait_for_file_active(&file.name, Duration::from_secs(120)).await?;

let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("What's in this video?")
    .add_video_uri(&file.uri, "video/mp4")
    .create()
    .await?;
```

### Supported Video Formats

| Format | MIME Type |
|--------|-----------|
| MP4 | `video/mp4` |
| MPEG | `video/mpeg` |
| MOV | `video/quicktime` |
| AVI | `video/x-msvideo` |
| WebM | `video/webm` |

## Documents

### PDF Files

```rust,ignore
// From file
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Summarize this document")
    .add_document_file("report.pdf")
    .await?
    .create()
    .await?;

// From base64
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Extract key points from this PDF")
    .add_document_data(base64_pdf, "application/pdf")
    .create()
    .await?;
```

### Plain Text

```rust,ignore
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Analyze this code")
    .add_document_data(base64_text, "text/plain")
    .create()
    .await?;
```

### Supported Document Formats

| Format | MIME Type |
|--------|-----------|
| PDF | `application/pdf` |
| Plain Text | `text/plain` |
| HTML | `text/html` |
| CSV | `text/csv` |
| Markdown | `text/markdown` |

**Example**: `cargo run --example pdf_input`

## Files API

For files >20MB or when you need to reuse content across requests.

### Upload

```rust,ignore
// Simple upload
let file = client.upload_file("large_video.mp4").await?;
println!("Uploaded: {}", file.name);
println!("URI: {}", file.uri);

// With custom display name
let file = client
    .upload_file_with_options("data.csv", Some("Q4 Sales Data"))
    .await?;

// Chunked upload for very large files
let file = client
    .upload_file_chunked("huge_video.mp4", 10 * 1024 * 1024)  // 10MB chunks
    .await?;
```

### Wait for Processing

Videos and some documents require processing time:

```rust,ignore
// Wait up to 2 minutes for file to be ready
client
    .wait_for_file_active(&file.name, Duration::from_secs(120))
    .await?;

// Check state manually
let metadata = client.get_file(&file.name).await?;
match metadata.state {
    FileState::Active => println!("Ready to use"),
    FileState::Processing => println!("Still processing..."),
    FileState::Failed => println!("Processing failed"),
    _ => {}
}
```

### Use in Requests

```rust,ignore
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Analyze this file")
    .with_file_uri(&file.uri, &file.mime_type)
    .create()
    .await?;
```

### List and Delete

```rust,ignore
// List all uploaded files
let files = client.list_files(None).await?;
for file in files.files {
    println!("{}: {} ({})", file.name, file.display_name, file.state);
}

// Delete a file
client.delete_file(&file.name).await?;
```

**Example**: `cargo run --example files_api`

## Resolution Control

Control the trade-off between image quality and token cost.

### Resolution Levels

| Level | Use Case | Token Cost |
|-------|----------|------------|
| `Low` | Simple detection (colors, shapes) | Lowest |
| `Medium` | General analysis | Moderate |
| `High` | Detailed inspection | Higher |
| `UltraHigh` | Maximum detail | Highest |

### Usage

```rust,ignore
use genai_rs::Resolution;

// Builder pattern
let response = client
    .interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("What color is this?")
    .add_image_data_with_resolution(base64, "image/png", Resolution::Low)
    .create()
    .await?;

// Content constructor
use genai_rs::image_data_content_with_resolution;

let content = image_data_content_with_resolution(
    base64,
    "image/png",
    Resolution::High
);
```

### When to Use Each

| Resolution | Scenario |
|------------|----------|
| Low | Color detection, presence/absence checks |
| Medium | General image description (default) |
| High | Text reading, fine details |
| UltraHigh | Medical imaging, technical diagrams |

## Content Constructors

All constructors are re-exported from the crate root.

### Text

```rust,ignore
use genai_rs::text_content;

let content = text_content("Analyze the following:");
```

### Images

```rust,ignore
use genai_rs::{
    image_data_content,
    image_data_content_with_resolution,
    image_uri_content,
    image_uri_content_with_resolution,
};

// Inline base64
let content = image_data_content(base64, "image/png");
let content = image_data_content_with_resolution(base64, "image/png", Resolution::High);

// URI reference
let content = image_uri_content(uri, "image/png");
```

### Audio

```rust,ignore
use genai_rs::{audio_data_content, audio_uri_content};

let content = audio_data_content(base64, "audio/mp3");
let content = audio_uri_content(uri, "audio/mp3");
```

### Video

```rust,ignore
use genai_rs::{
    video_data_content,
    video_data_content_with_resolution,
    video_uri_content,
    video_uri_content_with_resolution,
};

let content = video_data_content(base64, "video/mp4");
let content = video_uri_content(uri, "video/mp4");
```

### Documents

```rust,ignore
use genai_rs::{document_data_content, document_uri_content};

let content = document_data_content(base64, "application/pdf");
let content = document_uri_content(uri, "application/pdf");
```

### From File Metadata

```rust,ignore
use genai_rs::file_uri_content;

let file = client.upload_file("document.pdf").await?;
let content = file_uri_content(&file);
```

## Image Generation

Generate images from text prompts.

```rust,ignore
let response = client
    .interaction()
    .with_model("gemini-3-pro-image-preview")  // Image generation model
    .with_text("A sunset over mountains, digital art style")
    .with_image_output()
    .create()
    .await?;

// Get the first generated image
if let Some(bytes) = response.first_image_bytes()? {
    std::fs::write("generated.png", &bytes)?;
}

// Check for multiple images
if response.has_images() {
    for (i, image) in response.images().enumerate() {
        let bytes = image.bytes()?;
        let filename = format!("image_{}.{}", i, image.extension());
        std::fs::write(&filename, bytes)?;
    }
}
```

**Example**: `cargo run --example image_generation`

## Examples

| Example | Features |
|---------|----------|
| `multimodal_image` | Image input, comparison, resolution control |
| `audio_input` | Audio transcription and analysis |
| `pdf_input` | PDF document processing |
| `files_api` | Upload, list, delete files |
| `image_generation` | Text-to-image generation |

Run with:
```bash
cargo run --example <name>
```
