# Wire Format Verification: Workflow Improvements

## Executive Summary

Recent work (commits `ae52786` → `57a0eea`, Jan 10-14) identified **5+ wire format mismatches** between Google's API documentation and actual responses:

| Issue | Hallucinated | Actual | Detection Method | Time to Fix |
|-------|--------------|--------|------------------|-------------|
| `CodeExecutionResult.outcome` enum | `outcome: CodeExecutionOutcome` | `is_error: bool, result: String` | Manual `LOUD_WIRE=1` testing | 2 days |
| `Thought.text` field | `text: String` | `signature: String` (cryptographic) | Manual inspection + testing | 1 day |
| `FunctionCall.thought_signature` | Field existed | Never sent by API | Code review + testing | 1 day |
| `UrlContextCall.url` (singular) | Single URL field | Nested `arguments.urls` array | Manual parsing verification | 1 day |
| `CodeExecutionCall` arguments | Flat fields | Nested `arguments` object | Wire format inspection | 1 day |
| Multiple field case mismatches | camelCase assumptions | snake_case actual | Integration test verification | 3+ days |

**Root cause**: Documentation discrepancies + type system allows invalid structures during development.

**Cost**:
- 5 context windows for investigation and fixes
- Breaking changes in 0.5.2 release
- Integration tests discovered issues late (doc-based architecture assumptions persisted)

## Problem Analysis

### 1. Why Wire Format Mismatches Exist

