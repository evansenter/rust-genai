//! Thinking mode + function calling tests
//!
//! Tests for combining thinking mode (ThinkingLevel) with function calling,
//! including multi-turn conversations, parallel function calls, sequential
//! chains, and streaming scenarios.
//!
//! These tests require the GEMINI_API_KEY environment variable to be set.
//!
//! # Running Tests
//!
//! ```bash
//! cargo test --test thinking_function_tests -- --include-ignored --nocapture
//! ```
//!
//! # Thought Signatures
//!
//! Per Google's documentation (https://ai.google.dev/gemini-api/docs/thought-signatures):
//! - Thought signatures are encrypted representations of the model's reasoning
//! - For Gemini 3 models, signatures MUST be echoed back during function calling
//! - The Interactions API handles this automatically via `previous_interaction_id`
//! - Signatures may or may not be exposed in the response (API behavior varies)

mod common;

use common::{
    DEFAULT_MAX_RETRIES, consume_stream, get_client, retry_on_transient, stateful_builder,
};
use rust_genai::{FunctionDeclaration, InteractionStatus, ThinkingLevel, function_result_content};
use serde_json::json;

// =============================================================================
// Thinking + Function Calling + Multi-turn
// =============================================================================

/// Test thinking mode combined with function calling across multiple turns.
///
/// This validates that:
/// - Thinking mode (`ThinkingLevel`) works with client-side function calling
/// - Multi-turn conversations function correctly with thinking enabled
/// - Context is preserved across turns via `previous_interaction_id`
///
/// # Thinking Mode vs Thought Signatures
///
/// These are distinct concepts:
/// - `ThinkingLevel`: Exposes model's chain-of-thought as `Thought` content
/// - `thought_signature`: Cryptographic field on function calls for verification
///
/// Thoughts may be processed internally without visible text, especially when
/// the model is focused on function calling rather than explanation.
///
/// Turn 1: Enable thinking + ask question -> triggers function call
/// Turn 2: Provide function result -> model processes and responds
/// Turn 3: Follow-up question -> model reasons with full context preserved
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_thinking_with_function_calling_multi_turn() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let get_weather = FunctionDeclaration::builder("get_weather")
        .description("Get the current weather for a city including temperature and conditions")
        .parameter(
            "city",
            json!({"type": "string", "description": "The city name"}),
        )
        .required(vec!["city".to_string()])
        .build();

    // =========================================================================
    // Turn 1: Enable thinking + trigger function call
    // =========================================================================
    let response1 = {
        let client = client.clone();
        let get_weather = get_weather.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let get_weather = get_weather.clone();
            async move {
                stateful_builder(&client)
                    .with_text("What's the weather in Tokyo? Should I bring an umbrella?")
                    .with_function(get_weather)
                    .with_thinking_level(ThinkingLevel::Medium)
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Turn 1 failed")
    };

    println!("Turn 1 status: {:?}", response1.status);

    let function_calls = response1.function_calls();
    if function_calls.is_empty() {
        println!("Model chose not to call function - skipping rest of test");
        return;
    }

    let call = &function_calls[0];
    println!(
        "Turn 1 function call: {} (has thought_signature: {})",
        call.name,
        call.thought_signature.is_some()
    );

    // Note: thought_signature is not guaranteed by the API - it depends on model behavior.
    // We log its presence but don't hard-assert, as the existing tests show it can be None.
    if call.thought_signature.is_some() {
        println!("✓ Thought signature present on function call");
    } else {
        println!("ℹ Thought signature not present (API behavior varies)");
    }
    assert!(call.id.is_some(), "Function call must have an id");

    // =========================================================================
    // Verify storage: Explicitly confirm with_store(true) worked
    // =========================================================================
    let retrieved = client
        .get_interaction(response1.id.as_ref().expect("id should exist"))
        .await
        .expect("Should be able to retrieve stored interaction");
    assert_eq!(
        retrieved.id, response1.id,
        "Retrieved interaction ID should match"
    );
    println!(
        "✓ Storage verified: interaction {:?} is retrievable",
        response1.id
    );

    // =========================================================================
    // Turn 2: Provide function result - model should reason about it
    // =========================================================================
    let function_result = function_result_content(
        "get_weather",
        call.id.expect("call_id should exist").to_string(),
        json!({
            "temperature": "18°C",
            "conditions": "rainy",
            "precipitation": "80%",
            "humidity": "85%"
        }),
    );

    let response2 = {
        let client = client.clone();
        let prev_id = response1.id.clone().expect("id should exist");
        let get_weather = get_weather.clone();
        let function_result = function_result.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let prev_id = prev_id.clone();
            let get_weather = get_weather.clone();
            let function_result = function_result.clone();
            async move {
                stateful_builder(&client)
                    .with_previous_interaction(&prev_id)
                    .with_content(vec![function_result])
                    .with_function(get_weather)
                    .with_thinking_level(ThinkingLevel::Medium)
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Turn 2 failed")
    };

    println!("Turn 2 status: {:?}", response2.status);
    println!("Turn 2 has_thoughts: {}", response2.has_thoughts());
    println!("Turn 2 has_text: {}", response2.has_text());

    if response2.has_thoughts() {
        for (i, thought) in response2.thoughts().enumerate() {
            println!(
                "Turn 2 thought {}: {}...",
                i + 1,
                &thought[..thought.len().min(100)]
            );
        }
    }

    if response2.has_text() {
        println!("Turn 2 text: {}", response2.text().unwrap());
    }

    // Verify we got a response - thoughts may or may not be visible
    // (the API may process reasoning internally without exposing it)
    if response2.has_thoughts() {
        println!("✓ Thoughts visible in Turn 2");
    } else {
        println!("ℹ Thoughts processed internally (not exposed in response)");
    }

    assert!(
        response2.has_text(),
        "Turn 2 should have text response about the weather"
    );

    // Response should reference the weather conditions
    let text2 = response2.text().unwrap().to_lowercase();
    assert!(
        text2.contains("umbrella")
            || text2.contains("rain")
            || text2.contains("yes")
            || text2.contains("18"),
        "Turn 2 should reference weather conditions. Got: {}",
        text2
    );

    // =========================================================================
    // Turn 3: Follow-up question - model reasons with full context
    // =========================================================================
    let response3 = {
        let client = client.clone();
        let prev_id = response2.id.clone().expect("id should exist");
        let get_weather = get_weather.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let prev_id = prev_id.clone();
            let get_weather = get_weather.clone();
            async move {
                stateful_builder(&client)
                    .with_previous_interaction(&prev_id)
                    .with_text(
                        "Given this weather, what indoor activities would you recommend in Tokyo?",
                    )
                    .with_function(get_weather)
                    .with_thinking_level(ThinkingLevel::Medium)
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Turn 3 failed")
    };

    println!("Turn 3 status: {:?}", response3.status);
    println!("Turn 3 has_thoughts: {}", response3.has_thoughts());
    println!("Turn 3 has_text: {}", response3.has_text());

    if response3.has_thoughts() {
        for (i, thought) in response3.thoughts().enumerate() {
            println!(
                "Turn 3 thought {}: {}...",
                i + 1,
                &thought[..thought.len().min(100)]
            );
        }
    }

    if response3.has_text() {
        println!("Turn 3 text: {}", response3.text().unwrap());
    }

    // Verify we got a response - thoughts may or may not be visible
    if response3.has_thoughts() {
        println!("✓ Thoughts visible in Turn 3");
    } else {
        println!("ℹ Thoughts processed internally (not exposed in response)");
    }

    assert!(
        response3.has_text(),
        "Turn 3 should have text response with recommendations"
    );

    // Log reasoning tokens if available (indicates thinking is engaged)
    if let Some(ref usage) = response3.usage
        && let Some(reasoning_tokens) = usage.total_reasoning_tokens
    {
        println!("Turn 3 reasoning tokens: {}", reasoning_tokens);
    }

    // Response should be contextually relevant (about indoor activities)
    let text3 = response3.text().unwrap().to_lowercase();
    assert!(
        text3.contains("indoor")
            || text3.contains("inside")
            || text3.contains("museum")
            || text3.contains("shopping")
            || text3.contains("restaurant")
            || text3.contains("cafe")
            || text3.contains("temple")
            || text3.contains("activity")
            || text3.contains("activities"),
        "Turn 3 should recommend indoor activities. Got: {}",
        text3
    );

    println!("\n✓ All three turns completed successfully with thinking + function calling");
}

/// Test thinking mode with parallel function calls.
///
/// This validates that:
/// - Thinking mode works correctly when the model makes multiple function calls in one response
/// - Thought signatures follow the documented pattern (only first parallel call has signature)
/// - Results can be provided for all parallel calls and the model reasons about them
///
/// Per Google's documentation (https://ai.google.dev/gemini-api/docs/thought-signatures):
/// "If the model generates parallel function calls in a response, only the first
/// function call will contain a signature."
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_thinking_with_parallel_function_calls() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let get_weather = FunctionDeclaration::builder("get_weather")
        .description("Get the current weather for a city")
        .parameter(
            "city",
            json!({"type": "string", "description": "City name"}),
        )
        .required(vec!["city".to_string()])
        .build();

    let get_time = FunctionDeclaration::builder("get_time")
        .description("Get the current time in a timezone")
        .parameter(
            "timezone",
            json!({"type": "string", "description": "Timezone like UTC, PST, JST"}),
        )
        .required(vec!["timezone".to_string()])
        .build();

    // =========================================================================
    // Turn 1: Enable thinking + trigger parallel function calls
    // =========================================================================
    let response1 = {
        let client = client.clone();
        let get_weather = get_weather.clone();
        let get_time = get_time.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let get_weather = get_weather.clone();
            let get_time = get_time.clone();
            async move {
                stateful_builder(&client)
                    .with_text(
                        "What's the weather in Tokyo and what time is it there? \
                         I need both pieces of information.",
                    )
                    .with_functions(vec![get_weather, get_time])
                    .with_thinking_level(ThinkingLevel::Medium)
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Turn 1 failed")
    };

    println!("Turn 1 status: {:?}", response1.status);

    let function_calls = response1.function_calls();
    println!("Number of function calls: {}", function_calls.len());

    if function_calls.is_empty() {
        println!("Model chose not to call functions - skipping rest of test");
        return;
    }

    for (i, call) in function_calls.iter().enumerate() {
        println!(
            "  Call {}: {} (has thought_signature: {})",
            i + 1,
            call.name,
            call.thought_signature.is_some()
        );
    }

    // Per docs: only the first parallel call should have a signature
    if function_calls.len() >= 2 {
        println!("✓ Model made parallel function calls");
        if function_calls[0].thought_signature.is_some() {
            println!("✓ First call has thought_signature (as documented)");
        }
        // Note: We don't hard-assert on signature presence as API behavior varies
    }

    // Verify all calls have IDs
    for call in &function_calls {
        assert!(
            call.id.is_some(),
            "Function call '{}' should have an ID",
            call.name
        );
    }

    // =========================================================================
    // Turn 2: Provide results for all function calls
    // =========================================================================
    let mut results = Vec::new();
    for call in &function_calls {
        let result_data = match call.name {
            "get_weather" => json!({
                "temperature": "22°C",
                "conditions": "partly cloudy",
                "humidity": "65%"
            }),
            "get_time" => json!({
                "time": "14:30",
                "timezone": "JST",
                "date": "2025-01-15"
            }),
            _ => json!({"status": "unknown function"}),
        };

        results.push(function_result_content(
            call.name,
            call.id.expect("call should have ID"),
            result_data,
        ));
    }

    let response2 = {
        let client = client.clone();
        let prev_id = response1.id.clone().expect("id should exist");
        let get_weather = get_weather.clone();
        let get_time = get_time.clone();
        let results = results.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let prev_id = prev_id.clone();
            let get_weather = get_weather.clone();
            let get_time = get_time.clone();
            let results = results.clone();
            async move {
                stateful_builder(&client)
                    .with_previous_interaction(&prev_id)
                    .with_content(results)
                    .with_functions(vec![get_weather, get_time])
                    .with_thinking_level(ThinkingLevel::Medium)
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Turn 2 failed")
    };

    println!("Turn 2 status: {:?}", response2.status);
    println!("Turn 2 has_thoughts: {}", response2.has_thoughts());
    println!("Turn 2 has_text: {}", response2.has_text());

    if response2.has_thoughts() {
        println!("✓ Thoughts visible in Turn 2");
    } else {
        println!("ℹ Thoughts processed internally (not exposed in response)");
    }

    if response2.has_text() {
        let text = response2.text().unwrap();
        println!("Turn 2 text: {}", text);
    }

    assert!(
        response2.has_text(),
        "Turn 2 should have text response combining weather and time info"
    );

    // Response should reference both weather and time
    let text2 = response2.text().unwrap().to_lowercase();
    let has_weather_ref = text2.contains("weather")
        || text2.contains("temperature")
        || text2.contains("22")
        || text2.contains("cloud");
    let has_time_ref = text2.contains("time") || text2.contains("14:30") || text2.contains("2:30");

    println!(
        "References weather: {}, References time: {}",
        has_weather_ref, has_time_ref
    );

    // At minimum, should reference at least one of the function results
    assert!(
        has_weather_ref || has_time_ref,
        "Turn 2 should reference function results. Got: {}",
        text2
    );

    println!("\n✓ Parallel function calls with thinking completed successfully");
}

/// Test thinking mode with sequential function chain containing parallel calls at each step.
///
/// This is the most comprehensive test combining:
/// - Sequential function calling (multi-step chain)
/// - Parallel function calls at each step
/// - Thinking mode enabled throughout
///
/// Per Google's documentation (https://ai.google.dev/gemini-api/docs/thought-signatures):
/// "When there are sequential function calls (multi-step), each function call will have
/// a signature and you must pass all signatures back."
///
/// The Interactions API handles signature management automatically via `previous_interaction_id`.
///
/// Flow:
/// - Step 1: Model calls get_weather + get_time in parallel
/// - Step 2: After results, model calls get_forecast + get_activities in parallel
/// - Step 3: Model combines all information into final response
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_thinking_with_sequential_parallel_function_chain() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Define all functions we'll use
    let get_weather = FunctionDeclaration::builder("get_current_weather")
        .description("Get the current weather conditions for a city")
        .parameter(
            "city",
            json!({"type": "string", "description": "City name"}),
        )
        .required(vec!["city".to_string()])
        .build();

    let get_time = FunctionDeclaration::builder("get_local_time")
        .description("Get the current local time in a city")
        .parameter(
            "city",
            json!({"type": "string", "description": "City name"}),
        )
        .required(vec!["city".to_string()])
        .build();

    let get_forecast = FunctionDeclaration::builder("get_weather_forecast")
        .description("Get the weather forecast for the next few days")
        .parameter(
            "city",
            json!({"type": "string", "description": "City name"}),
        )
        .required(vec!["city".to_string()])
        .build();

    let get_activities = FunctionDeclaration::builder("get_recommended_activities")
        .description("Get recommended activities based on weather conditions")
        .parameter(
            "weather_condition",
            json!({"type": "string", "description": "Current weather like sunny, rainy, cloudy"}),
        )
        .required(vec!["weather_condition".to_string()])
        .build();

    let all_functions = vec![
        get_weather.clone(),
        get_time.clone(),
        get_forecast.clone(),
        get_activities.clone(),
    ];

    // =========================================================================
    // Step 1: Initial request - expect parallel calls for weather and time
    // =========================================================================
    println!("=== Step 1: Initial request ===");

    let response1 = {
        let client = client.clone();
        let functions = all_functions.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let functions = functions.clone();
            async move {
                stateful_builder(&client)
                    .with_text(
                        "I'm planning a trip to Tokyo. I need to know the current weather, \
                         current local time, the forecast for the next few days, and what \
                         activities you'd recommend. Please gather all this information.",
                    )
                    .with_functions(functions)
                    .with_thinking_level(ThinkingLevel::Medium)
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Step 1 failed")
    };

    println!("Step 1 status: {:?}", response1.status);

    let calls1 = response1.function_calls();
    println!("Step 1 function calls: {}", calls1.len());

    if calls1.is_empty() {
        println!("Model chose not to call functions - skipping rest of test");
        return;
    }

    for (i, call) in calls1.iter().enumerate() {
        println!(
            "  Call {}: {} (has signature: {})",
            i + 1,
            call.name,
            call.thought_signature.is_some()
        );
    }

    // Verify all calls have IDs
    for call in &calls1 {
        assert!(call.id.is_some(), "Function call should have ID");
    }

    // =========================================================================
    // Step 2: Provide results for step 1, expect more function calls
    // =========================================================================
    println!("\n=== Step 2: Provide first results ===");

    let mut results1 = Vec::new();
    for call in &calls1 {
        let result_data = match call.name {
            "get_current_weather" => json!({
                "temperature": "24°C",
                "conditions": "partly cloudy",
                "humidity": "60%",
                "wind": "10 km/h"
            }),
            "get_local_time" => json!({
                "time": "10:30 AM",
                "timezone": "JST",
                "date": "2025-01-15"
            }),
            "get_weather_forecast" => json!({
                "tomorrow": "sunny, 26°C",
                "day_after": "cloudy, 22°C",
                "in_3_days": "rainy, 18°C"
            }),
            "get_recommended_activities" => json!({
                "outdoor": ["visit Senso-ji Temple", "walk in Ueno Park"],
                "indoor": ["explore TeamLab", "shop in Shibuya"],
                "food": ["try ramen in Shinjuku", "sushi at Tsukiji"]
            }),
            _ => json!({"status": "unknown function"}),
        };

        results1.push(function_result_content(
            call.name,
            call.id.expect("call should have ID"),
            result_data,
        ));
    }

    let response2 = {
        let client = client.clone();
        let prev_id = response1.id.clone().expect("id should exist");
        let functions = all_functions.clone();
        let results = results1.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let prev_id = prev_id.clone();
            let functions = functions.clone();
            let results = results.clone();
            async move {
                stateful_builder(&client)
                    .with_previous_interaction(&prev_id)
                    .with_content(results)
                    .with_functions(functions)
                    .with_thinking_level(ThinkingLevel::Medium)
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Step 2 failed")
    };

    println!("Step 2 status: {:?}", response2.status);
    println!("Step 2 has_thoughts: {}", response2.has_thoughts());
    println!("Step 2 has_text: {}", response2.has_text());

    let calls2 = response2.function_calls();
    println!("Step 2 function calls: {}", calls2.len());

    for (i, call) in calls2.iter().enumerate() {
        println!(
            "  Call {}: {} (has signature: {})",
            i + 1,
            call.name,
            call.thought_signature.is_some()
        );
    }

    // The model might either:
    // 1. Call more functions (sequential chain continues)
    // 2. Return final text (it has enough info)
    //
    // Both are valid outcomes - we test the chain if it continues

    if !calls2.is_empty() {
        // =====================================================================
        // Step 3: Provide second round of results, expect final response
        // =====================================================================
        println!("\n=== Step 3: Provide second results ===");

        let mut results2 = Vec::new();
        for call in &calls2 {
            let result_data = match call.name {
                "get_current_weather" => json!({
                    "temperature": "24°C",
                    "conditions": "partly cloudy"
                }),
                "get_local_time" => json!({
                    "time": "10:35 AM",
                    "timezone": "JST"
                }),
                "get_weather_forecast" => json!({
                    "tomorrow": "sunny, 26°C",
                    "day_after": "cloudy, 22°C"
                }),
                "get_recommended_activities" => json!({
                    "outdoor": ["temple visits", "park walks"],
                    "indoor": ["museums", "shopping"]
                }),
                _ => json!({"status": "ok"}),
            };

            results2.push(function_result_content(
                call.name,
                call.id.expect("call should have ID"),
                result_data,
            ));
        }

        let response3 = {
            let client = client.clone();
            let prev_id = response2.id.clone().expect("id should exist");
            let functions = all_functions.clone();
            let results = results2.clone();
            retry_on_transient(DEFAULT_MAX_RETRIES, || {
                let client = client.clone();
                let prev_id = prev_id.clone();
                let functions = functions.clone();
                let results = results.clone();
                async move {
                    stateful_builder(&client)
                        .with_previous_interaction(&prev_id)
                        .with_content(results)
                        .with_functions(functions)
                        .with_thinking_level(ThinkingLevel::Medium)
                        .with_store(true)
                        .create()
                        .await
                }
            })
            .await
            .expect("Step 3 failed")
        };

        println!("Step 3 status: {:?}", response3.status);
        println!("Step 3 has_thoughts: {}", response3.has_thoughts());
        println!("Step 3 has_text: {}", response3.has_text());

        if response3.has_thoughts() {
            println!("✓ Thoughts visible in Step 3");
        } else {
            println!("ℹ Thoughts processed internally");
        }

        let calls3 = response3.function_calls();
        if calls3.is_empty() {
            println!("✓ No more function calls - chain complete");
        } else {
            println!("ℹ Model requested {} more function calls", calls3.len());
        }

        if response3.has_text() {
            let text = response3.text().unwrap();
            println!("Step 3 text preview: {}...", &text[..text.len().min(200)]);

            // Verify the response integrates information from the chain
            let text_lower = text.to_lowercase();
            assert!(
                text_lower.contains("tokyo")
                    || text_lower.contains("weather")
                    || text_lower.contains("temperature")
                    || text_lower.contains("activit"),
                "Final response should reference gathered information"
            );
        }

        println!("\n✓ Sequential parallel function chain (3 steps) completed successfully");
    } else {
        // Model returned text in step 2 (gathered all info in first round)
        println!("ℹ Model completed in 2 steps (no sequential chain needed)");

        if response2.has_text() {
            let text = response2.text().unwrap();
            println!("Step 2 text preview: {}...", &text[..text.len().min(200)]);
        }

        assert!(
            response2.has_text(),
            "Step 2 should have text if no more function calls"
        );

        println!("\n✓ Function calls with thinking completed in 2 steps");
    }
}

/// Test different ThinkingLevel values with function calling.
///
/// Validates that all ThinkingLevel variants (Low, Medium, High) work correctly
/// with function calling. Each level allocates different reasoning token budgets,
/// but all should successfully complete the function calling flow.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_thinking_levels_with_function_calling() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let get_weather = FunctionDeclaration::builder("get_weather")
        .description("Get the current weather for a city")
        .parameter(
            "city",
            json!({"type": "string", "description": "City name"}),
        )
        .required(vec!["city".to_string()])
        .build();

    // Test each thinking level
    let levels = [
        (ThinkingLevel::Low, "Low"),
        (ThinkingLevel::Medium, "Medium"),
        (ThinkingLevel::High, "High"),
    ];

    for (level, level_name) in levels {
        println!("\n=== Testing ThinkingLevel::{} ===", level_name);

        // Turn 1: Trigger function call with this thinking level
        let response1 = {
            let client = client.clone();
            let get_weather = get_weather.clone();
            retry_on_transient(DEFAULT_MAX_RETRIES, || {
                let client = client.clone();
                let get_weather = get_weather.clone();
                async move {
                    stateful_builder(&client)
                        .with_text("What's the weather in Paris?")
                        .with_function(get_weather)
                        .with_thinking_level(level)
                        .with_store(true)
                        .create()
                        .await
                }
            })
            .await
            .unwrap_or_else(|e| panic!("Turn 1 failed for ThinkingLevel::{}: {}", level_name, e))
        };

        println!(
            "  Turn 1 status: {:?}, has_thoughts: {}",
            response1.status,
            response1.has_thoughts()
        );

        let function_calls = response1.function_calls();
        if function_calls.is_empty() {
            println!("  Model chose not to call function - skipping this level");
            continue;
        }

        let call = &function_calls[0];
        println!(
            "  Function call: {} (has signature: {})",
            call.name,
            call.thought_signature.is_some()
        );

        // Turn 2: Provide result
        let function_result = function_result_content(
            "get_weather",
            call.id.expect("call should have ID"),
            json!({
                "temperature": "15°C",
                "conditions": "sunny"
            }),
        );

        let response2 = {
            let client = client.clone();
            let prev_id = response1.id.clone().expect("id should exist");
            let get_weather = get_weather.clone();
            let function_result = function_result.clone();
            retry_on_transient(DEFAULT_MAX_RETRIES, || {
                let client = client.clone();
                let prev_id = prev_id.clone();
                let get_weather = get_weather.clone();
                let function_result = function_result.clone();
                async move {
                    stateful_builder(&client)
                        .with_previous_interaction(&prev_id)
                        .with_content(vec![function_result])
                        .with_function(get_weather)
                        .with_thinking_level(level)
                        .create()
                        .await
                }
            })
            .await
            .unwrap_or_else(|e| panic!("Turn 2 failed for ThinkingLevel::{}: {}", level_name, e))
        };

        println!(
            "  Turn 2 status: {:?}, has_thoughts: {}, has_text: {}",
            response2.status,
            response2.has_thoughts(),
            response2.has_text()
        );

        // Log reasoning tokens if available
        if let Some(ref usage) = response2.usage
            && let Some(reasoning_tokens) = usage.total_reasoning_tokens
        {
            println!("  Reasoning tokens used: {}", reasoning_tokens);
        }

        assert!(
            response2.has_text(),
            "ThinkingLevel::{} should produce text response",
            level_name
        );

        println!("  ✓ ThinkingLevel::{} completed successfully", level_name);
    }

    println!("\n✓ All ThinkingLevel variants work with function calling");
}

/// Negative test: Function calling WITHOUT thinking mode.
///
/// This test provides a baseline comparison showing that function calling works
/// correctly without thinking enabled. This helps validate that thinking mode
/// is an enhancement, not a requirement for function calling.
///
/// Comparison with thinking-enabled tests:
/// - No `with_thinking_level()` call
/// - No `Thought` content in responses
/// - No `thought_signature` on function calls
/// - No `total_reasoning_tokens` in usage
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_function_calling_without_thinking() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let get_weather = FunctionDeclaration::builder("get_weather")
        .description("Get the current weather for a city including temperature and conditions")
        .parameter(
            "city",
            json!({"type": "string", "description": "The city name"}),
        )
        .required(vec!["city".to_string()])
        .build();

    // =========================================================================
    // Turn 1: Trigger function call WITHOUT thinking
    // =========================================================================
    println!("=== Turn 1: Request without thinking ===");

    let response1 = {
        let client = client.clone();
        let get_weather = get_weather.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let get_weather = get_weather.clone();
            async move {
                stateful_builder(&client)
                    .with_text("What's the weather in Tokyo?")
                    .with_function(get_weather)
                    // Note: NO with_thinking_level() call
                    .with_store(true)
                    .create()
                    .await
            }
        })
        .await
        .expect("Turn 1 failed")
    };

    println!("Turn 1 status: {:?}", response1.status);
    println!("Turn 1 has_thoughts: {}", response1.has_thoughts());

    // Without thinking mode, there should be no thoughts
    assert!(
        !response1.has_thoughts(),
        "Without thinking mode, response should NOT have thoughts"
    );

    let function_calls = response1.function_calls();
    if function_calls.is_empty() {
        println!("Model chose not to call function - skipping rest of test");
        return;
    }

    let call = &function_calls[0];
    println!(
        "Turn 1 function call: {} (has thought_signature: {})",
        call.name,
        call.thought_signature.is_some()
    );

    // Without thinking mode, thought_signature should not be present
    if call.thought_signature.is_none() {
        println!("✓ No thought_signature (expected without thinking mode)");
    } else {
        println!("ℹ thought_signature present (unexpected but not invalid)");
    }

    assert!(call.id.is_some(), "Function call must have an id");

    // =========================================================================
    // Turn 2: Provide function result
    // =========================================================================
    println!("\n=== Turn 2: Provide result ===");

    let function_result = function_result_content(
        "get_weather",
        call.id.expect("call_id should exist"),
        json!({
            "temperature": "22°C",
            "conditions": "clear",
            "humidity": "45%"
        }),
    );

    let response2 = {
        let client = client.clone();
        let prev_id = response1.id.clone().expect("id should exist");
        let get_weather = get_weather.clone();
        let function_result = function_result.clone();
        retry_on_transient(DEFAULT_MAX_RETRIES, || {
            let client = client.clone();
            let prev_id = prev_id.clone();
            let get_weather = get_weather.clone();
            let function_result = function_result.clone();
            async move {
                stateful_builder(&client)
                    .with_previous_interaction(&prev_id)
                    .with_content(vec![function_result])
                    .with_function(get_weather)
                    // Note: NO with_thinking_level() call
                    .create()
                    .await
            }
        })
        .await
        .expect("Turn 2 failed")
    };

    println!("Turn 2 status: {:?}", response2.status);
    println!("Turn 2 has_thoughts: {}", response2.has_thoughts());
    println!("Turn 2 has_text: {}", response2.has_text());

    // Without thinking mode, there should be no thoughts
    assert!(
        !response2.has_thoughts(),
        "Without thinking mode, response should NOT have thoughts"
    );

    assert!(
        response2.has_text(),
        "Turn 2 should have text response about the weather"
    );

    // Verify no reasoning tokens (thinking was not enabled)
    if let Some(ref usage) = response2.usage {
        if usage.total_reasoning_tokens.is_none() || usage.total_reasoning_tokens == Some(0) {
            println!("✓ No reasoning tokens (expected without thinking mode)");
        } else {
            println!(
                "ℹ Reasoning tokens: {:?} (unexpected without thinking)",
                usage.total_reasoning_tokens
            );
        }
    }

    let text = response2.text().unwrap();
    println!("Turn 2 text: {}", text);

    // Response should reference the weather
    let text_lower = text.to_lowercase();
    assert!(
        text_lower.contains("22") || text_lower.contains("clear") || text_lower.contains("tokyo"),
        "Response should reference weather data. Got: {}",
        text
    );

    println!("\n✓ Function calling without thinking completed successfully");
    println!("  (Provides baseline comparison for thinking-enabled tests)");
}

