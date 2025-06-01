use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, ItemFn, Pat, Type, Attribute, ExprLit, Meta, Lit, Expr, Ident, LitStr, ExprArray, Token, parenthesized, punctuated::Punctuated};
use syn::parse::{Parse, ParseStream, Result as ParseResult};
use utoipa::openapi::{RefOr};
use utoipa::openapi::schema::{ArrayBuilder, ObjectBuilder, Schema, Type as OpenApiType};
use std::collections::HashMap;

#[derive(Default, Debug, Clone)]
struct ParamSchemaDetails {
    description: Option<String>,
    enum_values: Option<Vec<serde_json::Value>>,
}

// Parses individual parameter config like: name(key="value", ...)
#[derive(Debug)]
struct SingleParamConfigInput {
    name: Ident,
    description: Option<LitStr>,
    enum_values: Option<ExprArray>,
}

impl Parse for SingleParamConfigInput {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let name: Ident = input.parse()?;
        let content;
        parenthesized!(content in input);

        let mut description: Option<LitStr> = None;
        let mut enum_values: Option<ExprArray> = None;

        let metas: Punctuated<Meta, Token![,]> = content.parse_terminated(Meta::parse, Token![,])?;

        for meta in metas {
            if let Meta::NameValue(nv) = meta {
                if nv.path.is_ident("description") {
                    if let Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) = nv.value {
                        description = Some(s);
                    } else {
                        return Err(syn::Error::new_spanned(nv.value, "Expected string literal for description"));
                    }
                } else if nv.path.is_ident("enum_values") {
                    if let Expr::Array(arr) = nv.value {
                        enum_values = Some(arr);
                    } else {
                        return Err(syn::Error::new_spanned(nv.value, "Expected array for enum_values"));
                    }
                }
            }
        }

        Ok(Self {
            name,
            description,
            enum_values,
        })
    }
}

// Parses the whole attribute: param1(...), param2(...)
#[derive(Debug)]
struct AllParamsConfigInput {
    configs: Punctuated<SingleParamConfigInput, Token![,]>
}

impl Parse for AllParamsConfigInput {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        if input.is_empty() {
            Ok(Self { configs: Punctuated::new() })
        } else {
            let configs = input.parse_terminated(SingleParamConfigInput::parse, Token![,])?;
            Ok(Self { configs })
        }
    }
}

fn extract_doc_comments(attrs: &[Attribute]) -> String {
    attrs.iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                if let Meta::NameValue(mnv) = &attr.meta {
                    if let Expr::Lit(ExprLit { lit: Lit::Str(lit_str), .. }) = &mnv.value {
                        return Some(lit_str.value().trim().to_string());
                    }
                }
            }
            None
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn get_type_info(ty: &Type) -> (bool, Type) {
    if let Type::Path(type_path) = ty {
        if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Option" {
            if let syn::PathArguments::AngleBracketed(args) = &type_path.path.segments[0].arguments {
                if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                    return (true, inner_ty.clone());
                }
            }
        }
    }
    (false, ty.clone())
}

fn map_rust_type_to_openapi_type(rust_type_path: &Type) -> OpenApiType {
    let type_str = rust_type_path.to_token_stream().to_string().replace(' ', "");
    match type_str.as_str() {
        "String" => OpenApiType::String,
        "i32" | "i64" | "isize" | "u32" | "u64" | "usize" => OpenApiType::Integer,
        "f32" | "f64" => OpenApiType::Number,
        "bool" => OpenApiType::Boolean,
        s if s.starts_with("Vec<") && s.ends_with('>') => {
            // For Vec types, we return Array type
            // The items schema will be handled separately in build_param_schema
            OpenApiType::Array
        }
        s if s == "Value" || s == "serde_json::Value" => OpenApiType::Object,
        _ => OpenApiType::Object,
    }
}

