# Wire Format Verification: Quick Summary

## The Problem (What Happened)

Recent session found **5+ wire format mismatches** in one day. Code compiled, tests passed, but API calls failed.

| What We Built | What API Actually Returns | How We Found It | Days to Fix |
|----------------|--------------------------|-----------------|------------|
| `CodeExecutionResult { outcome: CodeExecutionOutcome, output: String }` | `{ is_error: bool, result: String }` | `LOUD_WIRE=1` inspection | 2 |
| `Thought { text: String }` | `{ signature: String }` (cryptographic) | API response parsing | 1 |
| `FunctionCall { thought_signature: Option<String> }` | Field never sent | Code audit | 1 |
| `UrlContextCall { url: String }` | `{ arguments: { urls: [String] } }` | Wire format inspection | 1 |
| `CodeExecutionCall { language, code }` (flat) | `{ arguments: { language, code } }` (nested) | Response structure check | 1 |

**Root cause**: Google's API docs are sometimes wrong, but Rust types can't express "this doesn't exist at API boundary" without runtime validation.

---

## Current Detection Methods (Why They Failed)

| Method | When | Coverage | Gap |
|--------|------|----------|-----|
| Code review | PR time | Structural patterns | Serde annotations invisible |
| Unit tests | Build time | Serialization logic only | Doesn't test API contract |
| Integration tests | CI time | Full round-trip | Too late; already in release |
| Manual `LOUD_WIRE=1` testing | Random | Actual API responses | Not systematic; easy to skip |
| Documentation | Before coding | Reference only | Often wrong (verified problem) |

**Gap**: No automated boundary between "looks correct" and "API-compatible".

---

## Three-Tier Solutions

### Tier 1: Immediate (This Week) - Prevents Future Issues

Make wire format verification explicit and required in PR workflow.

**1.1 Verification Checklist** (`tests/wire_format_verification_template.md`)
```
When adding/modifying enums:
□ Consult Google docs AND verify with LOUD_WIRE=1
□ Document wire format in docstring
□ Add serialization round-trip test
□ Update docs/ENUM_WIRE_FORMATS.md with verification date
```

**1.2 PR Template Updates** (`.github/pull_request_template.md`)
```
Enum/Type Changes:
□ Verified wire format with LOUD_WIRE=1
□ Updated docs/ENUM_WIRE_FORMATS.md
□ No hallucinated fields (checked against actual response)
```

**1.3 Document Conflicts** (`docs/ENUM_WIRE_FORMATS.md` enhancement)
```markdown
### ThinkingSummaries

| Source | Claims | Actual |
|--------|--------|--------|
| Google docs | lowercase "auto" | SCREAMING_CASE "THINKING_SUMMARIES_AUTO" |
| Proto schema | Qualified names | Confirmed SCREAMING_CASE |

Reason: Docs may be from older API version
```

**Impact**: Prevents adding hallucinated fields, makes verification explicit.
**Effort**: 4-6 hours (documentation + template creation)

---

### Tier 2: Structural (Week 2) - Catches Issues During Development

Automate real API verification and integrate into CI.

**2.1 Integration Test Framework** (`tests/live_api_wire_format_tests.rs`)
```rust
#[tokio::test]
#[ignore]
async fn code_execution_result_wire_format() {
    let response = client.execute(...).await?;

    // Verify CodeExecutionResult has is_error + result, NOT outcome
    for content in response.all_outputs() {
        if let InteractionContent::CodeExecutionResult {
            call_id, is_error, result
        } = content {
            println!("✓ CodeExecutionResult wire format verified");
        }
    }
}
```

**2.2 CI Workflow** (`.github/workflows/wire-format-check.yml`)
```yaml
# Runs daily or on 'verify-wire' label
# Executes live API tests to catch regressions
```

**2.3 Developer Aid** (`.git/hooks/pre-push`)
```bash
# Warns if new enums added without LOUD_WIRE verification
# Doesn't block, just reminds
```

**Impact**: Real API validation happens daily, catches issues early.
**Effort**: 8-10 hours (test framework + CI setup)

---

### Tier 3: Long-Term (Ongoing) - Architecture

**3.1 Type-Safe Serde Configurations** (`src/serde_config.rs`)
Make case handling explicit and auditable.

**3.2 Auto-Generated Docs** (`scripts/gen_wire_format_docs.rs`)
Keep `ENUM_WIRE_FORMATS.md` in sync with tests (single source of truth).

