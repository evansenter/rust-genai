//! Files API module for uploading and managing files with Google's Generative AI.
//!
//! The Files API allows uploading large files once and referencing them across multiple
//! interactions, reducing bandwidth and improving performance.
//!
//! # Overview
//!
//! Files are uploaded to Google's servers and can be referenced by their URI in
//! subsequent API calls. Files are automatically deleted after 48 hours.
//!
//! # Limits
//!
//! - Maximum file size: 2 GB
//! - Storage capacity: 20 GB per project
//! - File retention: 48 hours
//!
//! # Implementation Notes
//!
//! The current implementation uses Google's resumable upload protocol but completes
//! the upload in a single request. True resumable uploads (where you can retry from
//! an offset after network failure) are not implemented. For most use cases under
//! the 2 GB limit, this single-request approach works reliably. If you need to
//! upload very large files in unreliable network conditions, consider implementing
//! chunked upload logic with the resumable upload URI.
//!
//! # Example
//!
//! ```ignore
//! use rust_genai::Client;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::new("api-key".to_string());
//!
//! // Upload a file
//! let file = client.upload_file("video.mp4").await?;
//! println!("Uploaded: {} ({})", file.display_name.as_deref().unwrap_or(""), file.uri);
//!
//! // Use in interaction
//! let response = client.interaction()
//!     .with_model("gemini-3-flash-preview")
//!     .with_file(&file)
//!     .with_text("Describe this video")
//!     .create()
//!     .await?;
//!
//! // Clean up when done
//! client.delete_file(&file.name).await?;
//! # Ok(())
//! # }
//! ```

use crate::error_helpers::{check_response, deserialize_with_context};
use crate::errors::GenaiError;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};

/// Represents an uploaded file in the Files API.
///
/// Files are stored on Google's servers for 48 hours and can be referenced
/// in interactions by their URI.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    /// The resource name of the file (e.g., "files/abc123")
    pub name: String,

    /// User-provided display name for the file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// MIME type of the file
    pub mime_type: String,

    /// Size of the file in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<String>,

    /// Timestamp when the file was created
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<String>,

    /// Timestamp when the file will be automatically deleted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_time: Option<String>,

    /// SHA256 hash of the file contents
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256_hash: Option<String>,

    /// URI to reference this file in API calls
    #[serde(default)]
    pub uri: String,

    /// Processing state of the file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<FileState>,

    /// Error information if processing failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<FileError>,

    /// Video metadata (if this is a video file)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_metadata: Option<VideoMetadata>,
}

impl FileMetadata {
    /// Returns true if the file is still being processed.
    pub fn is_processing(&self) -> bool {
        matches!(self.state, Some(FileState::Processing))
    }

    /// Returns true if the file is ready to use.
    pub fn is_active(&self) -> bool {
        matches!(self.state, Some(FileState::Active))
    }

    /// Returns true if file processing failed.
    pub fn is_failed(&self) -> bool {
        matches!(self.state, Some(FileState::Failed))
    }

    /// Parses the size_bytes field as a u64, if present and valid.
    ///
    /// The API returns file sizes as strings in the JSON response.
    /// This helper parses that string into a numeric type for convenience.
    ///
    /// # Returns
    ///
    /// - `Some(size)` if size_bytes is present and can be parsed as u64
    /// - `None` if size_bytes is absent or cannot be parsed
    ///
    /// # Example
    ///
    /// ```
    /// # use genai_client::FileMetadata;
    /// # let file: FileMetadata = serde_json::from_str(r#"{"name":"files/abc","mimeType":"video/mp4","uri":"","sizeBytes":"1234567"}"#).unwrap();
    /// if let Some(size) = file.size_bytes_as_u64() {
    ///     println!("File size: {} bytes", size);
    /// }
    /// ```
    #[must_use]
    pub fn size_bytes_as_u64(&self) -> Option<u64> {
        self.size_bytes.as_ref().and_then(|s| s.parse().ok())
    }
}

/// Processing state of an uploaded file.
///
/// This enum is marked `#[non_exhaustive]` for forward compatibility.
/// New state values may be added by the API in future versions.
///
/// # Unknown State Handling
///
/// When the API returns a state value that this library doesn't recognize,
/// it will be captured in the `Unknown` variant with the original state
/// string preserved. This follows the Evergreen philosophy of graceful
/// degradation and data preservation.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum FileState {
    /// File is being processed
    Processing,
    /// File is ready to use
    Active,
    /// File processing failed
    Failed,
    /// Unknown state (for forward compatibility).
    ///
    /// This variant captures any unrecognized state values from the API,
    /// allowing the library to handle new states gracefully.
    ///
    /// The `state_type` field contains the unrecognized state string,
    /// and `data` contains the JSON value (typically the same string).
    Unknown {
        /// The unrecognized state string from the API
        state_type: String,
        /// The raw JSON value, preserved for debugging
        data: serde_json::Value,
    },
}

