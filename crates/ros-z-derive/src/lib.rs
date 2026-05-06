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
        Fields::Unit => Vec::new(),
        Fields::Unnamed(_) => {
            return Err(syn::Error::new_spanned(
                name,
                "Message derive does not support tuple structs in v1",
            ));
        }
    };

    let bounded_generics = add_message_bounds(&input.generics);
    let (impl_generics, ty_generics, where_clause) = bounded_generics.split_for_impl();
    let type_params = input
        .generics
        .params
        .iter()
        .filter_map(|param| match param {
            GenericParam::Type(type_param) => Some(&type_param.ident),
            _ => None,
        })
        .collect::<Vec<_>>();
    let generic_arg_names = type_params
        .iter()
        .map(|ident| quote! { <#ident as ::ros_z::Message>::type_name() })
        .collect::<Vec<_>>();
    let type_name_body = if type_params.is_empty() {
        quote! { #type_name }
    } else {
        quote! {{
            let generic_arg_names = ::std::vec![#(#generic_arg_names),*];
            let type_name = ::std::format!("{}<{}>", #type_name, generic_arg_names.join(","));
            ::std::boxed::Box::leak(type_name.into_boxed_str())
        }}
    };

    Ok(quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            fn __ros_z_type_name() -> &'static str {
                static TYPE_NAME: ::std::sync::OnceLock<
                    ::std::sync::Mutex<
                        ::std::collections::HashMap<::std::any::TypeId, &'static str>
                    >
                > = ::std::sync::OnceLock::new();

                let key = ::std::any::TypeId::of::<Self>();
                let cache = TYPE_NAME.get_or_init(|| {
                    ::std::sync::Mutex::new(::std::collections::HashMap::new())
                });
                if let Some(type_name) = cache.lock().expect("type name cache poisoned").get(&key).copied() {
                    return type_name;
                }

                let type_name = #type_name_body;
                let mut cache = cache.lock().expect("type name cache poisoned");
                if let Some(existing) = cache.get(&key).copied() {
                    return existing;
                }
                cache.insert(key, type_name);
                type_name
            }

            fn __ros_z_schema() -> ::ros_z::dynamic::Schema {
                static SCHEMA: ::std::sync::OnceLock<
                    ::std::sync::Mutex<
                        ::std::collections::HashMap<
                            ::std::any::TypeId,
                            ::ros_z::dynamic::Schema
                        >
                    >
                > = ::std::sync::OnceLock::new();

                let key = ::std::any::TypeId::of::<Self>();
                let cache = SCHEMA.get_or_init(|| {
                    ::std::sync::Mutex::new(::std::collections::HashMap::new())
                });
                if let Some(schema) = cache.lock().expect("schema cache poisoned").get(&key).cloned() {
                    return schema;
                }

                let schema = ::std::sync::Arc::new(::ros_z::dynamic::TypeShape::Struct {
                    name: ::ros_z::__private::ros_z_schema::TypeName::new(Self::__ros_z_type_name())
                        .expect("derived message schema type name must be valid"),
                    fields: ::std::vec![#(#schema_fields),*],
                });
                let mut cache = cache.lock().expect("schema cache poisoned");
                if let Some(existing) = cache.get(&key).cloned() {
                    return existing;
                }
                cache.insert(key, schema.clone());
                schema
            }

            fn __ros_z_schema_hash() -> ::ros_z::entity::SchemaHash {
                static SCHEMA_HASH: ::std::sync::OnceLock<
                    ::std::sync::Mutex<
                        ::std::collections::HashMap<::std::any::TypeId, ::ros_z::entity::SchemaHash>
                    >
                > = ::std::sync::OnceLock::new();

                let key = ::std::any::TypeId::of::<Self>();
                let cache = SCHEMA_HASH.get_or_init(|| {
                    ::std::sync::Mutex::new(::std::collections::HashMap::new())
                });
                if let Some(hash) = cache.lock().expect("schema hash cache poisoned").get(&key).cloned() {
                    return hash;
                }

                let hash = ::ros_z::dynamic::schema_tree_hash(Self::__ros_z_type_name(), &Self::__ros_z_schema())
                    .expect("derived message schema must convert to a hash");
                let mut cache = cache.lock().expect("schema hash cache poisoned");
                if let Some(existing) = cache.get(&key).cloned() {
                    return existing;
                }
                cache.insert(key, hash.clone());
                hash
            }
        }

        impl #impl_generics ::ros_z::Message for #name #ty_generics #where_clause {
            type Codec = ::ros_z::msg::SerdeCdrCodec<Self>;

            fn type_name() -> &'static str {
                Self::__ros_z_type_name()
            }

            fn schema() -> ::ros_z::dynamic::Schema {
                Self::__ros_z_schema()
            }

            fn schema_hash() -> ::ros_z::entity::SchemaHash {
                Self::__ros_z_schema_hash()
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
            fn __ros_z_type_name() -> &'static str {
                #type_name
            }

            fn __ros_z_schema() -> ::ros_z::dynamic::Schema {
                static SCHEMA: ::std::sync::OnceLock<::ros_z::dynamic::Schema> =
                    ::std::sync::OnceLock::new();

                SCHEMA
                    .get_or_init(|| {
                        ::std::sync::Arc::new(::ros_z::dynamic::TypeShape::Enum {
                            name: ::ros_z::__private::ros_z_schema::TypeName::new(Self::__ros_z_type_name())
                                .expect("derived enum schema type name must be valid"),
                            variants: ::std::vec![#(#variant_tokens),*],
                        })
                    })
                    .clone()
            }

            fn __ros_z_schema_hash() -> ::ros_z::entity::SchemaHash {
                static SCHEMA_HASH: ::std::sync::OnceLock<::ros_z::entity::SchemaHash> =
                    ::std::sync::OnceLock::new();

                SCHEMA_HASH
                    .get_or_init(|| {
                        ::ros_z::dynamic::schema_tree_hash(Self::__ros_z_type_name(), &Self::__ros_z_schema())
                            .expect("derived message schema must convert to a hash")
                    })
                    .clone()
            }
        }

        impl ::ros_z::Message for #name {
            type Codec = ::ros_z::msg::SerdeCdrCodec<Self>;

            fn type_name() -> &'static str {
                Self::__ros_z_type_name()
            }

            fn schema() -> ::ros_z::dynamic::Schema {
                Self::__ros_z_schema()
            }

            fn schema_hash() -> ::ros_z::entity::SchemaHash {
                Self::__ros_z_schema_hash()
            }
        }
    })
}

