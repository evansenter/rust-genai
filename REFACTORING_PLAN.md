# Refactoring Plan: Interactions API Preparation

**Branch**: `feature/interactions-api-prep`
**Started**: 2025-12-21
**Status**: Phase 1 Complete ✅ - Ready for Phase 2 or Merge Decision
**Last Updated**: 2025-12-21

## Current Status Summary

### ✅ PHASE 1 COMPLETE (Checkpoints 0-8)
- All refactoring completed and tested
- **7 commits** on feature branch
- **21 tests passing** (17 unit + 4 integration)
- **All examples working** with real Gemini API
- API key configured in `~/.extra`
- Model names updated to `gemini-3-flash-preview`

### ⚠️ DECISION POINT
The refactoring added Interactions API infrastructure (Endpoint enum, URL construction) but **no actual Interactions API implementation yet**. Three options:

**Option A**: Merge now (with unused Interactions code)
- Pro: Improvements are standalone valuable (V1Beta default, shared types, model updates)
- Con: Unused code for features that don't exist yet

**Option B**: Continue to Phase 2 first (Recommended)
- Complete Interactions API implementation (Checkpoints 9-13)
- Deliver complete feature before merging
- No unused code in main branch

**Option C**: Remove unused Interactions code, merge improvements only
- Keep: shared types refactoring, V1Beta default, model updates
- Remove: unused Endpoint variants (CreateInteraction, GetInteraction, DeleteInteraction)
- Add Interactions in separate PR later

### 📋 Next Steps
See Phase 2 checkpoints below (9-13) for Interactions API implementation.

---

## Objectives

Refactor the codebase to support the new Gemini Interactions API while maintaining backwards compatibility and ensuring the codebase is in a working state after every commit.

## Strategy

- **Approach**: Incremental, checkpoint-based refactoring
- **Safety**: Every commit must build and pass all tests
- **Compatibility**: No breaking changes to existing API
- **Atomicity**: Each commit is self-contained and revertible

## Checkpoints & Commits

### ✅ Checkpoint 0: Initial Setup
- [ ] Create feature branch
- [ ] Verify all tests pass on current code
- [ ] Document refactoring plan

**Commit**: `docs: Add refactoring plan for Interactions API preparation`

---

### 🔲 Checkpoint 1: Extract Shared Types
Create `genai-client/src/models/shared.rs` with types used by both APIs.

**Changes**:
- Create `genai-client/src/models/shared.rs`
- Move/duplicate these types from `request.rs`:
  - `Content`
  - `Part`
  - `Tool`
  - `FunctionDeclaration`
  - `FunctionParameters`
  - `FunctionCall`
  - `FunctionResponse`
  - `CodeExecution`
- Update `genai-client/src/models/mod.rs` to export shared types
- Keep original `request.rs` types for now (backwards compatibility)

**Testing**:
```bash
cargo build
cargo test
```

**Commit**: `refactor(genai-client): Extract shared types to models/shared.rs`

---

### 🔲 Checkpoint 2: Re-export Shared Types
Make shared types available through `genai-client` public API.

**Changes**:
- Update `genai-client/src/lib.rs` to re-export from `models::shared`
- Add module documentation for shared types

**Testing**:
```bash
cargo build
cargo test
```

**Commit**: `refactor(genai-client): Re-export shared types from public API`

---

### 🔲 Checkpoint 3: Use Shared Types in Request Models
Update request.rs to use shared types instead of duplicates.

**Changes**:
- Update `genai-client/src/models/request.rs` to import from `shared`
- Update `GenerateContentRequest` to use `shared::Content`, etc.
- Ensure all existing tests still pass

**Testing**:
```bash
cargo build
cargo test
```

**Commit**: `refactor(genai-client): Migrate request models to use shared types`

---

### 🔲 Checkpoint 4: Update Public Types Layer
Update the public API to use genai-client shared types.

**Changes**:
- Update `rust-genai/src/types.rs` to re-export from `genai_client::models::shared`
- Remove duplicate type definitions where possible
- Keep conversion functions if needed for API compatibility

**Testing**:
```bash
cargo build --all-features
cargo test
cargo run --example simple_request
```

**Commit**: `refactor: Consolidate type definitions using shared types`

---

### 🔲 Checkpoint 5: Add Endpoint Abstraction
Create flexible URL construction for multiple API endpoints.

**Changes**:
- Add `Endpoint` enum to `genai-client/src/common.rs`
- Add `construct_endpoint_url()` function
- Keep existing `construct_url()` function (no breaking changes)
- Add comprehensive tests for new URL construction

**Testing**:
```bash
cargo test common::tests
```

**Commit**: `feat(genai-client): Add Endpoint abstraction for URL construction`

---

### 🔲 Checkpoint 6: Add URL Construction Tests
Ensure new URL construction handles all cases.

**Changes**:
- Add tests in `genai-client/src/common.rs`
- Test all endpoint variants
- Test with different API versions

**Testing**:
```bash
cargo test
```

**Commit**: `test(genai-client): Add comprehensive URL construction tests`

---

### 🔲 Checkpoint 7: Update Default API Version
Change default from V1Alpha to V1Beta for stability.

**Changes**:
- Update `rust-genai/src/client.rs` default version
- Update documentation to reflect new default
- Ensure examples still work

**Testing**:
```bash
cargo test
cargo run --example simple_request
```

**Commit**: `feat: Change default API version to V1Beta`

---

### 🔲 Checkpoint 8: Update Examples (Optional)
Modernize examples to use new patterns if applicable.

**Changes**:
- Review all examples in `examples/`
- Update if beneficial (not required)

**Testing**:
```bash
cargo run --example simple_request
cargo run --example stream_request
cargo run --example function_call
cargo run --example code_execution
```

**Commit**: `docs: Update examples to reflect best practices`

---

## Phase 2: Add Interactions API (Future)

Once refactoring is complete, these checkpoints will add Interactions API support:

### 🔲 Checkpoint 9: Add Interactions Models
- Create `genai-client/src/models/interactions/`
- Add request and response types

### 🔲 Checkpoint 10: Add Interactions Core Functions
- Create `genai-client/src/interactions.rs`
- Implement create, get, delete operations

### 🔲 Checkpoint 11: Add Public Interactions API
- Create `rust-genai/src/interactions.rs`
- Add `InteractionBuilder`

### 🔲 Checkpoint 12: Add Interactions Examples
- Add example files demonstrating Interactions API

### 🔲 Checkpoint 13: Add Interactions Tests
- Add integration tests

---

## Recovery Instructions

If you need to stop and resume:

### Saving Progress
```bash
# Commit work in progress
git add .
git commit -m "WIP: [checkpoint name]"

# Or stash changes
git stash save "WIP: [checkpoint name]"
```

### Resuming Work
```bash
# Switch back to branch
git checkout feature/interactions-api-prep

# Check where you left off
git log --oneline

# See this document for next steps
cat REFACTORING_PLAN.md
```

### Verifying State
```bash
# Always verify build and tests before continuing
cargo build
cargo test
```

### Abandoning Changes
```bash
# Return to main branch
git checkout master

# Delete feature branch
git branch -D feature/interactions-api-prep
```

---

## Notes

- Each checkpoint should take 5-15 minutes
- Total refactoring: ~8 checkpoints
- Stop at any checkpoint - codebase remains working
- Run `cargo build && cargo test` after every change
- Update this document as you complete checkpoints (mark ✅)

---

## Current Status

**Last Updated**: 2025-12-21
**Current Checkpoint**: 0 (Setup)
**Next Action**: Create feature branch and verify tests