impl FileState {
    /// Check if this is an unknown state.
    #[must_use]
    pub const fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }

    /// Returns the state type name if this is an unknown state.
    ///
    /// Returns `None` for known states.
    #[must_use]
    pub fn unknown_state_type(&self) -> Option<&str> {
        match self {
            Self::Unknown { state_type, .. } => Some(state_type),
            _ => None,
        }
    }

    /// Returns the raw JSON data if this is an unknown state.
    ///
    /// Returns `None` for known states.
    #[must_use]
    pub fn unknown_data(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Unknown { data, .. } => Some(data),
            _ => None,
        }
    }
}

impl Serialize for FileState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Processing => serializer.serialize_str("PROCESSING"),
            Self::Active => serializer.serialize_str("ACTIVE"),
            Self::Failed => serializer.serialize_str("FAILED"),
            Self::Unknown { state_type, .. } => serializer.serialize_str(state_type),
        }
    }
}

impl<'de> Deserialize<'de> for FileState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        match value.as_str() {
            Some("PROCESSING") => Ok(Self::Processing),
            Some("ACTIVE") => Ok(Self::Active),
            Some("FAILED") => Ok(Self::Failed),
            Some(other) => {
                log::warn!(
                    "Encountered unknown FileState '{}'. \
                     This may indicate a new API feature. \
                     The state will be preserved in the Unknown variant.",
                    other
                );
                Ok(Self::Unknown {
                    state_type: other.to_string(),
                    data: value,
                })
            }
            None => {
                // Non-string value - preserve it in Unknown
                let state_type = format!("<non-string: {}>", value);
                log::warn!(
                    "FileState received non-string value: {}. \
                     Preserving in Unknown variant.",
                    value
                );
                Ok(Self::Unknown {
                    state_type,
                    data: value,
                })
            }
        }
    }
}

/// Error information for failed file operations.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileError {
    /// Error code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i32>,
    /// Error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.code, &self.message) {
            (Some(code), Some(msg)) => write!(f, "error {}: {}", code, msg),
            (Some(code), None) => write!(f, "error {}", code),
            (None, Some(msg)) => write!(f, "{}", msg),
            (None, None) => write!(f, "unknown error"),
        }
    }
}

/// Metadata for video files.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetadata {
    /// Duration of the video
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_duration: Option<String>,
}

/// Response from listing files.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListFilesResponse {
    /// List of files
    #[serde(default)]
    pub files: Vec<FileMetadata>,

    /// Token for retrieving the next page of results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

/// Wrapper for file upload response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileUploadResponse {
    /// The uploaded file metadata
    pub file: FileMetadata,
}

// --- API Functions ---

const BASE_URL: &str = "https://generativelanguage.googleapis.com";
const UPLOAD_URL: &str = "https://generativelanguage.googleapis.com/upload/v1beta/files";
const API_VERSION: &str = "v1beta";

