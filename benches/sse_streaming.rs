//! SSE streaming benchmarks for the rust-genai crate.
//!
//! This is a wrapper around the detailed genai-client SSE parser benchmarks,
//! testing the streaming from the user-facing API perspective.
//!
//! For more granular SSE parser benchmarks, see genai-client/benches/sse_parser.rs

use bytes::Bytes;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use futures_util::{StreamExt, pin_mut, stream};
use genai_client::StreamChunk;
use genai_client::sse_parser::parse_sse_stream;
use tokio::runtime::Runtime;

/// Generate realistic SSE data simulating Gemini API streaming response
fn generate_gemini_sse_stream(chunk_count: usize, text_per_chunk: usize) -> Vec<u8> {
    let mut data = Vec::new();

    for i in 0..chunk_count {
        // Simulate a realistic Gemini streaming response
        let chunk = format!(
            r#"data: {{"outputs":[{{"type":"text","text":"{}"}}],"status":"in_progress","name":"interactions/test","usageMetadata":{{"totalInputTokens":10,"totalOutputTokens":{}}}}}"#,
            "x".repeat(text_per_chunk),
            i + 1
        );
        data.extend_from_slice(chunk.as_bytes());
        data.extend_from_slice(b"\n\n");
    }

    // Final completed response
    data.extend_from_slice(
        br#"data: {"outputs":[{"type":"text","text":"done"}],"status":"completed","name":"interactions/test","usageMetadata":{"totalInputTokens":10,"totalOutputTokens":100}}"#,
    );
    data.extend_from_slice(b"\n\n");

    data
}

/// Benchmark streaming with varying chunk counts
fn bench_stream_chunk_counts(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("stream_chunk_counts");

    for chunk_count in [5, 20, 50, 100] {
        let sse_data = generate_gemini_sse_stream(chunk_count, 50);

        group.throughput(Throughput::Elements(chunk_count as u64));
        group.bench_with_input(
            BenchmarkId::new("chunks", chunk_count),
            &sse_data,
            |b, sse_data| {
                b.to_async(&rt).iter(|| async {
                    let byte_stream =
                        stream::iter(vec![Ok::<_, reqwest::Error>(Bytes::from(sse_data.clone()))]);
                    let parsed_stream = parse_sse_stream::<StreamChunk>(byte_stream);
                    pin_mut!(parsed_stream);

                    let mut count = 0;
                    while let Some(result) = parsed_stream.next().await {
                        criterion::black_box(result.unwrap());
                        count += 1;
                    }
                    assert!(count > 0);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark streaming with varying text sizes per chunk
fn bench_stream_text_sizes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("stream_text_sizes");

    for text_size in [10, 100, 500, 1000] {
        let sse_data = generate_gemini_sse_stream(20, text_size);
        let data_len = sse_data.len();

        group.throughput(Throughput::Bytes(data_len as u64));
        group.bench_with_input(
            BenchmarkId::new("text_bytes", text_size),
            &sse_data,
            |b, sse_data| {
                b.to_async(&rt).iter(|| async {
                    let byte_stream =
                        stream::iter(vec![Ok::<_, reqwest::Error>(Bytes::from(sse_data.clone()))]);
                    let parsed_stream = parse_sse_stream::<StreamChunk>(byte_stream);
                    pin_mut!(parsed_stream);

                    while let Some(result) = parsed_stream.next().await {
                        criterion::black_box(result.unwrap());
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark simulating realistic network chunking
fn bench_realistic_network_chunks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("realistic_network");

    // Generate a typical streaming session
    let sse_data = generate_gemini_sse_stream(30, 100);

    // Simulate 1KB network chunks (typical TCP segment)
    let chunks_1k: Vec<Bytes> = sse_data
        .chunks(1024)
        .map(|c| Bytes::from(c.to_vec()))
        .collect();

    group.throughput(Throughput::Bytes(sse_data.len() as u64));
    group.bench_function("1kb_network_chunks", |b| {
        b.to_async(&rt).iter(|| async {
            let byte_stream =
                stream::iter(chunks_1k.clone().into_iter().map(Ok::<_, reqwest::Error>));
            let parsed_stream = parse_sse_stream::<StreamChunk>(byte_stream);
            pin_mut!(parsed_stream);

            while let Some(result) = parsed_stream.next().await {
                criterion::black_box(result.unwrap());
            }
        });
    });

    // Simulate smaller chunks (HTTP/2 frames)
    let chunks_256: Vec<Bytes> = sse_data
        .chunks(256)
        .map(|c| Bytes::from(c.to_vec()))
        .collect();

    group.bench_function("256b_network_chunks", |b| {
        b.to_async(&rt).iter(|| async {
            let byte_stream =
                stream::iter(chunks_256.clone().into_iter().map(Ok::<_, reqwest::Error>));
            let parsed_stream = parse_sse_stream::<StreamChunk>(byte_stream);
            pin_mut!(parsed_stream);

            while let Some(result) = parsed_stream.next().await {
                criterion::black_box(result.unwrap());
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_stream_chunk_counts,
    bench_stream_text_sizes,
    bench_realistic_network_chunks
);
criterion_main!(benches);
