// Integration tests for WithFunctionCalling trait implementation
// Verifies that both GenerateContentBuilder and InteractionBuilder
// behave identically when using trait methods

use rust_genai::{Client, FunctionDeclaration, WithFunctionCalling};
use serde_json::json;

fn create_test_client() -> Client {
    Client::new("test-api-key".to_string(), None)
}

fn create_test_function() -> FunctionDeclaration {
    FunctionDeclaration::builder("test_func")
        .description("Test function")
        .parameter("location", json!({"type": "string"}))
        .required(vec!["location".to_string()])
        .build()
}

#[test]
fn test_with_function_on_generate_content_builder() {
    let client = create_test_client();
    let func = create_test_function();

    let builder = client
        .with_model("gemini-3-flash-preview")
        .with_prompt("test")
        .with_function(func);

    // Builder should compile and accept the function
    // Internal state verification would require exposing private fields,
    // so we rely on type system correctness here
    drop(builder);
}

#[test]
fn test_with_function_on_interaction_builder() {
    let client = create_test_client();
    let func = create_test_function();

    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("test")
        .with_function(func);

    // Builder should compile and accept the function
    drop(builder);
}

#[test]
fn test_with_functions_on_generate_content_builder() {
    let client = create_test_client();

    let func1 = FunctionDeclaration::builder("func1")
        .description("First function")
        .build();

    let func2 = FunctionDeclaration::builder("func2")
        .description("Second function")
        .build();

    let builder = client
        .with_model("gemini-3-flash-preview")
        .with_prompt("test")
        .with_functions(vec![func1, func2]);

    drop(builder);
}

#[test]
fn test_with_functions_on_interaction_builder() {
    let client = create_test_client();

    let func1 = FunctionDeclaration::builder("func1")
        .description("First function")
        .build();

    let func2 = FunctionDeclaration::builder("func2")
        .description("Second function")
        .build();

    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("test")
        .with_functions(vec![func1, func2]);

    drop(builder);
}

#[test]
fn test_with_functions_empty_vec_generate_content() {
    let client = create_test_client();

    // Empty vec should be valid (though functionally useless)
    let builder = client
        .with_model("gemini-3-flash-preview")
        .with_prompt("test")
        .with_functions(vec![]);

    drop(builder);
}

#[test]
fn test_with_functions_empty_vec_interaction() {
    let client = create_test_client();

    // Empty vec should be valid (though functionally useless)
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("test")
        .with_functions(vec![]);

    drop(builder);
}

#[test]
fn test_chaining_with_function_calls_generate_content() {
    let client = create_test_client();

    let func1 = FunctionDeclaration::builder("func1")
        .description("First")
        .build();

    let func2 = FunctionDeclaration::builder("func2")
        .description("Second")
        .build();

    // Test that we can chain multiple with_function calls
    let builder = client
        .with_model("gemini-3-flash-preview")
        .with_prompt("test")
        .with_function(func1)
        .with_function(func2);

    drop(builder);
}

#[test]
fn test_chaining_with_function_calls_interaction() {
    let client = create_test_client();

    let func1 = FunctionDeclaration::builder("func1")
        .description("First")
        .build();

    let func2 = FunctionDeclaration::builder("func2")
        .description("Second")
        .build();

    // Test that we can chain multiple with_function calls
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("test")
        .with_function(func1)
        .with_function(func2);

    drop(builder);
}

#[test]
fn test_mixing_with_function_and_with_functions_generate_content() {
    let client = create_test_client();

    let func1 = FunctionDeclaration::builder("func1").build();
    let func2 = FunctionDeclaration::builder("func2").build();
    let func3 = FunctionDeclaration::builder("func3").build();

    // Mix single and batch function addition
    let builder = client
        .with_model("gemini-3-flash-preview")
        .with_prompt("test")
        .with_function(func1)
        .with_functions(vec![func2, func3]);

    drop(builder);
}

#[test]
fn test_mixing_with_function_and_with_functions_interaction() {
    let client = create_test_client();

    let func1 = FunctionDeclaration::builder("func1").build();
    let func2 = FunctionDeclaration::builder("func2").build();
    let func3 = FunctionDeclaration::builder("func3").build();

    // Mix single and batch function addition
    let builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("test")
        .with_function(func1)
        .with_functions(vec![func2, func3]);

    drop(builder);
}

#[test]
fn test_with_function_using_builder_pattern() {
    let client = create_test_client();

    // Test that builder-created functions work with trait method
    let func = FunctionDeclaration::builder("test")
        .description("Test function")
        .parameter("param1", json!({"type": "string"}))
        .required(vec!["param1".to_string()])
        .build();

    let builder = client
        .with_model("gemini-3-flash-preview")
        .with_prompt("test")
        .with_function(func);

    drop(builder);
}

#[test]
fn test_function_with_complex_parameters() {
    let client = create_test_client();

    let func = FunctionDeclaration::builder("complex")
        .description("Complex function")
        .parameter(
            "nested",
            json!({
                "type": "object",
                "properties": {
                    "inner": {
                        "type": "array",
                        "items": {"type": "string"}
                    }
                }
            }),
        )
        .required(vec!["nested".to_string()])
        .build();

    // Test on both builders
    let _gc_builder = client
        .with_model("gemini-3-flash-preview")
        .with_prompt("test")
        .with_function(func.clone());

    let _int_builder = client
        .interaction()
        .with_model("gemini-3-flash-preview")
        .with_text("test")
        .with_function(func);
}
