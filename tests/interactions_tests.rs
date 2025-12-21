use futures_util::StreamExt;
use rust_genai::{Client, CreateInteractionRequest, InteractionInput, InteractionStatus};
use std::env;

// Note: These integration tests make real API calls and may hit rate limits
// on free tier API keys. To run these tests:
// cargo test --test interactions_tests -- --ignored

#[tokio::test]
#[ignore = "Makes real API calls - may hit rate limits"]
async fn test_create_simple_interaction() {
    let Ok(api_key) = env::var("GEMINI_API_KEY") else {
        println!("Skipping test_create_simple_interaction: GEMINI_API_KEY not set.");
        return;
    };

    let client = Client::builder(api_key).build();

    let request = CreateInteractionRequest {
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: InteractionInput::Text("What is 2 + 2?".to_string()),
        previous_interaction_id: None,
        tools: None,
        response_modalities: None,
        response_format: None,
        generation_config: None,
        stream: None,
        background: None,
        store: Some(true),
        system_instruction: None,
    };

    let result = client.create_interaction(request).await;

    assert!(
        result.is_ok(),
        "create_interaction failed: {:?}",
        result.err()
    );

    let response = result.unwrap();

    // Verify basic response structure
    assert!(!response.id.is_empty(), "Interaction ID is empty");
    assert_eq!(
        response.status,
        InteractionStatus::Completed,
        "Expected status to be Completed"
    );
    assert!(!response.outputs.is_empty(), "Outputs are empty");

    // Verify output contains expected answer
    let has_four = response.outputs.iter().any(|output| match output {
        rust_genai::InteractionContent::Text { text } => {
            text.as_ref().map_or(false, |t| t.contains('4'))
        }
        _ => false,
    });

    assert!(has_four, "Response does not contain expected answer '4'");

    println!("Interaction ID: {}", response.id);
}

#[tokio::test]
#[ignore = "Makes real API calls - may hit rate limits"]
async fn test_stateful_interaction_with_previous_id() {
    let Ok(api_key) = env::var("GEMINI_API_KEY") else {
        println!("Skipping test_stateful_interaction_with_previous_id: GEMINI_API_KEY not set.");
        return;
    };

    let client = Client::builder(api_key).build();

    // First interaction
    let first_request = CreateInteractionRequest {
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: InteractionInput::Text("My favorite color is blue.".to_string()),
        previous_interaction_id: None,
        tools: None,
        response_modalities: None,
        response_format: None,
        generation_config: None,
        stream: None,
        background: None,
        store: Some(true),
        system_instruction: None,
    };

    let first_response = client.create_interaction(first_request).await.unwrap();
    let interaction_id = first_response.id.clone();

    assert_eq!(first_response.status, InteractionStatus::Completed);

    // Second interaction referencing the first
    let second_request = CreateInteractionRequest {
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: InteractionInput::Text("What is my favorite color?".to_string()),
        previous_interaction_id: Some(interaction_id.clone()),
        tools: None,
        response_modalities: None,
        response_format: None,
        generation_config: None,
        stream: None,
        background: None,
        store: Some(true),
        system_instruction: None,
    };

    let second_response = client.create_interaction(second_request).await;

    assert!(
        second_response.is_ok(),
        "Second interaction failed: {:?}",
        second_response.err()
    );

    let second_response = second_response.unwrap();

    assert_eq!(second_response.status, InteractionStatus::Completed);
    assert_eq!(
        second_response.previous_interaction_id,
        Some(interaction_id.clone())
    );

    // Verify the model remembers the color
    let mentions_blue = second_response.outputs.iter().any(|output| match output {
        rust_genai::InteractionContent::Text { text } => {
            text.as_ref()
                .map_or(false, |t| t.to_lowercase().contains("blue"))
        }
        _ => false,
    });

    assert!(
        mentions_blue,
        "Response does not mention the color 'blue' from previous interaction"
    );

    println!("First Interaction ID: {interaction_id}");
    println!("Second Interaction ID: {}", second_response.id);
}

