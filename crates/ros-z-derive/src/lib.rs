//! Derive macros for ros-z traits.
//!
//! Provides:
//! - `Message` for Rust-native message schema generation

#![allow(clippy::collapsible_if)]

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Fields, GenericParam, Generics, Ident, LitStr, Type,
    parse_macro_input, parse_quote,
};

type TokenStream2 = proc_macro2::TokenStream;

#[proc_macro_derive(Message, attributes(message))]
pub fn derive_message(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_message(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn impl_message(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let attrs = parse_message_args(&input.attrs)?;
    let type_name = attrs
        .name
        .map(|name| {
            if name.value().is_empty() {
                return Err(syn::Error::new(
                    name.span(),
                    "Message derive name must not be empty",
                ));
            }
            Ok(quote! { #name })
        })
        .transpose()?
        .unwrap_or_else(
            || quote! { ::std::concat!(::std::module_path!(), "::", ::std::stringify!(#name)) },
        );

    match &input.data {
        Data::Struct(data) => impl_message_for_struct(input, data, &type_name),
        Data::Enum(data) => {
            ensure_non_generic_enum(input, "Message")?;
            impl_message_for_enum(name, data, &type_name)
        }
        Data::Union(_) => Err(syn::Error::new_spanned(
            input,
            "Message derive does not support unions",
        )),
    }
}

fn impl_message_for_struct(
    input: &DeriveInput,
    data: &syn::DataStruct,
    type_name: &TokenStream2,
) -> syn::Result<TokenStream2> {
    ensure_supported_struct_generics(input, "Message")?;
    let name = &input.ident;

    let schema_fields = match &data.fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .map(|field| generate_message_field_schema_tokens(field, "Message"))
            .collect::<syn::Result<Vec<_>>>()?,
        Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .enumerate()
            .map(|(index, field)| {
                generate_unnamed_message_field_schema_tokens(index, field, "Message")
            })
            .collect::<syn::Result<Vec<_>>>()?,
        Fields::Unit => Vec::new(),
    };
    let field_types = match &data.fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .map(|field| &field.ty)
            .collect::<Vec<_>>(),
        Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .map(|field| &field.ty)
            .collect::<Vec<_>>(),
        Fields::Unit => Vec::new(),
    };

    let mut bounded_generics = add_message_bounds(&input.generics);
    if input
        .generics
        .params
        .iter()
        .any(|param| matches!(param, GenericParam::Const(_)))
    {
        let (_, self_ty_generics, _) = input.generics.split_for_impl();
        let where_clause = bounded_generics.make_where_clause();
        where_clause
            .predicates
            .push(parse_quote!(#name #self_ty_generics: ::serde::Serialize));
        where_clause
            .predicates
            .push(parse_quote!(#name #self_ty_generics: ::serde::de::DeserializeOwned));
        for field_ty in field_types {
            where_clause
                .predicates
                .push(parse_quote!(#field_ty: ::ros_z::Message));
        }
    }
    let (impl_generics, ty_generics, where_clause) = bounded_generics.split_for_impl();
    let generic_arg_names = input
        .generics
        .params
        .iter()
        .filter_map(|param| match param {
            GenericParam::Type(type_param) => {
                let ident = &type_param.ident;
                Some(quote! { <#ident as ::ros_z::Message>::type_name() })
            }
            GenericParam::Const(const_param) => {
                let ident = &const_param.ident;
                Some(quote! { ::std::format!("{}", #ident) })
            }
            GenericParam::Lifetime(_) => None,
        })
        .collect::<Vec<_>>();
    let type_name_body = if generic_arg_names.is_empty() {
        quote! { #type_name.to_string() }
    } else {
        quote! {{
            let generic_arg_names = ::std::vec![#(#generic_arg_names),*];
            ::std::format!("{}<{}>", #type_name, generic_arg_names.join(","))
        }}
    };

    Ok(quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            fn __ros_z_type_name() -> String {
                #type_name_body
            }

        }

        impl #impl_generics ::ros_z::schema::MessageSchema for #name #ty_generics #where_clause {
            fn build_schema(
                builder: &mut ::ros_z::schema::SchemaBuilder,
            ) -> ::std::result::Result<
                ::ros_z::__private::ros_z_schema::TypeDef,
                ::ros_z::__private::ros_z_schema::SchemaError,
            > {
                builder.define_message_struct::<Self>(|fields| {
                    #(#schema_fields)*
                    Ok(())
                })
            }
        }

        impl #impl_generics ::ros_z::Message for #name #ty_generics #where_clause {
            type Codec = ::ros_z::message::SerdeCdrCodec<Self>;

            fn type_name() -> String {
                Self::__ros_z_type_name()
            }
        }
    })
}

fn impl_message_for_enum(
    name: &Ident,
    data: &syn::DataEnum,
    type_name: &TokenStream2,
) -> syn::Result<TokenStream2> {
    if data.variants.is_empty() {
        return Err(syn::Error::new_spanned(
            name,
            "Message derive requires enums to have at least one variant",
        ));
    }

    let variant_tokens = data
        .variants
        .iter()
        .map(|variant| generate_enum_variant_schema_tokens(variant, "Message"))
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        impl #name {
            fn __ros_z_type_name() -> String {
                #type_name.to_string()
            }

        }

        impl ::ros_z::schema::MessageSchema for #name {
            fn build_schema(
                builder: &mut ::ros_z::schema::SchemaBuilder,
            ) -> ::std::result::Result<
                ::ros_z::__private::ros_z_schema::TypeDef,
                ::ros_z::__private::ros_z_schema::SchemaError,
            > {
                builder.define_message_enum::<Self>(|variants| {
                    #(#variant_tokens)*
                    Ok(())
                })
            }
        }

        impl ::ros_z::Message for #name {
            type Codec = ::ros_z::message::SerdeCdrCodec<Self>;

            fn type_name() -> String {
                Self::__ros_z_type_name()
            }
        }
    })
}

fn ensure_supported_struct_generics(input: &DeriveInput, derive_name: &str) -> syn::Result<()> {
    for param in &input.generics.params {
        match param {
            GenericParam::Type(_) | GenericParam::Const(_) => {}
            GenericParam::Lifetime(lifetime) => {
                return Err(syn::Error::new_spanned(
                    lifetime,
                    format!("{derive_name} derive does not support lifetime parameters in v1"),
                ));
            }
        }
    }

    Ok(())
}

fn ensure_non_generic_enum(input: &DeriveInput, derive_name: &str) -> syn::Result<()> {
    if input.generics.params.is_empty() {
        return Ok(());
    }

    Err(syn::Error::new_spanned(
        &input.generics,
        format!("{derive_name} derive does not support generic enums in v1"),
    ))
}

fn add_message_bounds(generics: &Generics) -> Generics {
    let mut bounded = generics.clone();
    for param in &mut bounded.params {
        if let GenericParam::Type(type_param) = param {
            type_param.bounds.push(parse_quote!(::ros_z::Message));
            type_param.bounds.push(parse_quote!(::serde::Serialize));
            type_param
                .bounds
                .push(parse_quote!(::serde::de::DeserializeOwned));
            type_param.bounds.push(parse_quote!(::std::marker::Send));
            type_param.bounds.push(parse_quote!(::std::marker::Sync));
            type_param.bounds.push(parse_quote!('static));
        }
    }
    bounded
}

fn generate_message_field_schema_tokens(
    field: &syn::Field,
    derive_name: &str,
) -> syn::Result<TokenStream2> {
    let field_name = field
        .ident
        .as_ref()
        .ok_or_else(|| syn::Error::new_spanned(field, "named fields are required"))?;
    let field_name_str = field_ident_to_config_path(field_name);
    let ty = &field.ty;
    validate_message_schema_type(ty, derive_name)?;

    Ok(quote! {
        fields.field::<#ty>(#field_name_str)?;
    })
}

fn generate_unnamed_message_field_schema_tokens(
    index: usize,
    field: &syn::Field,
    derive_name: &str,
) -> syn::Result<TokenStream2> {
    let field_name = index.to_string();
    let ty = &field.ty;
    validate_message_schema_type(ty, derive_name)?;

    Ok(quote! {
        fields.field::<#ty>(#field_name)?;
    })
}

fn validate_message_schema_type(ty: &Type, derive_name: &str) -> syn::Result<()> {
    match ty {
        Type::Tuple(_) => Err(syn::Error::new_spanned(
            ty,
            format!("tuple fields are not supported by {derive_name} derive in v1"),
        )),
        _ => Ok(()),
    }
}

fn generate_enum_variant_schema_tokens(
    variant: &syn::Variant,
    derive_name: &str,
) -> syn::Result<TokenStream2> {
    let variant_name = variant.ident.to_string();
    match &variant.fields {
        Fields::Unit => Ok(quote! {
            variants.unit(#variant_name);
        }),
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            let ty = &fields.unnamed[0].ty;
            validate_message_schema_type(ty, derive_name)?;
            Ok(quote! {
                variants.newtype::<#ty>(#variant_name)?;
            })
        }
        Fields::Unnamed(fields) => {
            let schemas = fields
                .unnamed
                .iter()
                .map(|field| {
                    let ty = &field.ty;
                    validate_message_schema_type(ty, derive_name)?;
                    Ok(quote! {
                        fields.element::<#ty>()?;
                    })
                })
                .collect::<syn::Result<Vec<_>>>()?;
            Ok(quote! {
                variants.tuple(#variant_name, |fields| {
                    #(#schemas)*
                    Ok(())
                })?;
            })
        }
        Fields::Named(fields) => {
            let field_schemas = fields
                .named
                .iter()
                .map(|field| generate_message_field_schema_tokens(field, derive_name))
                .collect::<syn::Result<Vec<_>>>()?;
            Ok(quote! {
                variants.struct_variant(#variant_name, |fields| {
                    #(#field_schemas)*
                    Ok(())
                })?;
            })
        }
    }
}

#[derive(Default)]
struct MessageArgs {
    name: Option<LitStr>,
}

fn parse_message_args(attrs: &[Attribute]) -> syn::Result<MessageArgs> {
    let mut parsed = MessageArgs::default();

    for attr in attrs {
        if !attr.path().is_ident("message") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                let value = meta.value()?.parse::<LitStr>()?;
                parsed.name = Some(value);
                return Ok(());
            }

            Err(meta.error("unsupported message attribute, expected: name"))
        })?;
    }

    Ok(parsed)
}

fn field_ident_to_config_path(ident: &Ident) -> String {
    let name = ident.to_string();
    if let Some(stripped) = name.strip_prefix("r#") {
        stripped.to_string()
    } else {
        name
    }
}
