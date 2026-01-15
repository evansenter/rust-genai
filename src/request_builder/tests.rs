//! Unit tests for InteractionBuilder.

use super::*;
use crate::Tool;
use crate::{Client, FunctionDeclaration};
use serde_json::json;

fn create_test_client() -> Client {
    Client::builder("test-api-key".to_string())
        .build()
        .expect("test client should build")
}

#[test]
fn test_function_declaration_builder() {
    let func_decl = FunctionDeclaration::builder("my_func")
        .description("Does something")
        .parameter("arg1", json!({"type": "string"}))
        .required(vec!["arg1".to_string()])
        .build();

    assert_eq!(func_decl.name(), "my_func");
    assert_eq!(func_decl.description(), "Does something");
    assert_eq!(func_decl.parameters().type_(), "object");
    assert_eq!(
        func_decl
            .parameters()
            .properties()
            .get("arg1")
            .unwrap()
            .get("type")
            .unwrap()
            .as_str(),
        Some("string")
    );
    assert_eq!(func_decl.parameters().required(), vec!["arg1".to_string()]);
}

#[test]
fn test_function_declaration_into_tool() {
    let func_decl = FunctionDeclaration::builder("test")
        .description("Test function")
        .build();

    let tool = func_decl.into_tool();
    match tool {
        Tool::Function { name, .. } => {
            assert_eq!(name, "test");
        }
        _ => panic!("Expected Tool::Function variant"),
    }
}

// --- InteractionBuilder Tests ---

#[test]
fn test_interaction_builder_with_model() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello");

    assert_eq!(builder.model.as_deref(), Some("gemini-3-flash-preview"));
    assert!(builder.agent.is_none());
    assert_eq!(builder.current_message.as_deref(), Some("Hello"));
}

#[test]
fn test_interaction_builder_with_agent() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_agent("deep-research-pro")
        .with_text("Research topic");

    assert!(builder.model.is_none());
    assert_eq!(builder.agent.as_deref(), Some("deep-research-pro"));
}

#[test]
fn test_interaction_builder_with_previous_interaction() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Follow-up question")
        .with_previous_interaction("interaction_123");

    assert_eq!(
        builder.previous_interaction_id.as_deref(),
        Some("interaction_123")
    );
}

#[test]
fn test_interaction_builder_with_system_instruction() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello")
        .with_system_instruction("You are a helpful assistant");

    assert!(matches!(
        builder.system_instruction,
        Some(crate::InteractionInput::Text(_))
    ));
}

#[test]
fn test_interaction_builder_with_generation_config() {
    let client = create_test_client();
    let config = crate::GenerationConfig {
        temperature: Some(0.7),
        max_output_tokens: Some(1000),
        top_p: Some(0.9),
        top_k: Some(40),
        thinking_level: Some(ThinkingLevel::Medium),
        ..Default::default()
    };

    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello")
        .with_generation_config(config.clone());

    assert!(builder.generation_config.is_some());
    assert_eq!(
        builder.generation_config.as_ref().unwrap().temperature,
        Some(0.7)
    );
}

#[test]
fn test_interaction_builder_with_function() {
    let client = create_test_client();
    let func = FunctionDeclaration::builder("test_func")
        .description("Test function")
        .parameter("location", json!({"type": "string"}))
        .required(vec!["location".to_string()])
        .build();

    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Call a function")
        .add_function(func);

    assert!(builder.tools.is_some());
    assert_eq!(builder.tools.as_ref().unwrap().len(), 1);
}

#[test]
fn test_interaction_builder_with_mcp_server() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Use MCP server")
        .with_mcp_server("my-server", "https://mcp.example.com/api");

    assert!(builder.tools.is_some());
    let tools = builder.tools.as_ref().unwrap();
    assert_eq!(tools.len(), 1);

    match &tools[0] {
        Tool::McpServer { name, url } => {
            assert_eq!(name, "my-server");
            assert_eq!(url, "https://mcp.example.com/api");
        }
        _ => panic!("Expected Tool::McpServer variant"),
    }
}

