//! Multimodal content utilities for file loading and MIME type detection.
//!
//! This module provides convenience functions for working with multimodal content
//! (images, audio, video, documents) by handling file loading, base64 encoding,
//! and MIME type detection automatically.
//!
//! # Example
//!
//! ```no_run
//! use rust_genai::{Client, image_from_file, text_content};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::new("api-key".to_string());
//!
//! // Load image from file with automatic MIME detection
//! let image = image_from_file("photo.jpg").await?;
//!
//! let contents = vec![
//!     text_content("What's in this image?"),
//!     image,
//! ];
//!
//! let response = client
//!     .interaction()
//!     .with_model("gemini-3-flash-preview")
//!     .with_content(contents)
//!     .create()
//!     .await?;
//! # Ok(())
//! # }
//! ```

use crate::interactions_api::{
    audio_data_content, document_data_content, image_data_content, video_data_content,
};
use base64::Engine;
use genai_client::{GenaiError, InteractionContent};
use std::path::Path;

/// Detects MIME type from file extension.
///
/// Returns the MIME type as a static string if the extension is recognized,
/// or `None` for unsupported or missing extensions.
///
/// # Supported Types
///
/// ## Images
/// - `jpg`, `jpeg` → `image/jpeg`
/// - `png` → `image/png`
/// - `gif` → `image/gif`
/// - `webp` → `image/webp`
/// - `heic`, `heif` → `image/heic`
///
/// ## Audio
/// - `mp3` → `audio/mp3`
/// - `wav` → `audio/wav`
/// - `ogg` → `audio/ogg`
/// - `flac` → `audio/flac`
/// - `aac` → `audio/aac`
/// - `m4a` → `audio/m4a`
///
/// ## Video
/// - `mp4` → `video/mp4`
/// - `webm` → `video/webm`
/// - `mov` → `video/quicktime`
/// - `avi` → `video/x-msvideo`
/// - `mkv` → `video/x-matroska`
///
/// ## Documents
/// - `pdf` → `application/pdf`
///
/// # Example
///
/// ```
/// use std::path::Path;
/// use rust_genai::detect_mime_type;
///
/// assert_eq!(detect_mime_type(Path::new("photo.jpg")), Some("image/jpeg"));
/// assert_eq!(detect_mime_type(Path::new("audio.mp3")), Some("audio/mp3"));
/// assert_eq!(detect_mime_type(Path::new("unknown.xyz")), None);
/// ```
pub fn detect_mime_type(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    match ext.as_str() {
        // Images
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "heic" | "heif" => Some("image/heic"),
        // Audio
        "mp3" => Some("audio/mp3"),
        "wav" => Some("audio/wav"),
        "ogg" => Some("audio/ogg"),
        "flac" => Some("audio/flac"),
        "aac" => Some("audio/aac"),
        "m4a" => Some("audio/m4a"),
        // Video
        "mp4" => Some("video/mp4"),
        "webm" => Some("video/webm"),
        "mov" => Some("video/quicktime"),
        "avi" => Some("video/x-msvideo"),
        "mkv" => Some("video/x-matroska"),
        // Documents
        "pdf" => Some("application/pdf"),
        _ => None,
    }
}

/// Internal helper to load and encode a file.
async fn load_and_encode_file(path: impl AsRef<Path>) -> Result<String, GenaiError> {
    let path = path.as_ref();
    let bytes = tokio::fs::read(path).await.map_err(|e| {
        let suggestion = match e.kind() {
            std::io::ErrorKind::NotFound => " Check that the file path is correct.",
            std::io::ErrorKind::PermissionDenied => " Check file permissions.",
            _ => "",
        };
        GenaiError::InvalidInput(format!(
            "Failed to read file '{}': {}.{}",
            path.display(),
            e,
            suggestion
        ))
    })?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
}

/// Helper to get extension-specific error information
fn get_extension_info(path: &Path) -> Result<String, GenaiError> {
    let ext_osstr = path.extension().ok_or_else(|| {
        GenaiError::InvalidInput(format!(
            "File '{}' has no extension. Cannot auto-detect MIME type. \
             Use the *_with_mime() variant to specify the type explicitly.",
            path.display()
        ))
    })?;

    ext_osstr.to_str().map(|s| s.to_lowercase()).ok_or_else(|| {
        GenaiError::InvalidInput(format!(
            "File extension for '{}' contains invalid UTF-8. \
             Use the *_with_mime() variant to specify the type explicitly.",
            path.display()
        ))
    })
}

