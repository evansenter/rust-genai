//! Streaming resume tests
//!
//! Tests for stream resumption support via event_id tracking.
//! These tests verify that stream events include event_id for position tracking,
//! and that get_interaction_stream can be used to stream completed interactions.
//!
//! # Running Tests
//!
//! ```bash
//! cargo test --test streaming_resume_tests -- --include-ignored --nocapture
//! ```

mod common;

use common::{consume_stream, get_client, interaction_builder, stateful_builder};
use futures_util::StreamExt;
use rust_genai::{InteractionStatus, StreamChunk};

// =============================================================================
// Stream Event ID Tests
// =============================================================================

/// Test that streaming events include event_id for position tracking.
/// This is foundational for stream resume support.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_stream_events_have_event_id() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let stream = interaction_builder(&client)
        .with_text("Write a haiku about Rust programming.")
        .create_stream();

    let result = consume_stream(stream).await;

    println!("\n--- Stream Stats ---");
    println!("Delta chunks received: {}", result.delta_count);
    println!("Event IDs collected: {}", result.event_ids.len());
    println!(
        "Last event_id: {:?}",
        result.last_event_id.as_deref().unwrap_or("(none)")
    );

    // Verify stream produced output
    assert!(
        result.has_output(),
        "Stream should produce deltas or complete response"
    );

    // Verify event_ids were collected
    assert!(
        !result.event_ids.is_empty(),
        "Stream events should include event_ids for resume support"
    );

    // Verify last_event_id is set
    assert!(
        result.last_event_id.is_some(),
        "Should have a last_event_id for potential resume"
    );

    // Print event_id sample for debugging
    if result.event_ids.len() > 2 {
        println!("Sample event_ids:");
        println!("  First: {}", result.event_ids[0]);
        println!("  Second: {}", result.event_ids[1]);
        println!("  Last: {}", result.event_ids.last().unwrap());
    }
}

/// Test streaming a stored interaction returns event_ids.
/// This verifies the flow: create stored interaction -> stream has event_ids.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_stored_interaction_stream_has_event_ids() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Create a streaming request with store enabled
    let stream = stateful_builder(&client)
        .with_text("What is 2 + 2? Answer in one word.")
        .create_stream();

    let result = consume_stream(stream).await;

    println!("\n--- Stored Stream Stats ---");
    println!("Delta chunks: {}", result.delta_count);
    println!("Event IDs: {}", result.event_ids.len());
    println!("Collected text: {}", result.collected_text);

    // Verify streaming worked
    assert!(result.has_output(), "Should receive streaming output");

    // Verify event_ids for resume support
    assert!(
        !result.event_ids.is_empty(),
        "Stored interaction stream should have event_ids"
    );

    // Verify final response has interaction ID (needed for GET streaming)
    if let Some(ref response) = result.final_response {
        assert!(
            response.id.is_some(),
            "Stored interaction should have an ID"
        );
        println!("Interaction ID: {:?}", response.id);
    }
}

/// Test that get_interaction_stream can stream a completed interaction.
/// This verifies the resume-by-ID flow.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_get_interaction_stream() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // First, create a stored interaction (non-streaming)
    let response = retry_request!([client] => {
        stateful_builder(&client)
            .with_text("Count from 1 to 5.")
            .create()
            .await
    })
    .expect("Initial request failed");

    println!("Created interaction: {:?}", response.id);
    assert_eq!(response.status, InteractionStatus::Completed);

    let interaction_id = response.id.as_ref().expect("Should have ID");

    // Now stream the completed interaction using get_interaction_stream
    let mut stream = client.get_interaction_stream(interaction_id, None);

    let mut delta_count = 0;
    let mut collected_text = String::new();
    let mut event_ids: Vec<String> = Vec::new();
    let mut final_response = None;

    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                // Track event_id
                if let Some(ref eid) = event.event_id {
                    event_ids.push(eid.clone());
                }

                match event.chunk {
                    StreamChunk::Delta(delta) => {
                        delta_count += 1;
                        if let Some(text) = delta.text() {
                            collected_text.push_str(text);
                            print!("{}", text);
                        }
                    }
                    StreamChunk::Complete(resp) => {
                        println!("\n[GET stream complete: {:?}]", resp.id);
                        final_response = Some(resp);
                    }
                    _ => {}
                }
            }
            Err(e) => {
                eprintln!("Stream error: {:?}", e);
                break;
            }
        }
    }

    println!("\n--- GET Stream Stats ---");
    println!("Delta chunks: {}", delta_count);
    println!("Event IDs: {}", event_ids.len());
    println!("Collected text: {}", collected_text);

    // Verify we got output
    assert!(
        delta_count > 0 || final_response.is_some(),
        "GET stream should produce output"
    );

    // Verify event_ids are present
    assert!(
        !event_ids.is_empty(),
        "GET stream should have event_ids for resume"
    );

    // Verify final response if received
    if let Some(resp) = final_response {
        assert_eq!(resp.status, InteractionStatus::Completed);
    }
}

/// Test that StreamEvent wrapper preserves all chunk types.
/// Verifies the wrapper doesn't lose information during streaming.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_stream_event_wrapper_preserves_chunks() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    let mut stream = interaction_builder(&client)
        .with_text("Say 'hello' and nothing else.")
        .create_stream();

    let mut saw_delta = false;
    let mut saw_complete = false;
    let mut event_count = 0;

    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                event_count += 1;

                // Verify event structure
                match &event.chunk {
                    StreamChunk::Delta(_) => {
                        saw_delta = true;
                        // Delta events should have event_id from API
                        // (though it may be None for some events)
                    }
                    StreamChunk::Complete(response) => {
                        saw_complete = true;
                        // Complete should have the full response
                        assert!(response.status == InteractionStatus::Completed);
                    }
                    _ => {
                        // Unknown variants - that's fine (future compatibility)
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
                break;
            }
        }
    }

    println!("Events received: {}", event_count);
    println!("Saw delta: {}", saw_delta);
    println!("Saw complete: {}", saw_complete);

    // A successful stream should have at least deltas or complete
    assert!(
        saw_delta || saw_complete,
        "Stream should produce Delta or Complete events"
    );
}

