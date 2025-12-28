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

use std::time::Duration;

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
    /// This variant is used when deserializing JSON that contains an unrecognized
    /// `chunk_type`. This allows the library to gracefully handle new event types
    /// added by the API in future versions without failing deserialization.
    ///
    /// When encountering an `Unknown` variant, code should typically log a warning
    /// and continue processing, as the stream may still contain useful events.
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
/// println!("  Call ID: {}, Duration: {:?}", result.call_id, result.duration);
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
    /// How long the function took to execute
    #[serde(with = "duration_millis")]
    pub duration: Duration,
}

impl FunctionExecutionResult {
    /// Creates a new function execution result.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        call_id: impl Into<String>,
        result: serde_json::Value,
        duration: Duration,
    ) -> Self {
        Self {
            name: name.into(),
            call_id: call_id.into(),
            result,
            duration,
        }
    }
}

/// Serialize Duration as milliseconds for JSON compatibility
mod duration_millis {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
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

/// Accumulator for building [`AutoFunctionResult`] from a stream of [`AutoFunctionStreamChunk`].
///
/// This helper collects all function execution results and the final response from
/// a streaming auto-function interaction, producing the same result type as the
/// non-streaming `create_with_auto_functions()` method.
///
/// # Example
///
/// ```no_run
/// use futures_util::StreamExt;
/// use rust_genai::{Client, AutoFunctionStreamChunk, AutoFunctionResultAccumulator};
///
/// # async fn example() -> Result<(), rust_genai::GenaiError> {
/// let client = Client::new("your-api-key".to_string());
///
/// let mut stream = client
///     .interaction()
///     .with_model("gemini-3-flash-preview")
///     .with_text("What's the weather in London?")
///     .create_stream_with_auto_functions();
///
/// let mut accumulator = AutoFunctionResultAccumulator::new();
///
/// while let Some(chunk) = stream.next().await {
///     let chunk = chunk?;
///
///     // Process deltas for UI updates
///     if let AutoFunctionStreamChunk::Delta(content) = &chunk {
///         if let Some(text) = content.text() {
///             print!("{}", text);
///         }
///     }
///
///     // Feed all chunks to the accumulator
///     if let Some(result) = accumulator.push(chunk) {
///         // Stream is complete, we have the full result
///         println!("\n\nExecuted {} functions", result.executions.len());
///         for exec in &result.executions {
///             println!("  {} took {:?}", exec.name, exec.duration);
///         }
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Default)]
pub struct AutoFunctionResultAccumulator {
    executions: Vec<FunctionExecutionResult>,
}

impl AutoFunctionResultAccumulator {
    /// Creates a new empty accumulator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds a chunk to the accumulator.
    ///
    /// Returns `Some(AutoFunctionResult)` when the stream is complete (i.e., when
    /// a `Complete` chunk is received). Returns `None` for all other chunk types.
    ///
    /// The accumulator collects all `FunctionResults` chunks and combines them
    /// with the final `Complete` response.
    #[allow(unreachable_patterns)] // Handle future variants from #[non_exhaustive] enum
    pub fn push(&mut self, chunk: AutoFunctionStreamChunk) -> Option<AutoFunctionResult> {
        match chunk {
            AutoFunctionStreamChunk::FunctionResults(results) => {
                self.executions.extend(results);
                None
            }
            AutoFunctionStreamChunk::Complete(response) => Some(AutoFunctionResult {
                response,
                executions: std::mem::take(&mut self.executions),
            }),
            AutoFunctionStreamChunk::Delta(_) | AutoFunctionStreamChunk::ExecutingFunctions(_) => {
                None
            }
            // Handle future variants gracefully
            _ => None,
        }
    }

    /// Returns the accumulated executions so far.
    ///
    /// Useful for checking progress without consuming the accumulator.
    #[must_use]
    pub fn executions(&self) -> &[FunctionExecutionResult] {
        &self.executions
    }