/// Uploads a file to the Files API using the resumable upload protocol.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use
/// * `api_key` - API key for authentication
/// * `file_data` - Raw bytes of the file
/// * `mime_type` - MIME type of the file
/// * `display_name` - Optional display name for the file
///
/// # Errors
///
/// Returns an error if the upload fails or the response cannot be parsed.
pub async fn upload_file(
    http_client: &ReqwestClient,
    api_key: &str,
    file_data: Vec<u8>,
    mime_type: &str,
    display_name: Option<&str>,
) -> Result<FileMetadata, GenaiError> {
    // Validate file is not empty
    if file_data.is_empty() {
        return Err(GenaiError::InvalidInput(
            "Cannot upload empty file".to_string(),
        ));
    }

    // Validate file size doesn't exceed API limit (2 GB)
    const MAX_FILE_SIZE: usize = 2_147_483_648; // 2 GB
    let file_size = file_data.len();
    if file_size > MAX_FILE_SIZE {
        return Err(GenaiError::InvalidInput(format!(
            "File size {} bytes exceeds maximum allowed size of {} bytes (2 GB)",
            file_size, MAX_FILE_SIZE
        )));
    }

    log::debug!(
        "Uploading file: size={} bytes, mime_type={}, display_name={:?}",
        file_size,
        mime_type,
        display_name
    );

    // Step 1: Start the resumable upload
    let start_url = format!("{UPLOAD_URL}?key={api_key}");

    let metadata = if let Some(name) = display_name {
        serde_json::json!({ "file": { "displayName": name } })
    } else {
        serde_json::json!({ "file": {} })
    };

    let start_response = http_client
        .post(&start_url)
        .header("X-Goog-Upload-Protocol", "resumable")
        .header("X-Goog-Upload-Command", "start")
        .header("X-Goog-Upload-Header-Content-Length", file_size.to_string())
        .header("X-Goog-Upload-Header-Content-Type", mime_type)
        .header("Content-Type", "application/json")
        .json(&metadata)
        .send()
        .await?;

    let start_response = check_response(start_response).await?;

    // Extract the upload URL from the response headers
    let upload_url = start_response
        .headers()
        .get("x-goog-upload-url")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            GenaiError::InvalidInput("Missing upload URL in response headers".to_string())
        })?
        .to_string();

    log::debug!("Got upload URL, uploading file data...");

    // Step 2: Upload the file bytes
    let upload_response = http_client
        .post(&upload_url)
        .header("X-Goog-Upload-Offset", "0")
        .header("X-Goog-Upload-Command", "upload, finalize")
        .header("Content-Length", file_size.to_string())
        .body(file_data)
        .send()
        .await?;

    let upload_response = check_response(upload_response).await?;
    let response_text = upload_response.text().await.map_err(GenaiError::Http)?;
    let file_response: FileUploadResponse =
        deserialize_with_context(&response_text, "FileUploadResponse")?;

    log::debug!(
        "File uploaded successfully: name={}, uri={}",
        file_response.file.name,
        file_response.file.uri
    );

    Ok(file_response.file)
}

/// Gets metadata for a specific file.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use
/// * `api_key` - API key for authentication
/// * `file_name` - The resource name of the file (e.g., "files/abc123")
///
/// # Errors
///
/// Returns an error if the request fails or the file doesn't exist.
pub async fn get_file(
    http_client: &ReqwestClient,
    api_key: &str,
    file_name: &str,
) -> Result<FileMetadata, GenaiError> {
    log::debug!("Getting file metadata: {}", file_name);

    let url = format!("{BASE_URL}/{API_VERSION}/{file_name}?key={api_key}");

    let response = http_client.get(&url).send().await?;
    let response = check_response(response).await?;
    let response_text = response.text().await.map_err(GenaiError::Http)?;
    let file: FileMetadata = deserialize_with_context(&response_text, "FileMetadata")?;

    log::debug!("Got file: state={:?}", file.state);

    Ok(file)
}

/// Lists all uploaded files.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use
/// * `api_key` - API key for authentication
/// * `page_size` - Optional maximum number of files to return
/// * `page_token` - Optional token for pagination
///
/// # Errors
///
/// Returns an error if the request fails.
pub async fn list_files(
    http_client: &ReqwestClient,
    api_key: &str,
    page_size: Option<u32>,
    page_token: Option<&str>,
) -> Result<ListFilesResponse, GenaiError> {
    log::debug!(
        "Listing files: page_size={:?}, page_token={:?}",
        page_size,
        page_token
    );

    let mut url = format!("{BASE_URL}/{API_VERSION}/files?key={api_key}");

    if let Some(size) = page_size {
        url.push_str(&format!("&pageSize={size}"));
    }
    if let Some(token) = page_token {
        url.push_str(&format!("&pageToken={token}"));
    }

    let response = http_client.get(&url).send().await?;
    let response = check_response(response).await?;
    let response_text = response.text().await.map_err(GenaiError::Http)?;
    let list_response: ListFilesResponse =
        deserialize_with_context(&response_text, "ListFilesResponse")?;

    log::debug!("Listed {} files", list_response.files.len());

    Ok(list_response)
}