/// Generates a function that returns a `FunctionDeclaration` for the annotated function.
/// 
/// # Example
/// ```ignore
/// use rust_genai_macros::generate_function_declaration;
/// 
/// #[generate_function_declaration(
///     location(description = "The city and state"),
///     unit(enum_values = ["celsius", "fahrenheit"])
/// )]
/// fn get_weather(location: String, unit: Option<String>) -> String {
///     format!("Weather for {}", location)
/// }
/// 
/// // The macro generates:
/// // pub fn get_weather_declaration() -> rust_genai::FunctionDeclaration { ... }
/// ```
#[proc_macro_attribute]
pub fn generate_function_declaration(attr_input: TokenStream, item: TokenStream) -> TokenStream {
    let all_params_config = parse_macro_input!(attr_input as AllParamsConfigInput);
    let func = parse_macro_input!(item as ItemFn);
    let func_name = func.sig.ident.to_string();

    let mut config_map: HashMap<String, SingleParamConfigInput> = HashMap::new();
    for config in all_params_config.configs {
        config_map.insert(config.name.to_string(), config);
    }

    let func_description = extract_doc_comments(&func.attrs);
    let mut object_builder = ObjectBuilder::new();
    let mut required_params_for_struct_field = Vec::new();

    for fn_arg in &func.sig.inputs {
        if let syn::FnArg::Typed(pat_type) = fn_arg {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                let param_name = pat_ident.ident.to_string();
                
                let attr_details = match process_param_config(&param_name, config_map.get(&param_name)) {
                    Ok(details) => details,
                    Err(e) => return e.to_compile_error().into(),
                };

                let param_schema = build_param_schema(pat_type, attr_details);
                object_builder = object_builder.property(param_name.clone(), param_schema);

                let (is_option, _) = get_type_info(&pat_type.ty);
                if !is_option {
                    required_params_for_struct_field.push(param_name.clone());
                }
            }
        }
    }
    
    let parameters_schema_obj = object_builder.build();
    let parameters_schema_ref_or: RefOr<Schema> = RefOr::T(Schema::Object(parameters_schema_obj));

    let parameters_json_string = match serde_json::to_string_pretty(&parameters_schema_ref_or) {
        Ok(s) => s,
        Err(e) => {
            return syn::Error::new_spanned(func.sig.ident, format!("Failed to serialize schema to JSON: {e}"))
                .to_compile_error()
                .into();
        }
    };
    
    generate_declaration_function(&func, &func_name, &func_description, &parameters_json_string, &required_params_for_struct_field)
}

fn process_param_config(
    param_name: &str, 
    config: Option<&SingleParamConfigInput>
) -> Result<ParamSchemaDetails, syn::Error> {
    let mut attr_details = ParamSchemaDetails::default();

    if let Some(config) = config {
        if let Some(desc_lit) = &config.description {
            attr_details.description = Some(desc_lit.value());
        }
        if let Some(enums_array) = &config.enum_values {
            let mut enums = Vec::new();
            for elem_expr in &enums_array.elems {
                if let Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) = elem_expr {
                    enums.push(serde_json::Value::String(s.value()));
                } else {
                    return Err(syn::Error::new_spanned(
                        elem_expr, 
                        format!("Enum values for param '{param_name}' must be string literals.")
                    ));
                }
            }
            if !enums.is_empty() {
                attr_details.enum_values = Some(enums);
            }
        }
    }

    Ok(attr_details)
}

fn build_param_schema(pat_type: &syn::PatType, attr_details: ParamSchemaDetails) -> RefOr<Schema> {
    let (_, inner_type) = get_type_info(&pat_type.ty);
    let api_type = map_rust_type_to_openapi_type(&inner_type);

    if api_type == OpenApiType::Array {
        let mut array_builder = ArrayBuilder::new();
        
        // Extract the inner type from Vec<T>
        if let Type::Path(type_path) = &inner_type {
            if let Some(segment) = type_path.path.segments.first() {
                if segment.ident == "Vec" {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                            let inner_api_type = map_rust_type_to_openapi_type(inner_ty);
                            let items_schema = Schema::Object(ObjectBuilder::new().schema_type(inner_api_type).build());
                            array_builder = array_builder.items(RefOr::T(items_schema));
                        }
                    }
                }
            }
        }
        
        RefOr::T(Schema::Array(array_builder.build()))
    } else {
        let mut individual_schema_builder = ObjectBuilder::new().schema_type(api_type);
        if let Some(desc_val) = &attr_details.description {
            if !desc_val.is_empty() {
                individual_schema_builder = individual_schema_builder.description(Some(desc_val.clone()));
            }
        }
        if let Some(enums) = attr_details.enum_values {
            individual_schema_builder = individual_schema_builder.enum_values(Some(enums));
        }
        RefOr::T(Schema::Object(individual_schema_builder.build()))
    }
}

fn generate_declaration_function(
    func: &ItemFn,
    func_name: &str,
    func_description: &str,
    parameters_json_string: &str,
    required_params_for_struct_field: &[String]
) -> TokenStream {
    let generated_fn_name = syn::Ident::new(&format!("{func_name}_declaration"), func.sig.ident.span());

    let required_field_tokens = if required_params_for_struct_field.is_empty() {
        quote! { ::std::vec::Vec::new() }
    } else {
        quote! {
            ::std::vec![
                #( #required_params_for_struct_field.to_string() ),*
            ]
        }
    };

    let output = quote! {
        #func
        pub fn #generated_fn_name() -> ::rust_genai::FunctionDeclaration {
            ::rust_genai::FunctionDeclaration {
                name: #func_name.to_string(),
                description: #func_description.to_string(),
                parameters: {
                    match ::serde_json::from_str(#parameters_json_string) {
                        Ok(p) => Some(p),
                        Err(e) => {
                            panic!("INTERNAL MACRO ERROR: Failed to parse generated parameters JSON: {e}");
                        }
                    }
                },
                required: #required_field_tokens,
            }
        }
    };

    output.into()
}


