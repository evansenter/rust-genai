# Wire Format Verification Checklist

Use this checklist when adding or modifying types in `InteractionContent`, `ApiRequest`, or `ApiResponse` enums.

**Goal**: Ensure your Rust types match what the Gemini API actually returns/accepts, preventing hallucinated fields and serialization mismatches.

---

## Phase 1: Documentation Review (15 min)

Before writing any code:

- [ ] **Find official source**: Check `CLAUDE.md` for authoritative Google docs URLs
  - Interactions API: https://ai.google.dev/static/api/interactions.md.txt
  - Function Calling: https://ai.google.dev/gemini-api/docs/function-calling.md.txt
  - Thought Signatures: https://ai.google.dev/gemini-api/docs/thought-signatures.md.txt

- [ ] **Note conflicts**: Read `docs/ENUM_WIRE_FORMATS.md` - check if this type/field is marked with known discrepancies
  - Example: "ThinkingSummaries docs claim lowercase, but actual API uses SCREAMING_CASE"
  - If you find contradictions between Google docs, flag them in your PR description

- [ ] **Check for UNVERIFIED marker**: Any types with "UNVERIFIED" status?
  - If yes: You're breaking new ground. Plan for Phase 4 (Real API testing)
  - If no: You can rely on existing verification

---

## Phase 2: Type Definition (30 min)

Write the Rust type with explicit documentation:

```rust
/// Code execution result from the model.
///
/// # Wire Format
/// The API returns a simple result object with success/error indication and output:
/// ```json
/// {
///   "type": "code_execution_result",
///   "call_id": "exec_123",
///   "is_error": false,
///   "result": "Hello, World!\n"
/// }
/// ```
///
/// **Key points:**
/// - Uses `is_error: bool`, NOT `outcome` enum (despite what docs suggest)
/// - Uses `result: String`, NOT `output`
/// - `call_id` is optional (appears in streaming responses only)
///
/// **Documentation Conflict**: Official Google docs mention `outcome` enum with
/// values like `OUTCOME_OK`, but actual wire format uses simple boolean.
/// See: docs/ENUM_WIRE_FORMATS.md#codeexecutionresult
///
/// **Verified**: 2026-01-12 - Tested with `LOUD_WIRE=1 cargo run --example code_execution`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InteractionContent {
    CodeExecutionResult {
        #[serde(skip_serializing_if = "Option::is_none")]
        call_id: Option<String>,
        is_error: bool,
        result: String,
    },
    // ...
}
```

**Checklist**:
- [ ] Field has docstring explaining each field's meaning
- [ ] Docstring includes "# Wire Format" section with example JSON
- [ ] Mark known conflicts between docs and reality
- [ ] If enum: Use `#[non_exhaustive]` marker
- [ ] If enum: Add `Unknown { *_type: String, data: serde_json::Value }` variant
- [ ] Optional fields use `Option<T>` type
- [ ] Case convention is documented (lowercase/SCREAMING_CASE/snake_case)

---

## Phase 3: Serialization Tests (30 min)

Add unit tests in `src/content_tests.rs` or `src/request_tests.rs`:

```rust
#[test]
fn code_execution_result_round_trips() {
    // Create the Rust value
    let content = InteractionContent::CodeExecutionResult {
        call_id: Some("exec_123".to_string()),
        is_error: false,
        result: "3628800".to_string(),
    };

    // Serialize to JSON
    let json = serde_json::to_value(&content).unwrap();

    // Verify structure
    assert_eq!(json["type"], "code_execution_result");
    assert_eq!(json["call_id"], "exec_123");
    assert_eq!(json["is_error"], false);
    assert_eq!(json["result"], "3628800");

    // Deserialize back
    let back: InteractionContent = serde_json::from_value(json).unwrap();
    assert_eq!(content, back);
}

#[test]
fn code_execution_result_deserializes_actual_api_response() {
    // This is the actual response format from the API
    let actual_api_response = serde_json::json!({
        "type": "code_execution_result",
        "call_id": "exec_123",
        "is_error": false,
        "result": "Hello, World!\n"
    });

    // Can we parse it?
    let result: InteractionContent = serde_json::from_value(actual_api_response).unwrap();

    // Verify structure
    if let InteractionContent::CodeExecutionResult { call_id, is_error, result } = result {
        assert_eq!(call_id, Some("exec_123".to_string()));
        assert_eq!(is_error, false);
        assert_eq!(result, "Hello, World!\n");
    } else {
        panic!("Expected CodeExecutionResult");
    }
}

#[test]
fn code_execution_result_rejects_hallucinated_outcome_enum() {
    // This is what Google docs suggest (WRONG)
    let wrong_format = serde_json::json!({
        "type": "code_execution_result",
        "outcome": "OUTCOME_OK",  // This field doesn't exist!
        "output": "Hello"
    });

    // Should this deserialize? Let's make sure it handles unknown fields gracefully
    let result: Result<InteractionContent, _> = serde_json::from_value(wrong_format);

    // Depending on serde config, either:
    // 1. Ignores unknown fields (OK)
    // 2. Requires is_error/result (GOOD)
    match result {
        Ok(InteractionContent::CodeExecutionResult { is_error, result }) => {
            // Unknown fields were ignored, defaults were used
            println!("Unknown fields ignored (lenient deserialization)");
        }
        Err(e) => {
            println!("Deserialization failed (strict): {}", e);
            println!("This is good - prevents accepting wrong format");
        }
    }
}
```

