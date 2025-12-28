# Benchmarking Guide

This project uses [Criterion](https://github.com/bheisler/criterion.rs) for performance benchmarking.

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench --workspace

# Run specific benchmark suite
cargo bench --bench sse_streaming
cargo bench --bench serialization
cargo bench --bench multimodal
cargo bench --bench function_registry
cargo bench --bench response_extraction

# Run with HTML report generation
cargo bench --workspace

# Run without plots (faster, used in CI)
cargo bench --workspace -- --noplot
```

## Benchmark Suites

### Root Crate (`benches/`)

User-facing API benchmarks:

| Suite | Description |
|-------|-------------|
| `sse_streaming` | SSE stream parsing with realistic Gemini API responses |
| `serialization` | Request/response JSON serialization, Evergreen unknown handling |
| `multimodal` | File loading, MIME detection, base64 encoding |
| `function_registry` | FunctionDeclaration building and HashMap operations |
| `response_extraction` | Response accessor methods (`text()`, `function_calls()`, etc.) |

### genai-client (`genai-client/benches/`)

Low-level internal benchmarks:

| Suite | Description |
|-------|-------------|
| `sse_parser` | Raw SSE parser with worst-case chunk scenarios |
| `content_serialization` | InteractionContent serde with all variants |

## Key Metrics

- **Throughput**: Bytes/sec or elements/sec for streaming operations
- **Latency**: Time per operation for accessor methods
- **Scaling**: How performance changes with input size

## Interpreting Results

Criterion generates HTML reports in `target/criterion/`. Open `target/criterion/report/index.html` for:

- Performance comparisons vs previous runs
- Statistical analysis (mean, median, std dev)
- Regression detection

## CI Integration

Benchmarks run automatically on pushes to `main` (not on PRs, to avoid slowing CI feedback):
- Results uploaded as artifacts (30-day retention)
- Available in Actions → workflow run → Artifacts → `benchmark-results`
- Compare results across commits to detect performance regressions

## Adding New Benchmarks

1. Create a new file in `benches/` or `genai-client/benches/`
2. Add `[[bench]]` entry to the appropriate `Cargo.toml`:
   ```toml
   [[bench]]
   name = "my_benchmark"
   harness = false
   ```
3. Use Criterion's async support for async operations:
   ```rust
   use criterion::{criterion_group, criterion_main, Criterion};
   use tokio::runtime::Runtime;

   fn my_bench(c: &mut Criterion) {
       let rt = Runtime::new().unwrap();
       c.bench_function("my_async_op", |b| {
           b.to_async(&rt).iter(|| async {
               // async operation
           });
       });
   }

   criterion_group!(benches, my_bench);
   criterion_main!(benches);
   ```

## Performance-Critical Areas

Based on codebase analysis, these areas have the highest performance impact:

1. **SSE Parser** (`sse_parser.rs`): Buffer management with `drain()` pattern
2. **Content Deserialization**: Double-deserialization for Evergreen unknown handling
3. **File Loading**: Full file read + base64 encoding for multimodal content
4. **Response Accessors**: Repeated Vec iteration per accessor call
