# Test Coverage Summary

This document provides an overview of the test coverage for the rust-genai library.

## Test Files Overview

### Unit Tests
- **src/lib.rs** - Tests for error conversion and public response struct (2 tests)
- **src/internal/response_processing.rs** - Tests for response part processing logic (6 tests)
- **src/request_builder.rs** - Tests for request builder and function declaration conversion (11 tests)
- **genai-client/src/common.rs** - Tests for URL construction and API versions (5 tests)  
- **genai-client/src/models/** - Internal serialization/deserialization tests (5 tests)

### Integration Tests

#### Core Functionality
1. **client_tests.rs** - Tests for Client creation and builder pattern
   - Client::builder() with various options
   - Client::new() with different API versions
   - with_model() method

2. **request_builder_tests.rs** - Tests for request building
   - Basic request building with prompt and system instruction
   - Function declaration handling
   - Edge cases (empty prompts, long prompts, special characters)

3. **content_api_tests.rs** - Tests for content API helper functions
   - user_text() function
   - model_function_call() function
   - user_tool_response() function
   - build_content_request() with various scenarios

4. **types_tests.rs** - Tests for type conversions and structs
   - FunctionDeclaration::to_tool() conversion
   - FunctionCall struct
   - CodeExecutionResult struct
   - GenerateContentResponse struct
   - Serialization/deserialization

5. **response_processing_tests.rs** - Tests for response handling
   - Response conversion from internal to public types
   - Empty response handling
   - Multiple function calls in response

#### Feature-Specific Tests
6. **function_calling_tests.rs** - Tests for function calling feature
   - Basic function calling integration
   - Edge cases (empty names, long descriptions, nested parameters)

7. **error_handling_tests.rs** - Tests for error scenarios
   - Invalid API key handling
   - Invalid model name handling
   - Error display implementations
   - JSON error conversion
   - Streaming error handling

8. **debug_mode_tests.rs** - Tests for debug mode functionality
   - Debug mode output verification
   - Debug mode with streaming
   - Debug mode builder

9. **integration_tests.rs** - End-to-end integration tests
   - Generate content with/without system instruction
   - Streaming with/without system instruction

#### Macro Tests
10. **macro_tests.rs** - Tests for the procedural macro
    - Basic function declaration generation
    - Function with doc comments
    - Parameter metadata handling
    - Optional parameters
    - Enum values
    - Various type mappings
    - Multiline doc comments

11. **verify_param_docs.rs** - Tests for parameter documentation behavior
    - Verifies that parameter descriptions work via macro attributes
    - Confirms that doc comments on parameters don't work (Rust limitation)

## Coverage Areas

### ✅ Well Covered
- Client creation and configuration
- Request building with various options
- Content API helper functions
- Type conversions and serialization
- Function calling functionality
- Error handling and conversion
- Debug mode functionality
- Procedural macro functionality

### ⚠️ Limited Coverage
- Network error scenarios (would require mocking)
- Rate limiting behavior
- Very large response handling
- Concurrent request handling

### 📝 Notes
- Integration tests that require GEMINI_API_KEY are skipped when the environment variable is not set
- **Integration tests that make real API calls are marked with `#[ignore]` to prevent rate limit failures**
- Debug output capturing in tests is limited due to stdout redirection challenges
- Some internal client behavior cannot be tested directly due to private fields
- The free tier API has a rate limit of 10 requests per minute which can cause test failures

## Running Tests

```bash
# Run all tests (excludes ignored integration tests)
cargo test

# Run specific test file
cargo test --test client_tests

# Run ignored integration tests (requires API key)
GEMINI_API_KEY=your_key cargo test --all -- --ignored

# Run integration tests with delays to avoid rate limits
./tests/run_integration_tests.sh

# Run tests with output
cargo test --all -- --nocapture
```

## Test Statistics
- Total test files: 11 integration + unit test modules
- Total test functions: ~80+
- Unit tests: 29
- Integration tests: 50+
- Macro tests: 10+ 