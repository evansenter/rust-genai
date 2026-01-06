//! Extension trait for `InteractionResponse` with convenience methods.
//!
//! This module provides the `InteractionResponseExt` trait which adds helpful
//! methods to `InteractionResponse` that require dependencies only available
//! in the public `rust-genai` crate (like base64 decoding).

use crate::errors::GenaiError;
use crate::{InteractionContent, InteractionResponse};
use base64::Engine;

/// Information about an image in the response.
///
/// This is a view type that provides convenient access to image data
/// in the response, with automatic base64 decoding.
///
/// # Example
///
/// ```no_run
/// use rust_genai::{Client, InteractionResponseExt};
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::new("api-key".to_string());
///
/// let response = client
///     .interaction()
///     .with_model("gemini-3-pro-image-preview")
///     .with_text("A cat playing with yarn")
///     .with_image_output()
///     .create()
///     .await?;
///
/// for image in response.images() {
///     let bytes = image.bytes()?;
///     let filename = format!("image.{}", image.extension());
///     std::fs::write(&filename, bytes)?;
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ImageInfo<'a> {
    data: &'a str,
    mime_type: Option<&'a str>,
}

impl ImageInfo<'_> {
    /// Decodes and returns the image bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the base64 data is invalid.
    #[must_use = "this `Result` should be used to handle potential decode errors"]
    pub fn bytes(&self) -> Result<Vec<u8>, GenaiError> {
        base64::engine::general_purpose::STANDARD
            .decode(self.data)
            .map_err(|e| GenaiError::InvalidInput(format!("Invalid base64 image data: {}", e)))
    }

    /// Returns the MIME type of the image, if available.
    #[must_use]
    pub fn mime_type(&self) -> Option<&str> {
        self.mime_type
    }

    /// Returns a file extension suitable for this image's MIME type.
    ///
    /// Returns "png" as default if MIME type is unknown or unrecognized.
    /// Logs a warning for unrecognized MIME types to surface API evolution
    /// (following the project's Evergreen philosophy).
    #[must_use]
    pub fn extension(&self) -> &str {
        match self.mime_type {
            Some("image/jpeg") | Some("image/jpg") => "jpg",
            Some("image/png") => "png",
            Some("image/webp") => "webp",
            Some("image/gif") => "gif",
            Some(unknown) => {
                log::warn!(
                    "Unknown image MIME type '{}', defaulting to 'png' extension. \
                     Consider updating rust-genai to handle this type.",
                    unknown
                );
                "png"
            }
            None => "png", // No MIME type provided, default to png
        }
    }
}

/// Extension trait for `InteractionResponse` providing convenience methods.
///
/// This trait is re-exported at the crate root. Import it with
/// `use rust_genai::InteractionResponseExt`.
///
/// # Example
///
/// ```no_run
/// use rust_genai::{Client, InteractionResponseExt};
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::new("api-key".to_string());
///
/// let response = client
///     .interaction()
///     .with_model("gemini-3-pro-image-preview")
///     .with_text("A cute cat")
///     .with_image_output()
///     .create()
///     .await?;
///
/// // Simple extraction of first image
/// if let Some(bytes) = response.first_image_bytes()? {
///     std::fs::write("cat.png", bytes)?;
/// }
/// # Ok(())
/// # }
/// ```
pub trait InteractionResponseExt {
    /// Returns the decoded bytes of the first image in the response.
    ///
    /// This is a convenience method for the common case of extracting a single
    /// generated image. For multiple images, use [`images()`](Self::images).
    ///
    /// # Errors
    ///
    /// Returns an error if the base64 data is invalid.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::{Client, InteractionResponseExt};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-pro-image-preview")
    ///     .with_text("A sunset over mountains")
    ///     .with_image_output()
    ///     .create()
    ///     .await?;
    ///
    /// if let Some(bytes) = response.first_image_bytes()? {
    ///     std::fs::write("sunset.png", &bytes)?;
    ///     println!("Saved {} bytes", bytes.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn first_image_bytes(&self) -> Result<Option<Vec<u8>>, GenaiError>;

    /// Returns an iterator over all images in the response.
    ///
    /// Each item is an [`ImageInfo`] that provides access to the image data,
    /// MIME type, and convenience methods for decoding.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::{Client, InteractionResponseExt};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new("api-key".to_string());
    ///
    /// let response = client
    ///     .interaction()
    ///     .with_model("gemini-3-pro-image-preview")
    ///     .with_text("Generate 3 variations of a cat")
    ///     .with_image_output()
    ///     .create()
    ///     .await?;
    ///
    /// for (i, image) in response.images().enumerate() {
    ///     let bytes = image.bytes()?;
    ///     let filename = format!("cat_{}.{}", i, image.extension());
    ///     std::fs::write(&filename, bytes)?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn images(&self) -> impl Iterator<Item = ImageInfo<'_>>;

    /// Check if the response contains any images.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::{Client, InteractionResponseExt};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Client::new("api-key".to_string());
    /// # let response = client.interaction().with_model("gemini-3-pro-image-preview")
    /// #     .with_text("A cat").with_image_output().create().await?;
    /// if response.has_images() {
    ///     for image in response.images() {
    ///         let bytes = image.bytes()?;
    ///         // process images...
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    fn has_images(&self) -> bool;
}

impl InteractionResponseExt for InteractionResponse {
    fn first_image_bytes(&self) -> Result<Option<Vec<u8>>, GenaiError> {
        for output in &self.outputs {
            if let InteractionContent::Image {
                data: Some(base64_data),
                ..
            } = output
            {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(base64_data)
                    .map_err(|e| {
                        GenaiError::InvalidInput(format!("Invalid base64 image data: {}", e))
                    })?;
                return Ok(Some(bytes));
            }
        }
        Ok(None)
    }

