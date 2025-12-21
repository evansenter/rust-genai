use rust_genai::{Client, CreateInteractionRequest, InteractionInput};
use std::env;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Get API Key from environment variable
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not found in environment");

    // Create the client with debug mode to see the requests
    let client = Client::builder(api_key).debug().build();

    let model_name = "gemini-3-flash-preview";

    // === First interaction ===
    println!("=== FIRST INTERACTION ===\n");

    let first_prompt = "My name is Alice and I like programming in Rust.";
    println!("User: {first_prompt}\n");

    let first_request = CreateInteractionRequest {
        model: Some(model_name.to_string()),
        agent: None,
        input: InteractionInput::Text(first_prompt.to_string()),
        previous_interaction_id: None,
        tools: None,
        response_modalities: None,
        response_format: None,
        generation_config: None,
        stream: None,
        background: None,
        store: Some(true), // Important: Store for stateful conversation
        system_instruction: None,
    };

    let first_response = client.create_interaction(first_request).await?;
    let interaction_id = first_response.id.clone();

    println!("Interaction ID: {interaction_id}");
    println!("Status: {:?}\n", first_response.status);

    if !first_response.outputs.is_empty() {
        println!("Assistant:");
        for output in &first_response.outputs {
            for part in &output.parts {
                if let Some(text) = &part.text {
                    println!("{text}");
                }
            }
        }
    }

    // === Second interaction - referencing the first ===
    println!("\n=== SECOND INTERACTION (Stateful) ===\n");

    let second_prompt = "What is my name and what language do I like?";
    println!("User: {second_prompt}\n");

    let second_request = CreateInteractionRequest {
        model: Some(model_name.to_string()),
        agent: None,
        input: InteractionInput::Text(second_prompt.to_string()),
        previous_interaction_id: Some(interaction_id.clone()), // Reference first interaction
        tools: None,
        response_modalities: None,
        response_format: None,
        generation_config: None,
        stream: None,
        background: None,
        store: Some(true),
        system_instruction: None,
    };

    let second_response = client.create_interaction(second_request).await?;

    println!("Interaction ID: {}", second_response.id);
    println!("Previous Interaction ID: {}", interaction_id);
    println!("Status: {:?}\n", second_response.status);

    if !second_response.outputs.is_empty() {
        println!("Assistant:");
        for output in &second_response.outputs {
            for part in &output.parts {
                if let Some(text) = &part.text {
                    println!("{text}");
                }
            }
        }
    }

    // === Third interaction - continuation ===
    println!("\n=== THIRD INTERACTION (Continued) ===\n");

    let third_prompt = "Why is that language interesting?";
    println!("User: {third_prompt}\n");

    let third_request = CreateInteractionRequest {
        model: Some(model_name.to_string()),
        agent: None,
        input: InteractionInput::Text(third_prompt.to_string()),
        previous_interaction_id: Some(second_response.id.clone()), // Reference second interaction
        tools: None,
        response_modalities: None,
        response_format: None,
        generation_config: None,
        stream: None,
        background: None,
        store: Some(true),
        system_instruction: None,
    };

    let third_response = client.create_interaction(third_request).await?;

    println!("Interaction ID: {}", third_response.id);
    println!("Previous Interaction ID: {}", second_response.id);
    println!("Status: {:?}\n", third_response.status);

    if !third_response.outputs.is_empty() {
        println!("Assistant:");
        for output in &third_response.outputs {
            for part in &output.parts {
                if let Some(text) = &part.text {
                    println!("{text}");
                }
            }
        }
    }

    // === Demonstrate retrieval ===
    println!("\n=== RETRIEVING FIRST INTERACTION ===\n");

    match client.get_interaction(&interaction_id).await {
        Ok(retrieved) => {
            println!("Retrieved Interaction ID: {}", retrieved.id);
            println!("Status: {:?}", retrieved.status);
            println!("Input parts: {}", retrieved.input.len());
            println!("Output parts: {}", retrieved.outputs.len());
        }
        Err(e) => {
            eprintln!("Error retrieving interaction: {e}");
        }
    }

    // === Cleanup (optional) ===
    println!("\n=== CLEANUP ===\n");

    // Uncomment to actually delete:
    // client.delete_interaction(&interaction_id).await?;
    // client.delete_interaction(&second_response.id).await?;
    // client.delete_interaction(&third_response.id).await?;
    println!("Interactions can be deleted using client.delete_interaction(id)");
    println!("(Skipped in this example - they will auto-expire based on your API tier)");

    Ok(())
}