/// Validates that the detected MIME type matches the expected category
fn validate_mime_category(
    mime_type: &str,
    expected_prefix: &str,
    path: &Path,
    category_name: &str,
) -> Result<(), GenaiError> {
    if !mime_type.starts_with(expected_prefix) {
        let suggestion = if mime_type.starts_with("image/") {
            "image_from_file()"
        } else if mime_type.starts_with("audio/") {
            "audio_from_file()"
        } else if mime_type.starts_with("video/") {
            "video_from_file()"
        } else if mime_type == "application/pdf" {
            "document_from_file()"
        } else {
            "the appropriate *_from_file() function"
        };

        return Err(GenaiError::InvalidInput(format!(
            "File '{}' has MIME type '{}' which is not {} type. Did you mean to use {}?",
            path.display(),
            mime_type,
            category_name,
            suggestion
        )));
    }
    Ok(())
}

/// Loads an image from a file with automatic MIME type detection.
///
/// Reads the file, encodes it as base64, and detects the MIME type from
/// the file extension.
///
/// # Arguments
///
/// * `path` - Path to the image file
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read
/// - The file extension is not recognized as an image type
///
/// # Example
///
/// ```no_run
/// use rust_genai::image_from_file;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let image = image_from_file("photo.jpg").await?;
/// # Ok(())
/// # }
/// ```
pub async fn image_from_file(path: impl AsRef<Path>) -> Result<InteractionContent, GenaiError> {
    let path = path.as_ref();

    // Get extension with specific error handling
    let ext = get_extension_info(path)?;

    // Try to detect MIME type
    let mime_type = detect_mime_type(path).ok_or_else(|| {
        GenaiError::InvalidInput(format!(
            "Unsupported image extension '.{}' for file '{}'. \
             Supported extensions: jpg, jpeg, png, gif, webp, heic, heif. \
             Use image_from_file_with_mime() to override.",
            ext,
            path.display()
        ))
    })?;

    // Validate this is actually an image MIME type
    validate_mime_category(mime_type, "image/", path, "an image")?;

    image_from_file_with_mime(path, mime_type).await
}

/// Loads an image from a file with an explicit MIME type.
///
/// Use this when you need to override the auto-detected MIME type.
///
/// # Example
///
/// ```no_run
/// use rust_genai::image_from_file_with_mime;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let image = image_from_file_with_mime("image.raw", "image/png").await?;
/// # Ok(())
/// # }
/// ```
pub async fn image_from_file_with_mime(
    path: impl AsRef<Path>,
    mime_type: impl Into<String>,
) -> Result<InteractionContent, GenaiError> {
    let data = load_and_encode_file(&path).await?;
    Ok(image_data_content(data, mime_type))
}

/// Loads an audio file with automatic MIME type detection.
///
/// # Supported Formats
///
/// WAV, MP3, OGG, FLAC, AAC, M4A
///
/// # Example
///
/// ```no_run
/// use rust_genai::audio_from_file;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let audio = audio_from_file("recording.mp3").await?;
/// # Ok(())
/// # }
/// ```
pub async fn audio_from_file(path: impl AsRef<Path>) -> Result<InteractionContent, GenaiError> {
    let path = path.as_ref();

    // Get extension with specific error handling
    let ext = get_extension_info(path)?;

    // Try to detect MIME type
    let mime_type = detect_mime_type(path).ok_or_else(|| {
        GenaiError::InvalidInput(format!(
            "Unsupported audio extension '.{}' for file '{}'. \
             Supported extensions: mp3, wav, ogg, flac, aac, m4a. \
             Use audio_from_file_with_mime() to override.",
            ext,
            path.display()
        ))
    })?;

    // Validate this is actually an audio MIME type
    validate_mime_category(mime_type, "audio/", path, "an audio")?;

    audio_from_file_with_mime(path, mime_type).await
}

/// Loads an audio file with an explicit MIME type.
///
/// Use this when you need to override the auto-detected MIME type,
/// such as for files with non-standard extensions.
///
/// # Example
///
/// ```no_run
/// use rust_genai::audio_from_file_with_mime;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Load a raw audio file with explicit MIME type
/// let audio = audio_from_file_with_mime("audio.raw", "audio/wav").await?;
/// # Ok(())
/// # }
/// ```
pub async fn audio_from_file_with_mime(
    path: impl AsRef<Path>,
    mime_type: impl Into<String>,
) -> Result<InteractionContent, GenaiError> {
    let data = load_and_encode_file(&path).await?;
    Ok(audio_data_content(data, mime_type))
}

