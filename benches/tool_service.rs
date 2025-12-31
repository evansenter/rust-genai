//! Benchmarks for ToolService patterns and auto-function calling infrastructure.
//!
//! These benchmarks measure the performance of:
//! - ToolService function map building
//! - Arc cloning overhead for callable functions
//! - Service vs global registry lookup patterns
//! - AutoFunctionResultAccumulator operations

use async_trait::async_trait;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use genai_client::{
    FunctionParameters, InteractionContent, InteractionResponse, InteractionStatus,
};
use rust_genai::{CallableFunction, FunctionDeclaration, FunctionError, ToolService};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

// =============================================================================
// Test Tool Implementations
// =============================================================================

/// A simple callable function for benchmarking
struct BenchmarkTool {
    name: String,
    #[allow(dead_code)]
    config: String,
}

#[async_trait]
impl CallableFunction for BenchmarkTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration::new(
            self.name.clone(),
            format!("Benchmark tool: {}", self.name),
            FunctionParameters::new(
                "object".to_string(),
                json!({
                    "input": {"type": "string", "description": "Input parameter"}
                }),
                vec!["input".to_string()],
            ),
        )
    }

    async fn call(&self, args: Value) -> Result<Value, FunctionError> {
        Ok(json!({ "result": format!("Processed: {:?}", args) }))
    }
}

/// A tool service that provides multiple tools
struct BenchmarkToolService {
    tool_count: usize,
    config: Arc<String>,
}

impl BenchmarkToolService {
    fn new(tool_count: usize) -> Self {
        Self {
            tool_count,
            config: Arc::new("shared_config".to_string()),
        }
    }
}

impl ToolService for BenchmarkToolService {
    fn tools(&self) -> Vec<Arc<dyn CallableFunction>> {
        (0..self.tool_count)
            .map(|i| {
                Arc::new(BenchmarkTool {
                    name: format!("tool_{}", i),
                    config: (*self.config).clone(),
                }) as Arc<dyn CallableFunction>
            })
            .collect()
    }
}

// =============================================================================
// Helper Functions (simulating internal patterns)
// =============================================================================

/// Simulates build_service_function_map from auto_functions.rs
fn build_service_function_map(
    tool_service: &Option<Arc<dyn ToolService>>,
) -> HashMap<String, Arc<dyn CallableFunction>> {
    tool_service
        .as_ref()
        .map(|svc| {
            svc.tools()
                .into_iter()
                .map(|f| (f.declaration().name().to_string(), f))
                .collect()
        })
        .unwrap_or_default()
}

// =============================================================================
// Benchmarks
// =============================================================================

