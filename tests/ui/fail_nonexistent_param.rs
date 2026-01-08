use genai_rs_macros::tool;

// This should fail because "nonexistent" is not a parameter of the function.
#[tool(nonexistent(description = "This param does not exist"))]
fn test_nonexistent_param(name: String) -> String {
    name
}

fn main() {}
