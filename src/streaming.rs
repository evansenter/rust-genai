//! Streaming types for automatic function calling.
//!
//! This module contains types for streaming responses with automatic function execution.
//! The [`AutoFunctionStreamChunk`] enum provides events for tracking the progress of
//! an interaction that may involve multiple function call rounds.
//!
//! # Example
//!
//! ```no_run
//! use futures_util::StreamExt;
//! use rust_genai::{Client, AutoFunctionStreamChunk};
//!
//! # async fn example() -> Result<(), rust_genai::GenaiError> {
//! let client = Client::new("your-api-key".to_string());
//!
//! let mut stream = client
//!     .interaction()
//!     .with_model("gemini-3-flash-preview")
//!     .with_text("What's the weather in London?")
//!     .create_stream_with_auto_functions();
//!
//! while let Some(chunk) = stream.next().await {
//!     match chunk? {
//!         AutoFunctionStreamChunk::Delta(content) => {
//!             if let Some(t) = content.text() {
//!                 print!("{}", t);
//!             }
//!         }
//!         AutoFunctionStreamChunk::ExecutingFunctions(response) => {
//!             let calls = response.function_calls();
//!             println!("[Executing: {:?}]", calls.iter().map(|c| c.name).collect::<Vec<_>>());
//!         }
//!         AutoFunctionStreamChunk::FunctionResults(results) => {
//!             println!("[Got {} results]", results.len());
//!         }
//!         AutoFunctionStreamChunk::Complete(_response) => {
//!             println!("[Done]");
//!         }
//!         _ => {} // Handle unknown future variants
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use genai_client::models::interactions::{InteractionContent, InteractionResponse};
use serde::{Deserialize, Serialize};

/// A chunk from streaming with automatic function calling.
///
/// This enum represents the different events that can occur during a streaming
/// interaction with automatic function execution. The stream yields deltas as
/// content arrives, signals when functions are being executed, and completes
/// when the model returns a response without function calls.
///
/// # Forward Compatibility
///
/// This enum uses `#[non_exhaustive]` to allow adding new event types in future
/// versions without breaking existing code. Always include a wildcard arm in
/// match statements.
///
/// # Serialization
///
/// This enum implements `Serialize` and `Deserialize` for logging, persistence,
/// and replay of streaming events. Unknown variants deserialize to `Unknown`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "chunk_type", content = "data", rename_all = "snake_case")]
#[non_exhaustive]
pub enum AutoFunctionStreamChunk {
    /// Incremental content from the model (text, thoughts, etc.)
    Delta(InteractionContent),

    /// Function calls detected, about to execute.
    ///
    /// This event is yielded when the model requests function calls and
    /// before the functions are executed. The response contains the function
    /// calls which can be accessed via [`InteractionResponse::function_calls()`].
    ///
    /// **Note**: In streaming mode, function calls may arrive incrementally via
    /// `Delta` chunks before this event. The library accumulates these and includes
    /// them when determining which functions to execute.
    ExecutingFunctions(InteractionResponse),

    /// Function execution completed with results.
    ///
    /// This event is yielded after all functions in a batch have been executed,
    /// before sending results back to the model for the next iteration.
    FunctionResults(Vec<FunctionExecutionResult>),

    /// Final complete response (no more function calls).
    ///
    /// This is the last event in the stream, yielded when the model returns
    /// a response that doesn't request any function calls.
    Complete(InteractionResponse),

    /// Unknown event type (for forward compatibility).
    ///
    /// This variant captures any unrecognized event types that may be added
    /// in future versions, allowing deserialization to succeed gracefully.
    #[serde(other)]
    Unknown,
}

/// Result of executing a function locally.
///
/// This represents the output from a function that was executed by the library
/// during automatic function calling. It contains the function name, the call ID
/// (used to match with the original request), and the result value.
///
/// # Example
///
/// ```no_run
/// # use rust_genai::FunctionExecutionResult;
/// # let result: FunctionExecutionResult = todo!();
/// println!("Function {} returned: {}", result.name, result.result);
/// println!("  For call ID: {}", result.call_id);
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FunctionExecutionResult {
    /// Name of the function that was called
    pub name: String,
    /// The call_id from the FunctionCall this result responds to
    pub call_id: String,
    /// The result returned by the function
    pub result: serde_json::Value,
}

impl FunctionExecutionResult {
    /// Creates a new function execution result.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        call_id: impl Into<String>,
        result: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            call_id: call_id.into(),
            result,
        }
    }
}

