//! OpenAPI schema generation for function parameters.
//!
//! Converts Rust types to OpenAPI/JSON Schema types for use in
//! AI function calling declarations.

use crate::parsing::ParamConfig;
use quote::ToTokens;
use syn::Type;
use utoipa::openapi::{
    RefOr,
    schema::{ArrayBuilder, ObjectBuilder, Schema, Type as OpenApiType},
};

/// Analyzes a type to determine if it's an `Option<T>` wrapper.
///
/// Returns a tuple of:
/// - `bool`: `true` if the type is `Option<T>`, `false` otherwise
/// - `Type`: The inner type (unwrapped if `Option`, original otherwise)
pub fn get_type_info(ty: &Type) -> (bool, Type) {
    if let Type::Path(type_path) = ty
        && type_path.path.segments.len() == 1
        && type_path.path.segments[0].ident == "Option"
        && let syn::PathArguments::AngleBracketed(args) = &type_path.path.segments[0].arguments
        && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
    {
        return (true, inner_ty.clone());
    }
    (false, ty.clone())
}

/// Maps a Rust type to its corresponding OpenAPI type.
///
/// Supported mappings:
/// - `String` -> `String`
/// - `i32`, `i64`, `isize`, `u32`, `u64`, `usize` -> `Integer`
/// - `f32`, `f64` -> `Number`
/// - `bool` -> `Boolean`
/// - `Vec<T>` -> `Array`
/// - `serde_json::Value` -> `Object`
/// - Other types default to `Object`
fn map_rust_type_to_openapi_type(rust_type_path: &Type) -> OpenApiType {
    let type_str = rust_type_path
        .to_token_stream()
        .to_string()
        .replace(' ', "");
    match type_str.as_str() {
        "String" => OpenApiType::String,
        "i32" | "i64" | "isize" | "u32" | "u64" | "usize" => OpenApiType::Integer,
        "f32" | "f64" => OpenApiType::Number,
        "bool" => OpenApiType::Boolean,
        s if s.starts_with("Vec<") && s.ends_with('>') => OpenApiType::Array,
        s if s == "Value" || s == "serde_json::Value" => OpenApiType::Object,
        _ => OpenApiType::Object,
    }
}

/// Builds an OpenAPI schema for a function parameter.
///
/// Uses the Rust type to determine the schema type, and applies any
/// configuration (description, enum_values) from the macro attribute.
pub fn build_param_schema(pat_type: &syn::PatType, config: Option<&ParamConfig>) -> RefOr<Schema> {
    let (_, inner_type) = get_type_info(&pat_type.ty);
    let api_type = map_rust_type_to_openapi_type(&inner_type);

    if api_type == OpenApiType::Array {
        let mut array_builder = ArrayBuilder::new();

        if let Type::Path(type_path) = &inner_type
            && let Some(segment) = type_path.path.segments.first()
            && segment.ident == "Vec"
            && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(syn::GenericArgument::Type(inner_ty_of_vec)) = args.args.first()
        {
            let inner_api_type = map_rust_type_to_openapi_type(inner_ty_of_vec);
            let items_schema = ObjectBuilder::new().schema_type(inner_api_type).build();
            array_builder = array_builder.items(RefOr::T(Schema::Object(items_schema)));
        }

        RefOr::T(Schema::Array(array_builder.build()))
    } else {
        let mut individual_schema_builder = ObjectBuilder::new().schema_type(api_type);

        if let Some(config) = config {
            if let Some(desc_val) = &config.description
                && !desc_val.is_empty()
            {
                individual_schema_builder =
                    individual_schema_builder.description(Some(desc_val.clone()));
            }
            if let Some(enums) = &config.enum_values {
                individual_schema_builder =
                    individual_schema_builder.enum_values(Some(enums.clone()));
            }
        }

        RefOr::T(Schema::Object(individual_schema_builder.build()))
    }
}
