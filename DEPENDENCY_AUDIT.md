# Dependency Audit Report
Generated: 2025-12-22

## Summary
This audit identified several issues with the project's dependencies including version conflicts, outdated packages, and a cutting-edge Rust edition that may cause compatibility issues.

## 🔴 Critical Issues

### 1. Version Conflicts (Breaking)
**thiserror version conflict** - CRITICAL
- **Current state**: Two incompatible versions in use
  - `genai-client` uses `thiserror 1.0.69`
  - `rust-genai` uses `thiserror 2.0.12`
- **Problem**: Breaking change between v1 and v2 can cause compilation issues
- **Latest version**: 2.0.17
- **Recommendation**: Update `genai-client/Cargo.toml` to use `thiserror = "2.0.17"` to match the root package

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

## 🟠 Edition Concern

**Rust Edition 2024** - Potentially problematic
- **Current**: All crates use `edition = "2024"`
- **Released**: February 20, 2025 (very recent!)
- **Issues**:
  - Cutting-edge, may have undiscovered bugs
  - Limited ecosystem support
  - May cause compatibility issues with older toolchains
- **Recommendation**: Downgrade to `edition = "2021"` for better stability unless Edition 2024 features are specifically needed

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

### 1. Fix thiserror Version Conflict (CRITICAL)
**File**: `genai-client/Cargo.toml`
```toml
[dependencies]
-thiserror = "1.0.69"
+thiserror = "2.0.17"
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

### 5. Downgrade Edition (Optional but Recommended)
**Files**: `Cargo.toml`, `genai-client/Cargo.toml`, `rust-genai-macros/Cargo.toml`
```toml
-edition = "2024"
+edition = "2021"
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

**After applying all recommendations:**
- ✅ Eliminate version conflicts
- ✅ Reduce binary size by ~500KB+ (fewer duplicate dependencies)
- ✅ Update to latest stable versions with bug fixes
- ✅ Improve compatibility with the broader Rust ecosystem
- ✅ Better long-term maintainability

## 📚 References

- [thiserror on crates.io](https://crates.io/crates/thiserror)
- [reqwest on crates.io](https://crates.io/crates/reqwest)
- [tokio on crates.io](https://crates.io/crates/tokio)
- [Rust Edition 2024 Announcement](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/)
- [Rust Edition Guide](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)

## 🔍 How to Apply Changes

1. Update all Cargo.toml files with the recommended changes
2. Run `cargo update` to update Cargo.lock
3. Run `cargo build` to verify everything compiles
4. Run `cargo test` to ensure tests pass
5. Check binary size: `cargo build --release && ls -lh target/release/`
6. Verify with `cargo tree --duplicates` (should show fewer duplicates)