#[test]
fn test_interaction_builder_with_multiple_mcp_servers() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Use multiple MCP servers")
        .with_mcp_server("server-1", "https://mcp1.example.com")
        .with_mcp_server("server-2", "https://mcp2.example.com");

    assert!(builder.tools.is_some());
    let tools = builder.tools.as_ref().unwrap();
    assert_eq!(tools.len(), 2);
}

#[test]
fn test_interaction_builder_with_mcp_server_and_other_tools() {
    let client = create_test_client();
    let func = FunctionDeclaration::builder("test_func")
        .description("Test function")
        .build();

    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Use MCP and other tools")
        .with_mcp_server("my-server", "https://mcp.example.com")
        .with_google_search()
        .add_function(func);

    assert!(builder.tools.is_some());
    let tools = builder.tools.as_ref().unwrap();
    assert_eq!(tools.len(), 3);
}

#[test]
fn test_interaction_builder_with_background() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_agent("deep-research-pro")
        .with_text("Long running task")
        .with_background(true);

    assert_eq!(builder.background, Some(true));
}

#[test]
fn test_interaction_builder_with_store_disabled() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Temporary interaction")
        .with_store_disabled();

    // Note: with_store_disabled() transitions to StoreDisabled state
    // and sets store = Some(false) internally
    assert_eq!(builder.store, Some(false));
}

#[test]
fn test_interaction_builder_with_store_enabled() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Stored interaction")
        .with_store_enabled();

    assert_eq!(builder.store, Some(true));
}

#[test]
fn test_interaction_builder_build_success() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello");

    let result = builder.build();
    assert!(result.is_ok());

    let request = result.unwrap();
    assert_eq!(request.model.as_deref(), Some("gemini-3-flash-preview"));
    assert!(matches!(request.input, crate::InteractionInput::Text(_)));
}

#[test]
fn test_interaction_builder_build_missing_input() {
    let client = create_test_client();
    let builder = client.interaction().with_model("gemini-3-flash-preview");

    let result = builder.build();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        crate::GenaiError::InvalidInput(_)
    ));
}

#[test]
fn test_interaction_builder_build_missing_model_and_agent() {
    let client = create_test_client();
    let builder = client.interaction().with_text("Hello");

    let result = builder.build();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        crate::GenaiError::InvalidInput(_)
    ));
}

#[test]
fn test_interaction_builder_with_response_modalities() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Generate an image")
        .with_response_modalities(vec!["IMAGE".to_string()]);

    assert_eq!(
        builder.response_modalities.as_ref().unwrap(),
        &vec!["IMAGE".to_string()]
    );
}

#[test]
fn test_interaction_builder_with_max_function_call_loops() {
    let client = create_test_client();

    // Test default value
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Test");
    assert_eq!(
        builder.max_function_call_loops,
        super::auto_functions::DEFAULT_MAX_FUNCTION_CALL_LOOPS
    );

    // Test custom value
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Test")
        .with_max_function_call_loops(10);
    assert_eq!(builder.max_function_call_loops, 10);

    // Test setting to minimum (1)
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Test")
        .with_max_function_call_loops(1);
    assert_eq!(builder.max_function_call_loops, 1);
}

// --- Typestate Tests ---
//
// These tests verify compile-time enforcement of API constraints via typestate.
// The actual compile-time checks are verified by the fact that this code compiles -
// invalid combinations won't compile. See ui_tests.rs for compile-fail tests.

#[test]
fn test_typestate_first_turn_has_system_instruction() {
    // FirstTurn builders can set system instruction
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello")
        .with_system_instruction("Be helpful");

    assert!(builder.system_instruction.is_some());
}

#[test]
fn test_typestate_chained_preserves_fields() {
    // When transitioning FirstTurn -> Chained, fields are preserved
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello")
        .with_system_instruction("Be helpful")
        .with_previous_interaction("prev-123");

    // Fields should be preserved through the transition
    assert_eq!(builder.model.as_deref(), Some("gemini-3-flash-preview"));
    assert!(builder.system_instruction.is_some());
    assert_eq!(builder.previous_interaction_id.as_deref(), Some("prev-123"));
}

