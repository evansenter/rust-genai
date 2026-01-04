//! Wire-level debugging via LOUD_WIRE environment variable.
//!
//! When `LOUD_WIRE` is set to any value, prints raw JSON of API requests and
//! responses to stderr with pretty formatting and colors.
//!
//! # Usage
//!
//! ```bash
//! LOUD_WIRE=1 cargo run --example simple_interaction
//! ```
//!
//! # Output Format
//!
//! - Green `>>>` for outgoing requests
//! - Red `<<<` for incoming responses
//! - Blue for SSE streaming chunks
//! - Timestamps and request IDs for correlation
//! - Request IDs use alternating colors (even/odd) for visual distinction:
//!   - `[REQ#N]`: Green (even) / Yellow (odd)
//!   - `[RES#N]`: Magenta (even) / Cyan (odd)
//!
//! Base64-encoded media content is truncated to keep output readable.

use chrono::Utc;
use colored::Colorize;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Request ID counter for correlating requests with responses
static REQUEST_COUNTER: AtomicUsize = AtomicUsize::new(1);

/// Cached check for whether LOUD_WIRE is enabled
static ENABLED: OnceLock<bool> = OnceLock::new();

/// Check if LOUD_WIRE debugging is enabled.
///
/// The result is cached after first check for performance. This means
/// `LOUD_WIRE` must be set before the first API call is made - setting
/// it after the first call will have no effect.
#[must_use]
pub fn is_enabled() -> bool {
    *ENABLED.get_or_init(|| std::env::var("LOUD_WIRE").is_ok())
}

/// Get the next request ID for correlation.
#[must_use]
pub fn next_request_id() -> usize {
    REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Fields that should have their values truncated if too long.
/// These typically contain base64-encoded binary data.
const TRUNCATE_FIELDS: &[&str] = &["data", "signature"];

/// Maximum length before truncation (keep first 100 chars).
const TRUNCATE_THRESHOLD: usize = 100;

/// Truncate long base64-encoded fields in a JSON value.
///
/// Walks the JSON tree and truncates `"data"` and `"signature"` fields
/// that contain strings longer than 100 characters. Text content and
/// other fields are preserved in full.
fn truncate_long_fields(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if TRUNCATE_FIELDS.contains(&key.as_str()) {
                    if let serde_json::Value::String(s) = val
                        && s.len() > TRUNCATE_THRESHOLD
                    {
                        *s = format!("{}...", &s[..TRUNCATE_THRESHOLD]);
                    }
                } else {
                    truncate_long_fields(val);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                truncate_long_fields(item);
            }
        }
        _ => {}
    }
}

/// Colorize and format JSON for terminal output.
/// Returns lines ready to print, or None if colorization fails.
fn colorize_json(value: &serde_json::Value) -> Option<String> {
    colored_json::to_colored_json_auto(value).ok()
}

/// Format the current timestamp for log output (ISO 8601 UTC).
fn timestamp() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Log prefix with timestamp and request ID (for outgoing requests).
/// Colors alternate: green (even) / yellow (odd) for visual distinction.
fn request_prefix(request_id: usize) -> String {
    let ts = timestamp().dimmed();
    let req_label = format!("[REQ#{}]", request_id);
    let colored_label = if request_id.is_multiple_of(2) {
        req_label.green().bold()
    } else {
        req_label.yellow().bold()
    };
    format!("{} {} {}", "[LOUD_WIRE]".bold(), ts, colored_label)
}

/// Log prefix with timestamp and response ID (for incoming responses).
/// Colors alternate: magenta (even) / cyan (odd) for visual distinction.
fn response_prefix(request_id: usize) -> String {
    let ts = timestamp().dimmed();
    let res_label = format!("[RES#{}]", request_id);
    let colored_label = if request_id.is_multiple_of(2) {
        res_label.magenta().bold()
    } else {
        res_label.cyan().bold()
    };
    format!("{} {} {}", "[LOUD_WIRE]".bold(), ts, colored_label)
}

/// Log an outgoing HTTP request.
pub fn log_request(request_id: usize, method: &str, url: &str, body: Option<&str>) {
    if !is_enabled() {
        return;
    }

    let prefix = request_prefix(request_id);
    let direction = ">>>".green().bold();

    eprintln!("{prefix} {direction} {method} {url}");

    if let Some(body) = body {
        if let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(body) {
            truncate_long_fields(&mut parsed);
            eprintln!("{prefix} {}:", "Body".green());
            if let Some(colored) = colorize_json(&parsed) {
                for line in colored.lines() {
                    eprintln!("{prefix} {line}");
                }
            } else if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
                for line in pretty.lines() {
                    eprintln!("{prefix} {line}");
                }
            }
        } else {
            // Not valid JSON, print as-is (truncated for safety)
            let truncated = if body.len() > 500 {
                format!("{}...", &body[..500])
            } else {
                body.to_string()
            };
            eprintln!("{prefix} {}: {truncated}", "Body".green());
        }
    }
}

