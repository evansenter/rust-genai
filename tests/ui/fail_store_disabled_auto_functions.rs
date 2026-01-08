//! Compile-fail test: StoreDisabled cannot use create_with_auto_functions()
//!
//! The typestate pattern ensures that when store is disabled (stateless mode),
//! the automatic function calling loop is not available because it requires
//! server-side state to chain interactions.

use genai_rs::Client;

fn main() {
    let client = Client::builder("test-key".to_string()).build().unwrap();

    // This should fail to compile: StoreDisabled cannot use create_with_auto_functions()
    // because CanAutoFunction is not implemented for StoreDisabled
    let _builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("Hello")
        .with_store_disabled()
        .create_with_auto_functions(); // ERROR: method not found
}
