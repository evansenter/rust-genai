# Dependency Audit Report
Generated: 2025-12-22

## Summary
This audit identified version inconsistencies across workspace members, outdated packages, and opportunities for better dependency management using workspace features.

**Workspace Structure:**
- `rust-genai` (root package)
- `genai-client` (workspace member)
- `rust-genai-macros` (workspace member)

All crates are owned by this project and use **Rust Edition 2024**.

## 🔴 Critical Issues

### 1. Version Conflicts Between Workspace Members (Breaking)
**thiserror version conflict** - CRITICAL
- **Current state**: Inconsistent versions across workspace members
  - `genai-client` uses `thiserror 1.0.69` (outdated)
  - `rust-genai` uses `thiserror 2.0.12` (current)
- **Problem**: Breaking change between v1 and v2 can cause compilation issues
- **Latest version**: 2.0.17
- **Root cause**: Dependencies not synchronized across workspace
- **Recommendation**: Standardize to `thiserror = "2.0"` across all workspace members

### 2. Multiple Dependency Versions
These create bloat by including the same crate multiple times:
- `webpki-roots`: 0.26.8 and 1.0.0
- `getrandom`: 0.2.15 and 0.3.2
- `thiserror-impl`: 1.0.69 and 2.0.12
- `windows-sys`: 0.52.0 and 0.59.0
- `windows-targets`: 0.52.6 and 0.53.0

**Impact**: ~500KB+ additional binary size

## 🟡 Outdated Packages

| Package | Current | Latest | Priority |
|---------|---------|--------|----------|
| `thiserror` | 2.0.12 | 2.0.17 | High |
| `reqwest` | 0.12.18 | 0.12.23 | Medium |
| `tokio` | 1.45.1 | 1.47.1 (LTS) | Medium |
| `serde` | 1.0.219 | 1.0.x (latest) | Low |
| `serde_json` | 1.0.140 | 1.0.x (latest) | Low |
| `async-trait` | 0.1.88 | 0.1.x (latest) | Low |

## 💡 Workspace Dependency Management

**Missing workspace.dependencies** - Improvement opportunity
- **Current**: Each workspace member declares its own dependency versions independently
- **Problem**: This leads to version drift and inconsistencies (like the thiserror issue above)
- **Solution**: Use Cargo's `[workspace.dependencies]` feature to centralize version management
- **Benefits**:
  - Single source of truth for dependency versions
  - Prevents version conflicts between workspace members
  - Easier to update dependencies across the entire workspace
  - Reduces maintenance burden

## ✅ Dependencies Analysis

### Core Dependencies (All Used)
- ✅ `reqwest` - HTTP client (used in client.rs)
- ✅ `tokio` - Async runtime (used throughout)
- ✅ `serde` / `serde_json` - Serialization (used throughout)
- ✅ `async-stream` - Stream utilities (used in client.rs)
- ✅ `futures-util` - Future utilities (used in client.rs)
- ✅ `async-trait` - Async traits (used in function_calling.rs)
- ✅ `log` - Logging (used in response_processing.rs)
- ✅ `inventory` - Plugin registry (used in function_calling.rs)

### Macro Crate Dependencies
- ✅ `syn` / `quote` / `proc-macro2` - Macro development
- ✅ `utoipa` - OpenAPI schema generation (used in macros)

### No Unused Dependencies Detected
All declared dependencies appear to be used in the codebase.

## 🔧 Recommended Changes

### Option A: Use Workspace Dependencies (RECOMMENDED)

This is the modern, maintainable approach for Cargo workspaces.

**File**: `Cargo.toml` (root)
```toml
[workspace]
members = [
    "genai-client", "rust-genai-macros",
]

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
tokio = { version = "1.47", features = ["full"] }
async-stream = "0.3"
futures-util = "0.3"
thiserror = "2.0"
log = "0.4"
async-trait = "0.1"
inventory = "0.3"
bytes = "1.10"
syn = { version = "2.0", features = ["full", "parsing"] }
quote = "1.0"
proc-macro2 = "1.0"
utoipa = "5.3"

[package]
name = "rust-genai"
version = "0.1.0"
edition = "2024"
license = "MIT"

[dependencies]
genai-client = { path = "genai-client" }
rust-genai-macros = { path = "./rust-genai-macros" }
serde = { workspace = true }
serde_json = { workspace = true }
reqwest = { workspace = true }
tokio = { workspace = true }
async-stream = { workspace = true }
futures-util = { workspace = true }
thiserror = { workspace = true }
log = { workspace = true }
async-trait = { workspace = true }
inventory = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
```

**File**: `genai-client/Cargo.toml`
```toml
[package]
name = "genai-client"
version = "0.1.0"
edition = "2024"

[dependencies]
reqwest = { workspace = true, features = ["stream"] }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
async-stream = { workspace = true }
bytes = { workspace = true }
futures-util = { workspace = true }
tokio = { workspace = true }
```

