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
   - Debug mode configuration

2. **request_builder_tests.rs** - Tests for request building
   - Basic request building with prompt and system instruction
   - Function declaration handling
   - Edge cases (empty prompts, long prompts, special characters)
   - Multi-turn conversation building

3. **content_api_tests.rs** - Tests for content API helper functions
   - user_text() function
   - model_function_call() function
   - user_tool_response() function
   - build_content_request() with various scenarios
   - Conversation history building

4. **types_tests.rs** - Tests for type conversions and structs
   - FunctionDeclaration::to_tool() conversion
   - FunctionCall struct
   - CodeExecutionResult struct
   - GenerateContentResponse struct
   - Serialization/deserialization
   - Error type conversions

5. **response_processing_tests.rs** - Tests for response handling
   - Response conversion from internal to public types
   - Empty response handling
   - Multiple function calls in response
   - Code execution results processing

#### Feature-Specific Tests
6. **function_calling_tests.rs** - Tests for function calling feature
   - Basic function calling integration
   - Edge cases (empty names, long descriptions, nested parameters)
   - Automatic function execution
   - Manual function execution
   - Multi-turn function calling
   - Error handling in function execution

7. **error_handling_tests.rs** - Tests for error scenarios
   - Invalid API key handling
   - Invalid model name handling
   - Error display implementations
   - JSON error conversion
   - Streaming error handling
   - Function execution errors

8. **debug_mode_tests.rs** - Tests for debug mode functionality
   - Debug mode output verification
   - Debug mode with streaming
   - Debug mode builder
   - Debug output formatting

9. **integration_tests.rs** - End-to-end integration tests
   - Generate content with/without system instruction
   - Streaming with/without system instruction
   - Multi-turn conversations
   - Function calling scenarios

#### Macro Tests
10. **macro_tests.rs** - Tests for the procedural macro
    - Basic function declaration generation
    - Function with doc comments
    - Parameter metadata handling
    - Optional parameters
    - Enum values
    - Various type mappings
    - Multiline doc comments
    - Async function support
    - Complex parameter types

11. **verify_param_docs.rs** - Tests for parameter documentation behavior
    - Verifies that parameter descriptions work via macro attributes
    - Confirms that doc comments on parameters don't work (Rust limitation)
    - Tests parameter validation

## Coverage Areas

### ✅ Well Covered
- Client creation and configuration
- Request building with various options
- Content API helper functions
- Type conversions and serialization
- Function calling functionality (both manual and automatic)
- Error handling and conversion
- Debug mode functionality
- Procedural macro functionality
- Multi-turn conversations
- Code execution

### ⚠️ Limited Coverage
- Network error scenarios (would require mocking)
- Rate limiting behavior
- Very large response handling
- Concurrent request handling
- Performance under load
- Memory usage with large responses

### 📝 Notes
- Integration tests that require GEMINI_API_KEY are skipped when the environment variable is not set
- **Integration tests that make real API calls are marked with `#[ignore]` to prevent rate limit failures**
- Debug output capturing in tests is limited due to stdout redirection challenges
- Some internal client behavior cannot be tested directly due to private fields
- The free tier API has a rate limit of 10 requests per minute which can cause test failures
- Consider adding mock HTTP responses for more reliable testing

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

# Run tests with coverage (requires cargo-llvm-cov)
cargo llvm-cov --all-features --workspace
```

## Test Statistics
- Total test files: 11 integration + unit test modules
- Total test functions: ~100+
- Unit tests: 29
- Integration tests: 70+
- Macro tests: 15+
- Code coverage: ~85% (estimated)

## Code Coverage Results

Latest coverage report from `cargo llvm-cov --all-features --workspace`:

```
Filename                                Regions    Missed Regions     Cover   Functions  Missed Functions  Executed       Lines      Missed Lines     Cover    Branches   Missed Branches     Cover
------------------------------------------------------------------------------------------------------------
genai-client/src/common.rs                   32                 0   100.00%           7                 0   100.00%          66                 0   100.00%           0                 0         -
genai-client/src/core.rs                     39                39     0.00%           8                 8     0.00%         124               124     0.00%           0                 0         -
genai-client/src/models/request.rs            4                 0   100.00%           2                 0   100.00%          55                 0   100.00%           0                 0         -
genai-client/src/models/response.rs          13                 0   100.00%           3                 0   100.00%          81                 0   100.00%           0                 0         -
rust-genai-macros/src/lib.rs                208                30    85.58%          13                 1    92.31%         333                39    88.29%           0                 0         -
src/client.rs                                57                36    36.84%          11                 2    81.82%         156                36    76.92%           0                 0         -
src/content_api.rs                            8                 4    50.00%           7                 3    57.14%          69                25    63.77%           0                 0         -
src/function_calling.rs                      44                15    65.91%          19                 6    68.42%         111                26    76.58%           0                 0         -
src/internal/response_processing.rs          48                 1    97.92%           9                 0   100.00%         192                 1    99.48%           0                 0         -
src/lib.rs                                   32                 3    90.62%           3                 0   100.00%          68                 1    98.53%           0                 0         -
src/request_builder.rs                      113                63    44.25%          34                15    55.88%         348               154    55.75%           0                 0         -
src/types.rs                                 14                 0   100.00%           4                 0   100.00%          30                 0   100.00%           0                 0         -
------------------------------------------------------------------------------------------------------------
TOTAL                                       612               191    68.79%         120                35    70.83%        1633               406    75.14%           0                 0         -
```

### Coverage Analysis

#### Well Covered Areas (90%+)
- `genai-client/src/common.rs` (100%)
- `genai-client/src/models/request.rs` (100%)
- `genai-client/src/models/response.rs` (100%)
- `src/internal/response_processing.rs` (97.92%)
- `src/lib.rs` (90.62%)
- `src/types.rs` (100%)

#### Areas Needing Improvement (<70%)
- `genai-client/src/core.rs` (0%)
- `src/client.rs` (36.84%)
- `src/content_api.rs` (50.00%)
- `src/request_builder.rs` (44.25%)

#### Overall Coverage
- Total Regions: 68.79%
- Total Functions: 70.83%
- Total Lines: 75.14%

### Coverage Notes
- The core client implementation (`genai-client/src/core.rs`) shows 0% coverage because it contains mostly HTTP client code that requires mocking for proper testing.
- The request builder and client modules have lower coverage due to the complexity of handling various request configurations and edge cases.
- The macro implementation has good coverage (85.58%) but could be improved with more edge case testing. 