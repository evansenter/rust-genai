//! Unit tests for request types (CreateInteractionRequest, GenerationConfig, etc.)

use super::*;

#[test]
fn test_serialize_create_interaction_request_with_model() {
    let request = CreateInteractionRequest {
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        agent_config: None,
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
        tool_choice: None,
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
        tool_choice: None,
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
        tool_choice: None,
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
    assert_eq!(deserialized.tool_choice, config.tool_choice);
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

    // Test unknown value deserializes to Unknown with data preserved (Evergreen principle)
    let unknown: ThinkingSummaries = serde_json::from_str("\"future_variant\"").unwrap();
    assert!(unknown.is_unknown());
    assert_eq!(unknown.unknown_summaries_type(), Some("future_variant"));
    assert_eq!(
        unknown.unknown_data(),
        Some(&serde_json::Value::String("future_variant".to_string()))
    );
}

#[test]
fn test_thinking_summaries_unknown_roundtrip() {
    // Test that unknown values roundtrip correctly
    let unknown = ThinkingSummaries::Unknown {
        summaries_type: "new_mode".to_string(),
        data: serde_json::Value::String("new_mode".to_string()),
    };

    let json = serde_json::to_string(&unknown).expect("Serialization failed");
    assert_eq!(json, "\"new_mode\"");

    let deserialized: ThinkingSummaries = serde_json::from_str(&json).unwrap();
    assert!(deserialized.is_unknown());
    assert_eq!(deserialized.unknown_summaries_type(), Some("new_mode"));
}

#[test]
fn test_thinking_level_deserialization() {
    // Test known values
    assert_eq!(
        serde_json::from_str::<ThinkingLevel>("\"minimal\"").unwrap(),
        ThinkingLevel::Minimal
    );
    assert_eq!(
        serde_json::from_str::<ThinkingLevel>("\"low\"").unwrap(),
        ThinkingLevel::Low
    );
    assert_eq!(
        serde_json::from_str::<ThinkingLevel>("\"medium\"").unwrap(),
        ThinkingLevel::Medium
    );
    assert_eq!(
        serde_json::from_str::<ThinkingLevel>("\"high\"").unwrap(),
        ThinkingLevel::High
    );

    // Test unknown value deserializes to Unknown with data preserved (Evergreen principle)
    let unknown: ThinkingLevel = serde_json::from_str("\"extreme\"").unwrap();
    assert!(unknown.is_unknown());
    assert_eq!(unknown.unknown_level_type(), Some("extreme"));
    assert_eq!(
        unknown.unknown_data(),
        Some(&serde_json::Value::String("extreme".to_string()))
    );
}

#[test]
fn test_thinking_level_serialization() {
    // Test known variants serialize correctly
    assert_eq!(
        serde_json::to_string(&ThinkingLevel::Minimal).unwrap(),
        "\"minimal\""
    );
    assert_eq!(
        serde_json::to_string(&ThinkingLevel::Low).unwrap(),
        "\"low\""
    );
    assert_eq!(
        serde_json::to_string(&ThinkingLevel::Medium).unwrap(),
        "\"medium\""
    );
    assert_eq!(
        serde_json::to_string(&ThinkingLevel::High).unwrap(),
        "\"high\""
    );
}

#[test]
fn test_thinking_level_unknown_roundtrip() {
    // Test that unknown values roundtrip correctly
    let unknown = ThinkingLevel::Unknown {
        level_type: "extreme".to_string(),
        data: serde_json::Value::String("extreme".to_string()),
    };

    let json = serde_json::to_string(&unknown).expect("Serialization failed");
    assert_eq!(json, "\"extreme\"");

    let deserialized: ThinkingLevel = serde_json::from_str(&json).unwrap();
    assert!(deserialized.is_unknown());
    assert_eq!(deserialized.unknown_level_type(), Some("extreme"));
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

#[test]
fn test_thinking_level_object_form_deserialization() {
    // Test that object-form thinking levels are handled (future API compatibility)
    let json = r#"{"level": "ultra", "budget": 5000}"#;
    let parsed: ThinkingLevel = serde_json::from_str(json).expect("Deserialization should succeed");

    assert!(parsed.is_unknown());
    assert_eq!(parsed.unknown_level_type(), Some("ultra"));

    // Verify the full object is preserved
    let data = parsed.unknown_data().unwrap();
    assert_eq!(data.get("budget").unwrap(), 5000);
}

#[test]
fn test_thinking_summaries_object_form_deserialization() {
    // Test that object-form thinking summaries are handled (future API compatibility)
    let json = r#"{"summaries": "detailed", "format": "markdown"}"#;
    let parsed: ThinkingSummaries =
        serde_json::from_str(json).expect("Deserialization should succeed");

    assert!(parsed.is_unknown());
    assert_eq!(parsed.unknown_summaries_type(), Some("detailed"));

    // Verify the full object is preserved
    let data = parsed.unknown_data().unwrap();
    assert_eq!(data.get("format").unwrap(), "markdown");
}

// =============================================================================
// AgentConfig Tests
// =============================================================================

#[test]
fn test_agent_config_deep_research_serialization() {
    let config = AgentConfig::DeepResearch {
        thinking_summaries: Some(ThinkingSummaries::Auto),
    };

    let json = serde_json::to_string(&config).expect("Serialization failed");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["type"], "deep-research");
    assert_eq!(value["thinkingSummaries"], "auto");
}

#[test]
fn test_agent_config_deep_research_without_thinking_summaries() {
    let config = AgentConfig::DeepResearch {
        thinking_summaries: None,
    };

    let json = serde_json::to_string(&config).expect("Serialization failed");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["type"], "deep-research");
    assert!(value.get("thinkingSummaries").is_none());
}