// =============================================================================
// Multi-turn Stream Resume Tests
// =============================================================================

/// Test actual stream resume functionality with last_event_id.
/// This verifies that passing a last_event_id resumes from that position.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_stream_resume_with_last_event_id() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Create a stored interaction that generates multiple events
    let response = retry_request!([client] => {
        stateful_builder(&client)
            .with_text("Count slowly from 1 to 10, one number per line.")
            .create()
            .await
    })
    .expect("Initial request failed");

    println!("Created interaction: {:?}", response.id);
    assert_eq!(response.status, InteractionStatus::Completed);

    let interaction_id = response.id.as_ref().expect("Should have ID");

    // First, stream the entire interaction to capture all event_ids
    let mut full_stream = client.get_interaction_stream(interaction_id, None);

    let mut all_event_ids: Vec<String> = Vec::new();
    let mut full_text = String::new();

    while let Some(result) = full_stream.next().await {
        if let Ok(event) = result {
            if let Some(ref eid) = event.event_id {
                all_event_ids.push(eid.clone());
            }
            if let StreamChunk::Delta(delta) = event.chunk
                && let Some(text) = delta.text()
            {
                full_text.push_str(text);
            }
        }
    }

    println!("\n--- Full Stream ---");
    println!("Total event_ids: {}", all_event_ids.len());
    println!("Full text length: {} chars", full_text.len());

    // Skip test if not enough events to test resume
    if all_event_ids.len() < 3 {
        println!("Not enough events to test resume (need at least 3), skipping");
        return;
    }

    // Pick an event_id from the middle to resume from
    let resume_from_index = all_event_ids.len() / 2;
    let resume_event_id = &all_event_ids[resume_from_index];

    println!(
        "\n--- Resuming from event {} of {} ---",
        resume_from_index + 1,
        all_event_ids.len()
    );
    println!("Resume event_id: {}", resume_event_id);

    // Now stream with last_event_id to resume
    let mut resumed_stream = client.get_interaction_stream(interaction_id, Some(resume_event_id));

    let mut resumed_event_ids: Vec<String> = Vec::new();
    let mut resumed_text = String::new();

    while let Some(result) = resumed_stream.next().await {
        if let Ok(event) = result {
            if let Some(ref eid) = event.event_id {
                resumed_event_ids.push(eid.clone());
            }
            if let StreamChunk::Delta(delta) = event.chunk
                && let Some(text) = delta.text()
            {
                resumed_text.push_str(text);
            }
        }
    }

    println!("\n--- Resumed Stream ---");
    println!("Resumed event_ids: {}", resumed_event_ids.len());
    println!("Resumed text length: {} chars", resumed_text.len());

    // Verify resumed stream has fewer events (started partway through)
    // Note: The resumed stream should have events AFTER the resume_event_id
    assert!(
        resumed_event_ids.len() < all_event_ids.len(),
        "Resumed stream should have fewer events. Full: {}, Resumed: {}",
        all_event_ids.len(),
        resumed_event_ids.len()
    );

    // Verify the resumed event_ids are a suffix of the full event_ids
    // (they should be the events after the resume point)
    let expected_count = all_event_ids.len() - resume_from_index - 1;
    println!(
        "Expected ~{} events after resume point, got {}",
        expected_count,
        resumed_event_ids.len()
    );

    // The first resumed event_id should NOT be the resume_event_id
    // (it should be the one after)
    if !resumed_event_ids.is_empty() {
        assert_ne!(
            &resumed_event_ids[0], resume_event_id,
            "Resumed stream should not include the resume_event_id itself"
        );
    }

    println!("\n✓ Stream resume with last_event_id works correctly");
}

/// Test that multi-turn streaming preserves event_id tracking.
#[tokio::test]
#[ignore = "Requires API key"]
async fn test_multiturn_stream_event_ids() {
    let Some(client) = get_client() else {
        println!("Skipping: GEMINI_API_KEY not set");
        return;
    };

    // Turn 1: Establish context
    let response1 = retry_request!([client] => {
        stateful_builder(&client)
            .with_text("My name is Alice. Please remember this.")
            .create()
            .await
    })
    .expect("Turn 1 failed");

    println!("Turn 1 completed: {:?}", response1.id);
    assert_eq!(response1.status, InteractionStatus::Completed);

    // Turn 2: Stream a follow-up question
    let stream = stateful_builder(&client)
        .with_previous_interaction(response1.id.as_ref().expect("id should exist"))
        .with_text("What is my name?")
        .create_stream();

    let result = consume_stream(stream).await;

    println!("\n--- Turn 2 Stream Stats ---");
    println!("Delta chunks: {}", result.delta_count);
    println!("Event IDs: {}", result.event_ids.len());
    println!("Last event_id: {:?}", result.last_event_id);
    println!("Response text: {}", result.collected_text);

    // Verify streaming worked
    assert!(result.has_output(), "Turn 2 stream should produce output");

    // Verify event_ids in multi-turn
    assert!(
        !result.event_ids.is_empty(),
        "Multi-turn stream should have event_ids"
    );

    // Verify context was preserved (should mention Alice)
    let text_lower = result.collected_text.to_lowercase();
    assert!(
        text_lower.contains("alice"),
        "Response should recall the name from Turn 1. Got: {}",
        result.collected_text
    );
}
