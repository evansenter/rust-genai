# Testing Guide

This guide explains the testing infrastructure, philosophy, and how to write tests for `genai-rs`.

## Table of Contents

- [Test Categories](#test-categories)
- [Running Tests](#running-tests)
- [CI Pipeline](#ci-pipeline)
- [Test Utilities](#test-utilities)
- [Writing New Tests](#writing-new-tests)
- [Assertion Strategies](#assertion-strategies)

## Test Categories

### Unit Tests (~24k lines)

Inline tests in source files covering serialization, builders, and internal logic.

```bash
cargo test --workspace                    # Run all unit tests
cargo test --workspace -- test_name       # Run specific test
```

**Location**: `src/*_tests.rs` files and `#[cfg(test)]` modules

**What they test**:
- Serialization/deserialization roundtrips
- Builder pattern validation
- Helper method behavior
- Error type formatting

### Integration Tests (25 files)

End-to-end tests that call the real Gemini API. Require `GEMINI_API_KEY`.

```bash
cargo test -- --include-ignored           # Run all tests including integration
cargo test --test interactions_api_tests -- --include-ignored  # Single file
```

**Location**: `tests/*.rs`

**Key test files**:

| File | Coverage |
|------|----------|
| `interactions_api_tests.rs` | Basic API interactions |
| `multiturn_tests.rs` | Stateful conversations |
| `streaming_tests.rs` | SSE streaming |
| `tools_and_config_tests.rs` | Built-in tools configuration |
| `advanced_function_calling_tests.rs` | `#[tool]` macro, auto-execution |
| `thinking_function_tests.rs` | Thinking mode + function calling |
| `multimodal_tests.rs` | Images, audio, video, documents |

### Property-Based Tests (proptest)

Automatic generation of test cases for serialization roundtrips.

```bash
cargo test proptest                       # Run proptest tests
```

**Location**:
- `src/proptest_tests.rs` - Strategy generators for all types
- `tests/proptest_roundtrip_tests.rs` - Integration proptests

**What they verify**:
- Any valid type serializes and deserializes to the same value
- Unknown variants preserve data through roundtrips
- Edge cases humans wouldn't think to test

### UI/Compile-Time Tests (trybuild)

Verify that invalid code fails to compile with helpful error messages.

```bash
cargo test --test ui_tests
```

**Location**: `tests/ui/*.rs`

**What they test**:
- Type-state pattern enforcement (can't call `with_system_instruction()` after chaining)
- `#[tool]` macro error messages
- Invalid builder configurations

### Canary Tests

Early-warning tests that detect when the API returns new content types.

```bash
cargo test --test api_canary_tests -- --include-ignored
```

**Purpose**: When Google adds new content types, these tests fail to alert us to add support.

**Note**: Skipped when `--features strict-unknown` is enabled.

### Wire Format Tests

Verify actual API wire formats match our expectations.

| File | Purpose |
|------|---------|
| `wire_format_verification_tests.rs` | Offline format verification |
| `api_wire_format_live_tests.rs` | Live API format verification |
| `unknown_variant_tests.rs` | Unknown variant handling |

### Strict Mode Tests

Test behavior with `--features strict-unknown` which makes unknown types error instead of gracefully degrade.

```bash
cargo test --features strict-unknown
```

## Running Tests

### Quick Development Cycle

```bash
cargo test                                # Unit tests only (~5s)
```

### Full Test Suite

```bash
cargo test -- --include-ignored           # All tests (~2-5 min with API)
```

### Specific Categories

```bash
# By file
cargo test --test multiturn_tests -- --include-ignored

# By name pattern
cargo test function_calling -- --include-ignored

# With output
cargo test -- --include-ignored --nocapture
```

### Environment Variables

| Variable | Purpose |
|----------|---------|
| `GEMINI_API_KEY` | Required for integration tests |
| `RUST_LOG=genai_rs=debug` | Enable debug logging |
| `LOUD_WIRE=1` | Show raw HTTP request/response |

## CI Pipeline

The GitHub Actions workflow runs these jobs:

| Job | What it does |
|-----|--------------|
| `check` | `cargo check --workspace --all-targets --all-features` |
| `test` | Unit tests without API key |
| `test-strict-unknown` | Unit tests with `--features strict-unknown` |
| `test-integration` | 4 matrix groups with API key (see below) |
| `fmt` | Format check |
| `clippy` | Lint check |
| `doc` | Documentation build |
| `security` | `cargo audit` |
| `build-metrics` | Clean build time measurement |

### Integration Test Matrix

Tests are split into 4 groups to parallelize and isolate failures:

| Group | Tests |
|-------|-------|
| `core` | interactions_api, multiturn, streaming_multiturn |
| `tools` | tools_and_config, tools_multiturn, agents |
| `functions` | advanced_function_calling, thinking_function, multiturn_function |
| `multimodal` | multimodal, api_canary, temp_file |

## Test Utilities

The `tests/common/mod.rs` module provides shared utilities:

### Client Setup

```rust,ignore
mod common;
use common::*;

let client = get_client().expect("GEMINI_API_KEY must be set");
let response = interaction_builder(&client)
    .with_text("Hello")
    .create()
    .await?;
```

### Retry for Transient Errors

```rust,ignore
// Retry on known transient errors (Spanner UTF-8, etc.)
let response = retry_on_transient(3, || async {
    interaction_builder(&client)
        .with_text("Hello")
        .create()
        .await
}).await?;

// Or use the macro for cleaner syntax
let response = retry_request!([client] => {
    interaction_builder(&client).with_text("Hello").create().await
})?;
```

### Timeouts

```rust,ignore
use common::{TEST_TIMEOUT, with_timeout};

with_timeout(TEST_TIMEOUT, async {
    // Test logic that might hang
}).await;
```

### Stream Consumption

```rust,ignore
let stream = interaction_builder(&client)
    .with_text("Hello")
    .create_stream();

let result = consume_stream(stream).await;
assert!(result.has_output());
assert!(!result.collected_text.is_empty());
```

### Semantic Validation

For behavioral tests where exact output varies:

```rust,ignore
let is_valid = validate_response_semantically(
    &client,
    "User asked about weather in Tokyo",
    response.text().unwrap(),
    "Does the response mention Tokyo's weather?"
).await?;
assert!(is_valid);
```

## Writing New Tests

### Integration Test Template

```rust,ignore
//! Description of what this test file covers.

mod common;
use common::*;

#[tokio::test]
#[ignore = "Requires GEMINI_API_KEY"]
async fn test_feature_name() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    with_timeout(TEST_TIMEOUT, async {
        let response = interaction_builder(&client)
            .with_text("Test prompt")
            .create()
            .await
            .expect("Request should succeed");

        // Structural assertions (preferred)
        assert!(response.text().is_some());
        assert_eq!(response.status, InteractionStatus::Completed);
    }).await;
}
```

### Property Test Template

```rust,ignore
use proptest::prelude::*;
use crate::proptest_tests::*;

proptest! {
    #[test]
    fn roundtrip_my_type(value in my_type_strategy()) {
        let json = serde_json::to_string(&value).unwrap();
        let parsed: MyType = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(value, parsed);
    }
}
```

## Assertion Strategies

### Decision Flowchart

Use this to choose the right assertion type:

```text
Is it checking LLM-generated text content?
├── NO → Use structural assertions
└── YES → Is the expected value deterministic?
          ├── YES (error message, code execution result) → .contains() is OK
          └── NO (natural language response) → Use semantic validation
```

### Structural Assertions (Preferred)

Check API mechanics without depending on LLM output:

```rust,ignore
// Good - structural
assert!(response.text().is_some());
assert_eq!(response.status, InteractionStatus::Completed);
assert!(response.function_calls().len() > 0);
```

### Semantic Assertions (For LLM Output)

For behavioral tests where the LLM's response content matters:

```rust,ignore
let is_valid = validate_response_semantically(
    &client,
    "Context: user asked X, function returned Y",
    response.text().unwrap(),
    "Does the response incorporate Y?"
).await?;
assert!(is_valid, "Response should use function result");
```

**When to use semantic validation**:
- Multi-turn context preservation ("Does this recall the user's name?")
- Function result incorporation ("Does this use the weather data?")
- Factual correctness ("Does this identify Paris as the capital?")
- Content understanding ("Does this describe the image colors?")

**When NOT to use**:
- Status code verification
- Field presence checks
- Error message validation (deterministic strings)

### Anti-Patterns to Avoid

These patterns cause flaky tests because LLM output varies:

```rust,ignore
// BAD - Single keyword that may be rephrased
assert!(text.contains("paris"));
// Model might say "The capital is Paris", "Paris, France", or "It's Paris"

// BAD - OR chains trying to handle variability
assert!(text.contains("red") || text.contains("crimson") || text.contains("scarlet"));
// Still misses "reddish", "ruby", "a shade of red", etc.

// BAD - Partial match that's too specific
assert!(text.contains("hik"));  // Trying to catch "hiking"
// Misses "outdoor activities", "trekking", "walks"
```

**Correct approach**:

```rust,ignore
// GOOD - Semantic validation handles natural language variability
let is_valid = validate_response_semantically(
    &client,
    "Asked about the capital of France",
    text,
    "Does this response correctly identify Paris as the capital of France?"
).await?;
assert!(is_valid);

// GOOD - For color identification
let is_valid = validate_response_semantically(
    &client,
    "Showed a red image",
    text,
    "Does this response identify the color as red or a shade of red?"
).await?;
assert!(is_valid);
```

### Acceptable `.contains()` Usage

These patterns ARE appropriate because the values are deterministic:

```rust,ignore
// OK - Error messages from the library (deterministic strings)
assert!(error.to_string().contains("invalid API key"));

// OK - Code execution results (exact computed values)
assert!(text.contains("3628800"));  // factorial(10)
assert!(text.contains("24133"));    // sum of primes

// OK - JSON/schema structure checks
assert!(schema.contains("\"type\": \"string\""));

// OK - Format validation
assert!(email.contains("@"));
```

### Unknown Content Checks

For forward-compatibility testing:

```rust,ignore
// Verify no unknown content (canary test)
assert!(!response.has_unknown(),
    "API returned unknown types: {:?}",
    response.content_summary().unknown_types);

// Or handle gracefully
if response.has_unknown() {
    for (type_name, data) in response.unknown_content() {
        log::warn!("Unknown content type: {} = {:?}", type_name, data);
    }
}
```

## Test Data

### Minimal Test Assets

`tests/common/mod.rs` provides minimal valid test data:

| Constant | Description |
|----------|-------------|
| `TINY_RED_PNG_BASE64` | 1x1 red PNG |
| `TINY_BLUE_PNG_BASE64` | 1x1 blue PNG |
| `TINY_WAV_BASE64` | Minimal WAV header |
| `TINY_MP4_BASE64` | Minimal MP4 container |
| `TINY_PDF_BASE64` | "Hello World" PDF |

### Test Fixtures

```rust,ignore
use common::{DEFAULT_MODEL, interaction_builder, stateful_builder};

// Pre-configured with default model
let builder = interaction_builder(&client);

// Pre-configured for stateful conversations
let builder = stateful_builder(&client);
```

## Debugging Tests

### Enable Logging

```bash
RUST_LOG=genai_rs=debug cargo test test_name -- --nocapture
```

### See Wire Traffic

```bash
LOUD_WIRE=1 cargo test test_name -- --nocapture --include-ignored
```

### Run Single Test with Full Output

```bash
cargo test test_specific_feature -- --include-ignored --nocapture --test-threads=1
```
