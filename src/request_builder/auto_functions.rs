//! Automatic function calling logic for InteractionBuilder.
//!
//! This module contains the `create_with_auto_functions()` and
//! `create_stream_with_auto_functions()` methods that handle
//! automatic function discovery, execution, and multi-turn orchestration.

use std::time::Instant;

use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use genai_client::{InteractionInput, InteractionResponse, StreamChunk};
use log::{error, warn};
use serde_json::json;

use crate::GenaiError;
use crate::function_calling::get_global_function_registry;
use crate::interactions_api::function_result_content;
use crate::streaming::{AutoFunctionResult, AutoFunctionStreamChunk, FunctionExecutionResult};

use super::InteractionBuilder;

/// Default maximum iterations for auto function calling.
pub(crate) const DEFAULT_MAX_FUNCTION_CALL_LOOPS: usize = 5;

/// Validates that a function call has a call_id and returns it.
fn validate_call_id(call_id: Option<&str>, function_name: &str) -> Result<String, GenaiError> {
    call_id
        .ok_or_else(|| {
            error!(
                "Function call '{}' is missing required call_id field.",
                function_name
            );
            GenaiError::InvalidInput(format!(
                "Function call '{}' is missing required call_id field. \
                 This may indicate an API response format change.",
                function_name
            ))
        })
        .map(|id| id.to_string())
}

