use genai_rs_macros::tool;

// This should fail because "invalid_key" is not a recognized attribute.
// Valid attributes are: description, enum_values
#[tool(name(invalid_key = "value"))]
fn test_invalid_attr(name: String) -> String {
    name
}

fn main() {}