/// Deletes an uploaded file.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use
/// * `api_key` - API key for authentication
/// * `file_name` - The resource name of the file to delete (e.g., "files/abc123")
///
/// # Errors
///
/// Returns an error if the request fails or the file doesn't exist.
pub async fn delete_file(
    http_client: &ReqwestClient,
    api_key: &str,
    file_name: &str,
) -> Result<(), GenaiError> {
    log::debug!("Deleting file: {}", file_name);

    let url = format!("{BASE_URL}/{API_VERSION}/{file_name}?key={api_key}");

    let response = http_client.delete(&url).send().await?;
    check_response(response).await?;

    log::debug!("File deleted successfully");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_metadata_deserialization() {
        let json = r#"{
            "name": "files/abc123",
            "displayName": "test.mp4",
            "mimeType": "video/mp4",
            "sizeBytes": "1234567",
            "createTime": "2024-01-01T00:00:00Z",
            "expirationTime": "2024-01-03T00:00:00Z",
            "uri": "https://generativelanguage.googleapis.com/v1beta/files/abc123",
            "state": "ACTIVE"
        }"#;

        let file: FileMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(file.name, "files/abc123");
        assert_eq!(file.display_name.as_deref(), Some("test.mp4"));
        assert_eq!(file.mime_type, "video/mp4");
        assert!(file.is_active());
        assert!(!file.is_processing());
    }

    #[test]
    fn test_file_state_processing() {
        let json =
            r#"{"name": "files/test", "mimeType": "video/mp4", "state": "PROCESSING", "uri": ""}"#;
        let file: FileMetadata = serde_json::from_str(json).unwrap();
        assert!(file.is_processing());
        assert!(!file.is_active());
    }

    #[test]
    fn test_file_state_failed() {
        let json =
            r#"{"name": "files/test", "mimeType": "video/mp4", "state": "FAILED", "uri": ""}"#;
        let file: FileMetadata = serde_json::from_str(json).unwrap();
        assert!(file.is_failed());
        assert!(!file.is_active());
    }

    #[test]
    fn test_list_files_response_deserialization() {
        let json = r#"{
            "files": [
                {"name": "files/a", "mimeType": "video/mp4", "uri": ""},
                {"name": "files/b", "mimeType": "image/png", "uri": ""}
            ],
            "nextPageToken": "token123"
        }"#;

        let response: ListFilesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.files.len(), 2);
        assert_eq!(response.next_page_token.as_deref(), Some("token123"));
    }

    #[test]
    fn test_empty_list_files_response() {
        let json = r#"{}"#;
        let response: ListFilesResponse = serde_json::from_str(json).unwrap();
        assert!(response.files.is_empty());
        assert!(response.next_page_token.is_none());
    }

    #[test]
    fn test_file_state_unknown_preserves_data() {
        // Test that unknown states preserve the original value
        let json =
            r#"{"name": "files/test", "mimeType": "video/mp4", "state": "UPLOADING", "uri": ""}"#;
        let file: FileMetadata = serde_json::from_str(json).unwrap();

        assert!(!file.is_active());
        assert!(!file.is_processing());
        assert!(!file.is_failed());

        // Check the Unknown variant captured the state
        if let Some(FileState::Unknown { state_type, data }) = &file.state {
            assert_eq!(state_type, "UPLOADING");
            assert_eq!(data.as_str(), Some("UPLOADING"));
        } else {
            panic!("Expected FileState::Unknown variant, got {:?}", file.state);
        }
    }

    #[test]
    fn test_file_state_unknown_helper_methods() {
        let unknown = FileState::Unknown {
            state_type: "NEW_STATE".to_string(),
            data: serde_json::json!("NEW_STATE"),
        };

        assert!(unknown.is_unknown());
        assert_eq!(unknown.unknown_state_type(), Some("NEW_STATE"));
        assert_eq!(
            unknown.unknown_data(),
            Some(&serde_json::json!("NEW_STATE"))
        );

        // Known states should return None for unknown helpers
        let active = FileState::Active;
        assert!(!active.is_unknown());
        assert_eq!(active.unknown_state_type(), None);
        assert_eq!(active.unknown_data(), None);
    }

    #[test]
    fn test_file_state_roundtrip_serialization() {
        // Known state roundtrips
        let active = FileState::Active;
        let json = serde_json::to_string(&active).unwrap();
        assert_eq!(json, r#""ACTIVE""#);
        let deserialized: FileState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, FileState::Active);

        // Unknown state roundtrips
        let unknown = FileState::Unknown {
            state_type: "QUEUED".to_string(),
            data: serde_json::json!("QUEUED"),
        };
        let json = serde_json::to_string(&unknown).unwrap();
        assert_eq!(json, r#""QUEUED""#);
    }

    #[test]
    fn test_file_metadata_failed_state_with_error() {
        let json = r#"{
            "name": "files/failed123",
            "mimeType": "video/mp4",
            "state": "FAILED",
            "uri": "",
            "error": {
                "code": 400,
                "message": "Unsupported video codec"
            }
        }"#;
        let file: FileMetadata = serde_json::from_str(json).unwrap();
        assert!(file.is_failed());
        assert!(file.error.is_some());

        let error = file.error.unwrap();
        assert_eq!(error.code, Some(400));
        assert_eq!(error.message.as_deref(), Some("Unsupported video codec"));
    }

    #[test]
    fn test_file_error_partial_fields() {
        // Error with only code
        let json = r#"{"code": 500}"#;
        let error: FileError = serde_json::from_str(json).unwrap();
        assert_eq!(error.code, Some(500));
        assert_eq!(error.message, None);

        // Error with only message
        let json = r#"{"message": "Something went wrong"}"#;
        let error: FileError = serde_json::from_str(json).unwrap();
        assert_eq!(error.code, None);
        assert_eq!(error.message.as_deref(), Some("Something went wrong"));

        // Empty error (edge case)
        let json = r#"{}"#;
        let error: FileError = serde_json::from_str(json).unwrap();
        assert_eq!(error.code, None);
        assert_eq!(error.message, None);
    }

    #[test]
    fn test_file_error_display() {
        // Both code and message
        let error = FileError {
            code: Some(400),
            message: Some("Invalid file format".to_string()),
        };
        assert_eq!(error.to_string(), "error 400: Invalid file format");

        // Only code
        let error = FileError {
            code: Some(500),
            message: None,
        };
        assert_eq!(error.to_string(), "error 500");

        // Only message
        let error = FileError {
            code: None,
            message: Some("Something went wrong".to_string()),
        };
        assert_eq!(error.to_string(), "Something went wrong");

        // Neither code nor message
        let error = FileError {
            code: None,
            message: None,
        };
        assert_eq!(error.to_string(), "unknown error");
    }

    #[test]
    fn test_size_bytes_as_u64() {
        // Valid size_bytes parses correctly
        let file = FileMetadata {
            name: "files/test".to_string(),
            display_name: None,
            mime_type: "video/mp4".to_string(),
            size_bytes: Some("1234567890".to_string()),
            create_time: None,
            expiration_time: None,
            sha256_hash: None,
            uri: "".to_string(),
            state: None,
            error: None,
            video_metadata: None,
        };
        assert_eq!(file.size_bytes_as_u64(), Some(1234567890));

        // None size_bytes returns None
        let file = FileMetadata {
            name: "files/test".to_string(),
            display_name: None,
            mime_type: "video/mp4".to_string(),
            size_bytes: None,
            create_time: None,
            expiration_time: None,
            sha256_hash: None,
            uri: "".to_string(),
            state: None,
            error: None,
            video_metadata: None,
        };
        assert_eq!(file.size_bytes_as_u64(), None);

        // Invalid size_bytes (non-numeric) returns None
        let file = FileMetadata {
            name: "files/test".to_string(),
            display_name: None,
            mime_type: "video/mp4".to_string(),
            size_bytes: Some("not a number".to_string()),
            create_time: None,
            expiration_time: None,
            sha256_hash: None,
            uri: "".to_string(),
            state: None,
            error: None,
            video_metadata: None,
        };
        assert_eq!(file.size_bytes_as_u64(), None);

        // Large file size (2GB+) parses correctly
        let file = FileMetadata {
            name: "files/test".to_string(),
            display_name: None,
            mime_type: "video/mp4".to_string(),
            size_bytes: Some("2147483648".to_string()), // 2GB
            create_time: None,
            expiration_time: None,
            sha256_hash: None,
            uri: "".to_string(),
            state: None,
            error: None,
            video_metadata: None,
        };
        assert_eq!(file.size_bytes_as_u64(), Some(2147483648));
    }

    // Note: Tests for upload_file validation (empty file, max size) are in
    // tests/files_api_tests.rs as integration tests since they require mocking
    // the HTTP client or hitting the real API.
}

