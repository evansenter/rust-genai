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
//!
//! Base64-encoded media content is truncated to keep output readable.

use colored::Colorize;
use regex::Regex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Request ID counter for correlating requests with responses
static REQUEST_COUNTER: AtomicUsize = AtomicUsize::new(1);

/// Cached check for whether LOUD_WIRE is enabled
static ENABLED: OnceLock<bool> = OnceLock::new();

/// Check if LOUD_WIRE debugging is enabled.
///
/// The result is cached after first check for performance.
#[must_use]
pub fn is_enabled() -> bool {
    *ENABLED.get_or_init(|| std::env::var("LOUD_WIRE").is_ok())
}

/// Get the next request ID for correlation.
#[must_use]
pub fn next_request_id() -> usize {
    REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Truncate base64-encoded data fields in JSON to keep output readable.
///
/// Finds patterns like `"data": "base64..."` and truncates the value to
/// the first 100 characters followed by "...".
#[must_use]
pub fn truncate_base64(json: &str) -> String {
    // Match "data" fields with long base64-like values
    // Pattern: "data" : "AAAA..." where value is >100 chars of base64 alphabet
    static REGEX: OnceLock<Regex> = OnceLock::new();
    let re = REGEX.get_or_init(|| {
        Regex::new(r#"("data"\s*:\s*")([A-Za-z0-9+/=]{100})([A-Za-z0-9+/=]+)""#)
            .expect("Invalid regex")
    });

    re.replace_all(json, r#"$1$2...""#).into_owned()
}

/// Format the current timestamp for log output.
fn timestamp() -> String {
    // Use a simple format that works without chrono
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();

    // Convert to human-readable format (simplified UTC)
    let days_since_epoch = secs / 86400;
    let secs_today = secs % 86400;
    let hours = secs_today / 3600;
    let minutes = (secs_today % 3600) / 60;
    let seconds = secs_today % 60;

    // Calculate year/month/day from days since epoch (1970-01-01)
    // This is a simplified calculation that works for dates after 2000
    let mut remaining_days = days_since_epoch as i64;
    let mut year = 1970;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let (month, day) = day_of_year_to_month_day(remaining_days as u32 + 1, is_leap_year(year));

    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn day_of_year_to_month_day(day_of_year: u32, leap: bool) -> (u32, u32) {
    let days_in_months: [u32; 12] = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut remaining = day_of_year;
    for (i, &days) in days_in_months.iter().enumerate() {
        if remaining <= days {
            return (i as u32 + 1, remaining);
        }
        remaining -= days;
    }
    (12, 31) // Fallback
}

/// Log prefix with timestamp and request ID.
fn prefix(request_id: usize) -> String {
    let ts = timestamp().dimmed();
    format!(
        "{} {} {}",
        "[LOUD_WIRE]".bold(),
        ts,
        format!("[REQ#{}]", request_id).cyan()
    )
}

/// Log an outgoing HTTP request.
pub fn log_request(request_id: usize, method: &str, url: &str, body: Option<&str>) {
    if !is_enabled() {
        return;
    }

    let prefix = prefix(request_id);
    let direction = ">>>".green().bold();

    eprintln!("{prefix} {direction} {method} {url}");

    if let Some(body) = body {
        // Pretty-print and truncate base64
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(body) {
            if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
                let truncated = truncate_base64(&pretty);
                eprintln!("{prefix} {}:", "Body".green());
                for line in truncated.lines() {
                    eprintln!("{prefix} {line}");
                }
            } else {
                // JSON parsed but pretty-print failed - use compact form
                let truncated = truncate_base64(body);
                eprintln!("{prefix} {}: {truncated}", "Body".green());
            }
        } else {
            // Not valid JSON, print as-is (truncated)
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

    let prefix = prefix(request_id);
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

    let prefix = prefix(request_id);

    // Pretty-print and truncate base64
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(body) {
        if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
            let truncated = truncate_base64(&pretty);
            eprintln!("{prefix} {}:", "Response".red());
            for line in truncated.lines() {
                eprintln!("{prefix} {line}");
            }
        } else {
            // JSON parsed but pretty-print failed - use compact form
            let truncated = truncate_base64(body);
            eprintln!("{prefix} {}: {truncated}", "Response".red());
        }
    } else {
        // Not valid JSON, print as-is (truncated)
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

    let prefix = prefix(request_id);
    let label = "SSE".blue().bold();

    // Pretty-print and truncate base64
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw_json) {
        if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
            let truncated = truncate_base64(&pretty);
            eprintln!("{prefix} {label}:");
            for line in truncated.lines() {
                eprintln!("{prefix} {line}");
            }
        } else {
            // JSON parsed but pretty-print failed - use compact form
            let truncated = truncate_base64(raw_json);
            eprintln!("{prefix} {label}: {truncated}");
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

    let prefix = prefix(request_id);
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

    let prefix = prefix(request_id);
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
    fn test_truncate_base64_short_data() {
        let json = r#"{"data": "short"}"#;
        let result = truncate_base64(json);
        assert_eq!(result, json, "Short data should not be truncated");
    }

    #[test]
    fn test_truncate_base64_long_data() {
        // Create a string with >100 chars of base64-like content
        let long_base64 = "A".repeat(200);
        let json = format!(r#"{{"data": "{long_base64}"}}"#);
        let result = truncate_base64(&json);

        assert!(result.contains("..."), "Long data should be truncated");
        assert!(
            result.len() < json.len(),
            "Result should be shorter than input"
        );
        // Should keep first 100 chars
        assert!(
            result.contains(&"A".repeat(100)),
            "Should preserve first 100 chars"
        );
    }

    #[test]
    fn test_truncate_base64_preserves_structure() {
        let long_base64 = "B".repeat(150);
        let json = format!(
            r#"{{"model": "gemini", "content": {{"data": "{long_base64}"}}, "other": "value"}}"#
        );
        let result = truncate_base64(&json);

        assert!(result.contains(r#""model": "gemini""#));
        assert!(result.contains(r#""other": "value""#));
        assert!(result.contains("..."));
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