// =============================================================================
// Thinking + Function Calling + Streaming
// =============================================================================

/// Test thinking mode with function calling in streaming responses.
///
/// This validates that:
/// - Thinking mode works correctly when streaming responses that include function calls
/// - Thought deltas stream incrementally
/// - ThoughtSignature deltas appear in the stream
/// - Function call deltas are properly detected alongside thinking content
///
/// Per Google's documentation (https://ai.google.dev/gemini-api/docs/thought-signatures):
/// - Thought signatures appear on function calls for Gemini 3 models
/// - When streaming, thought content and signatures arrive as deltas
///
/// # Stream Content Types
///
/// When streaming with thinking enabled, the stream may contain:
/// - `Thought` deltas: Incremental reasoning text
/// - `ThoughtSignature` deltas: Cryptographic signatures for verification
/// - `FunctionCall` deltas: The actual function call data
/// - `Text` deltas: Regular text output (in follow-up responses)
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_streaming_with_thinking_and_function_calling() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let get_weather = FunctionDeclaration::builder("get_weather")
        .description("Get the current weather for a city including temperature and conditions")
        .parameter(
            "city",
            json!({"type": "string", "description": "The city name"}),
        )
        .required(vec!["city".to_string()])
        .build();

    // =========================================================================
    // Turn 1: Stream a request that should trigger function call with thinking
    // =========================================================================
    println!("=== Turn 1: Streaming with thinking + function call ===");

    let stream = stateful_builder(&client)
        .with_text("What's the weather in Tokyo? I need to know if I should bring an umbrella.")
        .with_function(get_weather.clone())
        .with_thinking_level(ThinkingLevel::Medium)
        .with_store(true)
        .create_stream();

    let result = consume_stream(stream).await;

    println!("\n--- Streaming Results ---");
    println!("Total deltas: {}", result.delta_count);
    println!("Saw thought deltas: {}", result.saw_thought);
    println!("Saw thought signature: {}", result.saw_thought_signature);
    println!("Saw function call: {}", result.saw_function_call);
    println!("Collected text length: {}", result.collected_text.len());
    println!(
        "Collected thoughts length: {}",
        result.collected_thoughts.len()
    );

    // Verify we received streaming content
    assert!(result.has_output(), "Should receive streaming chunks");

    // Check for thinking-related content
    // Note: The API may or may not expose thoughts in streaming - log but don't hard-assert
    if result.saw_thought {
        println!("✓ Thought deltas received during streaming");
        if !result.collected_thoughts.is_empty() {
            println!(
                "  Thoughts preview: {}...",
                &result.collected_thoughts[..result.collected_thoughts.len().min(100)]
            );
        }
    } else {
        println!("ℹ Thoughts processed internally (not exposed in stream)");
    }

    if result.saw_thought_signature {
        println!("✓ ThoughtSignature delta received during streaming");
    } else {
        println!("ℹ ThoughtSignature not received in stream (API behavior varies)");
    }

    // Check for function call
    let response1 = result
        .final_response
        .expect("Should receive complete response");

    println!("Turn 1 status: {:?}", response1.status);

    let function_calls = response1.function_calls();
    if function_calls.is_empty() {
        // If saw_function_call was true during streaming, the test passes
        if result.saw_function_call {
            println!(
                "✓ Function call deltas detected in stream (final response may not include them)"
            );
            return;
        }
        println!("Model chose not to call function - skipping rest of test");
        return;
    }

    let call = &function_calls[0];
    println!(
        "Function call: {} (has thought_signature: {})",
        call.name,
        call.thought_signature.is_some()
    );

    // =========================================================================
    // Turn 2: Stream the follow-up after providing function result
    // =========================================================================
    println!("\n=== Turn 2: Streaming response after function result ===");

    let function_result = function_result_content(
        "get_weather",
        call.id.expect("call should have ID"),
        json!({
            "temperature": "18°C",
            "conditions": "rainy",
            "precipitation": "85%",
            "humidity": "90%"
        }),
    );

    let stream2 = stateful_builder(&client)
        .with_previous_interaction(response1.id.as_ref().expect("id should exist"))
        .with_content(vec![function_result])
        .with_function(get_weather)
        .with_thinking_level(ThinkingLevel::Medium)
        .with_store(true)
        .create_stream();

    let result2 = consume_stream(stream2).await;

    println!("\n--- Turn 2 Streaming Results ---");
    println!("Total deltas: {}", result2.delta_count);
    println!("Saw thought deltas: {}", result2.saw_thought);
    println!("Saw thought signature: {}", result2.saw_thought_signature);
    println!("Collected text length: {}", result2.collected_text.len());

    // Verify streaming worked
    assert!(result2.has_output(), "Should receive streaming chunks");

    // Log thinking observations
    if result2.saw_thought {
        println!("✓ Thought deltas received in Turn 2");
    } else {
        println!("ℹ Thoughts processed internally in Turn 2");
    }

    // Verify we got text output
    assert!(
        !result2.collected_text.is_empty(),
        "Turn 2 should stream text content"
    );

    // Verify context was maintained - response should reference weather
    let text_lower = result2.collected_text.to_lowercase();
    assert!(
        text_lower.contains("umbrella")
            || text_lower.contains("rain")
            || text_lower.contains("yes")
            || text_lower.contains("18"),
        "Streaming response should reference weather context. Got: {}",
        result2.collected_text
    );

    // Verify final response
    if let Some(response2) = result2.final_response {
        println!("Turn 2 final status: {:?}", response2.status);
        assert_eq!(
            response2.status,
            InteractionStatus::Completed,
            "Turn 2 should complete successfully"
        );
    }

    println!("\n✓ Streaming with thinking + function calling completed successfully");
}

