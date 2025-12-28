//! Procedural macros for the Rust Gemini SDK.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Expr, ExprLit, Fields, GenericArgument, Lit,
    PathArguments, Type,
};

#[proc_macro_derive(GeminiTool, attributes(gemini))]
pub fn gemini_tool(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_gemini_tool(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand_gemini_tool(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let struct_attrs = parse_gemini_attrs(&input.attrs)?;
    let struct_doc = extract_doc_comment(&input.attrs);

    let GeminiAttr {
        name: struct_name,
        description: struct_description,
        ..
    } = struct_attrs;
    let function_name = struct_name.unwrap_or_else(|| name.to_string());
    let function_description = struct_description.or(struct_doc);

    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => return Err(syn::Error::new_spanned(input, "GeminiTool 仅支持结构体")),
    };

    let (property_inserts, required_fields, ordering_fields) = collect_schema_fields(fields)?;
    let description_expr = build_description_expr(function_description);

    Ok(quote! {
        impl #name {
            pub fn as_tool() -> ::rust_genai_types::tool::Tool {
                let mut properties: ::std::collections::HashMap<String, Box<::rust_genai_types::tool::Schema>> =
                    ::std::collections::HashMap::new();
                #(#property_inserts)*

                let required: Vec<String> = vec![#(#required_fields),*];
                let ordering: Vec<String> = vec![#(#ordering_fields),*];

                let schema = ::rust_genai_types::tool::Schema {
                    ty: Some(::rust_genai_types::enums::Type::Object),
                    properties: Some(properties),
                    required: if required.is_empty() { None } else { Some(required) },
                    property_ordering: if ordering.is_empty() { None } else { Some(ordering) },
                    ..Default::default()
                };

                let declaration = ::rust_genai_types::tool::FunctionDeclaration {
                    name: #function_name.to_string(),
                    description: #description_expr,
                    parameters: Some(schema),
                    parameters_json_schema: None,
                    response: None,
                    response_json_schema: None,
                    behavior: None,
                };

                ::rust_genai_types::tool::Tool {
                    function_declarations: Some(vec![declaration]),
                    ..Default::default()
                }
            }

            pub fn from_call(call: &::rust_genai_types::content::FunctionCall) -> ::rust_genai::Result<Self> {
                if let Some(name) = &call.name {
                    if name != #function_name {
                        return Err(::rust_genai::Error::InvalidConfig {
                            message: format!("Expected {}, got {}", #function_name, name),
                        });
                    }
                }

                let args = call.args.as_ref().ok_or_else(|| ::rust_genai::Error::InvalidConfig {
                    message: "Missing args".into(),
                })?;

                let parsed = ::serde_json::from_value(args.clone())?;
                Ok(parsed)
            }
        }
    })
}

fn collect_schema_fields(
    fields: &Fields,
) -> syn::Result<(Vec<TokenStream2>, Vec<TokenStream2>, Vec<TokenStream2>)> {
    let mut property_inserts = Vec::new();
    let mut required_fields = Vec::new();
    let mut ordering_fields = Vec::new();

    match fields {
        Fields::Named(named) => {
            for field in &named.named {
                let field_ident = field
                    .ident
                    .as_ref()
                    .ok_or_else(|| syn::Error::new_spanned(field, "GeminiTool 仅支持命名字段"))?;
                let field_attrs = parse_gemini_attrs(&field.attrs)?;
                if field_attrs.skip {
                    continue;
                }

                let field_doc = extract_doc_comment(&field.attrs);
                let property_name = field_attrs
                    .name
                    .clone()
                    .unwrap_or_else(|| field_ident.to_string());

                let is_optional = is_option_type(&field.ty);
                let schema_expr =
                    build_schema_expr(&field.ty, is_optional, &field_attrs, field_doc);

                property_inserts.push(quote! {
                    {
                        let schema = #schema_expr;
                        properties.insert(#property_name.to_string(), Box::new(schema));
                    }
                });

                ordering_fields.push(quote! { #property_name.to_string() });

                if field_attrs.required || (!is_optional && !field_attrs.optional) {
                    required_fields.push(quote! { #property_name.to_string() });
                }
            }
        }
        _ => {
            return Err(syn::Error::new_spanned(
                fields,
                "GeminiTool 仅支持具名字段结构体",
            ))
        }
    }

    Ok((property_inserts, required_fields, ordering_fields))
}

fn build_description_expr(function_description: Option<String>) -> TokenStream2 {
    function_description.map_or_else(
        || quote!(None),
        |description| quote!(Some(#description.to_string())),
    )
}

#[derive(Default)]
struct GeminiAttr {
    name: Option<String>,
    description: Option<String>,
    enum_values: Option<Vec<String>>,
    required: bool,
    optional: bool,
    skip: bool,
}

fn parse_gemini_attrs(attrs: &[Attribute]) -> syn::Result<GeminiAttr> {
    let mut output = GeminiAttr::default();
    for attr in attrs {
        if !attr.path().is_ident("gemini") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") || meta.path.is_ident("rename") {
                let value: syn::LitStr = meta.value()?.parse()?;
                output.name = Some(value.value());
                return Ok(());
            }
            if meta.path.is_ident("description") {
                let value: syn::LitStr = meta.value()?.parse()?;
                output.description = Some(value.value());
                return Ok(());
            }
            if meta.path.is_ident("enum_values") {
                let value: syn::LitStr = meta.value()?.parse()?;
                let values = value
                    .value()
                    .split(',')
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                if !values.is_empty() {
                    output.enum_values = Some(values);
                }
                return Ok(());
            }
            if meta.path.is_ident("required") {
                output.required = true;
                return Ok(());
            }
            if meta.path.is_ident("optional") {
                output.optional = true;
                return Ok(());
            }
            if meta.path.is_ident("skip") {
                output.skip = true;
                return Ok(());
            }
            Ok(())
        })?;
    }
    Ok(output)
}

fn extract_doc_comment(attrs: &[Attribute]) -> Option<String> {
    let mut docs = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        if let syn::Meta::NameValue(meta) = &attr.meta {
            if let Expr::Lit(ExprLit {
                lit: Lit::Str(text),
                ..
            }) = &meta.value
            {
                docs.push(text.value().trim().to_string());
            }
        }
    }
    if docs.is_empty() {
        None
    } else {
        Some(docs.join("\n"))
    }
}

fn build_schema_expr(
    ty: &Type,
    is_optional: bool,
    attrs: &GeminiAttr,
    doc: Option<String>,
) -> TokenStream2 {
    let base_expr = schema_expr_for_type(ty);
    let mut statements = Vec::new();
    statements.push(quote! { let mut schema = #base_expr; });

    if is_optional {
        statements.push(quote! { schema.nullable = Some(true); });
    }

    let description = attrs.description.clone().or(doc);
    if let Some(description) = description {
        statements.push(quote! { schema.description = Some(#description.to_string()); });
    }

    if let Some(values) = &attrs.enum_values {
        let values_tokens = values.iter().map(|v| quote!(#v.to_string()));
        statements.push(quote! { schema.enum_values = Some(vec![#(#values_tokens),*]); });
    }

    statements.push(quote! { schema });
    quote!({ #(#statements)* })
}

fn schema_expr_for_type(ty: &Type) -> TokenStream2 {
    if let Some(inner) = option_inner(ty) {
        return schema_expr_for_type(inner);
    }
    if let Some(inner) = vec_inner(ty) {
        let inner_expr = schema_expr_for_type(inner);
        return quote! {
            ::rust_genai_types::tool::Schema {
                ty: Some(::rust_genai_types::enums::Type::Array),
                items: Some(Box::new(#inner_expr)),
                ..Default::default()
            }
        };
    }

    let ty = strip_reference(ty);
    if is_serde_json_value(ty) {
        return quote!(::rust_genai_types::tool::Schema::default());
    }

    if let Some(ident) = last_path_ident(ty) {
        let schema = match ident.as_str() {
            "String" | "str" => quote!(::rust_genai_types::tool::Schema::string()),
            "bool" | "Boolean" => quote!(::rust_genai_types::tool::Schema::boolean()),
            "f32" | "f64" => quote!(::rust_genai_types::tool::Schema::number()),
            "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => {
                quote!(::rust_genai_types::tool::Schema::integer())
            }
            _ => quote!(::rust_genai_types::tool::Schema {
                ty: Some(::rust_genai_types::enums::Type::Object),
                ..Default::default()
            }),
        };
        return schema;
    }

    quote!(::rust_genai_types::tool::Schema {
        ty: Some(::rust_genai_types::enums::Type::Object),
        ..Default::default()
    })
}

fn is_option_type(ty: &Type) -> bool {
    option_inner(ty).is_some()
}

fn option_inner(ty: &Type) -> Option<&Type> {
    let ty = strip_reference(ty);
    if let Type::Path(path) = ty {
        if let Some(segment) = path.path.segments.last() {
            if segment.ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner);
                    }
                }
            }
        }
    }
    None
}

fn vec_inner(ty: &Type) -> Option<&Type> {
    let ty = strip_reference(ty);
    if let Type::Path(path) = ty {
        if let Some(segment) = path.path.segments.last() {
            if segment.ident == "Vec" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner);
                    }
                }
            }
        }
    }
    None
}

fn strip_reference(ty: &Type) -> &Type {
    if let Type::Reference(reference) = ty {
        return strip_reference(&reference.elem);
    }
    ty
}

fn is_serde_json_value(ty: &Type) -> bool {
    if let Type::Path(path) = ty {
        let segments: Vec<_> = path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        return segments.as_slice() == ["serde_json", "Value"] || segments.as_slice() == ["Value"];
    }
    false
}

fn last_path_ident(ty: &Type) -> Option<String> {
    if let Type::Path(path) = ty {
        return path.path.segments.last().map(|seg| seg.ident.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::ToTokens;
    use syn::parse_quote;

    fn normalize_tokens(tokens: &TokenStream2) -> String {
        tokens.to_string().split_whitespace().collect()
    }

    #[test]
    fn parse_gemini_attrs_reads_values() {
        let attrs: Vec<Attribute> = vec![parse_quote!(
            #[gemini(
                name = "tool_name",
                description = "desc",
                enum_values = "a, b",
                required,
                optional,
                skip
            )]
        )];
        let parsed = parse_gemini_attrs(&attrs).unwrap();
        assert_eq!(parsed.name.as_deref(), Some("tool_name"));
        assert_eq!(parsed.description.as_deref(), Some("desc"));
        assert_eq!(
            parsed.enum_values.as_ref().unwrap(),
            &vec!["a".to_string(), "b".to_string()]
        );
        assert!(parsed.required);
        assert!(parsed.optional);
        assert!(parsed.skip);
    }

    #[test]
    fn parse_gemini_attrs_ignores_empty_enum_values() {
        let attrs: Vec<Attribute> =
            vec![parse_quote!(#[gemini(rename = "alias", enum_values = " , ")])];
        let parsed = parse_gemini_attrs(&attrs).unwrap();
        assert_eq!(parsed.name.as_deref(), Some("alias"));
        assert!(parsed.enum_values.is_none());
    }

    #[test]
    fn extract_doc_comment_combines_lines() {
        let attrs: Vec<Attribute> = vec![
            parse_quote!(#[doc = " First line "]),
            parse_quote!(#[doc = "Second line"]),
        ];
        let docs = extract_doc_comment(&attrs).unwrap();
        assert_eq!(docs, "First line\nSecond line");
    }

    #[test]
    fn expand_gemini_tool_rejects_enum() {
        let input: DeriveInput = parse_quote!(
            enum Bad {
                A,
            }
        );
        let err = expand_gemini_tool(&input).unwrap_err();
        assert!(err.to_string().contains("GeminiTool 仅支持结构体"));
    }

    #[test]
    fn expand_gemini_tool_rejects_tuple_struct() {
        let input: DeriveInput = parse_quote!(
            struct Bad(String);
        );
        let err = expand_gemini_tool(&input).unwrap_err();
        assert!(err.to_string().contains("具名字段"));
    }

    #[test]
    fn schema_helpers_cover_variants() {
        let opt_vec: Type = parse_quote!(Option<Vec<String>>);
        let tokens = normalize_tokens(&schema_expr_for_type(&opt_vec));
        assert!(tokens.contains("Type::Array"));
        assert!(tokens.contains("Schema::string"));

        let int_ty: Type = parse_quote!(i64);
        let tokens = normalize_tokens(&schema_expr_for_type(&int_ty));
        assert!(tokens.contains("Schema::integer"));

        let unknown: Type = parse_quote!(CustomType);
        let tokens = normalize_tokens(&schema_expr_for_type(&unknown));
        assert!(tokens.contains("Type::Object"));
    }

    #[test]
    fn build_schema_expr_applies_metadata() {
        let ty: Type = parse_quote!(Option<String>);
        let attrs = GeminiAttr {
            description: Some("desc".to_string()),
            enum_values: Some(vec!["x".to_string(), "y".to_string()]),
            ..Default::default()
        };
        let tokens = normalize_tokens(&build_schema_expr(&ty, true, &attrs, None));
        assert!(tokens.contains("nullable=Some(true)"));
        assert!(tokens.contains("schema.description=Some(\"desc\".to_string())"));
        assert!(tokens.contains("schema.enum_values=Some"));
    }

    #[test]
    fn type_helpers_detect_options_and_vecs() {
        let ty: Type = parse_quote!(&Option<Vec<u32>>);
        assert!(is_option_type(&ty));
        let inner = option_inner(&ty).unwrap();
        let inner_tokens = inner.to_token_stream().to_string();
        assert!(inner_tokens.contains("Vec"));

        let vec_ty: Type = parse_quote!(Vec<bool>);
        assert!(vec_inner(&vec_ty).is_some());
        assert!(last_path_ident(&vec_ty).is_some());
        let reference: Type = parse_quote!(&&str);
        let stripped = strip_reference(&reference);
        assert!(last_path_ident(stripped).is_some());
    }

    #[test]
    fn detects_serde_json_value() {
        let ty: Type = parse_quote!(serde_json::Value);
        assert!(is_serde_json_value(&ty));
        let ty: Type = parse_quote!(Value);
        assert!(is_serde_json_value(&ty));
        let ty: Type = parse_quote!(String);
        assert!(!is_serde_json_value(&ty));
    }
}
