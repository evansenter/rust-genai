# Multi-Turn Function Calling Guide

This guide covers everything you need to know about multi-turn conversations and function calling in `rust-genai`. It explains the patterns, trade-offs, and design decisions behind the API.

## Table of Contents

- [Overview](#overview)
- [Stateful vs Stateless](#stateful-vs-stateless)
- [Function Declaration Approaches](#function-declaration-approaches)
- [Auto vs Manual Function Calling](#auto-vs-manual-function-calling)
- [API Inheritance Behavior](#api-inheritance-behavior)
- [Thought Signatures](#thought-signatures)
- [Design Patterns](#design-patterns)
- [Decision Matrix](#decision-matrix)
- [Examples Reference](#examples-reference)

## Overview

Multi-turn conversations allow you to build agents that maintain context across multiple exchanges. Combined with function calling, you can create sophisticated tools that interact with external systems.

The key decision points are:

1. **State management**: Server-side (stateful) vs client-side (stateless)
2. **Function execution**: Automatic vs manual control
3. **Function registration**: Compile-time (`#[tool]`) vs runtime (`ToolService` / `FunctionDeclaration`)

## Stateful vs Stateless

The Gemini Interactions API supports two modes controlled by the `store` parameter:

### Stateful Mode (`store: true`)

```rust
// First turn
let response = client.interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("Hi, I'm Alice")
    .with_system_instruction("You are a helpful assistant")
    .with_store_enabled()  // Server stores conversation
    .create()
    .await?;

// Subsequent turns - just chain with previous_interaction_id
let response = client.interaction()
    .with_model("gemini-3-flash-preview")
    .with_text("What's my name?")  // Model remembers: "Alice"
    .with_previous_interaction(&response.id.unwrap())
    .with_store_enabled()
    .create()
    .await?;
```

**Characteristics:**
- Server maintains conversation history
- Use `previous_interaction_id` to chain turns
- System instruction is inherited (only send on first turn)
- Tools are NOT inherited (must resend each turn)
- Enables `create_with_auto_functions()` for automatic function execution

### Stateless Mode (`store: false`)

```rust
let mut history: Vec<InteractionContent> = vec![];

// First turn
history.push(text_content("Hi, I'm Alice"));

let response = client.interaction()
    .with_model("gemini-3-flash-preview")
    .with_input(InteractionInput::Content(history.clone()))
    .with_system_instruction("You are a helpful assistant")
    .with_store_disabled()  // No server state
    .create()
    .await?;

// Add response to history
history.push(text_content(response.text().unwrap()));

// Second turn - must include full history
history.push(text_content("What's my name?"));

let response = client.interaction()
    .with_model("gemini-3-flash-preview")
    .with_input(InteractionInput::Content(history.clone()))
    .with_system_instruction("You are a helpful assistant")
    .with_store_disabled()
    .create()
    .await?;
```

**Characteristics:**
- No server-side conversation storage
- Must manually build and send conversation history
- Cannot use `previous_interaction_id`
- `create_with_auto_functions()` is blocked at compile time
- Must use manual function calling with `create()`

### When to Use Each

| Use Case | Stateful | Stateless |
|----------|----------|-----------|
| Most agent applications | ✅ | |
| Privacy-sensitive applications | | ✅ |
| Custom conversation persistence | | ✅ |
| Conversation filtering/modification | | ✅ |
| Testing and debugging | | ✅ |
| Automatic function execution | ✅ | |

## Function Declaration Approaches

Three ways to declare functions, each suited to different needs:

### 1. `#[tool]` Macro (Compile-time, Stateless)

```rust
use rust_genai_macros::tool;

/// Look up customer information by ID or email
#[tool(identifier(description = "Customer ID or email"))]
fn lookup_customer(identifier: String) -> String {
    // Function body - no access to external state
    format!("Found customer: {}", identifier)
}

// Using it:
let result = client.interaction()
    .with_text("Look up customer alice@example.com")
    .create_with_auto_functions()  // Auto-discovers #[tool] functions
    .await?;
```

**When to use:**
- Pure functions with no external dependencies
- Simple tools where statelessness is acceptable
- Quick prototyping

**Limitations:**
- Cannot access database connections, API clients, or shared state
- Global registration via `inventory` crate
- Compile-time fixed

### 2. `ToolService` (Runtime, Stateful)

```rust
use rust_genai::{ToolService, CallableFunction, FunctionDeclaration};

struct MyToolService {
    db: Arc<DatabasePool>,
    config: Arc<RwLock<Config>>,
}

impl ToolService for MyToolService {
    fn tools(&self) -> Vec<Arc<dyn CallableFunction>> {
        vec![
            Arc::new(CustomerLookupTool { db: self.db.clone() }),
            Arc::new(OrderTool { config: self.config.clone() }),
        ]
    }
}

// Using it:
let service = Arc::new(MyToolService { db, config });

let result = client.interaction()
    .with_text("Look up customer alice@example.com")
    .with_tool_service(service.clone())  // Inject stateful service
    .create_with_auto_functions()
    .await?;
```

**When to use:**
- Tools need database connections
- Tools need HTTP clients
- Tools need runtime configuration
- Tools need per-request context (user ID, auth tokens)

**Pattern:** Use `Arc<RwLock<T>>` for interior mutability. Clone the Arc, not the service.

### 3. `FunctionDeclaration` Builder (Manual)

```rust
use rust_genai::FunctionDeclaration;

let functions = vec![
    FunctionDeclaration::builder("lookup_customer")
        .description("Look up customer by ID")
        .parameter("id", json!({
            "type": "string",
            "description": "Customer ID"
        }))
        .required(vec!["id".to_string()])
        .build(),
];

let response = client.interaction()
    .with_text("Look up customer CUST-001")
    .with_functions(functions)
    .create()  // Manual handling required
    .await?;

// Manually execute function calls
for call in response.function_calls() {
    let result = execute_my_function(call.name, call.args);
    // Send result back...
}
```

**When to use:**
- Full control over function execution
- Dynamic function definitions
- Custom execution logic (rate limiting, logging, etc.)
- Stateless mode where `create_with_auto_functions()` is blocked

## Auto vs Manual Function Calling

### Auto (`create_with_auto_functions()`)

```rust
// With #[tool] functions
let result = client.interaction()
    .with_text("Calculate 2 + 2")
    .create_with_auto_functions()
    .await?;

// With ToolService
let result = client.interaction()
    .with_text("Calculate 2 + 2")
    .with_tool_service(service)
    .create_with_auto_functions()
    .await?;

// Result includes function executions
for exec in &result.executions {
    println!("{} -> {}", exec.name, exec.result);
}
println!("Final: {}", result.response.text().unwrap());
```

**Characteristics:**
- Automatic function execution loop
- Built-in retry and error handling
- Configurable max iterations
- Only available with `store: true`

### Manual (`create()`)

```rust
let mut response = client.interaction()
    .with_text("What's the weather in Tokyo?")
    .with_functions(functions)
    .create()
    .await?;

// Manual function calling loop
const MAX_ITERATIONS: usize = 5;
for _ in 0..MAX_ITERATIONS {
    let calls = response.function_calls();
    if calls.is_empty() { break; }

    let mut results = Vec::new();
    for call in &calls {
        let call_id = call.id.ok_or("Missing call_id")?;
        let result = execute_function(call.name, call.args);
        results.push(function_result_content(
            call.name.to_string(),
            call_id.to_string(),
            result,
        ));
    }

    response = client.interaction()
        .with_model("gemini-3-flash-preview")
        .with_previous_interaction(response.id.as_ref().unwrap())
        .with_content(results)
        .create()
        .await?;
}
```

**Characteristics:**
- Full control over execution
- Custom error handling, logging, rate limiting
- Works with both stateful and stateless modes
- Required for `store: false`

## API Inheritance Behavior

When using `previous_interaction_id` (stateful mode), some settings are inherited:

| Setting | Inherited? | Implication |
|---------|-----------|-------------|
| System instruction | ✅ Yes | Only send on first turn |
| Conversation history | ✅ Yes | Model remembers context |
| Tools/Functions | ❌ No | Must resend each turn that needs function calling |
| Model | ❌ No | Must specify each request |

This leads to the recommended pattern:

```rust
match &self.last_interaction_id {
    Some(prev_id) => {
        // Subsequent turns: chain, tools required, system inherited
        client.interaction()
            .with_model("gemini-3-flash-preview")
            .with_text(message)
            .with_functions(functions)  // Must resend
            .with_previous_interaction(prev_id)
            .create()
            .await?
    }
    None => {
        // First turn: set system instruction
        client.interaction()
            .with_model("gemini-3-flash-preview")
            .with_text(message)
            .with_functions(functions)
            .with_system_instruction(system_prompt)  // Only here
            .create()
            .await?
    }
}
```

### Important: Function Result Turns

When sending function results back to the model, you do NOT need to resend tools:

```rust
// After model requests function calls...
let results = execute_functions(&calls);

// Function result turn - no tools needed
response = client.interaction()
    .with_previous_interaction(&response.id.unwrap())
    .with_content(results)  // Just the function results
    .create()
    .await?;
```

The model remembers available tools within the same interaction chain. Only new user message turns need tools resent.

## Thought Signatures

The Gemini API returns "thought" outputs when thinking is enabled. Here's what you need to know:

### What They Are

Thought signatures are cryptographic proofs that thoughts haven't been modified. They appear as:

```json
{
  "outputs": [
    {
      "type": "thought",
      "signature": "EtYFCtMF..."
    },
    {
      "type": "text",
      "text": "The answer is 42"
    }
  ]
}
```

### Key Finding: Not Required for Function Calling

Through testing, we discovered:

1. **Thought signatures are output-only** - they appear in API responses, not in function calls
2. **The `thought_signature` field on `FunctionCallInfo` is always `None`** - thoughts are separate outputs
3. **You do NOT need to echo thought signatures in function results**
4. **Basic stateless multi-turn works without any thought handling**

```rust
// This works fine - no thought signature needed
for call in response.function_calls() {
    let call_id = call.id.ok_or("Missing call_id")?;

    // Just add the function call (no thought signature)
    history.push(function_call_content(call.name, call.args.clone()));

    // Execute and add result
    let result = execute_function(call.name, call.args);
    history.push(function_result_content(call.name, call_id, result));
}
```

### When Thoughts Matter

Thought signatures matter when:
- Using `thought_echo` feature (echoing thinking back to the model)
- Verifying thought integrity for compliance/auditing

For standard function calling, you can ignore them entirely.

## Design Patterns

### Pattern 1: First Turn vs Subsequent Turns (Match)

The typestate pattern enforces that `with_system_instruction()` is only available on the first turn. Use a match:

```rust
struct Agent {
    client: Client,
    last_id: Option<String>,
    functions: Vec<FunctionDeclaration>,
}

impl Agent {
    async fn process(&mut self, message: &str) -> Result<String, Error> {
        let response = match &self.last_id {
            Some(prev_id) => {
                self.client.interaction()
                    .with_model("gemini-3-flash-preview")
                    .with_text(message)
                    .with_functions(self.functions.clone())
                    .with_previous_interaction(prev_id)
                    .create()
                    .await?
            }
            None => {
                self.client.interaction()
                    .with_model("gemini-3-flash-preview")
                    .with_text(message)
                    .with_functions(self.functions.clone())
                    .with_system_instruction("...")
                    .create()
                    .await?
            }
        };

        self.last_id = response.id.clone();
        Ok(response.text().unwrap_or("").to_string())
    }
}
```

**Why not simplify with optional?**

A method like `with_optional_previous_interaction(Option<&str>)` would:
1. Hide the fact that system instruction behavior changes
2. Still require tracking `last_id: Option<String>` somewhere
3. Make the API less explicit about what's happening

The match pattern is verbose but clear about the two distinct states.

### Pattern 2: Stateless History Builder

```rust
struct StatelessSession {
    client: Client,
    history: Vec<InteractionContent>,
    functions: Vec<FunctionDeclaration>,
    system_instruction: String,
}

impl StatelessSession {
    async fn process(&mut self, message: &str) -> Result<String, Error> {
        self.history.push(text_content(message));

        let mut response = self.client.interaction()
            .with_model("gemini-3-flash-preview")
            .with_input(InteractionInput::Content(self.history.clone()))
            .with_functions(self.functions.clone())
            .with_system_instruction(&self.system_instruction)
            .with_store_disabled()
            .create()
            .await?;

        // Handle function calls...

        if let Some(text) = response.text() {
            self.history.push(text_content(text));
            Ok(text.to_string())
        } else {
            Ok(String::new())
        }
    }
}
```

### Pattern 3: ToolService with Shared State

```rust
struct ProductionToolService {
    db: Arc<DatabasePool>,
    http: Arc<reqwest::Client>,
    config: Arc<RwLock<AppConfig>>,
}

impl ToolService for ProductionToolService {
    fn tools(&self) -> Vec<Arc<dyn CallableFunction>> {
        vec![
            Arc::new(DatabaseTool { pool: self.db.clone() }),
            Arc::new(ApiTool { client: self.http.clone() }),
            Arc::new(ConfigTool { config: self.config.clone() }),
        ]
    }
}

// Usage
let service = Arc::new(ProductionToolService { db, http, config });

// Multiple requests share the same service instance
let r1 = client.interaction()
    .with_tool_service(service.clone())
    .create_with_auto_functions().await?;

let r2 = client.interaction()
    .with_tool_service(service.clone())  // Same instance
    .create_with_auto_functions().await?;
```

## Decision Matrix

### Choosing State Management

```
Need privacy/no server storage? ─────────────────> Stateless
Need custom conversation persistence? ───────────> Stateless
Need automatic function execution? ──────────────> Stateful
Building a typical agent? ───────────────────────> Stateful
```

### Choosing Function Declaration

```
Pure functions, no dependencies? ────────────────> #[tool] macro
Need database/API access? ───────────────────────> ToolService
Need full control/custom execution? ─────────────> FunctionDeclaration + manual
Stateless mode? ─────────────────────────────────> FunctionDeclaration + manual
```

### Choosing Execution Mode

```
store: false? ───────────────────────────────────> Manual (auto is blocked)
Need custom logging/rate limiting? ──────────────> Manual
Need simplest code? ─────────────────────────────> Auto
Standard agent? ─────────────────────────────────> Auto
```

## Examples Reference

| Example | State | Functions | Execution |
|---------|-------|-----------|-----------|
| `multi_turn_agent_auto` | Stateful | `#[tool]` | Auto |
| `multi_turn_agent_manual` | Stateful | `FunctionDeclaration` | Manual |
| `stateless_multi_turn_agent_manual` | Stateless | `FunctionDeclaration` | Manual |
| `tool_service` | Stateful | `ToolService` | Auto |
| `auto_function_calling` | Single-turn | `#[tool]` | Auto |
| `manual_function_calling` | Single-turn | `FunctionDeclaration` | Manual |

Run any example:

```bash
cargo run --example multi_turn_agent_auto

# With wire-level debugging
LOUD_WIRE=1 cargo run --example stateless_multi_turn_agent_manual
```
