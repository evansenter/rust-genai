#![allow(dead_code)] // Functions are used by the macro, not called directly
#![allow(clippy::needless_pass_by_value)] // Macro needs owned types for schema generation
#![allow(clippy::uninlined_format_args)] // Not important in tests
#![allow(clippy::items_after_statements)] // Test functions can be defined anywhere

use rust_genai::CallableFunction;
use rust_genai_macros::generate_function_declaration;

// This test file demonstrates what works and what doesn't for parameter documentation

#[test]
fn verify_parameter_descriptions_work() {
    // This test verifies that parameter descriptions can be added via the macro attribute
    // Parameter doc comments don't work in Rust, so this is the only way

    #[generate_function_declaration(
        name(description = "The person's name"),
        age(description = "The person's age in years")
    )]
    fn works_with_attribute(name: String, age: u32) -> String {
        format!("{} is {} years old", name, age)
    }

    let callable = WorksWithAttributeCallable;
    let declaration = callable.declaration();

    // Verify the parameters contain descriptions
    let schema_properties = declaration
        .parameters()
        .properties()
        .get("properties")
        .expect("Should have properties field");

    let name_desc = schema_properties
        .get("name")
        .and_then(|p| p.get("description"))
        .and_then(|d| d.as_str())
        .expect("name should have description");
    assert_eq!(name_desc, "The person's name");

    let age_desc = schema_properties
        .get("age")
        .and_then(|p| p.get("description"))
        .and_then(|d| d.as_str())
        .expect("age should have description");
    assert_eq!(age_desc, "The person's age in years");
}

#[test]
fn verify_function_documentation_preserved() {
    /// This function calculates the area of a rectangle
    #[generate_function_declaration]
    fn with_function_docs(width: f64, height: f64) -> f64 {
        width * height
    }

    let callable = WithFunctionDocsCallable;
    let declaration = callable.declaration();
    assert_eq!(
        declaration.description(),
        "This function calculates the area of a rectangle"
    );
}

#[test]
fn verify_comment_about_param_docs_is_accurate() {
    // This test confirms that regular comments on parameters are ignored
    #[generate_function_declaration]
    fn regular_comments(
        // This comment is ignored
        x: i32,
        /* This too */ y: i32,
    ) -> i32 {
        x + y
    }

    let callable = RegularCommentsCallable;
    let declaration = callable.declaration();
    let schema_properties = declaration
        .parameters()
        .properties()
        .get("properties")
        .expect("Should have properties field");

    // Neither parameter should have a description
    let x_param = schema_properties.get("x").expect("Should have x parameter");
    assert!(x_param.get("description").is_none());

    let y_param = schema_properties.get("y").expect("Should have y parameter");
    assert!(y_param.get("description").is_none());
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
