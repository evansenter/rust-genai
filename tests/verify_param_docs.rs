#![allow(dead_code)] // Functions are used by the macro, not called directly
#![allow(clippy::needless_pass_by_value)] // Macro needs owned types for schema generation
#![allow(clippy::uninlined_format_args)] // Not important in tests
#![allow(clippy::items_after_statements)] // Test functions can be defined anywhere

use rust_genai_macros::generate_function_declaration;

// This test file demonstrates what works and what doesn't for parameter documentation

#[test]
fn verify_parameter_documentation_behavior() {
    // This WORKS - descriptions via macro attribute
    #[generate_function_declaration(
        name(description = "The person's name"),
        age(description = "The person's age in years")
    )]
    fn works_with_attribute(name: String, age: i32) -> String {
        format!("{} is {} years old", name, age)
    }
    
    let decl = works_with_attribute_declaration();
    let params_json = serde_json::to_string(&decl.parameters).unwrap();
    
    // Verify descriptions are included
    assert!(params_json.contains("The person's name"));
    assert!(params_json.contains("The person's age in years"));
    
    // This also WORKS - function doc comments are captured
    /// This function greets someone
    #[generate_function_declaration]
    fn with_function_docs(name: String) -> String {
        format!("Hello, {}", name)
    }
    
    let decl2 = with_function_docs_declaration();
    assert_eq!(decl2.description, "This function greets someone");
    
    // Regular comments on parameters are ignored (not doc comments)
    #[generate_function_declaration]
    fn regular_comments(
        // This is just a regular comment - NOT captured
        name: String,
        /* This block comment is also NOT captured */ age: i32
    ) -> String {
        format!("{} is {} years old", name, age)
    }
    
    let decl3 = regular_comments_declaration();
    let params_json3 = serde_json::to_string(&decl3.parameters).unwrap();
    
    // Verify no descriptions are captured from regular comments
    assert!(!params_json3.contains("regular comment"));
    assert!(!params_json3.contains("block comment"));
}

// This test would fail to compile if uncommented, proving Rust doesn't allow doc comments on parameters
/*
#[test]
fn this_would_not_compile() {
    #[generate_function_declaration]
    fn bad_syntax(
        /// This doc comment causes: error: documentation comments cannot be applied to function parameters
        name: String
    ) -> String {
        name
    }
}
*/