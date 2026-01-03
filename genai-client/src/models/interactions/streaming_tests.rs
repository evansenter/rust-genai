//! Unit tests for streaming types (StreamChunk, InteractionStreamEvent, etc.)

use super::*;

#[test]
fn test_deserialize_streaming_text_content() {
    // Streaming deltas now use InteractionContent directly
    let delta_json = r#"{"type": "text", "text": "Hello world"}"#;
    let delta: InteractionContent =
        serde_json::from_str(delta_json).expect("Deserialization failed");

    match &delta {
        InteractionContent::Text { text, .. } => {
            assert_eq!(text.as_deref(), Some("Hello world"));
        }
        _ => panic!("Expected Text content"),
    }

    assert!(delta.is_text());
    assert!(!delta.is_thought());
    assert_eq!(delta.text(), Some("Hello world"));
}

#[test]
fn test_deserialize_streaming_thought_content() {
    let delta_json = r#"{"type": "thought", "text": "I'm thinking..."}"#;
    let delta: InteractionContent =
        serde_json::from_str(delta_json).expect("Deserialization failed");

    match &delta {
        InteractionContent::Thought { text } => {
            assert_eq!(text.as_deref(), Some("I'm thinking..."));
        }
        _ => panic!("Expected Thought content"),
    }

    assert!(!delta.is_text());
    assert!(delta.is_thought());
    // text() returns None for thoughts (only returns text for Text variant)
    assert_eq!(delta.text(), None);
}

#[test]
fn test_deserialize_streaming_function_call() {
    // Function calls can now be streamed - this was issue #27
    let delta_json =
        r#"{"type": "function_call", "name": "get_weather", "arguments": {"city": "Paris"}}"#;
    let delta: InteractionContent =
        serde_json::from_str(delta_json).expect("Deserialization failed");

    match &delta {
        InteractionContent::FunctionCall { name, args, .. } => {
            assert_eq!(name, "get_weather");
            assert_eq!(args["city"], "Paris");
        }
        _ => panic!("Expected FunctionCall content"),
    }

    assert!(delta.is_function_call());
    assert!(!delta.is_unknown()); // function_call is now a KNOWN type!
}

#[test]
fn test_deserialize_streaming_thought_signature() {
    let delta_json = r#"{"type": "thought_signature", "signature": "abc123"}"#;
    let delta: InteractionContent =
        serde_json::from_str(delta_json).expect("Deserialization failed");

    match &delta {
        InteractionContent::ThoughtSignature { signature } => {
            assert_eq!(signature, "abc123");
        }
        _ => panic!("Expected ThoughtSignature content"),
    }

    assert!(delta.is_thought_signature());
}

#[test]
fn test_deserialize_content_delta_event() {
    let event_json = r#"{
        "event_type": "content.delta",
        "interaction_id": "test_123",
        "delta": {"type": "text", "text": "Hello"}
    }"#;

    let event: InteractionStreamEvent =
        serde_json::from_str(event_json).expect("Deserialization failed");

    assert_eq!(event.event_type, "content.delta");
    assert_eq!(event.interaction_id.as_deref(), Some("test_123"));
    assert!(event.delta.is_some());
    assert!(event.interaction.is_none());

    let delta = event.delta.unwrap();
    assert!(delta.is_text());
    assert_eq!(delta.text(), Some("Hello"));
}

#[test]
fn test_deserialize_interaction_complete_event() {
    let event_json = r#"{
        "event_type": "interaction.complete",
        "interaction": {
            "id": "interaction_456",
            "model": "gemini-3-flash-preview",
            "input": [{"type": "text", "text": "Count to 3"}],
            "outputs": [{"type": "text", "text": "1, 2, 3"}],
            "status": "completed"
        }
    }"#;

    let event: InteractionStreamEvent =
        serde_json::from_str(event_json).expect("Deserialization failed");

    assert_eq!(event.event_type, "interaction.complete");
    assert!(event.interaction.is_some());
    assert!(event.delta.is_none());

    let interaction = event.interaction.unwrap();
    assert_eq!(interaction.id.as_deref(), Some("interaction_456"));
    assert_eq!(interaction.text(), Some("1, 2, 3"));
}