impl<'a> InteractionBuilder<'a> {
    /// Creates interaction with automatic function call handling.
    ///
    /// This method implements the auto-function execution loop:
    /// 1. Send initial input to model with available tools
    /// 2. If response contains function calls, execute them
    /// 3. Send function results back to model in new interaction
    /// 4. Repeat until model returns text or max iterations reached
    ///
    /// Functions are auto-discovered from the global registry (via `#[tool]` macro)
    /// or can be explicitly provided via `.with_function()` or `.with_tools()`.
    ///
    /// The loop automatically stops when:
    /// - Model returns text without function calls
    /// - Function calls array is empty
    /// - Maximum iterations is reached (default 5, configurable via `with_max_function_call_loops()`)
    ///
    /// # Thought Signatures
    ///
    /// For Gemini 3 models, thought signatures are required to maintain reasoning context
    /// across function calling turns. This method uses `previous_interaction_id` to link
    /// turns, which allows the server to manage thought signatures automatically.
    ///
    /// See <https://ai.google.dev/gemini-api/docs/thought-signatures> for more details.
    ///
    /// # Example
    /// ```no_run
    /// # use rust_genai::{Client, FunctionDeclaration};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder("api_key".to_string()).build();
    ///
    /// // Functions are auto-discovered from registry
    /// let result = client.interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("What's the weather in Tokyo?")
    ///     .create_with_auto_functions()
    ///     .await?;
    ///
    /// // Access the final response
    /// println!("{}", result.response.text().unwrap_or("No text"));
    ///
    /// // Access execution history
    /// for exec in &result.executions {
    ///     println!("Called {} -> {}", exec.name, exec.result);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Serialization
    ///
    /// Both [`AutoFunctionResult`] and its contained [`InteractionResponse`]
    /// implement `Serialize`, enabling logging, caching, and persistence of complete
    /// execution histories for debugging and evaluation workflows.
    ///
    /// # Max Loops Behavior
    ///
    /// When the maximum number of iterations is reached (default 5, configurable via
    /// `with_max_function_call_loops()`), the method returns an `Ok` result with
    /// `reached_max_loops: true` instead of an error. This preserves the execution
    /// history and the last response for debugging stuck loops.
    ///
    /// ```no_run
    /// # use rust_genai::Client;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Client::new("key".to_string());
    /// let result = client.interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("...")
    ///     .with_max_function_call_loops(3)
    ///     .create_with_auto_functions()
    ///     .await?;
    ///
    /// if result.reached_max_loops {
    ///     eprintln!("Hit max loops! Executed {} functions", result.executions.len());
    ///     // Inspect result.response.function_calls() to see what's still pending
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No input was provided
    /// - Neither model nor agent was specified
    /// - The API request fails
    /// - `max_function_call_loops` is set to 0 (invalid configuration)
    pub async fn create_with_auto_functions(self) -> Result<AutoFunctionResult, GenaiError> {
        let client = self.client;
        let max_loops = self.max_function_call_loops;
        let mut request = self.build_request()?;

        // Track all function executions for the result
        let mut all_executions: Vec<FunctionExecutionResult> = Vec::new();

        // Auto-discover functions from registry if not explicitly provided
        let function_registry = get_global_function_registry();
        if request.tools.is_none() {
            let auto_discovered_declarations = function_registry.all_declarations();
            if !auto_discovered_declarations.is_empty() {
                request.tools = Some(
                    auto_discovered_declarations
                        .into_iter()
                        .map(|decl| decl.into_tool())
                        .collect(),
                );
            }
        }

        // Track the last response for returning partial results if max loops is reached
        let mut last_response: Option<InteractionResponse> = None;

        // Main auto-function loop (configurable iterations to prevent infinite loops)
        for _loop_count in 0..max_loops {
            let response = client.create_interaction(request.clone()).await?;

            // Extract function calls using convenience method
            let function_calls = response.function_calls();

            // If no function calls, we're done!
            if function_calls.is_empty() {
                return Ok(AutoFunctionResult {
                    response,
                    executions: all_executions,
                    reached_max_loops: false,
                });
            }

            // Build function results for next iteration
            let mut function_results = Vec::new();

            for call in function_calls {
                // Validate that we have a call_id (required by API)
                let call_id = validate_call_id(call.id, call.name)?;

                // Execute the function with timing
                let start = Instant::now();
                // Design decision: Errors are converted to JSON and sent to the model rather than
                // failing the entire interaction. This allows the model to recover gracefully -
                // e.g., "I couldn't get the weather, let me try something else." The error is
                // logged for debugging, but the conversation continues. For strict error handling,
                // use manual function calling via `create()` instead of `create_with_auto_functions()`.
                let result = if let Some(function) = function_registry.get(call.name) {
                    match function.call(call.args.clone()).await {
                        Ok(result) => result,
                        Err(e) => {
                            error!(
                                "Function execution failed: function='{}', error='{}'",
                                call.name, e
                            );
                            json!({ "error": e.to_string() })
                        }
                    }
                } else {
                    // Function not in registry - could be a typo in declarations or missing #[tool] macro.
                    // We inform the model rather than failing, allowing it to adapt or use other functions.
                    warn!(
                        "Function not found in registry: function='{}'. Informing model.",
                        call.name
                    );
                    json!({ "error": format!("Function '{}' is not available or not found.", call.name) })
                };
                let duration = start.elapsed();

                // Track execution for the result
                all_executions.push(FunctionExecutionResult::new(
                    call.name,
                    &call_id,
                    result.clone(),
                    duration,
                ));

                // Add function result (only the result, not the call - server has it via previous_interaction_id)
                function_results.push(function_result_content(
                    call.name.to_string(),
                    call_id,
                    result,
                ));
            }

            // Save this response before moving to next iteration
            // (in case we hit max loops, we want to return the last response)
            last_response = Some(response.clone());

            // Create new request with function results
            // The server maintains function call context via previous_interaction_id
            request.previous_interaction_id = Some(response.id);
            request.input = InteractionInput::Content(function_results);
        }

        // Max loops reached - return partial result with whatever we have
        // This preserves execution history for debugging instead of discarding it
        warn!(
            "Reached maximum function call loops ({max_loops}). \
             Returning partial result with {} executions. \
             The model may be stuck in a loop.",
            all_executions.len()
        );

        // If we never made it through even one iteration (shouldn't happen with max_loops > 0),
        // return an error since we have no response to return
        let response = last_response.ok_or_else(|| {
            GenaiError::InvalidInput(format!(
                "max_function_call_loops ({max_loops}) must be at least 1"
            ))
        })?;

        Ok(AutoFunctionResult {
            response,
            executions: all_executions,
            reached_max_loops: true,
        })
    }