/// Loads a video file with automatic MIME type detection.
///
/// # Supported Formats
///
/// MP4, WebM, MOV, AVI, MKV
///
/// # Example
///
/// ```no_run
/// use rust_genai::video_from_file;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let video = video_from_file("clip.mp4").await?;
/// # Ok(())
/// # }
/// ```
pub async fn video_from_file(path: impl AsRef<Path>) -> Result<InteractionContent, GenaiError> {
    let path = path.as_ref();

    // Get extension with specific error handling
    let ext = get_extension_info(path)?;

    // Try to detect MIME type
    let mime_type = detect_mime_type(path).ok_or_else(|| {
        GenaiError::InvalidInput(format!(
            "Unsupported video extension '.{}' for file '{}'. \
             Supported extensions: mp4, webm, mov, avi, mkv. \
             Use video_from_file_with_mime() to override.",
            ext,
            path.display()
        ))
    })?;

    // Validate this is actually a video MIME type
    validate_mime_category(mime_type, "video/", path, "a video")?;

    video_from_file_with_mime(path, mime_type).await
}

/// Loads a video file with an explicit MIME type.
///
/// Use this when you need to override the auto-detected MIME type,
/// such as for files with non-standard extensions.
///
/// # Example
///
/// ```no_run
/// use rust_genai::video_from_file_with_mime;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Load a video file with explicit MIME type
/// let video = video_from_file_with_mime("video.raw", "video/mp4").await?;
/// # Ok(())
/// # }
/// ```
pub async fn video_from_file_with_mime(
    path: impl AsRef<Path>,
    mime_type: impl Into<String>,
) -> Result<InteractionContent, GenaiError> {
    let data = load_and_encode_file(&path).await?;
    Ok(video_data_content(data, mime_type))
}

/// Loads a document file with automatic MIME type detection.
///
/// Currently only PDF is supported. Use `document_from_file_with_mime()` for other formats.
///
/// # Example
///
/// ```no_run
/// use rust_genai::document_from_file;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let doc = document_from_file("report.pdf").await?;
/// # Ok(())
/// # }
/// ```
pub async fn document_from_file(path: impl AsRef<Path>) -> Result<InteractionContent, GenaiError> {
    let path = path.as_ref();

    // Get extension with specific error handling
    let ext = get_extension_info(path)?;

    // Try to detect MIME type
    let mime_type = detect_mime_type(path).ok_or_else(|| {
        GenaiError::InvalidInput(format!(
            "Unsupported document extension '.{}' for file '{}'. \
             Supported extensions: pdf. \
             Use document_from_file_with_mime() to override.",
            ext,
            path.display()
        ))
    })?;

    // Validate this is actually a document MIME type
    validate_mime_category(mime_type, "application/", path, "a document")?;

    document_from_file_with_mime(path, mime_type).await
}

