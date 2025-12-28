//! Benchmarks for function declaration building and registration patterns.
//!
//! These benchmarks measure the performance of:
//! - Building FunctionDeclaration objects
//! - HashMap lookup patterns (simulating registry access)
//! - Vec collection of declarations

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use genai_client::{FunctionDeclaration, FunctionParameters};
use serde_json::json;
use std::collections::HashMap;

/// Create a function declaration with specified number of parameters
fn create_declaration(name: &str, param_count: usize) -> FunctionDeclaration {
    let properties: serde_json::Map<String, serde_json::Value> = (0..param_count)
        .map(|i| {
            (
                format!("param{}", i),
                json!({
                    "type": "string",
                    "description": format!("Parameter number {}", i)
                }),
            )
        })
        .collect();

    let required: Vec<String> = (0..param_count).map(|i| format!("param{}", i)).collect();

    FunctionDeclaration::new(
        name.to_string(),
        format!("A test function with {} parameters", param_count),
        FunctionParameters::new("object".to_string(), json!(properties), required),
    )
}

/// Benchmark building FunctionDeclaration objects
fn bench_declaration_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("declaration_building");

    for param_count in [1, 5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::new("params", param_count),
            &param_count,
            |b, &param_count| {
                b.iter(|| {
                    let decl = create_declaration("test_function", param_count);
                    criterion::black_box(decl)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark using the builder pattern
fn bench_declaration_builder(c: &mut Criterion) {
    let mut group = c.benchmark_group("declaration_builder");

    group.bench_function("simple_3_params", |b| {
        b.iter(|| {
            let decl = FunctionDeclaration::builder("get_weather")
                .description("Get current weather for a location")
                .parameter("location", json!({"type": "string", "description": "The city name"}))
                .parameter("unit", json!({"type": "string", "description": "Temperature unit (celsius/fahrenheit)"}))
                .parameter("format", json!({"type": "string", "description": "Response format"}))
                .required(vec!["location".to_string()])
                .build();
            criterion::black_box(decl)
        });
    });

    group.bench_function("complex_10_params", |b| {
        b.iter(|| {
            let mut builder = FunctionDeclaration::builder("complex_function")
                .description("A function with many parameters");

            for i in 0..10 {
                builder = builder.parameter(
                    &format!("param{}", i),
                    json!({"type": "string", "description": format!("Description for param {}", i)}),
                );
            }

            // First 5 are required
            let required: Vec<String> = (0..5).map(|i| format!("param{}", i)).collect();
            let decl = builder.required(required).build();
            criterion::black_box(decl)
        });
    });

    group.finish();
}

/// Benchmark HashMap lookup patterns (simulating registry access)
fn bench_registry_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("registry_lookup");

    for function_count in [10, 50, 100, 500] {
        // Create a registry with N functions
        let registry: HashMap<String, FunctionDeclaration> = (0..function_count)
            .map(|i| {
                let name = format!("function_{}", i);
                let decl = create_declaration(&name, 5);
                (name, decl)
            })
            .collect();

        // Benchmark lookup of existing function
        group.bench_with_input(
            BenchmarkId::new("existing", function_count),
            &registry,
            |b, registry| {
                let target = format!("function_{}", function_count / 2);
                b.iter(|| {
                    let result = registry.get(criterion::black_box(&target));
                    criterion::black_box(result)
                });
            },
        );

        // Benchmark lookup of non-existing function
        group.bench_with_input(
            BenchmarkId::new("missing", function_count),
            &registry,
            |b, registry| {
                b.iter(|| {
                    let result = registry.get(criterion::black_box("nonexistent_function"));
                    criterion::black_box(result)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark collecting all declarations (simulating all_declarations())
fn bench_collect_declarations(c: &mut Criterion) {
    let mut group = c.benchmark_group("collect_declarations");

    for function_count in [10, 50, 100, 500] {
        // Create a registry with N functions
        let registry: HashMap<String, FunctionDeclaration> = (0..function_count)
            .map(|i| {
                let name = format!("function_{}", i);
                let decl = create_declaration(&name, 5);
                (name, decl)
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("functions", function_count),
            &registry,
            |b, registry| {
                b.iter(|| {
                    // This simulates all_declarations() which clones each declaration
                    let declarations: Vec<FunctionDeclaration> =
                        registry.values().cloned().collect();
                    criterion::black_box(declarations)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark serializing function declarations to JSON (for API requests)
fn bench_declaration_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("declaration_serialization");

    for param_count in [1, 5, 10, 20] {
        let decl = create_declaration("test_function", param_count);

        group.bench_with_input(BenchmarkId::new("params", param_count), &decl, |b, decl| {
            b.iter(|| {
                let json = serde_json::to_string(criterion::black_box(decl)).unwrap();
                criterion::black_box(json)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_declaration_building,
    bench_declaration_builder,
    bench_registry_lookup,
    bench_collect_declarations,
    bench_declaration_serialization
);
criterion_main!(benches);
