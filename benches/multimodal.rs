//! Benchmarks for multimodal file loading and base64 encoding.
//!
//! These benchmarks measure the performance of:
//! - File loading from disk
//! - Base64 encoding of binary data
//! - MIME type detection
//!
//! Note: These benchmarks create temporary files for testing.

use base64::Engine;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rust_genai::detect_mime_type;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;
use tokio::runtime::Runtime;

/// Create a temporary file with random-ish binary content of specified size
fn create_temp_file(size: usize, extension: &str) -> NamedTempFile {
    let mut file = tempfile::Builder::new()
        .suffix(&format!(".{}", extension))
        .tempfile()
        .expect("Failed to create temp file");

    // Write deterministic "random" data (repeating pattern for reproducibility)
    let pattern: Vec<u8> = (0..256).map(|i| i as u8).collect();
    let mut remaining = size;
    while remaining > 0 {
        let chunk_size = remaining.min(pattern.len());
        file.write_all(&pattern[..chunk_size])
            .expect("Failed to write to temp file");
        remaining -= chunk_size;
    }
    file.flush().expect("Failed to flush temp file");
    file
}

/// Benchmark MIME type detection (synchronous, very fast operation)
fn bench_mime_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("mime_detection");

    let test_paths = [
        ("image_jpeg", "photo.jpg"),
        ("image_png", "image.png"),
        ("audio_mp3", "song.mp3"),
        ("video_mp4", "clip.mp4"),
        ("document_pdf", "doc.pdf"),
        ("unknown", "file.xyz"),
        ("deep_path", "/very/deep/nested/path/to/file.png"),
    ];

    for (name, path_str) in test_paths {
        let path = Path::new(path_str);
        group.bench_function(name, |b| {
            b.iter(|| {
                let mime = detect_mime_type(criterion::black_box(path));
                criterion::black_box(mime)
            });
        });
    }

    group.finish();
}

/// Benchmark raw base64 encoding (isolated from file I/O)
fn bench_base64_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("base64_encoding");

    for size in [1_000, 10_000, 100_000, 1_000_000, 10_000_000] {
        // Create test data
        let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("bytes", size), &data, |b, data| {
            b.iter(|| {
                let encoded =
                    base64::engine::general_purpose::STANDARD.encode(criterion::black_box(data));
                criterion::black_box(encoded)
            });
        });
    }

    group.finish();
}

/// Benchmark file reading (tokio::fs::read)
fn bench_file_reading(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("file_reading");

    for size in [1_000, 10_000, 100_000, 1_000_000] {
        let temp_file = create_temp_file(size, "bin");
        let path = temp_file.path().to_path_buf();

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("bytes", size), &path, |b, path| {
            b.to_async(&rt).iter(|| async {
                let data = tokio::fs::read(criterion::black_box(path)).await.unwrap();
                criterion::black_box(data)
            });
        });
    }

    group.finish();
}

/// Benchmark complete file loading + base64 encoding pipeline
fn bench_file_load_and_encode(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("file_load_and_encode");

    for size in [1_000, 10_000, 100_000, 1_000_000] {
        let temp_file = create_temp_file(size, "bin");
        let path = temp_file.path().to_path_buf();

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("bytes", size), &path, |b, path| {
            b.to_async(&rt).iter(|| async {
                let data = tokio::fs::read(criterion::black_box(path)).await.unwrap();
                let encoded = base64::engine::general_purpose::STANDARD.encode(&data);
                criterion::black_box(encoded)
            });
        });
    }

    group.finish();
}

/// Benchmark using the actual rust_genai helper functions
fn bench_image_from_file(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("image_from_file");

    for size in [1_000, 10_000, 100_000, 1_000_000] {
        let temp_file = create_temp_file(size, "png");
        let path = temp_file.path().to_path_buf();

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("bytes", size), &path, |b, path| {
            b.to_async(&rt).iter(|| async {
                let content = rust_genai::image_from_file(criterion::black_box(path))
                    .await
                    .unwrap();
                criterion::black_box(content)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_mime_detection,
    bench_base64_encoding,
    bench_file_reading,
    bench_file_load_and_encode,
    bench_image_from_file
);
criterion_main!(benches);