#[test]
fn test_typestate_store_disabled_sets_store_false() {
    // StoreDisabled transition sets store = Some(false)
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello")
        .with_store_disabled();

    assert_eq!(builder.store, Some(false));
}

#[test]
fn test_typestate_store_disabled_clears_background() {
    // StoreDisabled transition clears background
    let client = create_test_client();
    let first_turn = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello")
        .with_background(true);

    assert_eq!(first_turn.background, Some(true));

    // Transitioning to StoreDisabled clears background
    let disabled = first_turn.with_store_disabled();
    assert_eq!(disabled.background, None);
}

#[test]
fn test_typestate_chained_can_set_background() {
    // Chained builders can set background (unlike StoreDisabled)
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello")
        .with_previous_interaction("prev-123")
        .with_background(true);

    assert_eq!(builder.background, Some(true));
}

#[test]
fn test_typestate_first_turn_can_set_background() {
    // FirstTurn builders can set background
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello")
        .with_background(true);

    assert_eq!(builder.background, Some(true));
}

// NOTE: The following compile-time constraints are enforced by typestate:
//
// 1. StoreDisabled builders cannot call:
//    - with_previous_interaction() - requires storage
//    - with_background(true) - requires storage
//    - create_with_auto_functions() - requires storage
//    - create_stream_with_auto_functions() - requires storage
//
// 2. Chained builders cannot call:
//    - with_system_instruction() - inherited from previous
//    - with_store_disabled() - chained requires storage
//
// These are verified by compile-fail tests in tests/ui_tests.rs

#[tokio::test]
async fn test_auto_functions_allows_store_true() {
    // This test verifies that store=true (explicit) doesn't trigger the validation error.
    // The actual API call will fail (invalid key), but validation should pass.
    let client = create_test_client();
    let func = FunctionDeclaration::builder("test_func")
        .description("Test function")
        .build();

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Test")
        .add_function(func)
        .with_store_enabled() // Explicitly true
        .create_with_auto_functions()
        .await;

    // Should fail with API error (invalid key), not InvalidInput validation error
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        !matches!(err, crate::GenaiError::InvalidInput(_)),
        "Should not be an InvalidInput error (validation passed), got: {:?}",
        err
    );
}

#[tokio::test]
async fn test_auto_functions_allows_store_default() {
    // This test verifies that store=None (default) doesn't trigger the validation error.
    // The actual API call will fail (invalid key), but validation should pass.
    let client = create_test_client();
    let func = FunctionDeclaration::builder("test_func")
        .description("Test function")
        .build();

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Test")
        .add_function(func)
        // No .with_store() call - uses default (None, which means true on server)
        .create_with_auto_functions()
        .await;

    // Should fail with API error (invalid key), not InvalidInput validation error
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        !matches!(err, crate::GenaiError::InvalidInput(_)),
        "Should not be an InvalidInput error (validation passed), got: {:?}",
        err
    );
}

// --- Turn Array Input Tests ---

#[test]
fn test_interaction_builder_with_history() {
    use crate::Turn;

    let client = create_test_client();
    let turns = vec![
        Turn::user("What is 2+2?"),
        Turn::model("2+2 equals 4."),
        Turn::user("And what's that times 3?"),
    ];

    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_history(turns);

    assert_eq!(builder.history.len(), 3);
}

#[test]
fn test_interaction_builder_build_with_history() {
    use crate::Turn;

    let client = create_test_client();
    let turns = vec![
        Turn::user("Hello"),
        Turn::model("Hi there!"),
        Turn::user("How are you?"),
    ];

    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_history(turns);

    let result = builder.build();
    assert!(result.is_ok());

    let request = result.unwrap();
    assert_eq!(request.model.as_deref(), Some("gemini-3-flash-preview"));
    assert!(matches!(request.input, crate::InteractionInput::Turns(_)));
}

#[test]
fn test_interaction_builder_with_single_turn() {
    use crate::Turn;

    let client = create_test_client();
    let turns = vec![Turn::user("Hello")];

    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_history(turns);

    let result = builder.build();
    assert!(result.is_ok());
}

