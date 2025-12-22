# Test Coverage Analysis and Plan

**Last Updated:** 2025-12-22
**Branch:** claude/improve-test-coverage-bbg7s
**Overall Coverage:** ~75% lines (Target: 90%+)

## Executive Summary

The rust-genai project has a solid testing foundation with 86 passing unit tests and 14 integration tests. Recent additions of the Interactions API include excellent test coverage for SSE parsing and URL construction. However, critical infrastructure components (HTTP client, response deserialization) remain untested, posing reliability risks.

### Current State
- ✅ **Total Tests:** 100 (86 passing + 14 ignored integration tests)
- ✅ **Well-tested:** SSE parser, URL construction, builders, function calling
- ⚠️ **Critical gaps:** HTTP client core, response deserialization
- ⚠️ **Missing:** Mock-based unit tests for client methods

### Target State
- 🎯 **Coverage:** 90%+ across all modules
- 🎯 **Infrastructure:** Full coverage of HTTP client and response handling
- 🎯 **Reliability:** Comprehensive error scenario testing

---

## Coverage by Module

### genai-client Crate

| Module | LOC | Tests | Coverage | Priority |
|--------|-----|-------|----------|----------|
| `sse_parser.rs` | 172 | 7 | ⭐⭐⭐⭐⭐ Excellent | ✅ Complete |
| `common.rs` | 300 | 17 | ⭐⭐⭐⭐⭐ Excellent (100%) | ✅ Complete |
| `interactions.rs` | 126 | 3 | ⭐⭐⭐ Good | 🟡 Needs mocks |
| `models/interactions.rs` | 304 | 5 | ⭐⭐⭐ Good | 🟢 Adequate |
| `models/shared.rs` | 161 | 3 | ⭐⭐ Fair | 🟢 Adequate |
| `models/request.rs` | 85 | 2 | ⭐⭐⭐⭐⭐ Excellent (100%) | ✅ Complete |
| `models/response.rs` | 148 | 0 | ⭐⭐⭐⭐⭐ Excellent (100%)¹ | ✅ Complete |
| **`core.rs`** | 123 | **0** | 🔴 **0% Coverage** | 🔴 **CRITICAL** |

¹ Note: Despite showing 100% in old coverage report, needs additional edge case tests

### Main Crate

| Module | LOC | Tests | Coverage | Priority |
|--------|-----|-------|----------|----------|
| `request_builder.rs` | 906 | 31 | ⭐⭐⭐ Good (55.75%) | 🟡 Needs improvement |
| `client.rs` | 449 | 5² | ⭐⭐⭐ Good (76.92%)² | 🟡 Needs mocks |
| `function_calling.rs` | ~300 | Multiple | ⭐⭐⭐⭐ Very Good (76.58%) | 🟢 Adequate |
| `content_api.rs` | ~200 | Multiple | ⭐⭐⭐ Fair (63.77%) | 🟢 Adequate |
| `types.rs` | ~250 | Multiple | ⭐⭐⭐⭐⭐ Excellent (100%) | ✅ Complete |
| `internal/response_processing.rs` | ~200 | 6 | ⭐⭐⭐⭐⭐ Excellent (97.92%) | ✅ Complete |
| `lib.rs` | ~70 | 2 | ⭐⭐⭐⭐⭐ Excellent (90.62%) | ✅ Complete |

² Integration tests only (require API key); 76.92% line coverage but only 36.84% region/branch coverage

---

## Critical Gaps (Must Fix)

### 1. 🔴 `genai-client/src/core.rs` - 0% COVERAGE

**Impact:** CRITICAL - Core HTTP client is completely untested
**Risk:** Network errors, API changes, authentication issues could go undetected
**Effort:** Medium (requires HTTP mocking setup)

**Missing Coverage:**
- ✗ `generate_content_internal()` - Basic content generation
- ✗ `generate_content_stream_internal()` - Streaming generation
- ✗ HTTP error handling (401, 429, 500, 503)
- ✗ Network failures (timeout, connection refused, DNS)
- ✗ Malformed API responses
- ✗ Request header construction
- ✗ System instruction handling

**Recommended Tests (8-10):**
1. Successful non-streaming request with valid response
2. Successful streaming request with multiple chunks
3. Authentication error (401 Unauthorized)
4. Rate limiting (429 Too Many Requests)
5. Server error (500 Internal Server Error)
6. Network timeout scenario
7. Connection refused scenario
8. Malformed JSON response
9. Empty response body
10. Large response handling (>1MB)

**Implementation Approach:**
- Use `wiremock` or `mockito` for HTTP mocking
- Create test fixtures for common API responses
- Test both success and failure paths

---

### 2. 🟡 `genai-client/src/models/response.rs` - Needs Edge Case Tests

**Impact:** HIGH - Response parsing failures could crash applications
**Risk:** API changes, unexpected formats, missing fields
**Effort:** Low (pure deserialization tests)