/// Log an incoming HTTP response status.
pub fn log_response_status(request_id: usize, status: u16) {
    if !is_enabled() {
        return;
    }

    let prefix = response_prefix(request_id);
    let direction = "<<<".red().bold();
    let status_text = if status < 300 {
        format!("{status} OK").green()
    } else {
        format!("{status} ERROR").red()
    };

    eprintln!("{prefix} {direction} {status_text}");
}

/// Log an incoming HTTP response body.
pub fn log_response_body(request_id: usize, body: &str) {
    if !is_enabled() {
        return;
    }

    let prefix = response_prefix(request_id);

    if let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(body) {
        truncate_long_fields(&mut parsed);
        eprintln!("{prefix} {}:", "Response".red());
        if let Some(colored) = colorize_json(&parsed) {
            for line in colored.lines() {
                eprintln!("{prefix} {line}");
            }
        } else if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
            for line in pretty.lines() {
                eprintln!("{prefix} {line}");
            }
        }
    } else {
        // Not valid JSON, print as-is (truncated for safety)
        let truncated = if body.len() > 1000 {
            format!("{}...", &body[..1000])
        } else {
            body.to_string()
        };
        eprintln!("{prefix} {}: {truncated}", "Response".red());
    }
}

/// Log an SSE streaming chunk.
pub fn log_sse_chunk(request_id: usize, raw_json: &str) {
    if !is_enabled() {
        return;
    }

    let prefix = response_prefix(request_id);
    let label = "SSE".blue().bold();

    if let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(raw_json) {
        truncate_long_fields(&mut parsed);
        eprintln!("{prefix} {label}:");
        if let Some(colored) = colorize_json(&parsed) {
            for line in colored.lines() {
                eprintln!("{prefix} {line}");
            }
        } else if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
            for line in pretty.lines() {
                eprintln!("{prefix} {line}");
            }
        }
    } else {
        eprintln!("{prefix} {label}: {raw_json}");
    }
}

/// Log file upload progress.
pub fn log_upload_start(request_id: usize, file_name: &str, mime_type: &str, size: u64) {
    if !is_enabled() {
        return;
    }

    let prefix = request_prefix(request_id);
    let direction = ">>>".green().bold();
    let size_mb = size as f64 / 1_048_576.0;

    eprintln!(
        "{prefix} {direction} {} \"{file_name}\" ({mime_type}, {size_mb:.2} MB)",
        "UPLOAD".green().bold()
    );
}

/// Log file upload completion.
pub fn log_upload_complete(request_id: usize, file_uri: &str) {
    if !is_enabled() {
        return;
    }

    let prefix = response_prefix(request_id);
    let direction = "<<<".red().bold();

    eprintln!(
        "{prefix} {direction} {} {file_uri}",
        "UPLOADED".green().bold()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_data() {
        let mut value = serde_json::json!({"data": "short"});
        truncate_long_fields(&mut value);
        assert_eq!(value["data"], "short", "Short data should not be truncated");
    }

    #[test]
    fn test_truncate_long_data() {
        let long_base64 = "A".repeat(200);
        let mut value = serde_json::json!({"data": long_base64});
        truncate_long_fields(&mut value);

        let result = value["data"].as_str().unwrap();
        assert!(result.ends_with("..."), "Long data should be truncated");
        assert_eq!(result.len(), 103, "Should be 100 chars + '...'");
        assert!(
            result.starts_with(&"A".repeat(100)),
            "Should preserve first 100 chars"
        );
    }

    #[test]
    fn test_truncate_signature_field() {
        let long_sig = "B".repeat(200);
        let mut value = serde_json::json!({"signature": long_sig});
        truncate_long_fields(&mut value);

        let result = value["signature"].as_str().unwrap();
        assert!(
            result.ends_with("..."),
            "Long signature should be truncated"
        );
    }

    #[test]
    fn test_truncate_preserves_text() {
        let long_text = "Hello world! ".repeat(50);
        let mut value = serde_json::json!({
            "text": long_text.clone(),
            "data": "A".repeat(200)
        });
        truncate_long_fields(&mut value);

        assert_eq!(
            value["text"], long_text,
            "Text field should not be truncated"
        );
        assert!(
            value["data"].as_str().unwrap().ends_with("..."),
            "Data should be truncated"
        );
    }

    #[test]
    fn test_truncate_nested_structure() {
        let long_base64 = "C".repeat(150);
        let mut value = serde_json::json!({
            "model": "gemini",
            "content": {"data": long_base64},
            "other": "value"
        });
        truncate_long_fields(&mut value);

        assert_eq!(value["model"], "gemini");
        assert_eq!(value["other"], "value");
        assert!(value["content"]["data"].as_str().unwrap().ends_with("..."));
    }

    #[test]
    fn test_timestamp_format() {
        let ts = timestamp();
        // Should match ISO 8601 format: YYYY-MM-DDTHH:MM:SSZ
        assert!(ts.len() == 20, "Timestamp should be 20 chars: {ts}");
        assert!(ts.ends_with('Z'), "Should end with Z");
        assert!(ts.contains('T'), "Should contain T separator");
    }

    #[test]
    fn test_request_id_increments() {
        let id1 = next_request_id();
        let id2 = next_request_id();
        assert!(id2 > id1, "Request IDs should increment");
    }
}