// --- History + Current Message Composition Tests ---

#[test]
fn test_with_history_then_with_text_composes_correctly() {
    use crate::{InteractionInput, Role, Turn};

    let client = create_test_client();
    let history = vec![Turn::user("Hello"), Turn::model("Hi there!")];

    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_history(history)
        .with_text("How are you?");

    // Build should compose history + current_message
    let request = builder.build().expect("Build should succeed");

    // Verify the input is Turns with 3 items
    match &request.input {
        InteractionInput::Turns(turns) => {
            assert_eq!(turns.len(), 3, "Should have 3 turns");
            assert_eq!(*turns[0].role(), Role::User);
            assert_eq!(*turns[1].role(), Role::Model);
            assert_eq!(*turns[2].role(), Role::User);
        }
        _ => panic!("Expected Turns input"),
    }
}

#[test]
fn test_with_text_then_with_history_composes_correctly() {
    use crate::{InteractionInput, Role, Turn};

    let client = create_test_client();
    let history = vec![Turn::user("Hello"), Turn::model("Hi there!")];

    // Order reversed - should produce same result
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("How are you?")
        .with_history(history);

    let request = builder.build().expect("Build should succeed");

    // Verify the input is Turns with 3 items (history + current)
    match &request.input {
        InteractionInput::Turns(turns) => {
            assert_eq!(turns.len(), 3, "Should have 3 turns");
            assert_eq!(*turns[0].role(), Role::User);
            assert_eq!(*turns[1].role(), Role::Model);
            assert_eq!(*turns[2].role(), Role::User); // Current message appended
        }
        _ => panic!("Expected Turns input"),
    }
}

#[test]
fn test_history_and_text_order_independent() {
    use crate::Turn;

    let client = create_test_client();
    let history = vec![Turn::user("First"), Turn::model("Response")];

    // Build in one order
    let req1 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_history(history.clone())
        .with_text("Current")
        .build()
        .expect("Build should succeed");

    // Build in reverse order
    let req2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Current")
        .with_history(history)
        .build()
        .expect("Build should succeed");

    // Both should produce equivalent requests
    let json1 = serde_json::to_string(&req1.input).unwrap();
    let json2 = serde_json::to_string(&req2.input).unwrap();
    assert_eq!(json1, json2, "Order should not affect result");
}

#[test]
fn test_conversation_builder_then_with_text() {
    use crate::{InteractionInput, Role};

    let client = create_test_client();

    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .conversation()
        .user("What is 2+2?")
        .model("4")
        .done()
        .with_text("And times 3?");

    let request = builder.build().expect("Build should succeed");

    // Should have 3 turns: original 2 + appended current message
    match &request.input {
        InteractionInput::Turns(turns) => {
            assert_eq!(turns.len(), 3, "Should have 3 turns");
            assert_eq!(*turns[0].role(), Role::User);
            assert_eq!(*turns[1].role(), Role::Model);
            assert_eq!(*turns[2].role(), Role::User);
        }
        _ => panic!("Expected Turns input"),
    }
}

#[test]
fn test_with_text_only_produces_text_input() {
    use crate::InteractionInput;

    let client = create_test_client();

    let request = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello!")
        .build()
        .expect("Build should succeed");

    // Should produce Text input when no history
    assert!(
        matches!(request.input, InteractionInput::Text(_)),
        "Expected Text input, got {:?}",
        request.input
    );
}

#[test]
fn test_with_history_only_produces_turns_input() {
    use crate::{InteractionInput, Turn};

    let client = create_test_client();
    let history = vec![Turn::user("Hello"), Turn::model("Hi!")];

    let request = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_history(history)
        .build()
        .expect("Build should succeed");

    // Should produce Turns input
    match &request.input {
        InteractionInput::Turns(turns) => {
            assert_eq!(turns.len(), 2);
        }
        _ => panic!("Expected Turns input"),
    }
}

