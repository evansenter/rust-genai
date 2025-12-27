use rust_genai_macros::tool;

#[tool(
    unit(enum_values = [1, 2, 3])
)]
fn test_invalid_enum(unit: String) -> String {
    unit
}

fn main() {}