fn ensure_supported_struct_generics(input: &DeriveInput, derive_name: &str) -> syn::Result<()> {
    for param in &input.generics.params {
        match param {
            GenericParam::Type(_) => {}
            GenericParam::Lifetime(lifetime) => {
                return Err(syn::Error::new_spanned(
                    lifetime,
                    format!("{derive_name} derive does not support lifetime parameters in v1"),
                ));
            }
            GenericParam::Const(const_param) => {
                return Err(syn::Error::new_spanned(
                    const_param,
                    format!("{derive_name} derive does not support const generics in v1"),
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
                .push(parse_quote!(for<'de> ::serde::Deserialize<'de>));
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
    let field_schema = generate_message_schema_tokens(&field.ty, derive_name)?;

    Ok(quote! {
        ::ros_z::dynamic::RuntimeFieldSchema::new(#field_name_str, #field_schema)
    })
}

fn generate_message_schema_tokens(ty: &Type, derive_name: &str) -> syn::Result<TokenStream2> {
    match ty {
        Type::Tuple(_) => unsupported_message_type(
            ty,
            &format!("tuple fields are not supported by {derive_name} derive in v1"),
        ),
        _ => Ok(quote! { <#ty as ::ros_z::Message>::schema() }),
    }
}

fn generate_enum_variant_schema_tokens(
    variant: &syn::Variant,
    derive_name: &str,
) -> syn::Result<TokenStream2> {
    let variant_name = variant.ident.to_string();
    let payload = match &variant.fields {
        Fields::Unit => quote! { ::ros_z::dynamic::RuntimeDynamicEnumPayload::Unit },
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            let schema = generate_message_schema_tokens(&fields.unnamed[0].ty, derive_name)?;
            quote! {
                ::ros_z::dynamic::RuntimeDynamicEnumPayload::Newtype(#schema)
            }
        }
        Fields::Unnamed(fields) => {
            let schemas = fields
                .unnamed
                .iter()
                .map(|field| generate_message_schema_tokens(&field.ty, derive_name))
                .collect::<syn::Result<Vec<_>>>()?;
            quote! {
                ::ros_z::dynamic::RuntimeDynamicEnumPayload::Tuple(::std::vec![#(#schemas),*])
            }
        }
        Fields::Named(fields) => {
            let field_schemas = fields
                .named
                .iter()
                .map(|field| generate_message_field_schema_tokens(field, derive_name))
                .collect::<syn::Result<Vec<_>>>()?;
            quote! {
                ::ros_z::dynamic::RuntimeDynamicEnumPayload::Struct(::std::vec![#(#field_schemas),*])
            }
        }
    };

    Ok(quote! {
        ::ros_z::dynamic::RuntimeDynamicEnumVariant::new(#variant_name, #payload)
    })
}

fn unsupported_message_type<T>(node: &T, message: &str) -> syn::Result<TokenStream2>
where
    T: quote::ToTokens,
{
    Err(syn::Error::new_spanned(node, message))
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
