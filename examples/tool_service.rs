//! Tool Service Example
//!
//! This example demonstrates dependency injection for function calling using
//! the `ToolService` trait. This enables tools to access shared state like
//! database connections, API clients, or configuration.
//!
//! # Running
//!
//! ```bash
//! cargo run --example tool_service
//! ```
//!
//! # Prerequisites
//!
//! Set the `GEMINI_API_KEY` environment variable with your API key.

use async_trait::async_trait;
use rust_genai::{CallableFunction, Client, FunctionDeclaration, FunctionError, ToolService};
use serde_json::{Value, json};
use std::env;
use std::sync::Arc;

// =============================================================================
// Example: A calculator tool with configurable precision
// =============================================================================

/// Configuration that will be injected into our tools
struct CalculatorConfig {
    precision: u32,
}

/// A calculator tool that uses injected configuration
struct CalculatorTool {
    config: Arc<CalculatorConfig>,
}

impl CalculatorTool {
    fn new(config: Arc<CalculatorConfig>) -> Self {
        Self { config }
    }
}

#[async_trait]
impl CallableFunction for CalculatorTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration::builder("calculate")
            .description("Performs arithmetic calculations with configurable precision")
            .parameter(
                "expression",
                json!({
                    "type": "string",
                    "description": "A mathematical expression like '2 + 3' or '4 * 5'"
                }),
            )
            .required(vec!["expression".to_string()])
            .build()
    }

    async fn call(&self, args: Value) -> Result<Value, FunctionError> {
        let expression = args
            .get("expression")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FunctionError::ArgumentMismatch("Missing 'expression'".into()))?;

        println!(
            "  [CalculatorTool called with precision={}]",
            self.config.precision
        );

        // Simple expression parsing (in real code, use a proper parser)
        let result = if expression.contains('+') {
            let parts: Vec<&str> = expression.split('+').collect();
            if parts.len() == 2 {
                let a: f64 = parts[0].trim().parse().unwrap_or(0.0);
                let b: f64 = parts[1].trim().parse().unwrap_or(0.0);
                a + b
            } else {
                0.0
            }
        } else if expression.contains('*') {
            let parts: Vec<&str> = expression.split('*').collect();
            if parts.len() == 2 {
                let a: f64 = parts[0].trim().parse().unwrap_or(0.0);
                let b: f64 = parts[1].trim().parse().unwrap_or(0.0);
                a * b
            } else {
                0.0
            }
        } else {
            expression.parse().unwrap_or(0.0)
        };

        // Apply precision from config
        let formatted = format!("{:.prec$}", result, prec = self.config.precision as usize);

        Ok(json!({
            "expression": expression,
            "result": formatted,
            "precision": self.config.precision
        }))
    }
}

// =============================================================================
// ToolService implementation
// =============================================================================

/// A service that provides tools with injected dependencies
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
        vec![Arc::new(CalculatorTool::new(self.config.clone()))]
    }
}

// =============================================================================
// Main
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable not set");

    let client = Client::builder(api_key).build();

    println!("=== TOOL SERVICE EXAMPLE ===\n");

    // Create a tool service with specific configuration
    // This injects the precision setting into the calculator tool
    let service = Arc::new(MathToolService::new(4)); // 4 decimal places

    println!("Tool service created with precision=4\n");

    let prompt = "What is 123.456 + 789.012?";
    println!("User: {}\n", prompt);
    println!("Processing...\n");

    // Use the tool service with auto function calling
    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(prompt)
        .with_tool_service(service)
        .create_with_auto_functions()
        .await?;

    // Show execution details
    if !result.executions.is_empty() {
        println!("Function executions:");
        for exec in &result.executions {
            println!(
                "  - {} ({:?}): {}",
                exec.name,
                exec.duration,
                exec.result
                    .to_string()
                    .chars()
                    .take(100)
                    .collect::<String>()
            );
        }
        println!();
    }

    // Show the response
    println!("Assistant: {}", result.response.text().unwrap_or_default());

    // Demonstrate with different configuration
    println!("\n--- With different precision ---\n");

    let service_high_precision = Arc::new(MathToolService::new(8)); // 8 decimal places

    let prompt2 = "Calculate 1 + 2";
    println!("User: {}\n", prompt2);

    let result2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(prompt2)
        .with_tool_service(service_high_precision)
        .create_with_auto_functions()
        .await?;

    println!("Assistant: {}", result2.response.text().unwrap_or_default());

    println!("\n=== END EXAMPLE ===");

    Ok(())
}