#[test]
fn test_agent_config_dynamic_serialization() {
    let config = AgentConfig::Dynamic;

    let json = serde_json::to_string(&config).expect("Serialization failed");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["type"], "dynamic");
}

#[test]
fn test_agent_config_deep_research_deserialization() {
    let json = r#"{"type": "deep-research", "thinkingSummaries": "auto"}"#;
    let parsed: AgentConfig = serde_json::from_str(json).expect("Deserialization should succeed");

    match parsed {
        AgentConfig::DeepResearch { thinking_summaries } => {
            assert_eq!(thinking_summaries, Some(ThinkingSummaries::Auto));
        }
        _ => panic!("Expected DeepResearch variant"),
    }
}

#[test]
fn test_agent_config_dynamic_deserialization() {
    let json = r#"{"type": "dynamic"}"#;
    let parsed: AgentConfig = serde_json::from_str(json).expect("Deserialization should succeed");

    assert!(matches!(parsed, AgentConfig::Dynamic));
}

#[test]
fn test_agent_config_unknown_deserialization() {
    // Test that unknown agent config types deserialize to Unknown (Evergreen principle)
    let json = r#"{"type": "future-agent", "customField": 42}"#;
    let parsed: AgentConfig = serde_json::from_str(json).expect("Deserialization should succeed");

    assert!(parsed.is_unknown());
    assert_eq!(parsed.unknown_config_type(), Some("future-agent"));

    // Verify the full object is preserved
    let data = parsed.unknown_data().unwrap();
    assert_eq!(data.get("customField").unwrap(), 42);
}

#[test]
fn test_agent_config_unknown_roundtrip() {
    // Test that unknown values roundtrip correctly
    let unknown = AgentConfig::Unknown {
        config_type: "future-agent".to_string(),
        data: serde_json::json!({
            "type": "future-agent",
            "customField": 42
        }),
    };

    let json = serde_json::to_string(&unknown).expect("Serialization failed");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Should preserve the type and flatten the data
    assert_eq!(value["type"], "future-agent");
    assert_eq!(value["customField"], 42);

    // Should roundtrip back to Unknown
    let deserialized: AgentConfig = serde_json::from_str(&json).unwrap();
    assert!(deserialized.is_unknown());
    assert_eq!(deserialized.unknown_config_type(), Some("future-agent"));
}

#[test]
fn test_agent_config_helper_methods() {
    // Known variants
    let deep_research = AgentConfig::DeepResearch {
        thinking_summaries: None,
    };
    assert!(!deep_research.is_unknown());
    assert_eq!(deep_research.unknown_config_type(), None);
    assert_eq!(deep_research.unknown_data(), None);

    let dynamic = AgentConfig::Dynamic;
    assert!(!dynamic.is_unknown());

    // Unknown variant
    let unknown = AgentConfig::Unknown {
        config_type: "test".to_string(),
        data: serde_json::json!({}),
    };
    assert!(unknown.is_unknown());
    assert_eq!(unknown.unknown_config_type(), Some("test"));
    assert!(unknown.unknown_data().is_some());
}

#[test]
fn test_create_interaction_request_with_agent_config() {
    let request = CreateInteractionRequest {
        model: None,
        agent: Some("deep-research-pro-preview-12-2025".to_string()),
        agent_config: Some(AgentConfig::DeepResearch {
            thinking_summaries: Some(ThinkingSummaries::Auto),
        }),
        input: InteractionInput::Text("Research question".to_string()),
        previous_interaction_id: None,
        tools: None,
        response_modalities: None,
        response_format: None,
        response_mime_type: None,
        generation_config: None,
        stream: None,
        background: Some(true),
        store: Some(true),
        system_instruction: None,
    };

    let json = serde_json::to_string(&request).expect("Serialization failed");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["agent"], "deep-research-pro-preview-12-2025");
    assert_eq!(value["agent_config"]["type"], "deep-research");
    assert_eq!(value["agent_config"]["thinkingSummaries"], "auto");
    assert_eq!(value["background"], true);
    assert_eq!(value["store"], true);
}

/// Test that verifies the field naming conventions used in AgentConfig serialization.
///
/// This test explicitly documents the casing decisions:
/// - `type` key uses kebab-case for values: "deep-research", "dynamic"
/// - `thinkingSummaries` key uses camelCase (consistent with other Gemini API fields like
///   `maxOutputTokens`, `topP`, `topK` in GenerationConfig)
///
/// The outer `agent_config` field is snake_case per API documentation, while inner
/// fields follow the camelCase convention used throughout the Gemini Interactions API.
#[test]
fn test_agent_config_field_naming_conventions() {
    // Verify the exact JSON structure matches API expectations
    let config = AgentConfig::DeepResearch {
        thinking_summaries: Some(ThinkingSummaries::Auto),
    };

    let json = serde_json::to_string(&config).expect("Serialization failed");

    // Expected: {"type":"deep-research","thinkingSummaries":"auto"}
    // NOT: {"type":"deep-research","thinking_summaries":"auto"}
    assert!(
        json.contains("thinkingSummaries"),
        "Field should be camelCase 'thinkingSummaries', got: {}",
        json
    );
    assert!(
        !json.contains("thinking_summaries"),
        "Field should NOT be snake_case 'thinking_summaries', got: {}",
        json
    );

    // Verify value is lowercase
    assert!(
        json.contains(r#""auto""#),
        "ThinkingSummaries::Auto should serialize to 'auto', got: {}",
        json
    );
}