**Current:** Shows 100% coverage in old report, but lacks edge case testing
**Missing Coverage:**
- ✗ Empty/minimal responses with only required fields
- ✗ Responses with missing optional fields
- ✗ Malformed JSON handling
- ✗ Invalid data types
- ✗ Very large responses
- ✗ Response with multiple candidates

**Recommended Tests (5-7):**
1. Minimal response with only required fields
2. Response with multiple candidates
3. Response with various finish reasons
4. Malformed JSON (missing required fields)
5. Invalid data types in fields
6. Empty candidates array
7. Missing usage metadata

---

### 3. 🟡 `src/request_builder.rs` - 55.75% Line Coverage

**Impact:** MEDIUM - Complex builder with many untested paths
**Risk:** Configuration errors, invalid requests
**Effort:** Low-Medium (expand existing test patterns)

**Current:** 31 tests, but only 55.75% line coverage
**Missing Coverage:**
- ✗ Auto-function execution edge cases
- ✗ Function call loop reaching MAX_FUNCTION_CALL_LOOPS (5)
- ✗ Function execution errors during auto-execution
- ✗ Function not found in registry
- ✗ Complex tool configurations
- ✗ InteractionBuilder stream error propagation

**Recommended Tests (8-12):**
1. Auto-function loop reaching max iterations (5)
2. Function execution error during loop
3. Function not found in global registry
4. Multiple functions called in sequence
5. InteractionBuilder: both model AND agent set (should error)
6. InteractionBuilder: complex content input
7. InteractionBuilder: response format with JSON schema
8. InteractionBuilder: stream error scenarios
9. GenerateContentBuilder: very large function lists
10. GenerateContentBuilder: invalid tool configurations

---

### 4. 🟡 `src/client.rs` - 76.92% Lines, 36.84% Regions

**Impact:** MEDIUM - Good line coverage but poor branch coverage
**Risk:** Untested error paths, edge cases
**Effort:** Medium (mock HTTP responses)

**Current:** Integration tests only (require API key)
**Missing Coverage:**
- ✗ Mock-based unit tests for interactions methods
- ✗ Error handling branches (explains 36.84% region coverage)
- ✗ Stream interruption scenarios
- ✗ API key validation edge cases

**Recommended Tests (6-8):**
1. Create interaction with mocked success response
2. Create streaming interaction with mocked SSE stream
3. Get interaction with valid ID
4. Delete interaction success scenario
5. Error handling for 404 (interaction not found)
6. Error handling for authentication failures
7. Stream interruption and recovery
8. Timeout scenarios

---

### 5. 🟢 `src/content_api.rs` - 63.77% Coverage

**Impact:** LOW - Helper functions, mostly tested via integration
**Risk:** Edge cases with special characters or invalid inputs
**Effort:** Low (add unit tests for edge cases)

**Recommended Tests (3-5):**
1. Empty string handling in `user_text()` and `model_text()`
2. Special characters and Unicode in text content
3. Very large text inputs (multi-megabyte strings)
4. Invalid function call structures
5. Tool response serialization edge cases

---

## Implementation Plan

### Phase 1: Critical Infrastructure (Priority: CRITICAL)

**Goal:** Achieve 90%+ coverage on core HTTP client

**Tasks:**
- [ ] **1.1** Setup HTTP mocking infrastructure
  - Add `wiremock` to dev dependencies
  - Create test fixtures directory structure
  - Write helper functions for common mock setups

- [ ] **1.2** Test `genai-client/src/core.rs` (8-10 tests)
  - Success scenarios (streaming + non-streaming)
  - HTTP error codes (401, 429, 500, 503)
  - Network errors (timeout, connection refused)
  - Malformed responses

- [ ] **1.3** Add edge case tests for `models/response.rs` (5-7 tests)
  - Minimal responses
  - Multiple candidates
  - Missing optional fields
  - Error cases

- [ ] **1.4** Verify Phase 1
  - Run: `cargo test -- --include-ignored`
  - Commit: "test: Add comprehensive tests for HTTP client and response parsing"

### Phase 2: Builder and Client Methods (Priority: HIGH)

**Goal:** Improve builder coverage to 80%+, add mock tests for client

**Tasks:**
- [ ] **2.1** Test auto-function execution (5-8 tests)
  - Function call loops
  - Max iterations
  - Function errors
  - Registry lookups

- [ ] **2.2** Add InteractionBuilder edge cases (3-5 tests)
  - Complex content types
  - Validation errors
  - Stream errors

- [ ] **2.3** Add mock tests for Client interactions methods (4-6 tests)
  - Create interaction (success + error)
  - Stream interaction (with mocked SSE)
  - Get interaction (success + 404)
  - Delete interaction

- [ ] **2.4** Verify Phase 2
  - Run: `cargo test -- --include-ignored`
  - Commit: "test: Improve builder and client test coverage"

### Phase 3: Edge Cases and Polish (Priority: MEDIUM)

**Goal:** Comprehensive edge case coverage, reach 90%+ overall