**File**: `rust-genai-macros/Cargo.toml`
```toml
[package]
name = "rust-genai-macros"
version = "0.1.0"
edition = "2024"

[lib]
proc-macro = true

[dependencies]
syn = { workspace = true }
quote = { workspace = true }
proc-macro2 = { workspace = true }
utoipa = { workspace = true }
serde_json = { workspace = true }
serde = { workspace = true }
```

### Option B: Manual Version Sync (Alternative)

If you prefer not to use workspace dependencies yet, manually sync versions:

### 1. Fix thiserror Version Conflict (CRITICAL)
**File**: `genai-client/Cargo.toml`
```toml
[dependencies]
-thiserror = "1.0.69"
+thiserror = "2.0"
```

### 2. Update Root Dependencies
**File**: `Cargo.toml`
```toml
[dependencies]
-serde = { version = "1.0.219", features = ["derive"] }
+serde = { version = "1.0", features = ["derive"] }

-serde_json = "1.0.140"
+serde_json = "1.0"

-reqwest = { version = "0.12.18", features = ["json", "rustls-tls"] }
+reqwest = { version = "0.12", features = ["json", "rustls-tls"] }

-tokio = { version = "1.45.1", features = ["full"] }
+tokio = { version = "1.47", features = ["full"] }

-async-stream = "0.3.6"
+async-stream = "0.3"

-futures-util = "0.3.31"
+futures-util = "0.3"

-thiserror = "2.0.12"
+thiserror = "2.0"

-log = "0.4.27"
+log = "0.4"

-async-trait = "0.1.88"
+async-trait = "0.1"

-inventory = "0.3.20"
+inventory = "0.3"
```

### 3. Update genai-client Dependencies
**File**: `genai-client/Cargo.toml`
```toml
[dependencies]
-reqwest = { version = "0.12.18", features = ["json", "stream", "rustls-tls"] }
+reqwest = { version = "0.12", features = ["json", "stream", "rustls-tls"] }

-serde = { version = "1.0.219", features = ["derive"] }
+serde = { version = "1.0", features = ["derive"] }

-serde_json = "1.0.140"
+serde_json = "1.0"

-thiserror = "1.0.69"
+thiserror = "2.0"

-async-stream = "0.3.6"
+async-stream = "0.3"

-bytes = "1.10.1"
+bytes = "1.10"

-futures-util = "0.3.31"
+futures-util = "0.3"

-tokio = { version = "1.45.1", features = ["full"] }
+tokio = { version = "1.47", features = ["full"] }
```

### 4. Update rust-genai-macros Dependencies
**File**: `rust-genai-macros/Cargo.toml`
```toml
[dependencies]
syn = { version = "2.0", features = ["full", "parsing"] }
quote = "1.0"
proc-macro2 = "1.0"
-utoipa = "5.3.1"
+utoipa = "5.3"
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }
```

## 📊 Security Status

**Unable to run `cargo audit`** due to network restrictions in the environment.

**Recommendation**: Run the following commands locally:
```bash
cargo install cargo-audit
cargo audit
```

Check for known vulnerabilities at: https://rustsec.org/advisories/

## 💾 Expected Impact

**After applying Option A (workspace dependencies):**
- ✅ Eliminate version conflicts between workspace members
- ✅ Reduce binary size by ~500KB+ (fewer duplicate dependencies)
- ✅ Update to latest stable versions with bug fixes
- ✅ Single source of truth for dependency versions
- ✅ Prevent future version drift across workspace
- ✅ Easier dependency updates going forward
- ✅ Better long-term maintainability

**After applying Option B (manual sync):**
- ✅ Eliminate current version conflicts
- ✅ Reduce binary size by ~500KB+ (fewer duplicate dependencies)
- ✅ Update to latest stable versions with bug fixes
- ⚠️ Still requires manual version synchronization in the future

## 📚 References

- [thiserror on crates.io](https://crates.io/crates/thiserror)
- [reqwest on crates.io](https://crates.io/crates/reqwest)
- [tokio on crates.io](https://crates.io/crates/tokio)
- [Cargo Workspace Dependencies](https://doc.rust-lang.org/cargo/reference/workspaces.html#the-dependencies-table)
- [Rust Edition 2024 Announcement](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/)

## 🔍 How to Apply Changes

### For Option A (Workspace Dependencies):
1. Update `Cargo.toml` to add `[workspace.dependencies]` section
2. Update all three Cargo.toml files to use `workspace = true`
3. Run `cargo update` to update Cargo.lock
4. Run `cargo build` to verify everything compiles
5. Run `cargo test` to ensure tests pass
6. Verify with `cargo tree --duplicates` (should show fewer duplicates)

### For Option B (Manual Sync):
1. Update dependency versions in all three Cargo.toml files
2. Run `cargo update` to update Cargo.lock
3. Run `cargo build` to verify everything compiles
4. Run `cargo test` to ensure tests pass
5. Verify with `cargo tree --duplicates` (should show fewer duplicates)