/// Result from `create_with_auto_functions()` containing the final response
/// and a history of all function executions.
///
/// This type provides visibility into which functions were called during
/// automatic function execution, useful for debugging, logging, and evaluation.
///
/// # Example
///
/// ```no_run
/// # use rust_genai::{Client, AutoFunctionResult};
/// # async fn example() -> Result<(), rust_genai::GenaiError> {
/// # let client = Client::new("key".to_string());
/// let result = client
///     .interaction()
///     .with_model("gemini-3-flash-preview")
///     .with_text("What's the weather in London?")
///     .create_with_auto_functions()
///     .await?;
///
/// // Access the final response
/// if let Some(text) = result.response.text() {
///     println!("Answer: {}", text);
/// }
///
/// // Access execution history
/// for exec in &result.executions {
///     println!("Called {} -> {}", exec.name, exec.result);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AutoFunctionResult {
    /// The final response from the model (after all function calls completed)
    pub response: InteractionResponse,
    /// All functions that were executed during the auto-function loop
    pub executions: Vec<FunctionExecutionResult>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_function_execution_result() {
        let result = FunctionExecutionResult::new(
            "get_weather",
            "call-123",
            json!({"temp": 20, "unit": "celsius"}),
        );

        assert_eq!(result.name, "get_weather");
        assert_eq!(result.call_id, "call-123");
        assert_eq!(result.result, json!({"temp": 20, "unit": "celsius"}));
    }

    #[test]
    fn test_auto_function_stream_chunk_variants() {
        // Test that Delta and FunctionResults variants can be created
        let _delta = AutoFunctionStreamChunk::Delta(InteractionContent::Text {
            text: Some("Hello".to_string()),
        });

        let _results = AutoFunctionStreamChunk::FunctionResults(vec![FunctionExecutionResult {
            name: "test".to_string(),
            call_id: "1".to_string(),
            result: json!({"ok": true}),
        }]);

        // Note: ExecutingFunctions and Complete require InteractionResponse which is harder to construct in tests
    }

    #[test]
    fn test_function_execution_result_serialization() {
        let result = FunctionExecutionResult::new(
            "get_weather",
            "call-456",
            json!({"temp": 22, "conditions": "sunny"}),
        );

        let json_str = serde_json::to_string(&result).expect("Serialization should succeed");

        // Verify key fields are present in serialized output
        assert!(
            json_str.contains("get_weather"),
            "Should contain function name"
        );
        assert!(json_str.contains("call-456"), "Should contain call_id");
        assert!(json_str.contains("sunny"), "Should contain result data");

        // Verify full roundtrip
        let deserialized: FunctionExecutionResult =
            serde_json::from_str(&json_str).expect("Deserialization should succeed");
        assert_eq!(deserialized, result);
    }

    #[test]
    fn test_auto_function_stream_chunk_serialization_roundtrip() {
        // Test Delta variant roundtrip
        let delta = AutoFunctionStreamChunk::Delta(InteractionContent::Text {
            text: Some("Hello, world!".to_string()),
        });

        let json_str = serde_json::to_string(&delta).expect("Serialization should succeed");
        assert!(json_str.contains("chunk_type"), "Should contain tag field");
        assert!(json_str.contains("delta"), "Should contain variant name");
        assert!(json_str.contains("Hello, world!"), "Should contain text");

        let deserialized: AutoFunctionStreamChunk =
            serde_json::from_str(&json_str).expect("Deserialization should succeed");

        match deserialized {
            AutoFunctionStreamChunk::Delta(content) => {
                assert_eq!(content.text(), Some("Hello, world!"));
            }
            _ => panic!("Expected Delta variant"),
        }

        // Test FunctionResults variant roundtrip
        let results = AutoFunctionStreamChunk::FunctionResults(vec![
            FunctionExecutionResult::new("get_weather", "call-1", json!({"temp": 20})),
            FunctionExecutionResult::new("get_time", "call-2", json!({"time": "14:30"})),
        ]);

        let json_str = serde_json::to_string(&results).expect("Serialization should succeed");
        let deserialized: AutoFunctionStreamChunk =
            serde_json::from_str(&json_str).expect("Deserialization should succeed");

        match deserialized {
            AutoFunctionStreamChunk::FunctionResults(execs) => {
                assert_eq!(execs.len(), 2);
                assert_eq!(execs[0].name, "get_weather");
                assert_eq!(execs[1].name, "get_time");
            }
            _ => panic!("Expected FunctionResults variant"),
        }

        // Test Unknown variant handling (forward compatibility)
        let unknown_json = r#"{"chunk_type": "future_event_type"}"#;
        let deserialized: AutoFunctionStreamChunk =
            serde_json::from_str(unknown_json).expect("Should deserialize unknown variant");
        assert!(matches!(deserialized, AutoFunctionStreamChunk::Unknown));
    }
}
