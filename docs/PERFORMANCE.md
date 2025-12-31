# Performance Guide

This document tracks performance baselines and provides guidance for maintaining performance during development.

## Baseline Metrics (December 2025)

Performance measurements taken on Apple M2 Pro (macOS).

### Build Times

| Metric | Time | Notes |
|--------|------|-------|
| Clean release build | ~21s | Full workspace from scratch |
| Incremental release build | ~2-3s | After touching `src/lib.rs` |
| Full examples build | ~65s | All 27 examples |

### Binary Sizes

Example binaries compiled in release mode:

| Size Range | Examples |
|------------|----------|
| 5.6 MB | `simple_interaction`, `streaming` |
| 5.7-5.8 MB | Most examples (audio, video, function calling, tool_service) |
| 6.1-6.2 MB | Code execution, structured output, Google Search |
| 6.3 MB | `auto_function_calling`, `web_scraper_agent` |

All examples are under 7 MB, which is acceptable for a library with async runtime, HTTP client, and TLS.

### Dependencies

- **Direct dependencies**: 13 runtime + 7 dev-only
- **Total dependency tree**: ~467 lines (with deduplication)

## Benchmark Results

Run benchmarks with:

```bash
cargo +nightly bench --bench <name>
```

### Serialization (`benches/serialization.rs`)

Measures request/response JSON serialization performance.

| Benchmark | Typical Time | Throughput |
|-----------|-------------|------------|
| Request serialization (1 text) | ~245 ns | 1.1-1.2 GiB/s |
| Request serialization (10 text) | ~790 ns | 1.8 GiB/s |
| Request serialization (10 functions) | ~1.9 µs | 1.4 GiB/s |
| Response deserialization (1 output) | ~490 ns | 515 MiB/s |
| Response deserialization (20 outputs) | ~5.4 µs | 500 MiB/s |
| Unknown content handling (mixed) | ~2.3 µs | - |
| Unknown content handling (all unknown) | ~3.5 µs | - |

**Note**: Unknown content deserialization (Evergreen pattern) adds ~50% overhead vs known types, but preserves API forward compatibility.

### Multimodal (`benches/multimodal.rs`)

Measures file loading and base64 encoding performance.

| Benchmark | Typical Time | Throughput |
|-----------|-------------|------------|
| MIME detection | 20-24 ns | - |
| Base64 encoding (1 KB) | ~260 ns | 3.6 GiB/s |
| Base64 encoding (1 MB) | ~235 µs | 4.0 GiB/s |
| Base64 encoding (10 MB) | ~2.3 ms | 4.0 GiB/s |
| File read (1 KB) | ~13 µs | 73 MiB/s |
| File read (1 MB) | ~39 µs | 23 GiB/s |
| `image_from_file` (1 MB) | ~300 µs | 3.1 GiB/s |

**Key insight**: For large files (>100 KB), base64 encoding dominates over file I/O.

### Function Registry (`benches/function_registry.rs`)

Measures function declaration building and registry lookup.

| Benchmark | Typical Time |
|-----------|-------------|
| Declaration building (1 param) | ~420 ns |
| Declaration building (20 params) | ~6.6 µs |
| Builder pattern (3 params) | ~425 ns |
| Builder pattern (10 params) | ~1.9 µs |
| Registry lookup (10 functions) | ~9 ns |
| Registry lookup (500 functions) | ~8 ns |
| Collect all declarations (100 functions) | ~73 µs |
| Declaration serialization (1 param) | ~155 ns |
| Declaration serialization (20 params) | ~1.3 µs |

**Note**: HashMap lookup is O(1) and consistently fast regardless of registry size.

### Response Extraction (`benches/response_extraction.rs`)

Measures accessor methods on `InteractionResponse`.

| Benchmark | Typical Time |
|-----------|-------------|
| `text()` accessor | <1 ns (optimized away with constant) |
| `text()` at end of 50 items | ~14 ns |
| `all_text()` (100 items) | ~440 ns |
| `all_text()` (10000 char text × 10) | ~1.9 µs |
| `function_calls()` (20 calls) | ~6.5 ns |
| `thoughts()` (100 items) | ~115 ns |
| `has_function_calls()` | ~2-3 ns |
| All common accessors combined | ~265 ns |

**Note**: Accessors are extremely fast - suitable for hot paths.

### ToolService Patterns (`benches/tool_service.rs`)

Measures ToolService trait, function map building, and auto-function accumulator.