/// Test streaming with thinking but NO function calling (baseline for comparison).
///
/// This provides a baseline to verify that streaming with just thinking works,
/// and helps identify any differences in behavior when function calling is added.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_streaming_with_thinking_only() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    println!("=== Streaming with thinking (no function calling) ===");

    let stream = stateful_builder(&client)
        .with_text("Explain briefly why the sky is blue.")
        .with_thinking_level(ThinkingLevel::Medium)
        .create_stream();

    let result = consume_stream(stream).await;

    println!("\n--- Streaming Results ---");
    println!("Total deltas: {}", result.delta_count);
    println!("Saw thought deltas: {}", result.saw_thought);
    println!("Saw thought signature: {}", result.saw_thought_signature);
    println!("Collected text length: {}", result.collected_text.len());

    // Verify streaming worked
    assert!(result.has_output(), "Should receive streaming chunks");

    // Log thinking observations
    if result.saw_thought {
        println!("✓ Thought deltas received during streaming");
        if !result.collected_thoughts.is_empty() {
            println!(
                "  Thoughts preview: {}...",
                &result.collected_thoughts[..result.collected_thoughts.len().min(100)]
            );
        }
    } else {
        println!("ℹ Thoughts processed internally");
    }

    // Verify we got text
    assert!(
        !result.collected_text.is_empty(),
        "Should stream text content"
    );

    // Verify content is about the sky/light/scattering
    let text_lower = result.collected_text.to_lowercase();
    assert!(
        text_lower.contains("light")
            || text_lower.contains("scatter")
            || text_lower.contains("blue")
            || text_lower.contains("wavelength"),
        "Response should explain why sky is blue. Got: {}",
        result.collected_text
    );

    println!("\n✓ Streaming with thinking (no function calling) completed successfully");
}
