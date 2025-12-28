# Testing Assistant Example

An AI-powered test generation assistant that creates unit tests, analyzes coverage, and suggests property-based tests.

## Overview

This example demonstrates a testing assistant that:

1. **Test Suite Generation**: Creates comprehensive tests from code
2. **Coverage Analysis**: Identifies gaps in existing tests
3. **Property-Based Tests**: Suggests invariants for property testing
4. **Specific Scenarios**: Generates targeted tests on demand

## Features

### Multi-Language Support

Adapts to language-specific testing frameworks:

| Language | Framework | Style |
|----------|-----------|-------|
| Rust | Built-in `#[test]` | `assert!`, `assert_eq!` |
| Python | pytest | `assert`, fixtures |
| JavaScript | Jest | `describe`, `it`, `expect` |

### Structured Output

Tests are returned as parseable structures:

```rust
struct TestSuite {
    module_name: String,
    imports: Vec<String>,
    test_cases: Vec<TestCase>,
    setup_code: Option<String>,
}

struct TestCase {
    name: String,
    description: String,
    category: String,  // unit, edge_case, integration, property
    test_code: String,
    assertions: Vec<String>,
}
```

## Running

```bash
export GEMINI_API_KEY=your_api_key
cargo run --example testing_assistant
```

## Sample Output

```
🧪 GENERATE TEST SUITE

Generated Test Suite: stack_tests
========================

Test Cases (6 total):

1. test_new_creates_empty_stack [unit]
   Description: Verify new stack is empty with correct max_size
   Assertions: is_empty() returns true, len() returns 0
   Code:
   | #[test]
   | fn test_new_creates_empty_stack() {
   |     let stack: Stack<i32> = Stack::new(5);
   |     assert!(stack.is_empty());
   |     assert_eq!(stack.len(), 0);
   | }

2. test_push_overflow [edge_case]
   Description: Pushing beyond max_size returns error
   ...
```

## API Usage

### Generate Full Test Suite

```rust
let suite = assistant.generate_test_suite(code, "rust").await?;
for test in suite.test_cases {
    println!("{}: {}", test.name, test.description);
}
```

### Analyze Coverage Gaps

```rust
let analysis = assistant.analyze_coverage(code, tests, "rust").await?;
println!("Missing: {:?}", analysis.missing_scenarios);
println!("Edge cases: {:?}", analysis.edge_cases);
```

### Suggest Property Tests

```rust
let ideas = assistant.suggest_property_tests(code, "rust").await?;
for idea in ideas {
    println!("Property: {}", idea.property_name);
    println!("Invariant: {}", idea.expected_invariant);
}
```

### Generate Specific Test

```rust
let test = assistant.generate_single_test(
    code,
    "Test empty stack pop returns None",
    "rust"
).await?;
```

## Test Categories

The assistant generates tests across categories:

- **Unit Tests**: Basic functionality verification
- **Edge Cases**: Boundary conditions, empty inputs
- **Integration Tests**: Component interaction
- **Property Tests**: Invariants that hold for all inputs

## Property-Based Testing Ideas

For a Stack implementation, suggested properties might include:

| Property | Invariant |
|----------|-----------|
| Push-Pop | `push(x); pop()` returns `x` |
| Length | `push` increases len by 1 |
| Overflow | Stack never exceeds max_size |
| Empty | New stack is always empty |

## Coverage Analysis

Identifies testing gaps:

```
✅ Covered Scenarios:
   • Basic push/pop operations
   • Empty stack check

❌ Missing Scenarios:
   • Stack overflow handling
   • peek() on empty stack
   • Multiple consecutive operations

⚠️ Edge Cases to Add:
   • Zero max_size
   • Single element stack
   • Fill and empty completely
```

## Production Enhancements

### CI/CD Integration

```yaml
# Generate tests on PR
on: pull_request
jobs:
  generate-tests:
    steps:
      - run: cargo run --example testing_assistant -- $CHANGED_FILES
```

### Mutation Testing

```rust
// Suggest mutations to verify test effectiveness
async fn suggest_mutations(&self, code: &str) -> Vec<Mutation> {
    // Return code changes that tests should catch
}
```

### Mock Generation

```rust
async fn generate_mocks(&self, interfaces: &str) -> String {
    // Create mock implementations for testing
}
```

### Test Fixtures

```rust
async fn generate_fixtures(&self, data_types: &str) -> String {
    // Create test data builders and fixtures
}
```

## Best Practices

1. **Review Generated Tests**: AI tests need human verification
2. **Customize for Domain**: Add domain-specific edge cases
3. **Maintain Readability**: Refactor generated tests as needed
4. **Track Coverage**: Use coverage tools alongside generation
5. **Iterate**: Use coverage analysis to guide generation

## Limitations

- Generated tests may not compile without adjustments
- Complex business logic may need manual test design
- Property tests need compatible testing framework
- Test quality depends on code clarity