| Benchmark | Typical Time |
|-----------|-------------|
| **ToolService::tools()** | |
| 1 tool | ~60 ns |
| 10 tools | ~460 ns |
| 50 tools | ~2.2 µs |
| **Build service function map** | |
| 1 tool | ~275 ns |
| 10 tools | ~2.4 µs |
| 50 tools | ~12 µs |
| None (empty) | ~3.5 ns |
| **Service map lookup** | |
| Existing key (any size) | ~7-9 ns |
| Missing key | ~6-7 ns |
| **Arc cloning** | ~3.8 ns |
| **AutoFunctionResultAccumulator** | |
| Push Delta chunk (no-op) | ~38 ns |
| Push FunctionResults (1 result) | ~100 ns |
| Push FunctionResults (10 results) | ~580 ns |
| Push Complete chunk | ~68 ns |
| Multi-round (5 rounds, 2 funcs each) | ~1.5 µs |
| **FunctionExecutionResult creation** | |
| Simple payload | ~108 ns |
| Large payload (100 items) | ~1.7 µs |
| **AutoFunctionStreamChunk serialization** | |
| Delta | ~110 ns |
| FunctionResults (3 results) | ~307 ns |
| Complete | ~216 ns |

**Key insights**:
- Service function map building is O(n) with tools count
- HashMap lookup is O(1) and consistent regardless of map size
- Arc cloning is extremely cheap (~4 ns)
- Accumulator operations are efficient for streaming auto-function workflows

## Benchmark Coverage Analysis

### Well-Covered Areas

- JSON serialization/deserialization
- File loading and base64 encoding
- Function declaration building (builder pattern, direct construction)
- Registry lookup (HashMap performance)
- Response accessor methods
- ToolService patterns (map building, lookup, Arc cloning)
- AutoFunctionResult accumulator operations
- Stream chunk serialization

### Remaining Coverage Gaps

The following areas may benefit from additional benchmarks:

1. **Image Generation**
   - Response modality deserialization for image outputs
   - Base64 decode of generated image data
   - `InteractionContent::Image` pattern matching

2. **Streaming Performance**
   - SSE chunk parsing throughput
   - Backpressure handling with slow consumers

3. **Memory Allocation**
   - Allocation patterns during large response parsing
   - Peak memory usage with many function calls

## Running Benchmarks

### Save Baseline

```bash
cargo +nightly bench --bench serialization -- --save-baseline main
cargo +nightly bench --bench multimodal -- --save-baseline main
cargo +nightly bench --bench function_registry -- --save-baseline main
cargo +nightly bench --bench response_extraction -- --save-baseline main
```

### Compare Against Baseline

```bash
cargo +nightly bench --bench serialization -- --baseline main
```

### Benchmarking Tips

- Use `--save-baseline <name>` before making changes
- Use `--baseline <name>` after changes to compare
- Run benchmarks multiple times for stable results
- Consider `--warm-up-time 3 --measurement-time 10` for CI

## Performance Guidelines

### When to Be Concerned

- **Serialization**: >2x slowdown on typical request/response sizes
- **Build time**: Clean build exceeding 60s, incremental exceeding 10s
- **Binary size**: Examples exceeding 10 MB
- **Registry lookup**: Any measurable regression (currently ~9 ns)
- **File loading**: >2x regression on multimodal content

### Acceptable Trade-offs

- Unknown content handling adds overhead for forward compatibility (Evergreen pattern)
- ToolService lookup adds one HashMap check before global registry
- Streaming auto-functions duplicate some work for accumulated function calls

## Updating Baselines

When making intentional performance changes:

1. Run benchmarks before changes: `cargo bench -- --save-baseline before`
2. Make changes
3. Run benchmarks after: `cargo bench -- --save-baseline after`
4. Compare: `cargo bench -- --baseline before --load-baseline after`
5. Document any significant changes in commit message
6. Update baselines: `cargo bench -- --save-baseline main`

## CI Integration

Performance tracking is integrated into CI (`.github/workflows/rust.yml`):

### Benchmark Job

- **Triggers**: All PRs and main branch pushes
- **Runtime**: Uses nightly Rust for benchmarks
- **Comparison**: PRs are compared against main branch baseline
- **Output**: Benchmark comparison in GitHub Actions job summary
- **Artifacts**: Results stored for 30 days

### Build Metrics Job

- **Triggers**: All PRs
- **Tracks**: Clean build time, examples build time, binary sizes
- **Limits**: Fails if any example exceeds 10MB

### Interpreting CI Results

1. Check the "Benchmarks" job summary for performance comparison
2. Look for "regression" or "improvement" in the output
3. Build metrics show if compile times or binary sizes changed
4. Any regression >10% warrants investigation before merging