    fn images(&self) -> impl Iterator<Item = ImageInfo<'_>> {
        self.outputs.iter().filter_map(|output| {
            if let InteractionContent::Image {
                data: Some(base64_data),
                mime_type,
                ..
            } = output
            {
                Some(ImageInfo {
                    data: base64_data.as_str(),
                    mime_type: mime_type.as_deref(),
                })
            } else {
                None
            }
        })
    }

    fn has_images(&self) -> bool {
        self.outputs
            .iter()
            .any(|output| matches!(output, InteractionContent::Image { data: Some(_), .. }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InteractionStatus;

    fn make_response_with_image(base64_data: &str, mime_type: Option<&str>) -> InteractionResponse {
        InteractionResponse {
            id: Some("test-id".to_string()),
            model: Some("test-model".to_string()),
            agent: None,
            input: vec![],
            outputs: vec![InteractionContent::Image {
                data: Some(base64_data.to_string()),
                mime_type: mime_type.map(String::from),
                uri: None,
                resolution: None,
            }],
            status: InteractionStatus::Completed,
            usage: None,
            tools: None,
            grounding_metadata: None,
            url_context_metadata: None,
            previous_interaction_id: None,
            created: None,
            updated: None,
        }
    }

    fn make_response_no_images() -> InteractionResponse {
        InteractionResponse {
            id: Some("test-id".to_string()),
            model: Some("test-model".to_string()),
            agent: None,
            input: vec![],
            outputs: vec![InteractionContent::Text {
                text: Some("Hello".to_string()),
                annotations: None,
            }],
            status: InteractionStatus::Completed,
            usage: None,
            tools: None,
            grounding_metadata: None,
            url_context_metadata: None,
            previous_interaction_id: None,
            created: None,
            updated: None,
        }
    }

    #[test]
    fn test_first_image_bytes_success() {
        // Base64 for "test"
        let base64_data = "dGVzdA==";
        let response = make_response_with_image(base64_data, Some("image/png"));

        let result = response.first_image_bytes();
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(bytes.is_some());
        assert_eq!(bytes.unwrap(), b"test");
    }

    #[test]
    fn test_first_image_bytes_no_images() {
        let response = make_response_no_images();

        let result = response.first_image_bytes();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_first_image_bytes_invalid_base64() {
        let response = make_response_with_image("not-valid-base64!!!", Some("image/png"));

        let result = response.first_image_bytes();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid base64"));
    }

    #[test]
    fn test_images_iterator() {
        // Create response with multiple images
        let response = InteractionResponse {
            id: Some("test-id".to_string()),
            model: Some("test-model".to_string()),
            agent: None,
            input: vec![],
            outputs: vec![
                InteractionContent::Image {
                    data: Some("dGVzdDE=".to_string()), // "test1"
                    mime_type: Some("image/png".to_string()),
                    uri: None,
                    resolution: None,
                },
                InteractionContent::Text {
                    text: Some("text between".to_string()),
                    annotations: None,
                },
                InteractionContent::Image {
                    data: Some("dGVzdDI=".to_string()), // "test2"
                    mime_type: Some("image/jpeg".to_string()),
                    uri: None,
                    resolution: None,
                },
            ],
            status: InteractionStatus::Completed,
            usage: None,
            tools: None,
            grounding_metadata: None,
            url_context_metadata: None,
            previous_interaction_id: None,
            created: None,
            updated: None,
        };

        let images: Vec<_> = response.images().collect();
        assert_eq!(images.len(), 2);

        assert_eq!(images[0].bytes().unwrap(), b"test1");
        assert_eq!(images[0].mime_type(), Some("image/png"));
        assert_eq!(images[0].extension(), "png");

        assert_eq!(images[1].bytes().unwrap(), b"test2");
        assert_eq!(images[1].mime_type(), Some("image/jpeg"));
        assert_eq!(images[1].extension(), "jpg");
    }

    #[test]
    fn test_has_images() {
        let response_with = make_response_with_image("dGVzdA==", Some("image/png"));
        assert!(response_with.has_images());

        let response_without = make_response_no_images();
        assert!(!response_without.has_images());
    }

    #[test]
    fn test_image_info_extension() {
        let check = |mime: Option<&str>, expected: &str| {
            let info = ImageInfo {
                data: "",
                mime_type: mime,
            };
            assert_eq!(info.extension(), expected);
        };

        check(Some("image/jpeg"), "jpg");
        check(Some("image/jpg"), "jpg");
        check(Some("image/png"), "png");
        check(Some("image/webp"), "webp");
        check(Some("image/gif"), "gif");
        check(Some("image/unknown"), "png"); // default
        check(None, "png"); // default
    }

    #[test]
    fn test_image_info_bytes_invalid_base64() {
        let info = ImageInfo {
            data: "not-valid-base64!!!",
            mime_type: Some("image/png"),
        };
        let result = info.bytes();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid base64"));
    }

    #[test]
    fn test_image_info_extension_unknown_mime_type() {
        // This test documents Evergreen-compliant behavior:
        // Unknown MIME types default to "png" and log a warning (not verified here)
        // to surface API evolution without breaking user code.
        let info = ImageInfo {
            data: "",
            mime_type: Some("image/future-format"),
        };
        assert_eq!(info.extension(), "png");

        // Completely novel MIME type also defaults gracefully
        let info2 = ImageInfo {
            data: "",
            mime_type: Some("application/octet-stream"),
        };
        assert_eq!(info2.extension(), "png");
    }
}