**3.3 Feature Flags for Unverified Types**
Clearly separate verified from unverified code paths.

**Impact**: Makes wire format a first-class concern in codebase.
**Effort**: 15-20 hours (phased over months)

---

## Decision Matrix: What to Implement When

| Improvement | Cost | Benefit | Timing |
|-------------|------|---------|--------|
| **Checklist template** | 2 hours | Prevents hallucinations | NOW (tomorrow) |
| **PR template update** | 1 hour | Makes verification visible to reviewers | NOW |
| **Document conflicts in ENUM_WIRE_FORMATS** | 3 hours | Stops guessing which source is right | THIS WEEK |
| **Live API test framework** | 6-8 hours | Catches issues during development | NEXT WEEK |
| **GitHub Actions CI** | 2-3 hours | Automated daily verification | NEXT WEEK |
| **Developer pre-push hook** | 1 hour | Guides devs before push | NEXT WEEK |
| **Type-safe serde config** | 5-6 hours | Makes assumptions auditable | MONTH 2 |
| **Auto-doc generation** | 8-10 hours | Eliminates drift | MONTH 2 |
| **Feature flags for unverified** | 2-3 hours | Reduces surprise breakage | MONTH 2 |

---

## Measurement: Before vs. After

### Current State
- Wire format issues discovered: **5+ per release**
- Time to discover: **2-3 days** (after release)
- Cost per issue: **1 breaking change** + retest cycle
- Documentation state: **Manually maintained, drifts frequently**
- Developer awareness: "Check docs, maybe test with LOUD_WIRE?"

### After Phase 1 (Week 1)
- Issues prevented: **Most hallucinations caught before PR**
- Documentation state: **Conflicts explicitly marked**
- Developer process: **Checklist makes it explicit**

### After Phase 2 (Week 2)
- Real API validation: **Daily scheduled + on-demand**
- Developer friction: **Pre-push warning (helpful, not blocking)**
- CI integration: **Tests fail if wire format drifts**

### Target State (Ongoing)
- Wire format issues discovered: **0-1 per release** (only Evergreen unknowns)
- Documentation state: **Generated from tests, always in sync**
- Developer confidence: "If tests pass, wire format is verified"

---

## File Locations for Implementation

| File | Purpose | Status |
|------|---------|--------|
| `docs/ENUM_WIRE_FORMATS.md` | Master reference (2,100 lines) | Update to add conflicts |
| `tests/wire_format_verification_tests.rs` | Unit-level round-trip tests | Expand coverage |
| `tests/wire_format_verification_template.md` | PR checklist template | **CREATE** |
| `tests/live_api_wire_format_tests.rs` | Integration tests (real API) | **CREATE** |
| `.github/pull_request_template.md` | PR checklist | **UPDATE** |
| `.github/workflows/wire-format-check.yml` | CI verification | **CREATE** |
| `.git/hooks/pre-push` | Local development aid | **CREATE** |
| `src/serde_config.rs` | Case handling documentation | **CREATE** (Phase 3) |
| `scripts/gen_wire_format_docs.rs` | Auto-documentation generator | **CREATE** (Phase 3) |
| `docs/WIRE_FORMAT_VERIFICATION_IMPROVEMENTS.md` | Full strategy doc | **CREATED** |

---

## Next Steps

1. **This afternoon (1 hour)**:
   - Create `tests/wire_format_verification_template.md`
   - Update `.github/pull_request_template.md`

2. **This week (3 hours)**:
   - Update `docs/ENUM_WIRE_FORMATS.md` to mark conflicts
   - Document all UNVERIFIED types with feature flag plan

3. **Next week (10 hours)**:
   - Build `tests/live_api_wire_format_tests.rs` (5-6 tests)
   - Add `.github/workflows/wire-format-check.yml`
   - Create pre-push hook template

4. **Ongoing**:
   - Expand live API tests as new types added
   - Monitor for documentation drift
   - Plan auto-doc generation (Phase 3)

---

## Key Insight

**Wire format verification is not just a "nice to have"—it's the difference between code that compiles and code that works with the actual API.**

The Evergreen principle (preserve unknowns) handles future API expansion, but can't catch current mismatches. We need explicit verification at the boundary.

The good news: Google provides the ground truth (`LOUD_WIRE=1` shows it). We just need to make verification systematic and automated.