/// Property-based tests for serialization roundtrips using proptest.
#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy for generating FileState variants.
    #[cfg(not(feature = "strict-unknown"))]
    fn arb_file_state() -> impl Strategy<Value = FileState> {
        prop_oneof![
            Just(FileState::Processing),
            Just(FileState::Active),
            Just(FileState::Failed),
            // Include Unknown variant for graceful handling
            ("[A-Z_]{4,20}".prop_map(|state_type| FileState::Unknown {
                state_type,
                data: serde_json::Value::Null,
            })),
        ]
    }

    /// Strategy for FileState - no Unknown in strict mode.
    #[cfg(feature = "strict-unknown")]
    fn arb_file_state() -> impl Strategy<Value = FileState> {
        prop_oneof![
            Just(FileState::Processing),
            Just(FileState::Active),
            Just(FileState::Failed),
        ]
    }

    /// Strategy for generating FileError.
    fn arb_file_error() -> impl Strategy<Value = FileError> {
        (
            prop::option::of(any::<i32>()),
            prop::option::of(".{0,100}".prop_map(String::from)),
        )
            .prop_map(|(code, message)| FileError { code, message })
    }

    /// Strategy for generating VideoMetadata.
    fn arb_video_metadata() -> impl Strategy<Value = VideoMetadata> {
        prop::option::of("[0-9]+s".prop_map(String::from))
            .prop_map(|video_duration| VideoMetadata { video_duration })
    }

    /// Strategy for generating FileMetadata.
    fn arb_file_metadata() -> impl Strategy<Value = FileMetadata> {
        (
            "files/[a-zA-Z0-9_]+",       // name
            prop::option::of(".{1,50}"), // display_name
            "[a-z]+/[a-z0-9+-]+",        // mime_type
            prop::option::of("[0-9]+"),  // size_bytes
            prop::option::of("[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z"), // create_time
            prop::option::of("[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z"), // expiration_time
            prop::option::of("sha256:[a-f0-9]{64}"), // sha256_hash
            "https?://[a-z]+\\.[a-z]+/[a-z]+",       // uri
            prop::option::of(arb_file_state()),      // state is Option<FileState>
            prop::option::of(arb_file_error()),
            prop::option::of(arb_video_metadata()),
        )
            .prop_map(
                |(
                    name,
                    display_name,
                    mime_type,
                    size_bytes,
                    create_time,
                    expiration_time,
                    sha256_hash,
                    uri,
                    state,
                    error,
                    video_metadata,
                )| {
                    FileMetadata {
                        name,
                        display_name,
                        mime_type,
                        size_bytes,
                        create_time,
                        expiration_time,
                        sha256_hash,
                        uri,
                        state,
                        error,
                        video_metadata,
                    }
                },
            )
    }

    proptest! {
        /// Verify FileState roundtrips through JSON serialization.
        #[test]
        fn file_state_roundtrip(state in arb_file_state()) {
            let json = serde_json::to_string(&state).expect("serialize");
            let parsed: FileState = serde_json::from_str(&json).expect("deserialize");
            // For Unknown variants, we can't do exact equality since data may be different
            // Just verify it roundtrips to a valid state
            match (&state, &parsed) {
                (FileState::Processing, FileState::Processing) => {}
                (FileState::Active, FileState::Active) => {}
                (FileState::Failed, FileState::Failed) => {}
                (FileState::Unknown { .. }, FileState::Unknown { .. }) => {}
                _ => panic!("State changed during roundtrip: {:?} -> {:?}", state, parsed),
            }
        }

        /// Verify FileError roundtrips through JSON serialization.
        #[test]
        fn file_error_roundtrip(error in arb_file_error()) {
            let json = serde_json::to_string(&error).expect("serialize");
            let parsed: FileError = serde_json::from_str(&json).expect("deserialize");
            prop_assert_eq!(error.code, parsed.code);
            prop_assert_eq!(error.message, parsed.message);
        }

        /// Verify VideoMetadata roundtrips through JSON serialization.
        #[test]
        fn video_metadata_roundtrip(metadata in arb_video_metadata()) {
            let json = serde_json::to_string(&metadata).expect("serialize");
            let parsed: VideoMetadata = serde_json::from_str(&json).expect("deserialize");
            prop_assert_eq!(metadata.video_duration, parsed.video_duration);
        }

        /// Verify FileMetadata roundtrips through JSON serialization.
        #[test]
        fn file_metadata_roundtrip(metadata in arb_file_metadata()) {
            let json = serde_json::to_string(&metadata).expect("serialize");
            let parsed: FileMetadata = serde_json::from_str(&json).expect("deserialize");

            prop_assert_eq!(&metadata.name, &parsed.name);
            prop_assert_eq!(&metadata.display_name, &parsed.display_name);
            prop_assert_eq!(&metadata.mime_type, &parsed.mime_type);
            prop_assert_eq!(&metadata.size_bytes, &parsed.size_bytes);
            prop_assert_eq!(&metadata.uri, &parsed.uri);
            // Note: state comparison is relaxed for Unknown variants
        }
    }
}