#### 1a. Documentation Drift
Google's official API docs (`interactions.md.txt`, etc.) contain errors:
- Suggest `CodeExecutionResult.outcome` enum (doesn't exist)
- Show nested structure for `SpeechConfig.voiceConfig.prebuiltVoiceConfig` (fails with 400)
- State `ThinkingSummaries` uses lowercase `"auto"`/`"none"` (actually SCREAMING_CASE)

**Impact**: Type definitions can be correct-looking but incompatible with actual API.

#### 1b. Type System Too Permissive
Rust's serde allows:
- Optional fields to remain unset (hallucinated `thought_signature` never populated)
- Enum definitions that don't match wire format (assumed `CodeExecutionOutcome` was "just documentation")
- camelCase/snake_case mismatches silently deserialize due to `#[serde(rename_all)]`

**Impact**: Code compiles and unit tests pass despite API incompatibility.

#### 1c. Manual Testing Friction
Current verification workflow requires:
```bash
# Manual manual manual process
LOUD_WIRE=1 cargo run --example code_execution  # Inspect actual API response
# Parse JSON by eye, compare to docs
# Update types, repeat
```

Not automated, easy to skip for "obvious" fields.

#### 1d. Integration Tests Verify Late
- Tests pass with mock/cached responses
- Real API integration tests run in CI, not during development
- By the time tests fail, code is already broken in release

### 2. Where Detection Fails Today

**Detection method → Detection timing:**
1. **Code inspection**: ✅ Fast, ❌ Misses API changes (e.g., `outcome` looks reasonable in code)
2. **Unit tests**: ✅ Fast, ❌ Only test serialization logic, not API contract
3. **Wire format docs**: ✅ Reference, ❌ docs are often wrong (verified issue)
4. **Integration tests**: ✅ Authoritative, ❌ Late (only after release or CI failure)
5. **Peer review**: ✅ Some catch issues, ❌ Requires API expertise to review serde code
6. **Manual LOUD_WIRE testing**: ✅ Finds real issues, ❌ Requires explicit developer action

**Gap**: No automated boundary between "looks reasonable" and "API-compatible".

### 3. Session Context Windows

Investigation spanned 5 context windows:
1. Initial discovery: `LOUD_WIRE=1 cargo run --example code_execution` → `outcome` field mismatch
2. Structural investigation: Audit all `InteractionContent` fields (1.5 windows)
3. Documentation review: Verify against official API docs (0.5 windows)
4. Implementation: Fix types, update examples, 22 file changes
5. Test verification: Run `--include-ignored` tests to confirm fixes

**Loss**: Each window required re-context on git history, CLAUDE.md notes, documentation state.

## Recommended Improvements

### TIER 1: Immediate (Prevents Future Hallucinations)

#### 1.1 Wire Format Verification Test Template

**Problem**: Easy to add new fields without verifying wire format.

**Solution**: Create structured test template with required checks. File: `tests/wire_format_verification_template.md`

```markdown
# Wire Format Verification Checklist

When adding/modifying a field or enum in InteractionContent, ApiRequest, or ApiResponse:

## 1. Documentation Phase
- [ ] Consult official Google docs (see `CLAUDE.md` for URLs)
- [ ] If docs contradict each other, flag as "UNVERIFIED"
- [ ] Link to specific section in docs

## 2. Type Definition Phase
- [ ] Define Rust type
- [ ] Document expected wire format in docstring
  ```rust
  /// Wire format: `{"type": "code_execution_result", "is_error": bool, "result": string}`
  /// Note: Official docs mention `outcome` enum - this is INCORRECT
  ```
- [ ] Mark as `#[non_exhaustive]` if enum
- [ ] Add `Unknown` variant with context field if enum

## 3. Serialization Phase
- [ ] Add test in `src/content_tests.rs` for manual round-trip
- [ ] Test both directions: Rust → JSON and JSON → Rust
- [ ] Verify case convention (lowercase/SCREAMING_CASE/snake_case/camelCase)

## 4. Real API Phase (REQUIRED)
- [ ] Run `LOUD_WIRE=1 cargo run --example <your_example>` against live API
- [ ] Compare JSON output field-by-field:
  - [ ] Field names match (case-sensitive)
  - [ ] Field types match (bool vs enum, nested vs flat)
  - [ ] Optional fields are actually optional
  - [ ] No hallucinated fields appear
- [ ] Document verification in `docs/ENUM_WIRE_FORMATS.md`
  ```markdown
  **Verified**: 2026-01-14 - Captured from `LOUD_WIRE=1 cargo run --example code_execution`
  ```

## 5. Documentation Phase (Final)
- [ ] Update `docs/ENUM_WIRE_FORMATS.md` with:
  - Example JSON from actual API call
  - Rust field → Wire field mapping table
  - Any discrepancies from official docs (with explanation)
- [ ] Update `CHANGELOG.md` if it's a breaking change
- [ ] Link PR to issue if one exists

## Common Pitfalls to Check
- [ ] serde `rename_all` matches actual case
- [ ] Optional fields have `Option<T>` type
- [ ] Nested objects use `#[serde(flatten)]` correctly
- [ ] Enums use `#[serde(rename = "VALUE")]` not assumptions
- [ ] Unknown variants preserve JSON with `serde_json::Value`
```

**Location**: `/home/evansenter/Documents/projects/genai-rs/tests/wire_format_verification_template.md`

**Impact**: Makes wire format verification explicit in PR workflow (reviewers check against template).

#### 1.2 Pre-Submit Check: LOUD_WIRE for New Enums

**Problem**: Easy to forget manual testing before pushing.

**Solution**: Add pre-commit hook or CI check for new enum types.

File: `.git/hooks/pre-push` (local development aid)

```bash
#!/bin/bash

# Check if new enum types were added without LOUD_WIRE verification
# This is a local development aid, not CI enforcement

CHANGED_ENUMS=$(git diff --cached HEAD -- 'src/content.rs' 'src/request.rs' 'src/response.rs' | \
  grep -E '^\+.*\benum\b' | grep -v '^+++' | wc -l)

if [ "$CHANGED_ENUMS" -gt 0 ]; then
  echo "WARNING: Found new enums in this push. Run the following before pushing:"
  echo ""
  echo "  LOUD_WIRE=1 cargo test -- --include-ignored 2>&1 | grep -A5 'wire format'"
  echo ""
  echo "And check against docs/ENUM_WIRE_FORMATS.md"
  echo ""
  read -p "Continue anyway? (y/n) " -n 1 -r
  echo
  [[ ! $REPLY =~ ^[Yy]$ ]] && exit 1
fi
```

**Better approach**: CI check in GitHub Actions

File: `.github/workflows/wire-format-check.yml`

```yaml
name: Wire Format Check

on: [pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Check for new enum types
        run: |
          # Find new enum definitions in this PR
          NEW_ENUMS=$(git diff origin/main...HEAD -- 'src/content.rs' 'src/request.rs' | \
            grep -E '^\+.*\benum\b' | grep -v '^+++' || true)

          if [ -n "$NEW_ENUMS" ]; then
            echo "⚠️  New enum types detected:"
            echo "$NEW_ENUMS"
            echo ""
            echo "Please verify wire format using LOUD_WIRE=1"
            echo "See tests/wire_format_verification_template.md"
          fi

      - name: Check ENUM_WIRE_FORMATS.md updated
        run: |
          if git diff origin/main...HEAD -- docs/ENUM_WIRE_FORMATS.md | grep -q '^+'; then
            echo "✅ docs/ENUM_WIRE_FORMATS.md updated in this PR"
          else
            NEW_ENUMS=$(git diff origin/main...HEAD -- 'src/content.rs' 'src/request.rs' | \
              grep -E '^\+.*\benum\b' | grep -v '^+++' || true)
            if [ -n "$NEW_ENUMS" ]; then
              echo "⚠️  New enum types but docs/ENUM_WIRE_FORMATS.md not updated"
              echo "Did you verify the wire format?"
            fi
          fi
```

**Impact**: Prevents PRs with new types from merging without explicit wire format verification.

#### 1.3 Structured Documentation: Official vs. Actual

**Problem**: When docs are wrong, developers don't know which to trust.

**Solution**: Add validation section to `docs/ENUM_WIRE_FORMATS.md` with conflict tracking.

Current state (already done, improve further):

```markdown
### ThinkingSummaries (agent_config)

**Verified**: 2026-01-04 - API returned `"unknown enum value: 'auto'"` until we tested the fully-qualified format.

**Documentation Conflict:**
| Source | Claims | Actual | Reason Discrepancy Exists |
|--------|--------|--------|---------------------------|
| Google official docs | lowercase `"auto"`, `"none"` | SCREAMING_CASE `"THINKING_SUMMARIES_AUTO"` | Docs may be from older API version |
| Proto schema hints | Qualfied names typical | SCREAMING_CASE confirmed | Consistent with other enums |

**Resolution**: Always use SCREAMING_CASE for AgentConfig, lowercase for GenerationConfig.
**Test**: `cargo test thinking_summaries -- --include-ignored`
```

Update for all types (especially those marked UNVERIFIED):

```markdown
### Computer Use Tool (UNVERIFIED)

**Status**: Unverified - No live API access to test yet

**Documentation Claim:**
```json
{
  "tools": [{
    "type": "computer_use",
    "environment": "browser",
    "excludedPredefinedFunctions": ["submit_form", "download"]
  }]
}
```

**Verification Plan:**
1. When API access available: `LOUD_WIRE=1 cargo run --example computer_use`
2. Check field names (expectedPredefinedFunctions vs excludedPredefinedFunctions)
3. Check if nested structure is required
4. Update this section with actual results

**Current Trust Level**: LOW (Google docs inconsistent on camelCase/snake_case)
```

**Impact**: Makes documentation gaps explicit, prevents pseudo-verification.

### TIER 2: Structural (Catches Issues During Development)

#### 2.1 Integration Test Framework: Live API Verification

**Problem**: Wire format test coverage only in `tests/wire_format_verification_tests.rs` (unit-level).

**Solution**: Add integration test that round-trips types through real API.

File: `tests/live_api_wire_format_tests.rs`

```rust
//! Integration tests that verify types serialize/deserialize correctly
//! with the actual Gemini API.
//!
//! These tests require GEMINI_API_KEY and make real API calls.
//! Run with: cargo test --test live_api_wire_format_tests -- --include-ignored

#[tokio::test]
#[ignore]
async fn code_execution_result_wire_format() {
    let client = get_test_client();

    // Make a request that generates CodeExecutionResult
    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_system_instruction("Execute this code and show the result.")
        .with_input("print('Hello World')")
        .execute()
        .await
        .expect("API call failed");

    // Extract code execution result from response
    for content in response.all_outputs() {
        if let InteractionContent::CodeExecutionResult { call_id, is_error, result } = content {
            // These fields MUST be present (not hallucinated)
            assert!(call_id.is_some(), "call_id should be present");
            assert!(result.len() > 0, "result should have content");
            // is_error is a bool, not an enum
            println!("✓ CodeExecutionResult wire format verified");
            return;
        }
    }
    panic!("No CodeExecutionResult found in response");
}

#[tokio::test]
#[ignore]
async fn thought_has_signature_not_text() {
    let client = get_test_client();

    let response = client
        .interaction()
        .with_model("gemini-2.5-pro-preview")
        .with_thinking_level(ThinkingLevel::High)
        .with_input("Solve this math problem: 2+2")
        .execute()
        .await
        .expect("API call failed");

    // Check that Thought block has signature field
    for signature in response.thought_signatures() {
        assert!(!signature.is_empty(), "signature should not be empty");
        // Signature is cryptographic, should be base64-like
        assert!(!signature.contains(" "), "signature should not have spaces");
        println!("✓ Thought.signature field verified (not Thought.text)");
    }
}

#[tokio::test]
#[ignore]
async fn url_context_call_has_nested_arguments() {
    let client = get_test_client();

    let response = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_input("What's on https://example.com?")
        .with_tool(Tool::UrlContext)
        .execute()
        .await
        .expect("API call failed");

    for content in response.all_outputs() {
        if let InteractionContent::UrlContextCall { id, urls } = content {
            // urls should be extracted from nested arguments object
            assert!(id.is_some(), "id should be present");
            assert!(!urls.is_empty(), "urls should be present");
            for url in urls {
                assert!(url.starts_with("http"), "should be valid URLs");
            }
            println!("✓ UrlContextCall nested arguments verified");
            return;
        }
    }
}
```

**Integration into CI:**
```yaml
# .github/workflows/test.yml - Add new step:

- name: Run live API wire format tests
  if: github.event_name == 'schedule' || contains(github.event.pull_request.labels.*.name, 'verify-wire')
  env:
    GEMINI_API_KEY: ${{ secrets.GEMINI_API_KEY }}
  run: |
    cargo test --test live_api_wire_format_tests -- --include-ignored
```

**Impact**: Real API verification happens daily (scheduled) + on-demand via label.

#### 2.2 Enum Audit Checklist for Code Review

**Problem**: Reviewers can't easily spot wire format issues in serde code.

**Solution**: Add standardized checklist to PR template.

File: `.github/pull_request_template.md`

```markdown
## Wire Format Verification

If this PR modifies enums in `src/content.rs`, `src/request.rs`, or `src/response.rs`:

### For Authors:
- [ ] Checked official Google docs (see CLAUDE.md for URLs)
- [ ] Ran `LOUD_WIRE=1 cargo test -- --include-ignored` and verified actual wire format
- [ ] Updated `docs/ENUM_WIRE_FORMATS.md` with verified format and date
- [ ] Added unit test in `*_tests.rs` for serialization round-trip
- [ ] If new enum: marked `#[non_exhaustive]` and added `Unknown` variant
- [ ] Checked for hallucinated fields (fields that don't appear in actual API responses)

### For Reviewers:
When reviewing enum changes, verify:
- [ ] Does docstring include expected wire format? (e.g., "Wire format: lowercase")
- [ ] Does `#[serde(...)]` match the documented format?
- [ ] If docs are referenced, check for known contradictions in `ENUM_WIRE_FORMATS.md`
- [ ] Are optional fields typed as `Option<T>`?
- [ ] Does the PR update `docs/ENUM_WIRE_FORMATS.md`?

[Rest of template...]
```

**Impact**: Makes wire format a first-class concern in PR review process.

### TIER 3: Long-Term (Architecture)

#### 3.1 Type-Safe Serde Configurations

**Problem**: Easy to misapply `#[serde(rename_all)]` globally or miss case conversions.

**Solution**: Create a custom serde configuration module with explicit declarations.

File: `src/serde_config.rs`

```rust
//! Serde configuration and case handling for API wire formats.
//!
//! This module centralizes case-handling rules to make wire format
//! assumptions explicit and auditable.

use serde::{Deserialize, Deserializer, Serializer};

/// Configuration for snake_case API fields.
/// Used for: InteractionStatus, Resolution, InteractionContent types, etc.
pub mod snake_case {
    use serde::de::DeserializeOwned;

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct Wrapper<T: DeserializeOwned> {
        #[serde(rename_all = "snake_case")]
        data: T,
    }
}

/// Configuration for SCREAMING_CASE API fields.
/// Used for: FunctionCallingMode, CodeExecutionLanguage, etc.
pub mod screaming_case {
    // Similar pattern
}

/// Helper trait to document wire format in code
pub trait WireFormat {
    /// Returns the expected wire format description
    /// Example: "snake_case: 'code_execution_result'"
    fn wire_format_description() -> &'static str;
}

impl WireFormat for InteractionStatus {
    fn wire_format_description() -> &'static str {
        "snake_case: 'completed', 'in_progress', 'requires_action', etc."
    }
}
```

Then in type definitions:

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionStatus {
    Completed,
    InProgress,
    RequiresAction,
    // ...
}

// In comments: Explicit reference
impl InteractionStatus {
    /// Wire format: [snaked_case value]
    /// See: docs/ENUM_WIRE_FORMATS.md#interactionstatus
    /// Verified: 2026-01-09
}
```

**Impact**: Makes case handling auditable (can grep for `WireFormat`).

#### 3.2 Automated Wire Format Documentation Generation

**Problem**: `docs/ENUM_WIRE_FORMATS.md` is manually maintained (easy to drift).

**Solution**: Generate documentation from type definitions + verification tests.

File: `scripts/gen_wire_format_docs.rs`

```rust
//! Generates docs/ENUM_WIRE_FORMATS.md from verified wire format tests.
//!
//! Usage: cargo run --bin gen_wire_format_docs

use std::fs;

#[derive(Debug)]
struct WireFormatEntry {
    rust_type: String,
    wire_format: String,
    example: String,
    verified_date: Option<String>,
    doc_source: String,
}

fn main() {
    let entries = vec![
        WireFormatEntry {
            rust_type: "InteractionStatus".to_string(),
            wire_format: "snake_case".to_string(),
            example: r#""completed", "in_progress""#.to_string(),
            verified_date: Some("2026-01-09".to_string()),
            doc_source: "live_api_wire_format_tests.rs".to_string(),
        },
        // ... populated from test metadata
    ];

    let markdown = generate_table(&entries);
    fs::write("docs/ENUM_WIRE_FORMATS_GENERATED.md", markdown)
        .expect("Failed to write docs");
}
```

**Impact**: Keeps documentation in sync with actual tests (single source of truth).

#### 3.3 Staged Rollout for Unverified Types

**Problem**: Can't use types that aren't yet verified without risking API incompatibility.

**Solution**: Feature flag for unverified types with loud warnings.

File: `src/lib.rs`

```rust
#[cfg(feature = "unverified-wire-formats")]
compile_error!(
    "The following types have NOT been verified with live API:\
    - ComputerUse tool\
    - ComputerUseCall/Result content types\
    \
    See docs/ENUM_WIRE_FORMATS.md#unverified for details.\
    Remove this feature flag only after verification."
);

#[cfg(feature = "unverified-wire-formats")]
pub mod computer_use {
    // Types marked with #[deprecated]
}
```

Then in CI:

```yaml
# test-unverified feature runs in separate job
- name: Test unverified wire formats
  run: cargo test --features unverified-wire-formats
```

**Impact**: Clearly separates verified from unverified code paths.

## Implementation Roadmap

### Phase 1: Immediate (Week 1)
- [ ] Create `tests/wire_format_verification_template.md`
- [ ] Update PR template with wire format checklist
- [ ] Document current conflicts in `docs/ENUM_WIRE_FORMATS.md`
- [ ] Mark UNVERIFIED types with feature flag

**Effort**: 4-6 hours
**Benefit**: Prevents new hallucinations, makes gaps explicit

### Phase 2: Structural (Week 2)
- [ ] Create `tests/live_api_wire_format_tests.rs` with 5-10 key types
- [ ] Add GitHub Actions workflow for scheduled/labeled verification
- [ ] Create `.git/hooks/pre-push` aid (local development)

**Effort**: 8-10 hours
**Benefit**: Catches issues during development, not in release

### Phase 3: Long-Term (Ongoing)
- [ ] Implement `WireFormat` trait and audit case handling
- [ ] Create `gen_wire_format_docs.rs` generator
- [ ] Expand live API test coverage (one per major type)

**Effort**: 15-20 hours (phased over months)
**Benefit**: Makes wire format a first-class concern

## Measurement

### Current State (Before Improvements)
- Wire format issues discovered: 5+ per major release
- Average time to fix: 2 days + 1 breaking change
- Detection method: Manual `LOUD_WIRE=1` testing
- Documentation state: Frequently out of sync

### Target State (After Phase 1)
- Wire format issues discovered: 0-1 per release (Evergreen unknowns only)
- Average time to prevent: 1 hour (checklist prevents introduction)
- Detection method: Automated CI + test-driven
- Documentation state: Verified by tests

### Target State (After Phase 2)
- Wire format verification: Integrated into PR workflow
- Real API validation: Daily scheduled + on-demand
- Developer friction: Minimal (pre-push hook guides them)

## References

- **Current verification doc**: `/home/evansenter/Documents/projects/genai-rs/docs/ENUM_WIRE_FORMATS.md` (2,100 lines, manually maintained)
- **Wire format tests**: `/home/evansenter/Documents/projects/genai-rs/tests/wire_format_verification_tests.rs` (491 lines)
- **Recent fixes**: Commits `ae52786` → `57a0eea` (wire format alignment work)
- **CLAUDE.md guidance**: `/home/evansenter/Documents/projects/genai-rs/CLAUDE.md` (documents API doc locations)

## Key Takeaways

1. **Wire format mismatches are preventable** with explicit verification in PR workflow
2. **Manual testing (`LOUD_WIRE=1`) is effective but not automated** - opportunity for CI integration
3. **Documentation is a trust problem** - Google docs are wrong; library should verify everything
4. **Type system is too permissive** - easy to add hallucinated fields that compile but don't serialize
5. **Real API tests provide ground truth** - integrate into CI rather than skip them

The improvements focus on making verification explicit (template), automated (CI checks), and auditable (documented conflict sources).