**Checklist**:
- [ ] Round-trip test (Rust → JSON → Rust produces identical value)
- [ ] Test with actual API response JSON (copy-pasted from `LOUD_WIRE=1` output)
- [ ] Verify field names match (case-sensitive)
- [ ] Verify field types match (bool vs string, single vs array)
- [ ] Optional fields are truly optional in JSON
- [ ] NO hallucinated fields deserialize (test that wrong format is rejected or defaults)

---

## Phase 4: REAL API Testing (1-2 hours)

This is the critical step. **Do not skip this.**

### 4.1 Create/Use Example

Create an example that triggers this response type:

```bash
# If example doesn't exist, create one in examples/
# Example: examples/code_execution.rs

use genai_rs::{Client, InteractionContent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .api_key_from_env()
        .build()?;

    let response = client
        .interaction()
        .with_input("Execute Python code: print('test')")
        .execute()
        .await?;

    // Print what we get back
    for content in response.all_outputs() {
        println!("{:#?}", content);
    }

    Ok(())
}
```

### 4.2 Run with LOUD_WIRE=1

```bash
LOUD_WIRE=1 cargo run --example code_execution 2>&1 | tee /tmp/wire_output.txt
```

This produces output like:

```
[DEBUG] Request:
{
  "model": "gemini-3-flash-preview",
  "input": [...],
  ...
}

[DEBUG] Response:
{
  "outputs": [{
    "content": [{
      "type": "code_execution_result",
      "call_id": "exec_123",
      "is_error": false,
      "result": "test\n"
    }]
  }]
}
```

### 4.3 Verification Checklist

Compare actual API response against your type definition:

**For each field:**
- [ ] Does the field appear in the actual response? (Not hallucinated)
- [ ] Is the field name correct? (Exact case match)
- [ ] Is the field type correct? (String vs number vs boolean vs object)
- [ ] Is the field optional? (Check if it's missing in some responses)
- [ ] Is it nested correctly? (Object vs flattened)

**Example verification**:
```
Type definition says:
  ✓ is_error: bool          → JSON has "is_error": false ✓
  ✓ result: String          → JSON has "result": "test\n" ✓
  ✓ call_id: Option<String> → JSON has "call_id": "exec_123" ✓

Google docs claim:
  ✗ outcome: CodeExecutionOutcome    → DOES NOT APPEAR IN JSON ✗
  ✗ output: String                   → DOES NOT APPEAR IN JSON ✗
```

### 4.4 Test With Edge Cases

- [ ] Successful execution: `is_error: false` with output
- [ ] Failed execution: `is_error: true` with error message
- [ ] Streaming response: Check if `call_id` is optional
- [ ] Empty output: `result: ""` is valid
- [ ] Multiline output: `result: "line1\nline2\n"` preserves newlines

---

## Phase 5: Documentation Update (20 min)

Update `docs/ENUM_WIRE_FORMATS.md`:

```markdown
### CodeExecutionResult (content)

Returned when code execution completes. Uses simple `is_error` boolean and `result` string fields.

```json
{
  "type": "code_execution_result",
  "call_id": "exec_123",
  "is_error": false,
  "result": "Hello, World!\n"
}
```

| Rust Field | Wire Name | Type | Notes |
|------------|-----------|------|-------|
| `call_id` | `call_id` | `Option<String>` | Matches the CodeExecutionCall id |
| `is_error` | `is_error` | `bool` | `false` = success, `true` = error |
| `result` | `result` | `String` | Output text (stdout) or error message |

**Documentation Conflict**: The official API documentation mentions `outcome` enum with values like `OUTCOME_OK`, but the **actual wire format** uses `is_error: bool` and `result: String`. This discrepancy may be due to docs from an older API version.

**Verified**: 2026-01-12 - Captured from `LOUD_WIRE=1 cargo run --example code_execution`. Tested both success (`is_error: false`) and error (`is_error: true`) cases.
```

**Checklist**:
- [ ] Include actual JSON from API response (copy from `LOUD_WIRE=1` output)
- [ ] Create table mapping Rust fields to wire field names
- [ ] Note any conflicts between docs and reality
- [ ] Include verification date and method
- [ ] Update ENUM_WIRE_FORMATS.md quick reference table at top

---

## Phase 6: PR Submission (5 min)

In your PR description, include:

```markdown
## Wire Format Verification

This PR modifies the following types:
- [ ] `InteractionContent::CodeExecutionResult`

### Verification Summary

**Documentation source**: Official Google Interactions API docs

**Conflict found**: Google docs suggest `outcome` enum, but actual API uses `is_error: bool`

**Verification method**: `LOUD_WIRE=1 cargo run --example code_execution`

**Test coverage**:
- [ ] Unit test: `code_execution_result_round_trips()`
- [ ] Unit test: `code_execution_result_deserializes_actual_api_response()`
- [ ] Integration test: Manual `LOUD_WIRE=1` verification (documented above)

**Actual wire format verified**:
```json
{
  "type": "code_execution_result",
  "call_id": "exec_123",
  "is_error": false,
  "result": "Hello, World!\n"
}
```

No hallucinated fields found.
```

**Checklist**:
- [ ] Mentioned documentation source (which Google doc was consulted)
- [ ] Noted any conflicts found
- [ ] Included actual wire format JSON
- [ ] Linked to `docs/ENUM_WIRE_FORMATS.md` updates
- [ ] All unit tests pass
- [ ] Manual `LOUD_WIRE=1` verification completed

---

## Common Pitfalls

### Pitfall 1: Missing `#[non_exhaustive]` on Enums

**Wrong**:
```rust
pub enum CodeExecutionLanguage {
    Python,
}
```

**Right**:
```rust
#[non_exhaustive]
pub enum CodeExecutionLanguage {
    Python,
    Unknown { language_type: String, data: serde_json::Value },
}
```

**Why**: Google may add new languages. `#[non_exhaustive]` + `Unknown` variant ensures your code doesn't break when they do.

### Pitfall 2: Case Convention Mismatch

**Common mistake**: Assuming all fields are lowercase or camelCase.

**Reality**: Different fields use different conventions:
- `InteractionStatus` enums: snake_case (`"in_progress"`)
- `FunctionCallingMode` enums: SCREAMING_CASE (`"AUTO"`)
- `ThinkingLevel` enums: lowercase (`"low"`)
- Field names: snake_case (`"code_execution_result"`)
- Some nested objects: camelCase (`"speechConfig"`, `"generationConfig"`)

**Check each type with `LOUD_WIRE=1`.**

### Pitfall 3: Hallucinated Optional Fields

**Wrong**:
```rust
pub struct CodeExecutionResultInfo {
    is_error: bool,
    result: String,
    signature: Option<String>, // You think "maybe it's optional"
    output: Option<String>,    // You think "API probably supports this"
}
```

**Right**: Only fields that actually appear in API responses. Test with actual API to be sure.

### Pitfall 4: Flat vs. Nested Confusion

**Wrong** (assumes flat):
```rust
pub struct UrlContextCall {
    urls: Vec<String>,  // Assumed flat
}
```

**Right** (actual structure):
```rust
pub struct UrlContextCall {
    id: Option<String>,
    urls: Vec<String>,  // Extracted from nested arguments.urls
}
```

The JSON has `{"arguments": {"urls": [...]}}` but we extract to flat structure for ergonomics.

---

## Example: Complete Flow

Here's what a complete wire format verification looks like:

### 1. Read Docs
"Google docs mention CodeExecutionLanguage. Let me check."

### 2. Write Type
```rust
#[non_exhaustive]
pub enum CodeExecutionLanguage {
    Python,
    Unknown { language_type: String, data: serde_json::Value },
}
```

### 3. Add Tests
```rust
#[test]
fn python_serializes_to_screaming_case() {
    assert_eq!(serde_json::to_value(CodeExecutionLanguage::Python).unwrap(), "PYTHON");
}
```

### 4. Manual Test
```bash
LOUD_WIRE=1 cargo run --example code_execution 2>&1 | grep language
# Output: "language": "PYTHON"  ✓
```

### 5. Update Docs
```markdown
### CodeExecutionLanguage
| Rust | Wire |
|------|------|
| `Python` | `"PYTHON"` |

**Verified**: 2026-01-10
```

### 6. Submit PR
"Verified CodeExecutionLanguage wire format matches actual API (SCREAMING_CASE)."

---

## When to Skip Manual Testing

You can skip Phase 4 (LOUD_WIRE=1 testing) **only if**:

1. The type is already in `docs/ENUM_WIRE_FORMATS.md` with recent verification date, AND
2. You're only fixing code (not changing wire format), AND
3. All existing tests still pass

**Otherwise, always test.**

---

## Questions?

If you're uncertain about wire format:

1. **Consult `docs/ENUM_WIRE_FORMATS.md`** - It's the source of truth
2. **Run `LOUD_WIRE=1`** - Actual API responses are definitive
3. **Ask in PR** - Reviewers can help identify hallucinated fields
4. **Check recent commits** - Recent fixes in `57a0eea` show wire format patterns

**Never guess. Always verify with actual API.**