    /// Creates a streaming interaction with automatic function call handling.
    ///
    /// This method combines the streaming capabilities of `create_stream()` with the
    /// automatic function execution of `create_with_auto_functions()`. It yields
    /// [`AutoFunctionStreamChunk`] events that include:
    ///
    /// - `Delta`: Incremental content from the model (text, thoughts, etc.)
    /// - `ExecutingFunctions`: Notification when function calls are about to execute
    /// - `FunctionResults`: Results from executed functions
    /// - `Complete`: Final response when no more function calls are needed
    ///
    /// The stream automatically handles multiple function calling rounds, streaming
    /// content from each round and executing functions between rounds.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rust_genai::{Client, AutoFunctionStreamChunk, InteractionContent};
    /// # use futures_util::StreamExt;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder("api_key".to_string()).build();
    ///
    /// let mut stream = client.interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("What's the weather in Tokyo?")
    ///     .create_stream_with_auto_functions();
    ///
    /// while let Some(chunk) = stream.next().await {
    ///     match chunk? {
    ///         AutoFunctionStreamChunk::Delta(content) => {
    ///             if let InteractionContent::Text { text: Some(t) } = content {
    ///                 print!("{}", t);
    ///             }
    ///         }
    ///         AutoFunctionStreamChunk::ExecutingFunctions(response) => {
    ///             let names: Vec<_> = response.function_calls().iter().map(|c| c.name).collect();
    ///             println!("[Executing: {:?}]", names);
    ///         }
    ///         AutoFunctionStreamChunk::FunctionResults(results) => {
    ///             println!("[Got {} results]", results.len());
    ///         }
    ///         AutoFunctionStreamChunk::Complete(response) => {
    ///             println!("\n[Complete: {} tokens]", response.usage.as_ref()
    ///                 .and_then(|u| u.total_tokens).unwrap_or(0));
    ///         }
    ///         _ => {} // Handle unknown future variants
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Max Loops Behavior
    ///
    /// When the maximum number of iterations is reached, the stream yields a
    /// `MaxLoopsReached(response)` chunk instead of returning an error. This
    /// preserves access to prior `FunctionResults` chunks that were already yielded.
    ///
    /// The `AutoFunctionResultAccumulator` handles `MaxLoopsReached` automatically
    /// and returns an `AutoFunctionResult` with `reached_max_loops: true`.
    ///
    /// # Errors
    ///
    /// Returns errors if:
    /// - No input was provided
    /// - Neither model nor agent was specified
    /// - The API request fails
    /// - A function call is missing its required `call_id` field
    /// - `max_function_call_loops` is set to 0 (invalid configuration)
    pub fn create_stream_with_auto_functions(
        self,
    ) -> BoxStream<'a, Result<AutoFunctionStreamChunk, GenaiError>> {
        let client = self.client;
        let max_loops = self.max_function_call_loops;

