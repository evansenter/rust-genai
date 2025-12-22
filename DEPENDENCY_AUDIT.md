# Dependency Audit Report
**Last Updated**: 2025-12-22

## Summary
All dependencies are managed via Cargo workspace dependencies and are **up-to-date** with the latest stable versions.

**Workspace Structure:**
- `rust-genai` (root package)
- `genai-client` (workspace member)
- `rust-genai-macros` (workspace member)

All crates use **Rust Edition 2024**.

## ✅ Current Status

### All Packages Up-to-Date

| Package | Version | Status |
|---------|---------|--------|
| `reqwest` | 0.12.26 | ✅ Current |
| `tokio` | 1.48.0 | ✅ Current |
| `serde` | 1.0.228 | ✅ Current |
| `serde_json` | 1.0.145 | ✅ Current |
| `thiserror` | 2.0.17 | ✅ Current |
| `async-stream` | 0.3.6 | ✅ Current |
| `async-trait` | 0.1.89 | ✅ Current |
| `futures-util` | 0.3.31 | ✅ Current |
| `log` | 0.4.29 | ✅ Current |
| `inventory` | 0.3.21 | ✅ Current |
| `bytes` | 1.10.1 | ✅ Current |

### Dependencies Analysis

**Core Dependencies (All Used):**
- `reqwest` - HTTP client
- `tokio` - Async runtime
- `serde` / `serde_json` - Serialization
- `async-stream` - Stream utilities
- `futures-util` - Future utilities
- `async-trait` - Async traits
- `log` - Logging
- `inventory` - Plugin registry

**Macro Crate Dependencies:**
- `syn` / `quote` / `proc-macro2` - Macro development
- `utoipa` - OpenAPI schema generation

**No unused dependencies detected.**

## 🔧 Workspace Dependencies

All workspace members use centralized `[workspace.dependencies]` in the root `Cargo.toml`:
- Single source of truth for all dependency versions
- Zero version conflicts between workspace members
- Simplified dependency updates across the entire workspace
- Automatic prevention of version drift

## 🔄 Maintenance Guide

### Update a single dependency:
```bash
# Edit Cargo.toml [workspace.dependencies] section
# Change version (e.g., tokio = "1.48" → tokio = "1.49")
cargo update <package-name>
cargo test --all
```

### Update all dependencies:
```bash
cargo update
cargo test --all
```

### Check for issues:
```bash
cargo search <package-name> --limit 1  # Check latest version
cargo tree --duplicates                 # Check for duplicate versions
cargo audit                             # Check for security vulnerabilities
```

### Monthly Maintenance Checklist:
- [ ] Run `cargo update` to get latest compatible versions
- [ ] Run `cargo tree --duplicates` to check for new duplicates
- [ ] Run `cargo audit` for security vulnerabilities
- [ ] Review dependency changes in Cargo.lock before committing
- [ ] Ensure all tests pass: `cargo test --all`

## 📚 References
- [Cargo Workspace Dependencies](https://doc.rust-lang.org/cargo/reference/workspaces.html#the-dependencies-table)
- [RustSec Advisory Database](https://rustsec.org/advisories/)