/// Loads a document file with an explicit MIME type.
///
/// Use this when you need to override the auto-detected MIME type,
/// such as for text-based documents or files with non-standard extensions.
///
/// # Example
///
/// ```no_run
/// use rust_genai::document_from_file_with_mime;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Load a text file as a document
/// let doc = document_from_file_with_mime("notes.txt", "text/plain").await?;
/// # Ok(())
/// # }
/// ```
pub async fn document_from_file_with_mime(
    path: impl AsRef<Path>,
    mime_type: impl Into<String>,
) -> Result<InteractionContent, GenaiError> {
    let data = load_and_encode_file(&path).await?;
    Ok(document_data_content(data, mime_type))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_detect_mime_type_images() {
        assert_eq!(detect_mime_type(Path::new("photo.jpg")), Some("image/jpeg"));
        assert_eq!(
            detect_mime_type(Path::new("photo.jpeg")),
            Some("image/jpeg")
        );
        assert_eq!(detect_mime_type(Path::new("image.png")), Some("image/png"));
        assert_eq!(detect_mime_type(Path::new("anim.gif")), Some("image/gif"));
        assert_eq!(
            detect_mime_type(Path::new("photo.webp")),
            Some("image/webp")
        );
        assert_eq!(
            detect_mime_type(Path::new("photo.heic")),
            Some("image/heic")
        );
        assert_eq!(
            detect_mime_type(Path::new("photo.heif")),
            Some("image/heic")
        );
    }

    #[test]
    fn test_detect_mime_type_audio() {
        assert_eq!(detect_mime_type(Path::new("song.mp3")), Some("audio/mp3"));
        assert_eq!(detect_mime_type(Path::new("audio.wav")), Some("audio/wav"));
        assert_eq!(detect_mime_type(Path::new("audio.ogg")), Some("audio/ogg"));
        assert_eq!(
            detect_mime_type(Path::new("audio.flac")),
            Some("audio/flac")
        );
        assert_eq!(detect_mime_type(Path::new("audio.aac")), Some("audio/aac"));
        assert_eq!(detect_mime_type(Path::new("audio.m4a")), Some("audio/m4a"));
    }

    #[test]
    fn test_detect_mime_type_video() {
        assert_eq!(detect_mime_type(Path::new("video.mp4")), Some("video/mp4"));
        assert_eq!(
            detect_mime_type(Path::new("video.webm")),
            Some("video/webm")
        );
        assert_eq!(
            detect_mime_type(Path::new("video.mov")),
            Some("video/quicktime")
        );
        assert_eq!(
            detect_mime_type(Path::new("video.avi")),
            Some("video/x-msvideo")
        );
        assert_eq!(
            detect_mime_type(Path::new("video.mkv")),
            Some("video/x-matroska")
        );
    }

    #[test]
    fn test_detect_mime_type_documents() {
        assert_eq!(
            detect_mime_type(Path::new("doc.pdf")),
            Some("application/pdf")
        );
    }

    #[test]
    fn test_detect_mime_type_unknown() {
        assert_eq!(detect_mime_type(Path::new("file.xyz")), None);
        assert_eq!(detect_mime_type(Path::new("file.doc")), None);
        assert_eq!(detect_mime_type(Path::new("file.txt")), None);
        assert_eq!(detect_mime_type(Path::new("noextension")), None);
    }

    #[test]
    fn test_detect_mime_type_case_insensitive() {
        assert_eq!(detect_mime_type(Path::new("photo.JPG")), Some("image/jpeg"));
        assert_eq!(detect_mime_type(Path::new("photo.PNG")), Some("image/png"));
        assert_eq!(detect_mime_type(Path::new("photo.Mp3")), Some("audio/mp3"));
    }

    #[test]
    fn test_detect_mime_type_with_path() {
        assert_eq!(
            detect_mime_type(Path::new("/some/path/to/photo.jpg")),
            Some("image/jpeg")
        );
        assert_eq!(
            detect_mime_type(Path::new("./relative/path.png")),
            Some("image/png")
        );
    }

    #[test]
    fn test_detect_mime_type_trailing_dot() {
        // Edge case: file ending with period
        assert_eq!(detect_mime_type(Path::new("file.")), None);
        assert_eq!(detect_mime_type(Path::new("/path/to/file.")), None);
    }

    #[test]
    fn test_get_extension_info_missing_extension() {
        let result = get_extension_info(Path::new("noextension"));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("has no extension"));
        assert!(err.contains("*_with_mime()"));
    }

    #[test]
    fn test_get_extension_info_valid_extension() {
        let result = get_extension_info(Path::new("photo.jpg"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "jpg");
    }

    #[test]
    fn test_get_extension_info_uppercase() {
        let result = get_extension_info(Path::new("photo.JPG"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "jpg"); // Should be lowercase
    }

    #[test]
    fn test_validate_mime_category_correct() {
        let result =
            validate_mime_category("image/jpeg", "image/", Path::new("photo.jpg"), "an image");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_mime_category_wrong_category() {
        // Trying to use audio file as image
        let result =
            validate_mime_category("audio/mp3", "image/", Path::new("song.mp3"), "an image");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("audio/mp3"));
        assert!(err.contains("not an image type"));
        assert!(err.contains("audio_from_file()"));
    }

    #[test]
    fn test_validate_mime_category_suggests_correct_function() {
        // Video file used as audio
        let result =
            validate_mime_category("video/mp4", "audio/", Path::new("clip.mp4"), "an audio");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("video_from_file()"));

        // PDF file used as image
        let result = validate_mime_category(
            "application/pdf",
            "image/",
            Path::new("doc.pdf"),
            "an image",
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("document_from_file()"));
    }

    #[tokio::test]
    async fn test_image_from_file_nonexistent() {
        let result = image_from_file("/nonexistent/path/file.jpg").await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Failed to read file"));
        assert!(err.contains("Check that the file path is correct"));
    }

    #[tokio::test]
    async fn test_image_from_file_missing_extension() {
        let result = image_from_file("/tmp/noextension").await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("has no extension"));
    }

    #[tokio::test]
    async fn test_image_from_file_unsupported_extension() {
        let result = image_from_file("/tmp/file.xyz").await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unsupported image extension"));
        assert!(err.contains(".xyz"));
    }

    #[tokio::test]
    async fn test_audio_from_file_wrong_mime_category() {
        // audio_from_file given an image extension should fail with category error
        let result = audio_from_file("/tmp/photo.png").await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not an audio type"));
        assert!(err.contains("image_from_file()"));
    }
}
