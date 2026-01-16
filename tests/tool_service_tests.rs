//! ToolService dependency injection tests for the Interactions API
//!
//! Tests for the ToolService pattern which enables runtime function registration
//! with dependency injection for stateful tools (DB pools, API clients, etc.).
//!
//! These tests require the GEMINI_API_KEY environment variable to be set.
//!
//! # Running Tests
//!
//! ```bash
//! cargo test --test tool_service_tests -- --include-ignored --nocapture
//! ```

mod common;

use async_trait::async_trait;
use common::{
    consume_auto_function_stream, extended_test_timeout, get_client, interaction_builder,
    test_timeout, with_timeout,
};
use genai_rs::{CallableFunction, FunctionDeclaration, FunctionError, ToolService};
use serde_json::json;
use std::sync::Arc;

// =============================================================================
// ToolService Helper Types
// =============================================================================

/// Configuration for the calculator tool
struct CalculatorConfig {
    precision: u32,
}

/// A calculator tool that uses injected configuration
struct CalculatorTool {
    config: Arc<CalculatorConfig>,
}

#[async_trait]
impl CallableFunction for CalculatorTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration::builder("calculate")
            .description("Performs arithmetic calculations")
            .parameter(
                "operation",
                json!({"type": "string", "enum": ["add", "subtract", "multiply"]}),
            )
            .parameter(
                "a",
                json!({"type": "number", "description": "First operand"}),
            )
            .parameter(
                "b",
                json!({"type": "number", "description": "Second operand"}),
            )
            .required(vec![
                "operation".to_string(),
                "a".to_string(),
                "b".to_string(),
            ])
            .build()
    }

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, FunctionError> {
        let op = args
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FunctionError::ArgumentMismatch("Missing 'operation'".into()))?;
        let a = args
            .get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| FunctionError::ArgumentMismatch("Missing 'a'".into()))?;
        let b = args
            .get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| FunctionError::ArgumentMismatch("Missing 'b'".into()))?;

        let result = match op {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            _ => return Err(FunctionError::ArgumentMismatch("Invalid operation".into())),
        };

        // Apply precision from config
        let formatted = format!("{:.prec$}", result, prec = self.config.precision as usize);

        Ok(json!({
            "result": formatted,
            "precision": self.config.precision
        }))
    }
}

/// A service that provides the calculator tool with injected dependencies
struct MathToolService {
    config: Arc<CalculatorConfig>,
}

impl MathToolService {
    fn new(precision: u32) -> Self {
        Self {
            config: Arc::new(CalculatorConfig { precision }),
        }
    }
}

impl ToolService for MathToolService {
    fn tools(&self) -> Vec<Arc<dyn CallableFunction>> {
        vec![Arc::new(CalculatorTool {
            config: self.config.clone(),
        })]
    }
}

// =============================================================================
// ToolService Non-Streaming Tests
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_tool_service_non_streaming() {
    // Test that ToolService works with create_with_auto_functions()
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(test_timeout(), async {
        // Create a tool service with specific configuration
        let service = Arc::new(MathToolService::new(4)); // 4 decimal places

        let result = interaction_builder(&client)
            .with_text("What is 123.456 + 789.012? Use the calculate function.")
            .with_tool_service(service)
            .create_with_auto_functions()
            .await
            .expect("Auto function calling with ToolService failed");

        println!("Function executions: {:?}", result.executions);

        // Verify the function was called
        assert!(
            !result.executions.is_empty(),
            "Should have at least one function execution"
        );
        assert_eq!(
            result.executions[0].name, "calculate",
            "Should have called the calculate function"
        );

        // Verify the result includes the precision from the service
        let exec_result = &result.executions[0].result;
        println!("Execution result: {}", exec_result);
        assert!(
            exec_result.get("precision").is_some(),
            "Result should include precision from service config"
        );

        // Verify final response
        let response = &result.response;
        assert!(response.has_text(), "Should have text response");
        let text = response.as_text().unwrap();
        println!("Final response: {}", text);

        // Should mention the sum (912.468)
        assert!(
            text.contains("912") || text.contains("sum") || text.contains("result"),
            "Response should mention the calculation result"
        );
    })
    .await;
}

// =============================================================================
// ToolService Streaming Tests
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_tool_service_streaming() {
    // Test that ToolService works with create_stream_with_auto_functions()
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(test_timeout(), async {
        // Create a tool service with specific configuration
        let service = Arc::new(MathToolService::new(2)); // 2 decimal places

        let stream = interaction_builder(&client)
            .with_text("Calculate 50 * 3. Use the calculate function.")
            .with_tool_service(service)
            .create_stream_with_auto_functions();

        let result = consume_auto_function_stream(stream).await;

        println!("\n--- Results ---");
        println!("Delta count: {}", result.delta_count);
        println!(
            "Executing functions count: {}",
            result.executing_functions_count
        );
        println!("Functions executed: {:?}", result.executed_function_names);

        // Should have received a final response
        assert!(
            result.final_response.is_some(),
            "Should receive a complete response"
        );

        // If functions were executed, verify calculate was called
        if result.executing_functions_count > 0 {
            println!("✓ Function execution was streamed with ToolService");
            assert!(
                result
                    .executed_function_names
                    .contains(&"calculate".to_string()),
                "Should have executed calculate function from ToolService"
            );
        }

        // Should have some response
        let response = result.final_response.unwrap();
        assert!(
            response.has_text() || !result.collected_text.is_empty(),
            "Should have text response"
        );

        let text = response.as_text().unwrap_or(&result.collected_text);
        println!("Final response: {}", text);

        // Should mention 150 (the result of 50 * 3)
        assert!(
            text.contains("150"),
            "Response should mention the calculation result (150)"
        );
    })
    .await;
}

