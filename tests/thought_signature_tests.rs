// Tests for thought signature support in both GenerateContent and Interactions APIs

use genai_client::models::response::PartResponse;
use genai_client::{FunctionCall, InteractionContent, Part};
use rust_genai::{
    Client, GenerateContentResponse, WithFunctionCalling, function_call_content_with_signature,
    model_function_calls_request_with_signatures, user_text, user_tool_response,
};
use serde_json::json;
use std::env;

#[test]
fn test_part_response_deserializes_thought_signature() {
    let json_with_signature = r#"{
        "text": "I'll call the function",
        "thoughtSignature": "encrypted_signature_token_12345"
    }"#;

    let part: PartResponse = serde_json::from_str(json_with_signature)
        .expect("Failed to deserialize PartResponse with thought signature");

    assert_eq!(part.text, Some("I'll call the function".to_string()));
    assert_eq!(
        part.thought_signature,
        Some("encrypted_signature_token_12345".to_string())
    );
}

#[test]
fn test_part_response_deserializes_without_thought_signature() {
    let json_without_signature = r#"{
        "text": "Regular response"
    }"#;

    let part: PartResponse = serde_json::from_str(json_without_signature)
        .expect("Failed to deserialize PartResponse without thought signature");

    assert_eq!(part.text, Some("Regular response".to_string()));
    assert_eq!(part.thought_signature, None);
}

#[test]
fn test_part_serializes_thought_signature() {
    let part_with_signature = Part {
        text: None,
        function_call: Some(FunctionCall {
            name: "test_function".to_string(),
            args: json!({"param": "value"}),
        }),
        function_response: None,
        thought_signature: Some("signature_abc123".to_string()),
    };

    let serialized = serde_json::to_string(&part_with_signature)
        .expect("Failed to serialize Part with thought signature");

    assert!(serialized.contains("thoughtSignature"));
    assert!(serialized.contains("signature_abc123"));
}

#[test]
fn test_part_omits_none_thought_signature() {
    let part_without_signature = Part {
        text: Some("Hello".to_string()),
        function_call: None,
        function_response: None,
        thought_signature: None,
    };

    let serialized = serde_json::to_string(&part_without_signature)
        .expect("Failed to serialize Part without thought signature");

    // Should not include thoughtSignature field when None
    assert!(!serialized.contains("thoughtSignature"));
}

#[test]
fn test_generate_content_response_has_thought_signatures() {
    let response = GenerateContentResponse {
        text: Some("Response text".to_string()),
        function_calls: None,
        code_execution_results: None,
        thought_signatures: Some(vec!["sig1".to_string(), "sig2".to_string()]),
    };

    assert_eq!(response.thought_signatures.unwrap().len(), 2);
}

#[test]
fn test_model_function_calls_request_with_signatures() {
    let calls = vec![
        FunctionCall {
            name: "func1".to_string(),
            args: json!({}),
        },
        FunctionCall {
            name: "func2".to_string(),
            args: json!({"x": 1}),
        },
    ];

    let signatures = Some(vec!["signature_1".to_string(), "signature_2".to_string()]);

    let content = model_function_calls_request_with_signatures(calls, signatures);

    // Verify we have 2 parts
    assert_eq!(content.parts.len(), 2);

    // Verify signatures were attached correctly
    assert_eq!(
        content.parts[0].thought_signature,
        Some("signature_1".to_string())
    );
    assert_eq!(
        content.parts[1].thought_signature,
        Some("signature_2".to_string())
    );
}

#[test]
fn test_model_function_calls_request_with_mismatched_signatures() {
    // More function calls than signatures
    let calls = vec![
        FunctionCall {
            name: "func1".to_string(),
            args: json!({}),
        },
        FunctionCall {
            name: "func2".to_string(),
            args: json!({}),
        },
        FunctionCall {
            name: "func3".to_string(),
            args: json!({}),
        },
    ];

    let signatures = Some(vec![
        "signature_1".to_string(),
        "signature_2".to_string(),
        // Only 2 signatures for 3 calls
    ]);

    let content = model_function_calls_request_with_signatures(calls, signatures);

    assert_eq!(content.parts.len(), 3);
    assert_eq!(
        content.parts[0].thought_signature,
        Some("signature_1".to_string())
    );
    assert_eq!(
        content.parts[1].thought_signature,
        Some("signature_2".to_string())
    );
    assert_eq!(content.parts[2].thought_signature, None); // No signature for 3rd call
}

#[test]
fn test_interaction_content_function_call_with_signature() {
    let content = function_call_content_with_signature(
        "test_func",
        json!({"param": "value"}),
        Some("interaction_signature_xyz".to_string()),
    );

    match content {
        InteractionContent::FunctionCall {
            name,
            args,
            thought_signature,
        } => {
            assert_eq!(name, "test_func");
            assert_eq!(args, json!({"param": "value"}));
            assert_eq!(
                thought_signature,
                Some("interaction_signature_xyz".to_string())
            );
        }
        _ => panic!("Expected FunctionCall variant"),
    }
}