#[tokio::test]
#[ignore = "Makes real API calls - may hit rate limits"]
async fn test_get_interaction() {
    let Ok(api_key) = env::var("GEMINI_API_KEY") else {
        println!("Skipping test_get_interaction: GEMINI_API_KEY not set.");
        return;
    };

    let client = Client::builder(api_key).build();

    // Create an interaction first
    let request = CreateInteractionRequest {
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: InteractionInput::Text("Hello, world!".to_string()),
        previous_interaction_id: None,
        tools: None,
        response_modalities: None,
        response_format: None,
        generation_config: None,
        stream: None,
        background: None,
        store: Some(true),
        system_instruction: None,
    };

    let create_response = client.create_interaction(request).await.unwrap();
    let interaction_id = create_response.id.clone();

    // Retrieve the interaction
    let get_result = client.get_interaction(&interaction_id).await;

    assert!(
        get_result.is_ok(),
        "get_interaction failed: {:?}",
        get_result.err()
    );

    let retrieved = get_result.unwrap();

    assert_eq!(retrieved.id, interaction_id);
    assert_eq!(retrieved.status, InteractionStatus::Completed);
    assert!(!retrieved.input.is_empty(), "Input is empty");
    assert!(!retrieved.outputs.is_empty(), "Outputs are empty");

    println!("Retrieved Interaction ID: {}", retrieved.id);
}

#[tokio::test]
#[ignore = "Makes real API calls - may hit rate limits"]
async fn test_delete_interaction() {
    let Ok(api_key) = env::var("GEMINI_API_KEY") else {
        println!("Skipping test_delete_interaction: GEMINI_API_KEY not set.");
        return;
    };

    let client = Client::builder(api_key).build();

    // Create an interaction first
    let request = CreateInteractionRequest {
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: InteractionInput::Text("Test interaction for deletion".to_string()),
        previous_interaction_id: None,
        tools: None,
        response_modalities: None,
        response_format: None,
        generation_config: None,
        stream: None,
        background: None,
        store: Some(true),
        system_instruction: None,
    };

    let create_response = client.create_interaction(request).await.unwrap();
    let interaction_id = create_response.id.clone();

    // Delete the interaction
    let delete_result = client.delete_interaction(&interaction_id).await;

    assert!(
        delete_result.is_ok(),
        "delete_interaction failed: {:?}",
        delete_result.err()
    );

    println!("Deleted Interaction ID: {interaction_id}");

    // Verify it's deleted by trying to retrieve it
    let get_after_delete = client.get_interaction(&interaction_id).await;

    // Should fail because it's deleted
    assert!(
        get_after_delete.is_err(),
        "Expected error when getting deleted interaction, but got Ok"
    );

    println!("Confirmed interaction was deleted");
}

#[tokio::test]
#[ignore = "Makes real API calls - may hit rate limits"]
async fn test_streaming_interaction() {
    let Ok(api_key) = env::var("GEMINI_API_KEY") else {
        println!("Skipping test_streaming_interaction: GEMINI_API_KEY not set.");
        return;
    };

    let client = Client::builder(api_key).build();

    let request = CreateInteractionRequest {
        model: Some("gemini-3-flash-preview".to_string()),
        agent: None,
        input: InteractionInput::Text("Count from 1 to 5.".to_string()),
        previous_interaction_id: None,
        tools: None,
        response_modalities: None,
        response_format: None,
        generation_config: None,
        stream: Some(true),
        background: None,
        store: Some(true),
        system_instruction: None,
    };

    let mut stream = client.create_interaction_stream(request);

    let mut chunk_count = 0;
    let mut final_interaction_id = None;

    while let Some(result) = stream.next().await {
        assert!(
            result.is_ok(),
            "Streaming chunk failed: {:?}",
            result.err()
        );

        let response = result.unwrap();
        chunk_count += 1;

        assert!(!response.id.is_empty(), "Interaction ID is empty in chunk");
        final_interaction_id = Some(response.id.clone());

        println!("Chunk {chunk_count}: Status={:?}", response.status);
    }

    assert!(
        chunk_count > 0,
        "Expected at least one chunk from streaming"
    );
    assert!(
        final_interaction_id.is_some(),
        "No interaction ID received"
    );

    println!("Total chunks received: {chunk_count}");
    println!("Final Interaction ID: {:?}", final_interaction_id.unwrap());
}