// =============================================================================
// ToolService Override Tests
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_tool_service_overrides_global_registry() {
    // Test that ToolService functions take precedence over global registry
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(test_timeout(), async {
        // Create a custom weather tool that returns a distinct response
        struct CustomWeatherTool;

        #[async_trait]
        impl CallableFunction for CustomWeatherTool {
            fn declaration(&self) -> FunctionDeclaration {
                // Same name as the global get_weather_test function
                FunctionDeclaration::builder("get_weather_test")
                    .description("Get the current weather for a city")
                    .parameter(
                        "city",
                        json!({"type": "string", "description": "The city name"}),
                    )
                    .required(vec!["city".to_string()])
                    .build()
            }

            async fn call(
                &self,
                args: serde_json::Value,
            ) -> Result<serde_json::Value, FunctionError> {
                let city = args
                    .get("city")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                // Return a distinctive response to prove this override was used
                Ok(json!({
                    "city": city,
                    "temperature": "999°C",
                    "conditions": "OVERRIDE_FROM_TOOL_SERVICE",
                    "source": "custom_service"
                }))
            }
        }

        struct CustomWeatherService;

        impl ToolService for CustomWeatherService {
            fn tools(&self) -> Vec<Arc<dyn CallableFunction>> {
                vec![Arc::new(CustomWeatherTool)]
            }
        }

        let service = Arc::new(CustomWeatherService);

        let result = interaction_builder(&client)
            .with_text("What's the weather in Seattle? Use the get_weather_test function.")
            .with_tool_service(service)
            .create_with_auto_functions()
            .await
            .expect("Auto function calling with override failed");

        println!("Function executions: {:?}", result.executions);

        // Verify the function was called
        assert!(
            !result.executions.is_empty(),
            "Should have at least one function execution"
        );
        assert_eq!(
            result.executions[0].name, "get_weather_test",
            "Should have called get_weather_test"
        );

        // Verify the custom service's response was used
        let exec_result = &result.executions[0].result;
        println!("Execution result: {}", exec_result);

        // The result should come from our custom tool (has "source": "custom_service")
        assert!(
            exec_result.get("source").is_some()
                || exec_result
                    .to_string()
                    .contains("OVERRIDE_FROM_TOOL_SERVICE"),
            "Result should come from the custom ToolService, not global registry"
        );
    })
    .await;
}

// =============================================================================
// ToolService Multiple Functions Tests
// =============================================================================

#[tokio::test]
#[ignore = "Requires API key"]
async fn test_tool_service_streaming_with_multiple_functions() {
    // Test ToolService streaming with multiple functions available
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(extended_test_timeout(), async {
        // Create a service with multiple tools
        struct MultiToolService;

        struct AddTool;

        #[async_trait]
        impl CallableFunction for AddTool {
            fn declaration(&self) -> FunctionDeclaration {
                FunctionDeclaration::builder("add_numbers")
                    .description("Adds two numbers together")
                    .parameter("a", json!({"type": "number"}))
                    .parameter("b", json!({"type": "number"}))
                    .required(vec!["a".to_string(), "b".to_string()])
                    .build()
            }

            async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, FunctionError> {
                let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                Ok(json!({ "sum": a + b }))
            }
        }

        struct MultiplyTool;

        #[async_trait]
        impl CallableFunction for MultiplyTool {
            fn declaration(&self) -> FunctionDeclaration {
                FunctionDeclaration::builder("multiply_numbers")
                    .description("Multiplies two numbers together")
                    .parameter("a", json!({"type": "number"}))
                    .parameter("b", json!({"type": "number"}))
                    .required(vec!["a".to_string(), "b".to_string()])
                    .build()
            }

            async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, FunctionError> {
                let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                Ok(json!({ "product": a * b }))
            }
        }

        impl ToolService for MultiToolService {
            fn tools(&self) -> Vec<Arc<dyn CallableFunction>> {
                vec![Arc::new(AddTool), Arc::new(MultiplyTool)]
            }
        }

        let service = Arc::new(MultiToolService);

        // Ask a question that might trigger both functions
        let stream = interaction_builder(&client)
            .with_text("What is 5 + 3, and what is 4 * 7? Use the add_numbers and multiply_numbers functions.")
            .with_tool_service(service)
            .create_stream_with_auto_functions();

        let result = consume_auto_function_stream(stream).await;

        println!("\n--- Results ---");
        println!("Delta count: {}", result.delta_count);
        println!(
            "Executing functions count: {}",
            result.executing_functions_count
        );
        println!("Functions executed: {:?}", result.executed_function_names);

        assert!(
            result.final_response.is_some(),
            "Should receive a complete response"
        );

        // Model should have called at least one of the functions
        // (it might call them in parallel or sequentially)
        if result.executing_functions_count > 0 {
            println!("✓ Functions were executed via ToolService streaming");

            // Check that our custom functions were used
            let has_add = result.executed_function_names.contains(&"add_numbers".to_string());
            let has_multiply = result.executed_function_names.contains(&"multiply_numbers".to_string());

            println!("  - add_numbers called: {}", has_add);
            println!("  - multiply_numbers called: {}", has_multiply);

            // At least one should have been called
            assert!(
                has_add || has_multiply,
                "At least one ToolService function should have been called"
            );
        }

        // Response should contain the answers
        let response = result.final_response.unwrap();
        let text = response.as_text().unwrap_or(&result.collected_text);
        println!("Final response: {}", text);

        // Should mention 8 (5+3) or 28 (4*7)
        assert!(
            text.contains("8") || text.contains("28"),
            "Response should contain calculation results"
        );
    })
    .await;
}
