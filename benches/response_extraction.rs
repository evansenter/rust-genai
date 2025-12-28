//! Benchmarks for response accessor methods.
//!
//! These benchmarks measure the performance of extracting data from
//! InteractionResponse objects, simulating typical usage patterns.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use genai_client::InteractionResponse;

/// Create a response JSON with specified number of text content items
fn create_text_response_json(text_count: usize, text_size: usize) -> String {
    let outputs: Vec<String> = (0..text_count)
        .map(|i| {
            format!(
                r#"{{"type":"text","text":"Text content {} {}"}}"#,
                i,
                "x".repeat(text_size)
            )
        })
        .collect();

    format!(
        r#"{{"id":"test-response","status":"completed","outputs":[{}],"usage":{{"totalTokens":100}}}}"#,
        outputs.join(",")
    )
}

/// Create a response JSON with mixed content types
fn create_mixed_response_json(item_count: usize) -> String {
    let outputs: Vec<String> = (0..item_count)
        .map(|i| match i % 5 {
            0 => format!(r#"{{"type":"text","text":"Text content {}"}}"#, i),
            1 => format!(
                r#"{{"type":"functionCall","id":"call_{}","name":"function_{}","args":{{"param":"value"}}}}"#,
                i, i
            ),
            2 => format!(r#"{{"type":"thought","text":"Thinking about {}"}}"#, i),
            3 => format!(
                r#"{{"type":"functionResult","name":"function_{}","callId":"call_{}","result":{{"result":"success"}}}}"#,
                i - 2, i - 2
            ),
            _ => format!(r#"{{"type":"text","text":"More text {}"}}"#, i),
        })
        .collect();

    format!(
        r#"{{"id":"test-response","status":"completed","outputs":[{}],"usage":{{"totalTokens":100}}}}"#,
        outputs.join(",")
    )
}

/// Create a response JSON with function calls
fn create_function_call_response_json(call_count: usize, args_per_call: usize) -> String {
    let outputs: Vec<String> = (0..call_count)
        .map(|i| {
            let args: Vec<String> = (0..args_per_call)
                .map(|j| format!(r#""arg{}":"value{}""#, j, j))
                .collect();

            format!(
                r#"{{"type":"functionCall","id":"call_{}","name":"function_{}","args":{{{}}}}}"#,
                i,
                i,
                args.join(",")
            )
        })
        .collect();

    format!(
        r#"{{"id":"test-response","status":"requires_action","outputs":[{}],"usage":{{"totalTokens":100}}}}"#,
        outputs.join(",")
    )
}

/// Parse JSON into InteractionResponse
fn parse_response(json: &str) -> InteractionResponse {
    serde_json::from_str(json).expect("Failed to parse response JSON")
}

/// Benchmark text() accessor - finds first text content
fn bench_text_accessor(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_accessor");

    for item_count in [1, 10, 50, 100] {
        let json = create_text_response_json(item_count, 100);
        let response = parse_response(&json);

        group.bench_with_input(
            BenchmarkId::new("items", item_count),
            &response,
            |b, response| {
                b.iter(|| {
                    let text = response.text();
                    criterion::black_box(text)
                });
            },
        );
    }

    // Test when text is at the end (worst case for find_map)
    let mut json = create_function_call_response_json(50, 3);
    // Add text at the end by modifying the JSON
    json = json.replace(
        r#"],"usage""#,
        r#",{"type":"text","text":"Final text"}],"usage""#,
    );
    let response = parse_response(&json);

    group.bench_function("text_at_end_50_items", |b| {
        b.iter(|| {
            let text = response.text();
            criterion::black_box(text)
        });
    });

    group.finish();
}

/// Benchmark all_text() accessor - collects and joins all text
fn bench_all_text_accessor(c: &mut Criterion) {
    let mut group = c.benchmark_group("all_text_accessor");

    for item_count in [1, 10, 50, 100] {
        let json = create_text_response_json(item_count, 100);
        let response = parse_response(&json);

        group.bench_with_input(
            BenchmarkId::new("items", item_count),
            &response,
            |b, response| {
                b.iter(|| {
                    let text = response.all_text();
                    criterion::black_box(text)
                });
            },
        );
    }

    // Test with larger text sizes
    for text_size in [100, 1000, 10000] {
        let json = create_text_response_json(10, text_size);
        let response = parse_response(&json);

        group.bench_with_input(
            BenchmarkId::new("text_size", text_size),
            &response,
            |b, response| {
                b.iter(|| {
                    let text = response.all_text();
                    criterion::black_box(text)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark function_calls() accessor
fn bench_function_calls_accessor(c: &mut Criterion) {
    let mut group = c.benchmark_group("function_calls_accessor");

    for call_count in [1, 5, 10, 20] {
        let json = create_function_call_response_json(call_count, 5);
        let response = parse_response(&json);

        group.bench_with_input(
            BenchmarkId::new("calls", call_count),
            &response,
            |b, response| {
                b.iter(|| {
                    let calls = response.function_calls();
                    criterion::black_box(calls)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark thoughts() accessor
fn bench_thoughts_accessor(c: &mut Criterion) {
    let mut group = c.benchmark_group("thoughts_accessor");

    for item_count in [10, 50, 100] {
        let json = create_mixed_response_json(item_count);
        let response = parse_response(&json);

        group.bench_with_input(
            BenchmarkId::new("items", item_count),
            &response,
            |b, response| {
                b.iter(|| {
                    // Collect the iterator to force evaluation
                    let thoughts: Vec<&str> = response.thoughts().collect();
                    criterion::black_box(thoughts)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark multiple accessor calls on same response (common pattern)
fn bench_multiple_accessors(c: &mut Criterion) {
    let mut group = c.benchmark_group("multiple_accessors");

    let json = create_mixed_response_json(50);
    let response = parse_response(&json);

    group.bench_function("text_and_function_calls", |b| {
        b.iter(|| {
            let text = response.text();
            let calls = response.function_calls();
            criterion::black_box((text, calls))
        });
    });

    group.bench_function("all_common_accessors", |b| {
        b.iter(|| {
            let text = response.text();
            let all_text = response.all_text();
            let calls = response.function_calls();
            let thoughts: Vec<&str> = response.thoughts().collect();
            let status = &response.status;
            criterion::black_box((text, all_text, calls, thoughts, status))
        });
    });

    group.finish();
}

/// Benchmark has_function_calls() check (common branching pattern)
fn bench_has_function_calls(c: &mut Criterion) {
    let mut group = c.benchmark_group("has_function_calls");

    let json_with_calls = create_function_call_response_json(5, 3);
    let with_calls = parse_response(&json_with_calls);

    let json_without_calls = create_text_response_json(10, 100);
    let without_calls = parse_response(&json_without_calls);

    group.bench_function("response_with_calls", |b| {
        b.iter(|| {
            let has = with_calls.has_function_calls();
            criterion::black_box(has)
        });
    });

    group.bench_function("response_without_calls", |b| {
        b.iter(|| {
            let has = without_calls.has_function_calls();
            criterion::black_box(has)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_text_accessor,
    bench_all_text_accessor,
    bench_function_calls_accessor,
    bench_thoughts_accessor,
    bench_multiple_accessors,
    bench_has_function_calls
);
criterion_main!(benches);
