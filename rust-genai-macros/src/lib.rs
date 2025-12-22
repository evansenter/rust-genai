use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use std::collections::HashMap;
use syn::parse::{Parse, ParseStream, Result as ParseResult};
use syn::{
    Attribute, Expr, ExprArray, ExprLit, Ident, ItemFn, Lit, LitStr, Meta, Pat, Token, Type,
    parenthesized, parse_macro_input, punctuated::Punctuated,
};
use utoipa::openapi::RefOr;
use utoipa::openapi::schema::{ArrayBuilder, ObjectBuilder, Schema, Type as OpenApiType};

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

        let metas: Punctuated<Meta, Token![,]> =
            content.parse_terminated(Meta::parse, Token![,])?;

        for meta in metas {
            if let Meta::NameValue(nv) = meta {
                if nv.path.is_ident("description") {
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }) = nv.value
                    {
                        description = Some(s);
                    } else {
                        return Err(syn::Error::new_spanned(
                            nv.value,
                            "Expected string literal for description",
                        ));
                    }
                } else if nv.path.is_ident("enum_values") {
                    if let Expr::Array(arr) = nv.value {
                        enum_values = Some(arr);
                    } else {
                        return Err(syn::Error::new_spanned(
                            nv.value,
                            "Expected array for enum_values",
                        ));
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
    configs: Punctuated<SingleParamConfigInput, Token![,]>,
}

impl Parse for AllParamsConfigInput {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        if input.is_empty() {
            Ok(Self {
                configs: Punctuated::new(),
            })
        } else {
            let configs = input.parse_terminated(SingleParamConfigInput::parse, Token![,])?;
            Ok(Self { configs })
        }
    }
}

