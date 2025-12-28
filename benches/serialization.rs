//! Serialization benchmarks for the rust-genai crate.
//!
//! This benchmarks serialization/deserialization from the user-facing API perspective,
//! including request building and response parsing.
//!
//! For more granular content serialization benchmarks, see
//! genai-client/benches/content_serialization.rs

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use genai_client::{
    CreateInteractionRequest, FunctionParameters, GenerationConfig, InteractionContent,
    InteractionInput, InteractionResponse, Tool,
};
use serde_json::json;

/// Create a request with specified content
fn create_request(text_items: usize, text_size: usize) -> CreateInteractionRequest {
    let contents: Vec<InteractionContent> = (0..text_items)
        .map(|i| InteractionContent::Text {
            text: Some(format!("Message {} {}", i, "x".repeat(text_size))),
        })
        .collect();

    CreateInteractionRequest {
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: InteractionInput::Content(contents),
        previous_interaction_id: None,
        tools: None,
        response_modalities: None,
        response_format: None,
        generation_config: Some(GenerationConfig {
            temperature: Some(0.7),
            top_p: Some(0.9),
            top_k: Some(40),
            max_output_tokens: Some(1024),
            ..Default::default()
        }),
        stream: None,
        background: None,
        store: None,
        system_instruction: Some(InteractionInput::Text(
            "You are a helpful assistant.".to_string(),
        )),
    }
}

/// Create a request with function declarations
fn create_request_with_functions(function_count: usize) -> CreateInteractionRequest {
    let tools: Vec<Tool> = (0..function_count)
        .map(|i| Tool::Function {
            name: format!("function_{}", i),
            description: format!("Test function number {}", i),
            parameters: FunctionParameters::new(
                "object".to_string(),
                json!({
                    "input": {"type": "string", "description": "The input parameter"},
                    "option": {"type": "string", "description": "An optional parameter"}
                }),
                vec!["input".to_string()],
            ),
        })
        .collect();

    CreateInteractionRequest {
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: InteractionInput::Text("Call one of the functions".to_string()),
        previous_interaction_id: None,
        tools: Some(tools),
        response_modalities: None,
        response_format: None,
        generation_config: None,
        stream: None,
        background: None,
        store: None,
        system_instruction: None,
    }
}

/// Create a response JSON string
fn create_response_json(output_count: usize, text_size: usize) -> String {
    let outputs: Vec<String> = (0..output_count)
        .map(|i| {
            format!(
                r#"{{"type":"text","text":"Output {} {}"}}"#,
                i,
                "x".repeat(text_size)
            )
        })
        .collect();

    format!(
        r#"{{"id":"interactions/test","status":"completed","outputs":[{}],"usage":{{"totalInputTokens":100,"totalOutputTokens":200,"totalTokens":300}}}}"#,
        outputs.join(",")
    )
}

/// Benchmark request serialization
fn bench_request_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_serialization");

    for text_items in [1, 5, 10] {
        let request = create_request(text_items, 100);
        let json_len = serde_json::to_string(&request).unwrap().len();

        group.throughput(Throughput::Bytes(json_len as u64));
        group.bench_with_input(
            BenchmarkId::new("text_items", text_items),
            &request,
            |b, request| {
                b.iter(|| {
                    let json = serde_json::to_string(criterion::black_box(request)).unwrap();
                    criterion::black_box(json)
                });
            },
        );
    }

    // Benchmark with functions
    for function_count in [1, 5, 10] {
        let request = create_request_with_functions(function_count);
        let json_len = serde_json::to_string(&request).unwrap().len();

        group.throughput(Throughput::Bytes(json_len as u64));
        group.bench_with_input(
            BenchmarkId::new("functions", function_count),
            &request,
            |b, request| {
                b.iter(|| {
                    let json = serde_json::to_string(criterion::black_box(request)).unwrap();
                    criterion::black_box(json)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark response deserialization
fn bench_response_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("response_deserialization");

    for output_count in [1, 5, 10, 20] {
        let json = create_response_json(output_count, 100);

        group.throughput(Throughput::Bytes(json.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("outputs", output_count),
            &json,
            |b, json| {
                b.iter(|| {
                    let response: InteractionResponse =
                        serde_json::from_str(criterion::black_box(json)).unwrap();
                    criterion::black_box(response)
                });
            },
        );
    }

    // Benchmark with larger text
    for text_size in [100, 1000, 5000] {
        let json = create_response_json(5, text_size);

        group.throughput(Throughput::Bytes(json.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("text_size", text_size),
            &json,
            |b, json| {
                b.iter(|| {
                    let response: InteractionResponse =
                        serde_json::from_str(criterion::black_box(json)).unwrap();
                    criterion::black_box(response)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark roundtrip (serialize + deserialize)
fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");

    // Create a response JSON and parse it (we can't directly construct InteractionResponse
    // easily since it's meant to come from the API)
    let response_json = r#"{
        "id": "interactions/test-123",
        "status": "completed",
        "outputs": [
            {"type": "text", "text": "Hello, world!"},
            {"type": "functionCall", "id": "call_123", "name": "get_weather", "args": {"location": "San Francisco"}}
        ],
        "usage": {"totalInputTokens": 100, "totalOutputTokens": 200, "totalTokens": 300}
    }"#;

    let response: InteractionResponse = serde_json::from_str(response_json).unwrap();

    group.bench_function("typical_response", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&response).unwrap();
            let parsed: InteractionResponse = serde_json::from_str(&json).unwrap();
            criterion::black_box(parsed)
        });
    });

    group.finish();
}

/// Benchmark parsing responses with unknown content types (Evergreen pattern)
fn bench_unknown_content_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("unknown_content");

    // Response with a mix of known and unknown content types
    let json_with_unknown = r#"{
        "id": "interactions/test",
        "status": "completed",
        "outputs": [
            {"type": "text", "text": "Hello"},
            {"type": "futureFeature", "specialField": "value", "data": {"nested": true}},
            {"type": "text", "text": "World"},
            {"type": "anotherNewType", "field1": 123, "field2": "abc"}
        ],
        "usage": {"totalTokens": 100}
    }"#;

    group.bench_function("mixed_known_unknown", |b| {
        b.iter(|| {
            let response: InteractionResponse =
                serde_json::from_str(criterion::black_box(json_with_unknown)).unwrap();
            criterion::black_box(response)
        });
    });

    // Response with only unknown types
    let json_all_unknown = r#"{
        "id": "interactions/test",
        "status": "completed",
        "outputs": [
            {"type": "newType1", "data": "value1"},
            {"type": "newType2", "data": "value2"},
            {"type": "newType3", "data": "value3"},
            {"type": "newType4", "data": "value4"},
            {"type": "newType5", "data": "value5"}
        ],
        "usage": {"totalTokens": 100}
    }"#;

    group.bench_function("all_unknown", |b| {
        b.iter(|| {
            let response: InteractionResponse =
                serde_json::from_str(criterion::black_box(json_all_unknown)).unwrap();
            criterion::black_box(response)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_request_serialization,
    bench_response_deserialization,
    bench_roundtrip,
    bench_unknown_content_handling
);
criterion_main!(benches);