    /// Resets the accumulator to its initial empty state.
    pub fn reset(&mut self) {
        self.executions.clear();
    }
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
            Duration::from_millis(42),
        );

        assert_eq!(result.name, "get_weather");
        assert_eq!(result.call_id, "call-123");
        assert_eq!(result.result, json!({"temp": 20, "unit": "celsius"}));
        assert_eq!(result.duration, Duration::from_millis(42));
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
            duration: Duration::from_millis(10),
        }]);

        // Note: ExecutingFunctions and Complete require InteractionResponse which is harder to construct in tests
    }

    #[test]
    fn test_function_execution_result_serialization() {
        let result = FunctionExecutionResult::new(
            "get_weather",
            "call-456",
            json!({"temp": 22, "conditions": "sunny"}),
            Duration::from_millis(150),
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
            FunctionExecutionResult::new(
                "get_weather",
                "call-1",
                json!({"temp": 20}),
                Duration::from_millis(50),
            ),
            FunctionExecutionResult::new(
                "get_time",
                "call-2",
                json!({"time": "14:30"}),
                Duration::from_millis(30),
            ),
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

    #[test]
    fn test_auto_function_result_roundtrip() {
        use genai_client::InteractionStatus;

        // Create a realistic AutoFunctionResult with multiple executions
        let result = AutoFunctionResult {
            response: genai_client::InteractionResponse {
                id: "interaction-abc123".to_string(),
                model: Some("gemini-3-flash-preview".to_string()),
                agent: None,
                input: vec![InteractionContent::Text {
                    text: Some("What's the weather in Paris and London?".to_string()),
                }],
                outputs: vec![
                    InteractionContent::Text {
                        text: Some("Based on the weather data:".to_string()),
                    },
                    InteractionContent::Text {
                        text: Some("Paris is 18°C and London is 15°C.".to_string()),
                    },
                ],
                status: InteractionStatus::Completed,
                usage: Some(genai_client::UsageMetadata {
                    total_input_tokens: Some(50),
                    total_output_tokens: Some(30),
                    total_tokens: Some(80),
                    ..Default::default()
                }),
                tools: None,
                grounding_metadata: None,
                url_context_metadata: None,
                previous_interaction_id: Some("prev-interaction-xyz".to_string()),
            },
            executions: vec![
                FunctionExecutionResult::new(
                    "get_weather",
                    "call-001",
                    json!({"city": "Paris", "temp": 18, "unit": "celsius"}),
                    Duration::from_millis(120),
                ),
                FunctionExecutionResult::new(
                    "get_weather",
                    "call-002",
                    json!({"city": "London", "temp": 15, "unit": "celsius"}),
                    Duration::from_millis(95),
                ),
            ],
        };

        // Serialize
        let json_str = serde_json::to_string(&result).expect("Serialization should succeed");

        // Verify key data is present in JSON
        assert!(
            json_str.contains("interaction-abc123"),
            "Should contain interaction ID"
        );
        assert!(
            json_str.contains("gemini-3-flash-preview"),
            "Should contain model name"
        );
        assert!(
            json_str.contains("get_weather"),
            "Should contain function name"
        );
        assert!(
            json_str.contains("call-001"),
            "Should contain first call_id"
        );
        assert!(
            json_str.contains("call-002"),
            "Should contain second call_id"
        );
        assert!(json_str.contains("Paris"), "Should contain Paris");
        assert!(json_str.contains("London"), "Should contain London");
        assert!(
            json_str.contains("prev-interaction-xyz"),
            "Should contain previous interaction ID"
        );

        // Deserialize
        let deserialized: AutoFunctionResult =
            serde_json::from_str(&json_str).expect("Deserialization should succeed");

        // Verify response fields
        assert_eq!(deserialized.response.id, "interaction-abc123");
        assert_eq!(
            deserialized.response.model,
            Some("gemini-3-flash-preview".to_string())
        );
        assert_eq!(deserialized.response.status, InteractionStatus::Completed);
        assert_eq!(
            deserialized.response.previous_interaction_id,
            Some("prev-interaction-xyz".to_string())
        );

        // Verify usage metadata
        let usage = deserialized.response.usage.expect("Should have usage");
        assert_eq!(usage.total_input_tokens, Some(50));
        assert_eq!(usage.total_output_tokens, Some(30));
        assert_eq!(usage.total_tokens, Some(80));

        // Verify executions
        assert_eq!(deserialized.executions.len(), 2);
        assert_eq!(deserialized.executions[0].name, "get_weather");
        assert_eq!(deserialized.executions[0].call_id, "call-001");
        assert_eq!(deserialized.executions[0].result["city"], "Paris");
        assert_eq!(deserialized.executions[1].name, "get_weather");
        assert_eq!(deserialized.executions[1].call_id, "call-002");
        assert_eq!(deserialized.executions[1].result["city"], "London");
    }
}