#[test]
fn test_typestate_chained_preserves_history_and_current_message() {
    use crate::Turn;

    let client = create_test_client();
    let history = vec![Turn::user("Hello"), Turn::model("Hi!")];

    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_history(history)
        .with_text("Current message")
        .with_previous_interaction("prev-123");

    // Fields should be preserved through the FirstTurn -> Chained transition
    assert_eq!(builder.history.len(), 2);
    assert_eq!(builder.current_message.as_deref(), Some("Current message"));
    assert_eq!(builder.previous_interaction_id.as_deref(), Some("prev-123"));
}

#[test]
fn test_typestate_store_disabled_preserves_history_and_current_message() {
    use crate::Turn;

    let client = create_test_client();
    let history = vec![Turn::user("Hello"), Turn::model("Hi!")];

    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_history(history)
        .with_text("Current message")
        .with_store_disabled();

    // Fields should be preserved through the FirstTurn -> StoreDisabled transition
    assert_eq!(builder.history.len(), 2);
    assert_eq!(builder.current_message.as_deref(), Some("Current message"));
    assert_eq!(builder.store, Some(false));
}

#[test]
fn test_with_content_cannot_combine_with_history() {
    use crate::{InteractionContent, Turn};

    let client = create_test_client();
    let history = vec![Turn::user("Hello"), Turn::model("Hi!")];
    let content = vec![InteractionContent::Text {
        text: Some("test".to_string()),
        annotations: None,
    }];

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_history(history)
        .set_content(content)
        .build();

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("set_content()"),
        "Error should mention set_content(): {}",
        err
    );
}

#[test]
fn test_with_content_and_text_merge() {
    use crate::{InteractionContent, InteractionInput};

    let client = create_test_client();
    let image_content = InteractionContent::Image {
        data: Some("dGVzdA==".to_string()),
        uri: None,
        mime_type: Some("image/png".to_string()),
        resolution: None,
    };

    // with_text() then with_content() - text should be prepended to content
    let request = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Describe this image")
        .set_content(vec![image_content.clone()])
        .build()
        .expect("Should succeed");

    match &request.input {
        InteractionInput::Content(items) => {
            assert_eq!(items.len(), 2, "Should have 2 items (text + image)");
            // Text should be first (prepended)
            assert!(
                matches!(&items[0], InteractionContent::Text { text: Some(t), .. } if t == "Describe this image"),
                "First item should be the text"
            );
            assert!(
                matches!(&items[1], InteractionContent::Image { .. }),
                "Second item should be the image"
            );
        }
        _ => panic!("Expected Content input"),
    }

    // with_content() then with_text() - should also work (order-independent)
    let request2 = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .set_content(vec![image_content])
        .with_text("Describe this image")
        .build()
        .expect("Should succeed");

    match &request2.input {
        InteractionInput::Content(items) => {
            assert_eq!(items.len(), 2, "Should have 2 items (text + image)");
            // Text should still be first (prepended at build time)
            assert!(
                matches!(&items[0], InteractionContent::Text { text: Some(t), .. } if t == "Describe this image"),
                "First item should be the text"
            );
        }
        _ => panic!("Expected Content input"),
    }
}

#[test]
fn test_with_content_alone_works() {
    use crate::{InteractionContent, InteractionInput};

    let client = create_test_client();
    let content = vec![InteractionContent::Text {
        text: Some("test".to_string()),
        annotations: None,
    }];

    let result = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .set_content(content)
        .build();

    assert!(result.is_ok());
    let request = result.unwrap();
    assert!(matches!(request.input, InteractionInput::Content(_)));
}

#[test]
fn test_conversation_builder_fluent_api() {
    use crate::Role;

    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .conversation()
        .user("What is 2+2?")
        .model("2+2 equals 4.")
        .user("And what's that times 3?")
        .done();

    // Verify the history has correct length and roles
    assert_eq!(builder.history.len(), 3);
    assert_eq!(*builder.history[0].role(), Role::User);
    assert_eq!(*builder.history[1].role(), Role::Model);
    assert_eq!(*builder.history[2].role(), Role::User);
}