fn extract_doc_comments(attrs: &[Attribute]) -> String {
    attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc")
                && let Meta::NameValue(mnv) = &attr.meta
                && let Expr::Lit(ExprLit {
                    lit: Lit::Str(lit_str),
                    ..
                }) = &mnv.value
            {
                return Some(lit_str.value().trim().to_string());
            }
            None
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn get_type_info(ty: &Type) -> (bool, Type) {
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
        _ => OpenApiType::Object, // Default for complex types
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
        if let syn::FnArg::Typed(pat_type) = fn_arg
            && let Pat::Ident(pat_ident) = &*pat_type.pat
        {
            let param_name = pat_ident.ident.to_string();

            let attr_details = match process_param_config(&param_name, config_map.get(&param_name))
            {
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

    let parameters_schema_obj = object_builder.build();
    let parameters_schema_ref_or: RefOr<Schema> = RefOr::T(Schema::Object(parameters_schema_obj));

    let parameters_json_string = match serde_json::to_string_pretty(&parameters_schema_ref_or) {
        Ok(s) => s,
        Err(e) => {
            return syn::Error::new_spanned(
                func.sig.ident,
                format!("Failed to serialize schema to JSON: {e}"),
            )
            .to_compile_error()
            .into();
        }
    };

    generate_declaration_function(
        &func,
        &func_name,
        &func_description,
        &parameters_json_string,
        &required_params_for_struct_field,
    )
}

fn process_param_config(
    param_name: &str,
    config: Option<&SingleParamConfigInput>,
) -> Result<ParamSchemaDetails, syn::Error> {
    let mut attr_details = ParamSchemaDetails::default();

    if let Some(config) = config {
        if let Some(desc_lit) = &config.description {
            attr_details.description = Some(desc_lit.value());
        }
        if let Some(enums_array) = &config.enum_values {
            let mut enums = Vec::new();
            for elem_expr in &enums_array.elems {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = elem_expr
                {
                    enums.push(serde_json::Value::String(s.value()));
                } else {
                    return Err(syn::Error::new_spanned(
                        elem_expr,
                        format!("Enum values for param '{param_name}' must be string literals."),
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
        if let Some(desc_val) = &attr_details.description
            && !desc_val.is_empty()
        {
            individual_schema_builder =
                individual_schema_builder.description(Some(desc_val.clone()));
        }
        if let Some(enums) = attr_details.enum_values {
            individual_schema_builder = individual_schema_builder.enum_values(Some(enums));
        }
        RefOr::T(Schema::Object(individual_schema_builder.build()))
    }
}

#[allow(clippy::too_many_lines)]
fn generate_declaration_function(
    func: &ItemFn,
    func_name: &str,
    func_description: &str,
    parameters_json_string: &str,
    required_params_for_struct_field: &[String],
) -> TokenStream {
    let generated_fn_name =
        syn::Ident::new(&format!("{func_name}_declaration"), func.sig.ident.span());
    let callable_struct_name_str = func_name
        .split('_')
        .map(|s| {
            s.chars()
                .next()
                .map_or_else(|| s.to_string(), |c| c.to_uppercase().to_string() + &s[1..])
        })
        .collect::<String>()
        + "Callable";
    let callable_struct_name = syn::Ident::new(&callable_struct_name_str, func.sig.ident.span());

    let generated_callable_factory_fn_name = syn::Ident::new(
        &format!("{func_name}_callable_factory"),
        func.sig.ident.span(),
    );

    let required_field_tokens = if required_params_for_struct_field.is_empty() {
        quote! { ::std::vec::Vec::new() }
    } else {
        quote! {
            ::std::vec![
                #( #required_params_for_struct_field.to_string() ),*
            ]
        }
    };

    let mut arg_names = Vec::new();
    let mut arg_extraction_tokens = Vec::new();

    for fn_arg in &func.sig.inputs {
        if let syn::FnArg::Typed(pat_type) = fn_arg
            && let Pat::Ident(pat_ident) = &*pat_type.pat
        {
            let param_name_str = pat_ident.ident.to_string();
            let param_ident = &pat_ident.ident;
            let param_type = &pat_type.ty;
            arg_names.push(param_ident.clone());

            let (is_option, _inner_type) = get_type_info(param_type);

            if is_option {
                arg_extraction_tokens.push(quote! {
                    let #param_ident: #param_type = match args.get(#param_name_str) {
                        Some(val) if !val.is_null() => {
                            ::serde_json::from_value(val.clone()).map_err(|e| {
                                ::rust_genai::function_calling::FunctionError::ArgumentMismatch(
                                    format!("Failed to deserialize optional argument '{}': {}", #param_name_str, e)
                                )
                            })?
                        }
                        _ => None,
                    };
                });
            } else {
                arg_extraction_tokens.push(quote! {
                    let #param_ident: #param_type = args.get(#param_name_str)
                        .ok_or_else(|| ::rust_genai::function_calling::FunctionError::ArgumentMismatch(format!("Missing required argument '{}'", #param_name_str)))
                        .and_then(|val| ::serde_json::from_value(val.clone()).map_err(|e| {
                            ::rust_genai::function_calling::FunctionError::ArgumentMismatch(
                                format!("Failed to deserialize argument '{}': {}", #param_name_str, e)
                            )
                        }))?;
                });
            }
        }
    }

    let original_fn_ident = &func.sig.ident;
    let fn_call_args = quote! { #(#arg_names),* };

    let is_async_fn = func.sig.asyncness.is_some();
    let fn_call_expr = if is_async_fn {
        quote! { #original_fn_ident(#fn_call_args).await }
    } else {
        quote! { #original_fn_ident(#fn_call_args) }
    };

    let output = quote! {
        #func

        #[derive(Debug, Clone, Copy)]
        pub struct #callable_struct_name;

        impl #callable_struct_name {
            pub const fn new() -> Self {
                Self
            }
        }

        #[::async_trait::async_trait]
        impl ::rust_genai::function_calling::CallableFunction for #callable_struct_name {
            fn declaration(&self) -> ::rust_genai::FunctionDeclaration {
                let properties = match ::serde_json::from_str(#parameters_json_string) {
                    Ok(p) => p,
                    Err(_e) => {
                        panic!("Failed to parse generated parameters JSON for '{}': {}", #func_name, _e);
                    }
                };

                ::rust_genai::FunctionDeclaration {
                    name: #func_name.to_string(),
                    description: #func_description.to_string(),
                    parameters: ::rust_genai::FunctionParameters {
                        type_: "object".to_string(),
                        properties,
                        required: #required_field_tokens,
                    },
                }
            }

            async fn call(&self, args: ::serde_json::Value) -> Result<::serde_json::Value, ::rust_genai::function_calling::FunctionError> {
                #(#arg_extraction_tokens)*

                let original_fn_result = #fn_call_expr;

                match ::serde_json::to_value(original_fn_result) {
                    Ok(value_from_fn_result) => {
                        // If the value is already a JSON object, return it as is.
                        // Otherwise, wrap it in a {"result": ...} object.
                        if value_from_fn_result.is_object() {
                            Ok(value_from_fn_result)
                        } else {
                            Ok(::serde_json::json!({ "result": value_from_fn_result }))
                        }
                    }
                    Err(e) => Err(::rust_genai::function_calling::FunctionError::ExecutionError(Box::new(e)))
                }
            }
        }

        pub fn #generated_fn_name() -> ::rust_genai::FunctionDeclaration {
             #callable_struct_name::new().declaration()
        }

        pub fn #generated_callable_factory_fn_name() -> Box<dyn ::rust_genai::function_calling::CallableFunction> {
            Box::new(#callable_struct_name::new())
        }

        ::rust_genai::function_calling::submit! {
            ::rust_genai::function_calling::CallableFunctionFactory::new(#generated_callable_factory_fn_name)
        }
    };

    output.into()
}