/// Benchmark ToolService::tools() call and Arc creation
fn bench_tool_service_tools(c: &mut Criterion) {
    let mut group = c.benchmark_group("tool_service_tools");

    for tool_count in [1, 5, 10, 20, 50] {
        let service = Arc::new(BenchmarkToolService::new(tool_count));

        group.bench_with_input(
            BenchmarkId::new("tools", tool_count),
            &service,
            |b, service| {
                b.iter(|| {
                    let tools = service.tools();
                    criterion::black_box(tools)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark building service function map (HashMap from tools)
fn bench_build_service_function_map(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_service_function_map");

    for tool_count in [1, 5, 10, 20, 50] {
        let service: Option<Arc<dyn ToolService>> =
            Some(Arc::new(BenchmarkToolService::new(tool_count)));

        group.bench_with_input(
            BenchmarkId::new("tools", tool_count),
            &service,
            |b, service| {
                b.iter(|| {
                    let map = build_service_function_map(criterion::black_box(service));
                    criterion::black_box(map)
                });
            },
        );
    }

    // Also benchmark with None (empty case)
    let none_service: Option<Arc<dyn ToolService>> = None;
    group.bench_function("none", |b| {
        b.iter(|| {
            let map = build_service_function_map(criterion::black_box(&none_service));
            criterion::black_box(map)
        });
    });

    group.finish();
}

/// Benchmark service function map lookup vs global registry pattern
fn bench_service_map_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("service_map_lookup");

    for tool_count in [10, 50, 100] {
        let service = Arc::new(BenchmarkToolService::new(tool_count));
        let service_opt: Option<Arc<dyn ToolService>> = Some(service);
        let service_map = build_service_function_map(&service_opt);

        // Lookup existing function
        let target = format!("tool_{}", tool_count / 2);
        group.bench_with_input(
            BenchmarkId::new("existing", tool_count),
            &(&service_map, &target),
            |b, (map, target)| {
                b.iter(|| {
                    let result = map.get(criterion::black_box(*target));
                    criterion::black_box(result)
                });
            },
        );

        // Lookup missing function
        group.bench_with_input(
            BenchmarkId::new("missing", tool_count),
            &service_map,
            |b, map| {
                b.iter(|| {
                    let result = map.get(criterion::black_box("nonexistent_tool"));
                    criterion::black_box(result)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark Arc cloning overhead (common pattern in function execution)
fn bench_arc_cloning(c: &mut Criterion) {
    let mut group = c.benchmark_group("arc_cloning");

    let service = Arc::new(BenchmarkToolService::new(10));
    let tools = service.tools();
    let tool_arc = &tools[0];

    // Clone Arc (not the inner value)
    group.bench_function("arc_clone", |b| {
        b.iter(|| {
            let cloned = Arc::clone(criterion::black_box(tool_arc));
            criterion::black_box(cloned)
        });
    });

    // Clone the service Arc
    let service_arc: Arc<dyn ToolService> = Arc::new(BenchmarkToolService::new(10));
    group.bench_function("service_arc_clone", |b| {
        b.iter(|| {
            let cloned = Arc::clone(criterion::black_box(&service_arc));
            criterion::black_box(cloned)
        });
    });

    group.finish();
}

// =============================================================================
// AutoFunctionResult Accumulator Benchmarks
// =============================================================================

use rust_genai::{AutoFunctionResultAccumulator, AutoFunctionStreamChunk, FunctionExecutionResult};

/// Create a sample FunctionExecutionResult
fn create_execution_result(name: &str, index: usize) -> FunctionExecutionResult {
    FunctionExecutionResult::new(
        name,
        format!("call_{}", index),
        json!({ "result": format!("Result from {} #{}", name, index) }),
        Duration::from_millis(50),
    )
}

/// Create a sample InteractionResponse for completion
fn create_completion_response() -> InteractionResponse {
    InteractionResponse {
        id: Some("interaction-test".to_string()),
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: vec![],
        outputs: vec![InteractionContent::Text {
            text: Some("Final response".to_string()),
        }],
        status: InteractionStatus::Completed,
        usage: None,
        tools: None,
        grounding_metadata: None,
        url_context_metadata: None,
        previous_interaction_id: None,
    }
}

/// Benchmark accumulator push operations
fn bench_accumulator_push(c: &mut Criterion) {
    let mut group = c.benchmark_group("accumulator_push");

    // Benchmark pushing Delta chunks (should be no-op)
    group.bench_function("delta_chunk", |b| {
        b.iter_batched(
            AutoFunctionResultAccumulator::new,
            |mut acc| {
                let chunk = AutoFunctionStreamChunk::Delta(InteractionContent::Text {
                    text: Some("Hello".to_string()),
                });
                let result = acc.push(criterion::black_box(chunk));
                criterion::black_box(result)
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Benchmark pushing FunctionResults chunks
    for result_count in [1, 5, 10] {
        group.bench_with_input(
            BenchmarkId::new("function_results", result_count),
            &result_count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let results: Vec<FunctionExecutionResult> = (0..count)
                            .map(|i| create_execution_result("test_func", i))
                            .collect();
                        (AutoFunctionResultAccumulator::new(), results)
                    },
                    |(mut acc, results)| {
                        let chunk = AutoFunctionStreamChunk::FunctionResults(results);
                        let result = acc.push(criterion::black_box(chunk));
                        criterion::black_box(result)
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    // Benchmark pushing Complete chunk (finalizes accumulator)
    group.bench_function("complete_chunk", |b| {
        b.iter_batched(
            || {
                let mut acc = AutoFunctionResultAccumulator::new();
                // Pre-populate with some results
                let results: Vec<FunctionExecutionResult> = (0..5)
                    .map(|i| create_execution_result("test_func", i))
                    .collect();
                acc.push(AutoFunctionStreamChunk::FunctionResults(results));
                acc
            },
            |mut acc| {
                let chunk = AutoFunctionStreamChunk::Complete(create_completion_response());
                let result = acc.push(criterion::black_box(chunk));
                criterion::black_box(result)
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark accumulator with realistic multi-round scenarios
fn bench_accumulator_multi_round(c: &mut Criterion) {
    let mut group = c.benchmark_group("accumulator_multi_round");

    for rounds in [1, 3, 5] {
        let functions_per_round = 2;

        group.bench_with_input(
            BenchmarkId::new("rounds", rounds),
            &rounds,
            |b, &round_count| {
                b.iter_batched(
                    AutoFunctionResultAccumulator::new,
                    |mut acc| {
                        // Simulate multiple rounds of function execution
                        for round in 0..round_count {
                            let results: Vec<FunctionExecutionResult> = (0..functions_per_round)
                                .map(|i| {
                                    create_execution_result(
                                        &format!("func_{}", i),
                                        round * functions_per_round + i,
                                    )
                                })
                                .collect();
                            acc.push(AutoFunctionStreamChunk::FunctionResults(results));
                        }
                        // Final completion
                        let result = acc.push(AutoFunctionStreamChunk::Complete(
                            create_completion_response(),
                        ));
                        criterion::black_box(result)
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark FunctionExecutionResult creation
fn bench_execution_result_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("execution_result_creation");

    group.bench_function("simple", |b| {
        b.iter(|| {
            let result = FunctionExecutionResult::new(
                criterion::black_box("get_weather"),
                criterion::black_box("call_123"),
                criterion::black_box(json!({"temp": 20, "unit": "celsius"})),
                criterion::black_box(Duration::from_millis(42)),
            );
            criterion::black_box(result)
        });
    });

    // With larger result payload
    let large_result = json!({
        "data": (0..100).map(|i| format!("item_{}", i)).collect::<Vec<_>>(),
        "metadata": {
            "count": 100,
            "processed": true,
            "timestamp": "2024-01-01T00:00:00Z"
        }
    });

    group.bench_function("large_payload", |b| {
        b.iter(|| {
            let result = FunctionExecutionResult::new(
                criterion::black_box("process_data"),
                criterion::black_box("call_456"),
                criterion::black_box(large_result.clone()),
                criterion::black_box(Duration::from_millis(150)),
            );
            criterion::black_box(result)
        });
    });

    group.finish();
}

/// Benchmark AutoFunctionStreamChunk serialization (for logging/persistence)
fn bench_stream_chunk_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("stream_chunk_serialization");

    // Delta chunk
    let delta_chunk = AutoFunctionStreamChunk::Delta(InteractionContent::Text {
        text: Some("Hello, world!".to_string()),
    });
    group.bench_function("delta", |b| {
        b.iter(|| {
            let json = serde_json::to_string(criterion::black_box(&delta_chunk)).unwrap();
            criterion::black_box(json)
        });
    });

    // FunctionResults chunk
    let results: Vec<FunctionExecutionResult> = (0..3)
        .map(|i| create_execution_result("test_func", i))
        .collect();
    let results_chunk = AutoFunctionStreamChunk::FunctionResults(results);
    group.bench_function("function_results", |b| {
        b.iter(|| {
            let json = serde_json::to_string(criterion::black_box(&results_chunk)).unwrap();
            criterion::black_box(json)
        });
    });

    // Complete chunk
    let complete_chunk = AutoFunctionStreamChunk::Complete(create_completion_response());
    group.bench_function("complete", |b| {
        b.iter(|| {
            let json = serde_json::to_string(criterion::black_box(&complete_chunk)).unwrap();
            criterion::black_box(json)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_tool_service_tools,
    bench_build_service_function_map,
    bench_service_map_lookup,
    bench_arc_cloning,
    bench_accumulator_push,
    bench_accumulator_multi_round,
    bench_execution_result_creation,
    bench_stream_chunk_serialization
);
criterion_main!(benches);
