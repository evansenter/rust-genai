.PHONY: check fmt clippy test test-all docs clean

# Pre-push gate: format check + lint + unit tests
check: fmt clippy test

# Check formatting
fmt:
	cargo fmt -- --check

# Lint with warnings as errors
clippy:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

# Unit tests only (nextest does not run doctests)
test:
	cargo nextest run

# Full test suite including integration tests (requires GEMINI_API_KEY)
test-all:
	cargo nextest run --run-ignored all

# Build documentation with warnings as errors
docs:
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items

# Clean build artifacts
clean:
	cargo clean
