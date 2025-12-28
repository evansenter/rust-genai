//! Benchmarks for InteractionContent serialization and deserialization.
//!
//! These benchmarks measure the performance of:
//! - Deserializing different content types from JSON
//! - Serializing content back to JSON
//! - Handling Unknown variants (Evergreen compatibility)
//! - Mixed content type arrays (typical API responses)

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use genai_client::InteractionContent;

/// Generate JSON for a Text content item
fn text_json(text_size: usize) -> String {
    let text = "x".repeat(text_size);
    format!(r#"{{"type":"text","text":"{}"}}"#, text)
}

/// Generate JSON for a FunctionCall content item
fn function_call_json(arg_count: usize) -> String {
    let args: Vec<String> = (0..arg_count)
        .map(|i| format!(r#""arg{}":"value{}""#, i, i))
        .collect();
    format!(
        r#"{{"type":"functionCall","id":"call_123","name":"test_function","args":{{{}}}}}"#,
        args.join(",")
    )
}

/// Generate JSON for an Image content item with base64 data
fn image_json(data_size: usize) -> String {
    // Simulate base64-encoded data
    let data = "A".repeat(data_size);
    format!(
        r#"{{"type":"image","data":"{}","mimeType":"image/png"}}"#,
        data
    )
}

/// Generate JSON for a FunctionResult content item
fn function_result_json(result_size: usize) -> String {
    let result_text = "r".repeat(result_size);
    format!(
        r#"{{"type":"functionResult","name":"test_function","callId":"call_123","result":{{"output":"{}"}}}}"#,
        result_text
    )
}

/// Generate JSON for an Unknown content type
fn unknown_json(extra_fields: usize) -> String {
    let fields: Vec<String> = (0..extra_fields)
        .map(|i| format!(r#""field{}":"value{}""#, i, i))
        .collect();
    format!(
        r#"{{"type":"futureFeature","specialData":"something",{}}}"#,
        fields.join(",")
    )
}

/// Benchmark deserializing different content types
fn bench_deserialize_content_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("deserialize_content");

    // Text content - varying sizes
    for text_size in [100, 1000, 10000] {
        let json = text_json(text_size);
        group.throughput(Throughput::Bytes(json.len() as u64));
        group.bench_with_input(BenchmarkId::new("text", text_size), &json, |b, json| {
            b.iter(|| {
                let content: InteractionContent = serde_json::from_str(json).unwrap();
                criterion::black_box(content)
            });
        });
    }

    // FunctionCall - varying arg counts
    for arg_count in [1, 5, 20] {
        let json = function_call_json(arg_count);
        group.throughput(Throughput::Bytes(json.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("function_call_args", arg_count),
            &json,
            |b, json| {
                b.iter(|| {
                    let content: InteractionContent = serde_json::from_str(json).unwrap();
                    criterion::black_box(content)
                });
            },
        );
    }

    // Image with base64 data
    for data_size in [1000, 10000, 100000] {
        let json = image_json(data_size);
        group.throughput(Throughput::Bytes(json.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("image_data", data_size),
            &json,
            |b, json| {
                b.iter(|| {
                    let content: InteractionContent = serde_json::from_str(json).unwrap();
                    criterion::black_box(content)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark serializing content back to JSON
fn bench_serialize_content(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize_content");

    // Pre-deserialize content for serialization benchmarks
    let text_content: InteractionContent = serde_json::from_str(&text_json(1000)).unwrap();
    let function_call_content: InteractionContent =
        serde_json::from_str(&function_call_json(10)).unwrap();
    let image_content: InteractionContent = serde_json::from_str(&image_json(10000)).unwrap();
    let unknown_content: InteractionContent = serde_json::from_str(&unknown_json(5)).unwrap();

    group.bench_function("text_1000", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&text_content).unwrap();
            criterion::black_box(json)
        });
    });

    group.bench_function("function_call_10_args", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&function_call_content).unwrap();
            criterion::black_box(json)
        });
    });

    group.bench_function("image_10000", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&image_content).unwrap();
            criterion::black_box(json)
        });
    });

    group.bench_function("unknown_5_fields", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&unknown_content).unwrap();
            criterion::black_box(json)
        });
    });

    group.finish();
}

/// Benchmark Unknown variant handling (Evergreen pattern)
fn bench_unknown_variant(c: &mut Criterion) {
    let mut group = c.benchmark_group("unknown_variant");

    for field_count in [1, 5, 20, 50] {
        let json = unknown_json(field_count);
        group.throughput(Throughput::Bytes(json.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("deserialize_fields", field_count),
            &json,
            |b, json| {
                b.iter(|| {
                    let content: InteractionContent = serde_json::from_str(json).unwrap();
                    criterion::black_box(content)
                });
            },
        );
    }

    // Roundtrip: deserialize then serialize
    for field_count in [1, 5, 20] {
        let json = unknown_json(field_count);
        group.bench_with_input(
            BenchmarkId::new("roundtrip_fields", field_count),
            &json,
            |b, json| {
                b.iter(|| {
                    let content: InteractionContent = serde_json::from_str(json).unwrap();
                    let back = serde_json::to_string(&content).unwrap();
                    criterion::black_box(back)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark parsing mixed content arrays (simulating real API responses)
fn bench_mixed_content_array(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_content_array");

    // Simulate a typical response with multiple content types
    let mixed_array = format!(
        r#"[{},{},{},{},{}]"#,
        text_json(500),
        function_call_json(5),
        text_json(200),
        function_result_json(300),
        text_json(100)
    );

    group.throughput(Throughput::Bytes(mixed_array.len() as u64));
    group.bench_function("typical_response_5_items", |b| {
        b.iter(|| {
            let contents: Vec<InteractionContent> = serde_json::from_str(&mixed_array).unwrap();
            criterion::black_box(contents)
        });
    });

    // Larger response with many items
    let large_array = {
        let items: Vec<String> = (0..20)
            .map(|i| {
                if i % 3 == 0 {
                    text_json(200)
                } else if i % 3 == 1 {
                    function_call_json(3)
                } else {
                    function_result_json(100)
                }
            })
            .collect();
        format!("[{}]", items.join(","))
    };

    group.throughput(Throughput::Bytes(large_array.len() as u64));
    group.bench_function("large_response_20_items", |b| {
        b.iter(|| {
            let contents: Vec<InteractionContent> = serde_json::from_str(&large_array).unwrap();
            criterion::black_box(contents)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_deserialize_content_types,
    bench_serialize_content,
    bench_unknown_variant,
    bench_mixed_content_array
);
criterion_main!(benches);