**Tasks:**
- [ ] **3.1** content_api.rs edge cases (3-5 tests)
- [ ] **3.2** SSE parser stress tests (2-3 tests, optional)
- [ ] **3.3** Error handling scenarios (3-4 tests)
- [ ] **3.4** Final verification and documentation update

---

## Testing Best Practices

### Test Organization

```
tests/
├── fixtures/           # JSON fixtures for API responses
│   ├── responses/
│   ├── requests/
│   └── sse_streams/
├── helpers/           # Test helper functions (if needed)
│   ├── mock_http.rs
│   └── fixtures.rs
└── *.rs              # Test files (one per feature area)
```

### Mocking Strategy

1. **HTTP Layer:** Use `wiremock` for HTTP mocking
2. **Function Registry:** Use test functions with `#[generate_function_declaration]`
3. **API Responses:** Create JSON fixtures for realistic responses
4. **Streaming:** Mock SSE streams with chunked data

### Test Naming Convention

```rust
#[test]
fn test_<component>_<scenario>_<expected>() {
    // Example: test_generate_content_with_auth_error_returns_401
}
```

---

## Success Criteria

### Definition of Done

- ✅ All critical gaps (core.rs) have comprehensive tests
- ✅ Builder coverage improved to 80%+
- ✅ `cargo test -- --include-ignored` passes with 0 failures
- ✅ `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- ✅ Coverage reaches 85-90% across core modules
- ✅ Tests execute in <5 seconds (excluding integration tests)

### Target Metrics

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Total Tests | 100 | 125+ | 🟡 In Progress |
| Unit Tests | 86 | 105+ | 🟡 In Progress |
| Integration Tests | 14 | 20 | 🟢 Adequate |
| `core.rs` Coverage | 0% | 90%+ | 🔴 Critical |
| `request_builder.rs` Coverage | 55.75% | 80%+ | 🟡 In Progress |
| `client.rs` Region Coverage | 36.84% | 70%+ | 🟡 In Progress |
| Overall Line Coverage | 75.14% | 85-90% | 🟡 In Progress |

---

## Dependencies Required

### Test Dependencies to Add

```toml
[dev-dependencies]
# Existing
tokio = { version = "1", features = ["full"] }
serde_json = "1.0"

# New - for HTTP mocking
wiremock = "0.6"

# Optional - for property-based testing (Phase 3+)
proptest = "1.4"
```

---

## Code Coverage Report (Baseline)

From previous `cargo llvm-cov` run:

```
Module                                   Lines    Coverage
--------------------------------------------------------
genai-client/src/common.rs                 66      100.00%  ✅
genai-client/src/core.rs                  124        0.00%  🔴 CRITICAL
genai-client/src/models/request.rs         55      100.00%  ✅
genai-client/src/models/response.rs        81      100.00%  ✅
rust-genai-macros/src/lib.rs             333       88.29%  ✅
src/client.rs                             156       76.92%  🟡 (regions: 36.84%)
src/content_api.rs                         69       63.77%  🟡
src/function_calling.rs                   111       76.58%  🟡
src/internal/response_processing.rs       192       99.48%  ✅
src/lib.rs                                 68       98.53%  ✅
src/request_builder.rs                    348       55.75%  🟡
src/types.rs                               30      100.00%  ✅
--------------------------------------------------------
TOTAL                                    1633       75.14%
```

**Key Takeaway:** The HTTP client core (0%) and request builder (55.75%) are the primary targets for improvement.

---

## Test Examples

### Example 1: HTTP Client Test with Mocking

```rust
#[tokio::test]
async fn test_generate_content_success() {
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    let mock_server = MockServer::start().await;

    let response_json = json!({
        "candidates": [{
            "content": {
                "parts": [{"text": "Hello, world!"}],
                "role": "model"
            }
        }]
    });

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-pro:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_json))
        .mount(&mock_server)
        .await;

    // Test core.rs functions with mock server URL
}
```

### Example 2: Auto-Function Max Iterations Test

```rust
#[tokio::test]
async fn test_auto_functions_reaches_max_iterations() {
    // Register a function that always triggers another call
    // Mock API to always return function calls
    // Verify loop terminates after MAX_FUNCTION_CALL_LOOPS
    // Verify appropriate error/warning is generated
}
```

---

## Running Tests

```bash
# Run all tests (excludes ignored integration tests)
cargo test

# Run all tests including ignored (requires GEMINI_API_KEY)
cargo test -- --include-ignored

# Run specific test file
cargo test --test integration_tests

# Run with output visible
cargo test -- --nocapture

# Run tests with coverage
cargo llvm-cov --all-features --workspace

# Run clippy
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

---

## Next Steps

1. ✅ Review and approve this test plan
2. 🔄 **START:** Phase 1 - Critical Infrastructure tests
3. ⏳ Phase 2 - Builder and client methods
4. ⏳ Phase 3 - Edge cases and polish

**Estimated Timeline:** 1-2 weeks for Phases 1-2 (critical gaps)

---

*Last updated: 2025-12-22 - This document will be updated as test coverage improves.*
