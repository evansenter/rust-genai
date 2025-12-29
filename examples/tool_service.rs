//! Tool Service Example
//!
//! This example demonstrates dependency injection for function calling using
//! the `ToolService` trait. This enables tools to access shared state like
//! database connections, API clients, or configuration.
//!
//! Key concepts demonstrated:
//! - Shared mutable state via `Arc<RwLock<T>>`
//! - Same service instance reused across multiple requests
//! - Dynamic configuration changes between requests
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
use std::sync::{Arc, RwLock};

// =============================================================================
// Example: A calculator tool with dynamically configurable precision
// =============================================================================

/// A calculator tool that reads precision from shared mutable state.
///
/// The precision can be changed at runtime, and all subsequent function
/// calls will use the new value.
struct CalculatorTool {
    /// Shared precision config - uses RwLock for interior mutability
    precision: Arc<RwLock<u32>>,
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

        // Read current precision from shared state
        let precision = *self.precision.read().unwrap();

        println!("  [CalculatorTool called with precision={}]", precision);

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
        let formatted = format!("{:.prec$}", result, prec = precision as usize);

        Ok(json!({
            "expression": expression,
            "result": formatted,
            "precision": precision
        }))
    }
}

// =============================================================================
// ToolService implementation
// =============================================================================

/// A service that provides tools with shared mutable configuration.
///
/// This demonstrates how to inject dependencies that can change at runtime.
/// Real-world examples include:
/// - Database connection pools
/// - API clients with refreshable auth tokens
/// - Feature flags that can be toggled
/// - Per-request context (user ID, tracing spans)
struct MathToolService {
    /// Shared precision - can be modified between requests
    precision: Arc<RwLock<u32>>,
}

impl MathToolService {
    fn new(precision: u32) -> Self {
        Self {
            precision: Arc::new(RwLock::new(precision)),
        }
    }

    /// Update the precision setting. All subsequent function calls will use
    /// the new value.
    fn set_precision(&self, precision: u32) {
        *self.precision.write().unwrap() = precision;
    }
}

impl ToolService for MathToolService {
    fn tools(&self) -> Vec<Arc<dyn CallableFunction>> {
        vec![Arc::new(CalculatorTool {
            precision: self.precision.clone(),
        })]
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

    // Create a single service instance with initial precision of 2
    let service = Arc::new(MathToolService::new(2));

    println!("Created service with precision=2\n");

    // --- First request: precision=2 ---
    let prompt1 = "What is 123.456 + 789.012?";
    println!("User: {}\n", prompt1);

    let result1 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(prompt1)
        .with_tool_service(service.clone()) // Clone the Arc, not the service
        .create_with_auto_functions()
        .await?;

    if !result1.executions.is_empty() {
        println!("Function executions:");
        for exec in &result1.executions {
            println!("  - {} -> {}", exec.name, exec.result);
        }
        println!();
    }
    println!(
        "Assistant: {}\n",
        result1.response.text().unwrap_or_default()
    );

    // --- Change precision on the SAME service instance ---
    println!("--- Updating precision to 8 on same service instance ---\n");
    service.set_precision(8);

    // --- Second request: same service, now with precision=8 ---
    let prompt2 = "What is 1.0 * 3.0?";
    println!("User: {}\n", prompt2);

    let result2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text(prompt2)
        .with_tool_service(service.clone()) // Same service instance
        .create_with_auto_functions()
        .await?;

    if !result2.executions.is_empty() {
        println!("Function executions:");
        for exec in &result2.executions {
            println!("  - {} -> {}", exec.name, exec.result);
        }
        println!();
    }
    println!("Assistant: {}", result2.response.text().unwrap_or_default());

    println!("\n=== END EXAMPLE ===");

    Ok(())
}