        Box::pin(async_stream::try_stream! {
            let mut request = self.build_request()?;

            // Auto-discover functions from registry if not explicitly provided
            let function_registry = get_global_function_registry();
            if request.tools.is_none() {
                let auto_discovered_declarations = function_registry.all_declarations();
                if !auto_discovered_declarations.is_empty() {
                    request.tools = Some(
                        auto_discovered_declarations
                            .into_iter()
                            .map(|decl| decl.into_tool())
                            .collect(),
                    );
                }
            }

            // Track the last response for returning partial results if max loops is reached
            let mut last_response: Option<InteractionResponse> = None;

            // Main auto-function streaming loop
            for _loop_count in 0..max_loops {
                // Enable streaming for this request
                request.stream = Some(true);

                // Stream this iteration's response
                let mut stream = client.create_interaction_stream(request.clone());
                let mut complete_response: Option<InteractionResponse> = None;
                // Accumulate function calls from deltas (streaming API may not include them in Complete)
                let mut accumulated_calls: Vec<(Option<String>, String, serde_json::Value)> = Vec::new();

                while let Some(result) = stream.next().await {
                    match result? {
                        StreamChunk::Delta(delta) => {
                            // Check for function calls in delta
                            if let genai_client::InteractionContent::FunctionCall { id, name, args, .. } = &delta {
                                accumulated_calls.push((id.clone(), name.clone(), args.clone()));
                            }
                            yield AutoFunctionStreamChunk::Delta(delta);
                        }
                        StreamChunk::Complete(response) => {
                            complete_response = Some(response);
                        }
                        // Ignore unknown chunk types for forward compatibility
                        _ => {}
                    }
                }

                // Get the complete response (should always be present after stream ends)
                let response = complete_response.ok_or_else(|| {
                    GenaiError::Internal(
                        "Stream ended without Complete event".to_string()
                    )
                })?;

                // Check for function calls from two possible sources:
                // 1. response.function_calls(): Populated when the Complete event includes
                //    FunctionCall content items (typical for non-streaming or when the API
                //    batches function calls into the final response)
                // 2. accumulated_calls: Populated from Delta events during streaming when
                //    the API sends FunctionCall content incrementally via deltas
                //
                // We check both because API behavior may vary; prefer Complete response
                // data when available as it represents the finalized state.
                let response_function_calls = response.function_calls();
                let has_function_calls = !response_function_calls.is_empty() || !accumulated_calls.is_empty();

                // If no function calls, we're done!
                if !has_function_calls {
                    yield AutoFunctionStreamChunk::Complete(response);
                    return;
                }

                // Signal that we're executing functions (pass the response for inspection)
                yield AutoFunctionStreamChunk::ExecutingFunctions(response.clone());

                // Determine which function calls to execute.
                // Prefer response.function_calls() if available (finalized data),
                // fall back to accumulated deltas otherwise.
                let calls_to_execute: Vec<(String, String, serde_json::Value)> = if !response_function_calls.is_empty() {
                    let mut calls = Vec::new();
                    for call in &response_function_calls {
                        let call_id = validate_call_id(call.id, call.name)?;
                        calls.push((call_id, call.name.to_string(), call.args.clone()));
                    }
                    calls
                } else {
                    let mut calls = Vec::new();
                    for (id, name, args) in &accumulated_calls {
                        let call_id = validate_call_id(id.as_deref(), name)?;
                        calls.push((call_id, name.clone(), args.clone()));
                    }
                    calls
                };

                // Build function results for next iteration
                let mut function_results_content = Vec::new();
                let mut execution_results = Vec::new();

                for (call_id, name, args) in &calls_to_execute {
                    // Execute the function with timing
                    let start = Instant::now();
                    // Design decision: Errors are converted to JSON and sent to the model rather than
                    // failing the entire interaction. This allows the model to recover gracefully -
                    // e.g., "I couldn't get the weather, let me try something else." The error is
                    // logged for debugging, but the conversation continues. For strict error handling,
                    // use manual function calling via `create_stream()` instead of `create_stream_with_auto_functions()`.
                    let result = if let Some(function) = function_registry.get(name) {
                        match function.call(args.clone()).await {
                            Ok(result) => result,
                            Err(e) => {
                                error!(
                                    "Function execution failed: function='{}', error='{}'",
                                    name, e
                                );
                                json!({ "error": e.to_string() })
                            }
                        }
                    } else {
                        // Function not in registry - could be a typo in declarations or missing #[tool] macro.
                        // We inform the model rather than failing, allowing it to adapt or use other functions.
                        warn!(
                            "Function not found in registry: function='{}'. Informing model.",
                            name
                        );
                        json!({ "error": format!("Function '{}' is not available or not found.", name) })
                    };
                    let duration = start.elapsed();

                    // Track result for yielding
                    execution_results.push(FunctionExecutionResult::new(
                        name.clone(),
                        call_id.clone(),
                        result.clone(),
                        duration,
                    ));

                    // Add function result content for API
                    function_results_content.push(function_result_content(
                        name.clone(),
                        call_id.clone(),
                        result,
                    ));
                }

                // Yield function results
                yield AutoFunctionStreamChunk::FunctionResults(execution_results);

                // Save this response before moving to next iteration
                // (in case we hit max loops, we want to return the last response)
                last_response = Some(response.clone());

                // Create new request with function results
                request.previous_interaction_id = Some(response.id);
                request.input = InteractionInput::Content(function_results_content);
            }

            // Max loops reached - yield partial result with the last response
            // This preserves all prior FunctionResults chunks that were already yielded
            warn!(
                "Reached maximum function call loops ({max_loops}). \
                 Yielding MaxLoopsReached with last response. \
                 The model may be stuck in a loop."
            );

            // If we never made it through even one iteration (shouldn't happen with max_loops > 0),
            // return an error since we have no response to return
            let response = last_response.ok_or_else(|| {
                GenaiError::InvalidInput(format!(
                    "max_function_call_loops ({max_loops}) must be at least 1"
                ))
            })?;

            yield AutoFunctionStreamChunk::MaxLoopsReached(response);
        })
    }
}
