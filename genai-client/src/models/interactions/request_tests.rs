//! Unit tests for request types (CreateInteractionRequest, GenerationConfig, etc.)

use super::*;

#[test]
fn test_serialize_create_interaction_request_with_model() {
    let request = CreateInteractionRequest {
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: InteractionInput::Text("Hello, world!".to_string()),
        previous_interaction_id: None,
        tools: None,
        response_modalities: None,
        response_format: None,
        response_mime_type: None,
        generation_config: None,
        stream: None,
        background: None,
        store: None,
        system_instruction: None,
    };

    let json = serde_json::to_string(&request).expect("Serialization failed");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["model"], "gemini-3-flash-preview");
    assert_eq!(value["input"], "Hello, world!");
    assert!(value.get("agent").is_none());
}

#[test]
fn test_generation_config_serialization() {
    let config = GenerationConfig {
        temperature: Some(0.7),
        max_output_tokens: Some(500),
        top_p: Some(0.9),
        top_k: Some(40),
        thinking_level: Some(ThinkingLevel::Medium),
        seed: None,
        stop_sequences: None,
        thinking_summaries: None,
    };

    let json = serde_json::to_string(&config).expect("Serialization failed");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["temperature"], 0.7);
    assert_eq!(value["maxOutputTokens"], 500);
    assert_eq!(value["thinkingLevel"], "medium");
}

#[test]
fn test_generation_config_new_fields_serialization() {
    let config = GenerationConfig {
        temperature: None,
        max_output_tokens: None,
        top_p: None,
        top_k: None,
        thinking_level: Some(ThinkingLevel::High),
        seed: Some(42),
        stop_sequences: Some(vec!["END".to_string(), "---".to_string()]),
        thinking_summaries: Some(ThinkingSummaries::Auto),
    };

    let json = serde_json::to_string(&config).expect("Serialization failed");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["seed"], 42);
    assert_eq!(value["stopSequences"][0], "END");
    assert_eq!(value["stopSequences"][1], "---");
    assert_eq!(value["thinkingSummaries"], "auto");
    assert_eq!(value["thinkingLevel"], "high");
}

#[test]
fn test_generation_config_roundtrip() {
    let config = GenerationConfig {
        temperature: Some(0.5),
        max_output_tokens: Some(1000),
        top_p: Some(0.95),
        top_k: Some(50),
        thinking_level: Some(ThinkingLevel::Low),
        seed: Some(123456789),
        stop_sequences: Some(vec!["STOP".to_string()]),
        thinking_summaries: Some(ThinkingSummaries::None),
    };

    let json = serde_json::to_string(&config).expect("Serialization failed");
    let deserialized: GenerationConfig =
        serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(deserialized.temperature, config.temperature);
    assert_eq!(deserialized.max_output_tokens, config.max_output_tokens);
    assert_eq!(deserialized.top_p, config.top_p);
    assert_eq!(deserialized.top_k, config.top_k);
    assert_eq!(deserialized.thinking_level, config.thinking_level);
    assert_eq!(deserialized.seed, config.seed);
    assert_eq!(deserialized.stop_sequences, config.stop_sequences);
    assert_eq!(deserialized.thinking_summaries, config.thinking_summaries);
}

#[test]
fn test_thinking_summaries_serialization() {
    // Test Auto variant
    assert_eq!(
        serde_json::to_string(&ThinkingSummaries::Auto).unwrap(),
        "\"auto\""
    );

    // Test None variant
    assert_eq!(
        serde_json::to_string(&ThinkingSummaries::None).unwrap(),
        "\"none\""
    );
}

#[test]
fn test_thinking_summaries_deserialization() {
    // Test known values
    assert_eq!(
        serde_json::from_str::<ThinkingSummaries>("\"auto\"").unwrap(),
        ThinkingSummaries::Auto
    );
    assert_eq!(
        serde_json::from_str::<ThinkingSummaries>("\"none\"").unwrap(),
        ThinkingSummaries::None
    );

    // Test unknown value deserializes to Unknown (Evergreen principle)
    let unknown: ThinkingSummaries = serde_json::from_str("\"future_variant\"").unwrap();
    assert_eq!(unknown, ThinkingSummaries::Unknown);
}

#[test]
fn test_generation_config_skip_serializing_none_fields() {
    let config = GenerationConfig::default();

    let json = serde_json::to_string(&config).expect("Serialization failed");

    // Default config should serialize to empty object
    assert_eq!(json, "{}");
}

#[test]
fn test_generation_config_partial_fields() {
    let config = GenerationConfig {
        seed: Some(42),
        stop_sequences: Some(vec!["DONE".to_string()]),
        ..Default::default()
    };

    let json = serde_json::to_string(&config).expect("Serialization failed");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Only set fields should be present
    assert_eq!(value["seed"], 42);
    assert_eq!(value["stopSequences"][0], "DONE");
    assert!(value.get("temperature").is_none());
    assert!(value.get("thinkingLevel").is_none());
}
