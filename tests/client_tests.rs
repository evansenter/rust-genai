use rust_genai::{ApiVersion, Client};

#[test]
fn test_client_builder() {
    let api_key = "test-key".to_string();

    // Test default builder
    let _client = Client::builder(api_key.clone()).build();
    // We can't access private fields directly, but we can test the behavior

    // Test builder with debug mode
    let _debug_client = Client::builder(api_key.clone()).debug().build();
    // Debug mode will be tested through its effects when we add debug output tests

    // Test builder with API version
    let _v1_client = Client::builder(api_key.clone())
        .api_version(ApiVersion::V1Beta)
        .build();

    // Test builder with all options
    let _full_client = Client::builder(api_key)
        .api_version(ApiVersion::V1Alpha)
        .debug()
        .build();
}

#[test]
fn test_client_new() {
    let api_key = "test-key".to_string();

    // Test with default API version
    let _client = Client::new(api_key.clone(), None);

    // Test with specific API version
    let _client = Client::new(api_key.clone(), Some(ApiVersion::V1Beta));

    // Test with each API version variant
    let _client = Client::new(api_key.clone(), Some(ApiVersion::V1Alpha));
    let _client = Client::new(api_key, Some(ApiVersion::V1Beta));
}

#[test]
fn test_with_model() {
    let api_key = "test-key".to_string();
    let client = Client::builder(api_key).build();

    // Test creating a builder with various model names
    let _builder = client.with_model("gemini-pro");
    let _builder = client.with_model("gemini-1.5-flash");
    let _builder = client.with_model("gemini-3-flash-preview");
}