#[test]
fn test_interaction_content_serializes_thought_signature() {
    let content = InteractionContent::FunctionCall {
        name: "my_function".to_string(),
        args: json!({"key": "value"}),
        thought_signature: Some("sig_token".to_string()),
    };

    let serialized =
        serde_json::to_string(&content).expect("Failed to serialize InteractionContent");

    // Should include thoughtSignature field
    assert!(serialized.contains("thoughtSignature"));
    assert!(serialized.contains("sig_token"));
    assert!(serialized.contains("\"type\":\"function_call\""));
}

#[test]
fn test_interaction_content_omits_none_signature() {
    let content = InteractionContent::FunctionCall {
        name: "my_function".to_string(),
        args: json!({"key": "value"}),
        thought_signature: None,
    };

    let serialized =
        serde_json::to_string(&content).expect("Failed to serialize InteractionContent");

    // Should not include thoughtSignature field when None
    assert!(!serialized.contains("thoughtSignature"));
}

#[test]
fn test_conversation_history_preserves_signatures() {
    // Simulate a multi-turn conversation with thought signatures
    let call = FunctionCall {
        name: "get_weather".to_string(),
        args: json!({"location": "Tokyo"}),
    };

    let signatures = Some(vec!["gemini3_sig_abc".to_string()]);

    let contents = vec![
        user_text("What's the weather in Tokyo?".to_string()),
        model_function_calls_request_with_signatures(vec![call], signatures),
        user_text("It's 22 degrees and sunny".to_string()),
    ];

    // Verify the middle content has the thought signature
    assert_eq!(contents.len(), 3);
    assert_eq!(
        contents[1].parts[0].thought_signature,
        Some("gemini3_sig_abc".to_string())
    );

    // Verify it serializes correctly
    let serialized =
        serde_json::to_string(&contents[1]).expect("Failed to serialize conversation turn");
    assert!(serialized.contains("thoughtSignature"));
    assert!(serialized.contains("gemini3_sig_abc"));
}

// Integration test that verifies thought signatures work with real Gemini 3 API calls
#[tokio::test]
#[ignore = "Makes real API calls - requires GEMINI_API_KEY"]
async fn test_gemini3_thought_signatures_integration() {
    let Ok(api_key) = env::var("GEMINI_API_KEY") else {
        println!("Skipping test_gemini3_thought_signatures_integration: GEMINI_API_KEY not set.");
        return;
    };

    let client = Client::builder(api_key).build();

    // Define a simple mock function for weather
    let weather_function = rust_genai::FunctionDeclaration::builder("get_weather")
        .description("Get the weather for a location")
        .parameter(
            "location",
            json!({"type": "string", "description": "The city name"}),
        )
        .required(vec!["location".to_string()])
        .build();

    // Step 1: Make initial request with function declaration
    let response1 = client
        .with_model("gemini-3-flash-preview")
        .with_prompt("What's the weather in Tokyo?")
        .with_function(weather_function.clone())
        .generate()
        .await;

    assert!(
        response1.is_ok(),
        "Initial request failed: {:?}",
        response1.err()
    );

    let response1 = response1.unwrap();

    // Verify we got function calls
    assert!(
        response1.function_calls.is_some(),
        "Expected function calls from Gemini 3"
    );

    // Step 2: Extract thought signatures (critical for Gemini 3)
    let thought_signatures = response1.thought_signatures.clone();
    assert!(
        thought_signatures.is_some(),
        "Expected thought signatures from Gemini 3 response"
    );

    let function_calls = response1.function_calls.unwrap();
    let signatures = thought_signatures.as_ref().unwrap();

    // Verify signature count matches function call count
    assert_eq!(
        function_calls.len(),
        signatures.len(),
        "Thought signature count should match function call count"
    );

    // Step 3: Build conversation history WITH thought signatures
    let internal_calls: Vec<FunctionCall> = function_calls
        .into_iter()
        .map(|fc| FunctionCall {
            name: fc.name,
            args: fc.args,
        })
        .collect();

    let contents = vec![
        user_text("What's the weather in Tokyo?".to_string()),
        model_function_calls_request_with_signatures(internal_calls, thought_signatures),
        user_tool_response(
            "get_weather".to_string(),
            json!({"temperature": "22°C", "conditions": "sunny"}),
        ),
    ];

    // Step 4: Send follow-up request with conversation history
    let response2 = client
        .with_model("gemini-3-flash-preview")
        .with_contents(contents)
        .with_function(weather_function)
        .generate()
        .await;

    // This should succeed with thought signatures, but would fail with 400 without them
    assert!(
        response2.is_ok(),
        "Follow-up request with thought signatures failed: {:?}",
        response2.err()
    );

    let response2 = response2.unwrap();

    // Verify we got a text response (not another function call)
    assert!(
        response2.text.is_some(),
        "Expected text response after providing function result"
    );

    println!(
        "Integration test successful! Final response: {}",
        response2.text.as_deref().unwrap_or("")
    );
}
