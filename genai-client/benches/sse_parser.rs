//! Benchmarks for SSE (Server-Sent Events) parser throughput.
//!
//! These benchmarks measure the performance of parsing SSE streams under various conditions:
//! - Different chunk sizes (simulating network chunking behavior)
//! - Different message sizes
//! - High message counts (throughput measurement)

use bytes::Bytes;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use futures_util::{StreamExt, pin_mut, stream};
use genai_client::sse_parser::parse_sse_stream;
use serde::Deserialize;
use tokio::runtime::Runtime;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TestMessage {
    text: String,
}

/// Generate SSE data for a single message with given text size
fn generate_sse_message(text_size: usize) -> Vec<u8> {
    let text = "x".repeat(text_size);
    format!("data: {{\"text\":\"{}\"}}\n\n", text).into_bytes()
}

/// Generate SSE data for multiple messages
fn generate_sse_messages(count: usize, text_size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(count * (text_size + 30));
    for i in 0..count {
        data.extend_from_slice(
            format!(
                "data: {{\"text\":\"Message {} {}\"}}\n\n",
                i,
                "x".repeat(text_size)
            )
            .as_bytes(),
        );
    }
    data
}

/// Split data into chunks of specified size
fn chunk_data(data: Vec<u8>, chunk_size: usize) -> Vec<Bytes> {
    data.chunks(chunk_size)
        .map(|c| Bytes::from(c.to_vec()))
        .collect()
}

/// Benchmark SSE parsing with different chunk sizes
fn bench_chunk_sizes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("sse_chunk_sizes");

    // Generate a reasonably sized message (~1KB of text)
    let message_data = generate_sse_message(1000);
    let data_len = message_data.len();

    for chunk_size in [1, 16, 64, 256, 1024, 4096] {
        group.throughput(Throughput::Bytes(data_len as u64));
        group.bench_with_input(
            BenchmarkId::new("chunk_bytes", chunk_size),
            &chunk_size,
            |b, &chunk_size| {
                b.to_async(&rt).iter(|| async {
                    let chunks = chunk_data(message_data.clone(), chunk_size);
                    let byte_stream = stream::iter(chunks.into_iter().map(Ok::<_, reqwest::Error>));
                    let parsed_stream = parse_sse_stream::<TestMessage>(byte_stream);
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

/// Benchmark SSE parsing with different message sizes
fn bench_message_sizes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("sse_message_sizes");

    for text_size in [100, 1_000, 10_000, 100_000, 1_000_000] {
        let message_data = generate_sse_message(text_size);
        let data_len = message_data.len();

        group.throughput(Throughput::Bytes(data_len as u64));
        group.bench_with_input(
            BenchmarkId::new("text_bytes", text_size),
            &message_data,
            |b, message_data| {
                b.to_async(&rt).iter(|| async {
                    let byte_stream = stream::iter(vec![Ok::<_, reqwest::Error>(Bytes::from(
                        message_data.clone(),
                    ))]);
                    let parsed_stream = parse_sse_stream::<TestMessage>(byte_stream);
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

/// Benchmark SSE parsing throughput with many messages
fn bench_message_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("sse_message_throughput");

    for message_count in [10, 100, 1000] {
        let message_data = generate_sse_messages(message_count, 100);

        group.throughput(Throughput::Elements(message_count as u64));
        group.bench_with_input(
            BenchmarkId::new("messages", message_count),
            &message_data,
            |b, message_data| {
                b.to_async(&rt).iter(|| async {
                    let byte_stream = stream::iter(vec![Ok::<_, reqwest::Error>(Bytes::from(
                        message_data.clone(),
                    ))]);
                    let parsed_stream = parse_sse_stream::<TestMessage>(byte_stream);
                    pin_mut!(parsed_stream);

                    let mut count = 0;
                    while let Some(result) = parsed_stream.next().await {
                        criterion::black_box(result.unwrap());
                        count += 1;
                    }
                    assert_eq!(count, message_count);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark worst-case scenario: 1-byte chunks with many newlines
fn bench_worst_case_chunking(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("sse_worst_case");

    // Many small messages = many newlines to process
    let message_data = generate_sse_messages(100, 10);
    let data_len = message_data.len();

    group.throughput(Throughput::Bytes(data_len as u64));
    group.bench_function("single_byte_chunks_100_messages", |b| {
        b.to_async(&rt).iter(|| async {
            let chunks = chunk_data(message_data.clone(), 1);
            let byte_stream = stream::iter(chunks.into_iter().map(Ok::<_, reqwest::Error>));
            let parsed_stream = parse_sse_stream::<TestMessage>(byte_stream);
            pin_mut!(parsed_stream);

            let mut count = 0;
            while let Some(result) = parsed_stream.next().await {
                criterion::black_box(result.unwrap());
                count += 1;
            }
            assert_eq!(count, 100);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_chunk_sizes,
    bench_message_sizes,
    bench_message_throughput,
    bench_worst_case_chunking
);
criterion_main!(benches);