#[test]
fn test_conversation_builder_with_parts_content() {
    use crate::{InteractionContent, TurnContent};

    let client = create_test_client();
    let parts = vec![InteractionContent::Text {
        text: Some("What is in this image?".to_string()),
        annotations: None,
    }];

    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .conversation()
        .user(TurnContent::Parts(parts))
        .done();

    // Verify the history has 1 turn with parts content
    assert_eq!(builder.history.len(), 1);
    assert!(builder.history[0].content().is_parts());
}

#[test]
fn test_conversation_builder_with_turn_method() {
    use crate::Role;

    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .conversation()
        .turn(Role::User, "Hello")
        .turn(Role::Model, "Hi!")
        .done();

    assert_eq!(builder.history.len(), 2);
}

#[test]
fn test_conversation_builder_empty() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .conversation()
        .done();

    // Empty conversation results in empty history
    assert!(builder.history.is_empty());
}

#[test]
fn test_conversation_builder_preserves_model() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .conversation()
        .user("Hello")
        .done();

    // Model should be preserved through conversation builder
    assert_eq!(builder.model.as_deref(), Some("gemini-3-flash-preview"));
}

#[test]
fn test_conversation_builder_preserves_system_instruction() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_system_instruction("Be helpful")
        .conversation()
        .user("Hello")
        .done();

    // System instruction should be preserved through conversation builder
    assert!(builder.system_instruction.is_some());
}

// --- File Search Builder Tests ---

#[test]
fn test_interaction_builder_with_file_search() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Search my documents")
        .with_file_search(vec![
            "stores/store-123".to_string(),
            "stores/store-456".to_string(),
        ]);

    assert!(builder.tools.is_some());
    let tools = builder.tools.as_ref().unwrap();
    assert_eq!(tools.len(), 1);

    match &tools[0] {
        Tool::FileSearch {
            store_names,
            top_k,
            metadata_filter,
        } => {
            assert_eq!(store_names, &vec!["stores/store-123", "stores/store-456"]);
            assert_eq!(*top_k, None);
            assert_eq!(*metadata_filter, None);
        }
        _ => panic!("Expected Tool::FileSearch variant"),
    }
}

#[test]
fn test_interaction_builder_with_file_search_config() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Search with config")
        .with_file_search_config(
            vec!["stores/my-docs".to_string()],
            Some(10),
            Some("category = 'technical'".to_string()),
        );

    assert!(builder.tools.is_some());
    let tools = builder.tools.as_ref().unwrap();
    assert_eq!(tools.len(), 1);

    match &tools[0] {
        Tool::FileSearch {
            store_names,
            top_k,
            metadata_filter,
        } => {
            assert_eq!(store_names, &vec!["stores/my-docs"]);
            assert_eq!(*top_k, Some(10));
            assert_eq!(*metadata_filter, Some("category = 'technical'".to_string()));
        }
        _ => panic!("Expected Tool::FileSearch variant"),
    }
}

#[test]
fn test_interaction_builder_with_file_search_and_other_tools() {
    let client = create_test_client();
    let func = FunctionDeclaration::builder("process_result")
        .description("Process search result")
        .build();

    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Search and process")
        .with_file_search(vec!["stores/docs".to_string()])
        .with_google_search()
        .add_function(func);

    assert!(builder.tools.is_some());
    let tools = builder.tools.as_ref().unwrap();
    assert_eq!(tools.len(), 3);

    // Verify FileSearch is present
    assert!(tools.iter().any(|t| matches!(t, Tool::FileSearch { .. })));
    // Verify GoogleSearch is present
    assert!(tools.iter().any(|t| matches!(t, Tool::GoogleSearch)));
    // Verify Function is present
    assert!(tools.iter().any(|t| matches!(t, Tool::Function { .. })));
}

#[test]
fn test_interaction_builder_with_file_search_single_store() {
    let client = create_test_client();
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Search single store")
        .with_file_search(vec!["stores/single".to_string()]);

    let tools = builder.tools.as_ref().unwrap();
    match &tools[0] {
        Tool::FileSearch { store_names, .. } => {
            assert_eq!(store_names.len(), 1);
            assert_eq!(store_names[0], "stores/single");
        }
        _ => panic!("Expected Tool::FileSearch variant"),
    }
}
