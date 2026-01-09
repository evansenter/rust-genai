//! Compile-fail test: Chained state cannot use with_system_instruction()
//!
//! The typestate pattern ensures that when an interaction is chained via
//! previous_interaction_id, system instructions cannot be set because
//! they are inherited from the first turn.

use genai_rs::Client;

fn main() {
    let client = Client::builder("test-key".to_string()).build().unwrap();

    // This should fail to compile: Chained cannot use with_system_instruction()
    // because system instructions are only available on FirstTurn
    let _builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello")
        .with_previous_interaction("prev-id") // Transitions to Chained state
        .with_system_instruction("Be helpful"); // ERROR: method not found
}
