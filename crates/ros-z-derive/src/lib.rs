//! Derive macros for ros-z traits.
//!
//! Provides:
//! - `Message` for Rust-native message schema generation

#![allow(clippy::collapsible_if)]

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Expr, Fields, GenericArgument, GenericParam, Generics, Ident,
    LitStr, PathArguments, Type, parse_macro_input, parse_quote,
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
            let value = name.value();
            if !is_valid_native_type_path(&value) {
                return Err(syn::Error::new(
                    name.span(),
                    "Message derive name must be a native Rust type path like \"my_pkg::MyType\"; ROS slash-style names are not supported",
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

    let Fields::Named(fields) = &data.fields else {
        let message = match &data.fields {
            Fields::Unnamed(_) => "Message derive does not support tuple structs in v1",
            Fields::Unit => "Message derive does not support unit structs in v1",
            Fields::Named(_) => unreachable!(),
        };
        return Err(syn::Error::new_spanned(name, message));
    };

    let schema_fields = fields
        .named
        .iter()
        .map(|field| generate_message_field_schema_tokens(field, "Message"))
        .collect::<syn::Result<Vec<_>>>()?;

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
        .map(|ident| quote! { <#ident as ::ros_z::FieldTypeInfo>::generic_arg_name() })
        .collect::<Vec<_>>();
    let type_name_body = if type_params.is_empty() {
        quote! { #type_name }
    } else {
        quote! {{
            let generic_arg_names = ::std::vec![#(#generic_arg_names),*];

            fn __ros_z_sanitize_generic_name_fragment(fragment: &str) -> ::std::string::String {
                let mut sanitized = ::std::string::String::with_capacity(fragment.len());
                let mut previous_was_underscore = false;
                for ch in fragment.chars() {
                    let normalized = if ch.is_ascii_alphanumeric() { ch } else { '_' };
                    if normalized == '_' {
                        if !previous_was_underscore {
                            sanitized.push('_');
                            previous_was_underscore = true;
                        }
                    } else {
                        sanitized.push(normalized.to_ascii_lowercase());
                        previous_was_underscore = false;
                    }
                }
                sanitized.trim_matches('_').to_string()
            }

            fn __ros_z_short_stable_hash(value: &str) -> ::std::string::String {
                use ::ros_z::__private::sha2::Digest as _;
                let digest = ::ros_z::__private::sha2::Sha256::digest(value.as_bytes());
                let mut hash = ::std::string::String::with_capacity(12);
                for byte in &digest[..6] {
                    use ::std::fmt::Write;
                    let _ = write!(&mut hash, "{byte:02x}");
                }
                hash
            }

            let (prefix, leaf_name, separator) = #type_name
                .rsplit_once("::")
                .map(|(prefix, leaf)| (prefix, leaf, "::"))
                .unwrap_or(("", #type_name, ""));
            let mut suffix = generic_arg_names
                .iter()
                .map(|name| __ros_z_sanitize_generic_name_fragment(name))
                .filter(|name| !name.is_empty())
                .collect::<::std::vec::Vec<_>>()
                .join("__");
            if suffix.is_empty() {
                suffix = __ros_z_short_stable_hash(#type_name);
            } else if suffix.len() > 96 {
                let hash = __ros_z_short_stable_hash(&suffix);
                suffix.truncate(72);
                suffix.push_str("__");
                suffix.push_str(&hash);
            }
            let qualified_leaf = ::std::format!("{leaf_name}__{suffix}");
            let type_name = if prefix.is_empty() {
                qualified_leaf
            } else {
                ::std::format!("{prefix}{separator}{qualified_leaf}")
            };
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
                cache.lock().expect("type name cache poisoned").insert(key, type_name);
                type_name
            }

            fn __ros_z_schema() -> ::std::sync::Arc<::ros_z::dynamic::MessageSchema> {
                static SCHEMA: ::std::sync::OnceLock<
                    ::std::sync::Mutex<
                        ::std::collections::HashMap<
                            ::std::any::TypeId,
                            ::std::sync::Arc<::ros_z::dynamic::MessageSchema>
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

                let schema = ::ros_z::dynamic::MessageSchema::new(
                    Self::__ros_z_type_name(),
                    ::std::vec![#(#schema_fields),*],
                    None,
                )
                .expect("derived message schema type name must be valid");
                cache.lock().expect("schema cache poisoned").insert(key, schema.clone());
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

                let hash = ::ros_z::dynamic::schema_hash(&Self::__ros_z_schema())
                    .expect("derived message schema must convert to a hash");
                cache.lock().expect("schema hash cache poisoned").insert(key, hash.clone());
                hash
            }
        }

        impl #impl_generics ::ros_z::Message for #name #ty_generics #where_clause {
            type Codec = ::ros_z::msg::SerdeCdrCodec<Self>;

            fn type_name() -> &'static str {
                Self::__ros_z_type_name()
            }

            fn schema() -> ::std::sync::Arc<::ros_z::dynamic::MessageSchema> {
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

            fn __ros_z_enum_schema() -> ::std::sync::Arc<::ros_z::dynamic::EnumSchema> {
                static ENUM_SCHEMA: ::std::sync::OnceLock<::std::sync::Arc<::ros_z::dynamic::EnumSchema>> =
                    ::std::sync::OnceLock::new();

                ENUM_SCHEMA
                    .get_or_init(|| {
                        ::std::sync::Arc::new(::ros_z::dynamic::EnumSchema {
                            type_name: Self::__ros_z_type_name().to_string(),
                            variants: ::std::vec![#(#variant_tokens),*],
                        })
                    })
                    .clone()
            }

            fn __ros_z_schema() -> ::std::sync::Arc<::ros_z::dynamic::MessageSchema> {
                static SCHEMA: ::std::sync::OnceLock<::std::sync::Arc<::ros_z::dynamic::MessageSchema>> =
                    ::std::sync::OnceLock::new();

                SCHEMA
                    .get_or_init(|| {
                        ::ros_z::dynamic::MessageSchema::builder(Self::__ros_z_type_name())
                            .field("value", ::ros_z::dynamic::FieldType::Enum(Self::__ros_z_enum_schema()))
                            .build()
                            .expect("derived enum schema must be valid")
                    })
                    .clone()
            }

            fn __ros_z_schema_hash() -> ::ros_z::entity::SchemaHash {
                static SCHEMA_HASH: ::std::sync::OnceLock<::ros_z::entity::SchemaHash> =
                    ::std::sync::OnceLock::new();

                SCHEMA_HASH
                    .get_or_init(|| {
                        ::ros_z::dynamic::schema_hash(&Self::__ros_z_schema())
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

            fn schema() -> ::std::sync::Arc<::ros_z::dynamic::MessageSchema> {
                Self::__ros_z_schema()
            }

            fn schema_hash() -> ::ros_z::entity::SchemaHash {
                Self::__ros_z_schema_hash()
            }

            fn field_type() -> ::ros_z::dynamic::FieldType {
                ::ros_z::dynamic::FieldType::Enum(Self::__ros_z_enum_schema())
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
            type_param.bounds.push(parse_quote!(::ros_z::FieldTypeInfo));
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
    let field_type = generate_message_field_type_tokens(&field.ty, derive_name)?;

    Ok(quote! {
        ::ros_z::dynamic::FieldSchema::new(#field_name_str, #field_type)
    })
}

fn generate_message_field_type_tokens(ty: &Type, derive_name: &str) -> syn::Result<TokenStream2> {
    match ty {
        Type::Path(type_path) => {
            if type_path.qself.is_some() {
                return unsupported_message_type(
                    ty,
                    &format!(
                        "qualified self types are not supported by {derive_name} derive in v1"
                    ),
                );
            }

            let last_segment = type_path.path.segments.last().ok_or_else(|| {
                syn::Error::new_spanned(
                    ty,
                    format!("unsupported field type for {derive_name} derive"),
                )
            })?;
            let ident_str = last_segment.ident.to_string();
            let path_idents = type_path
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>();

            match ident_str.as_str() {
                _ if path_matches(&path_idents, &["bool"]) => {
                    Ok(quote! { ::ros_z::dynamic::FieldType::Bool })
                }
                _ if path_matches(&path_idents, &["i8"])
                    || path_matches(&path_idents, &["std", "primitive", "i8"])
                    || path_matches(&path_idents, &["core", "primitive", "i8"]) =>
                {
                    Ok(quote! { ::ros_z::dynamic::FieldType::Int8 })
                }
                _ if path_matches(&path_idents, &["u8"])
                    || path_matches(&path_idents, &["std", "primitive", "u8"])
                    || path_matches(&path_idents, &["core", "primitive", "u8"]) =>
                {
                    Ok(quote! { ::ros_z::dynamic::FieldType::Uint8 })
                }
                _ if path_matches(&path_idents, &["i16"])
                    || path_matches(&path_idents, &["std", "primitive", "i16"])
                    || path_matches(&path_idents, &["core", "primitive", "i16"]) =>
                {
                    Ok(quote! { ::ros_z::dynamic::FieldType::Int16 })
                }
                _ if path_matches(&path_idents, &["u16"])
                    || path_matches(&path_idents, &["std", "primitive", "u16"])
                    || path_matches(&path_idents, &["core", "primitive", "u16"]) =>
                {
                    Ok(quote! { ::ros_z::dynamic::FieldType::Uint16 })
                }
                _ if path_matches(&path_idents, &["i32"])
                    || path_matches(&path_idents, &["std", "primitive", "i32"])
                    || path_matches(&path_idents, &["core", "primitive", "i32"]) =>
                {
                    Ok(quote! { ::ros_z::dynamic::FieldType::Int32 })
                }
                _ if path_matches(&path_idents, &["u32"])
                    || path_matches(&path_idents, &["std", "primitive", "u32"])
                    || path_matches(&path_idents, &["core", "primitive", "u32"]) =>
                {
                    Ok(quote! { ::ros_z::dynamic::FieldType::Uint32 })
                }
                _ if path_matches(&path_idents, &["i64"])
                    || path_matches(&path_idents, &["std", "primitive", "i64"])
                    || path_matches(&path_idents, &["core", "primitive", "i64"]) =>
                {
                    Ok(quote! { ::ros_z::dynamic::FieldType::Int64 })
                }
                _ if path_matches(&path_idents, &["u64"])
                    || path_matches(&path_idents, &["std", "primitive", "u64"])
                    || path_matches(&path_idents, &["core", "primitive", "u64"]) =>
                {
                    Ok(quote! { ::ros_z::dynamic::FieldType::Uint64 })
                }
                _ if path_matches(&path_idents, &["f32"])
                    || path_matches(&path_idents, &["std", "primitive", "f32"])
                    || path_matches(&path_idents, &["core", "primitive", "f32"]) =>
                {
                    Ok(quote! { ::ros_z::dynamic::FieldType::Float32 })
                }
                _ if path_matches(&path_idents, &["f64"])
                    || path_matches(&path_idents, &["std", "primitive", "f64"])
                    || path_matches(&path_idents, &["core", "primitive", "f64"]) =>
                {
                    Ok(quote! { ::ros_z::dynamic::FieldType::Float64 })
                }
                _ if path_matches(&path_idents, &["String"])
                    || path_matches(&path_idents, &["std", "string", "String"])
                    || path_matches(&path_idents, &["alloc", "string", "String"]) =>
                {
                    Ok(quote! { ::ros_z::dynamic::FieldType::String })
                }
                _ if path_matches(&path_idents, &["usize"])
                    || path_matches(&path_idents, &["isize"])
                    || path_matches(&path_idents, &["std", "primitive", "usize"])
                    || path_matches(&path_idents, &["core", "primitive", "usize"])
                    || path_matches(&path_idents, &["std", "primitive", "isize"])
                    || path_matches(&path_idents, &["core", "primitive", "isize"]) =>
                {
                    unsupported_message_type(
                        ty,
                        &format!("usize and isize are not supported by {derive_name} derive in v1"),
                    )
                }
                _ if path_matches(&path_idents, &["HashMap"])
                    || path_matches(&path_idents, &["std", "collections", "HashMap"])
                    || path_matches(&path_idents, &["BTreeMap"])
                    || path_matches(&path_idents, &["std", "collections", "BTreeMap"])
                    || path_matches(&path_idents, &["alloc", "collections", "BTreeMap"]) =>
                {
                    let PathArguments::AngleBracketed(args) = &last_segment.arguments else {
                        return unsupported_message_type(
                            ty,
                            "map fields must specify key and value types",
                        );
                    };
                    let mut type_args = args.args.iter().filter_map(|arg| match arg {
                        GenericArgument::Type(ty) => Some(ty),
                        _ => None,
                    });
                    let Some(key) = type_args.next() else {
                        return unsupported_message_type(
                            ty,
                            "map fields must specify key and value types",
                        );
                    };
                    let Some(value) = type_args.next() else {
                        return unsupported_message_type(
                            ty,
                            "map fields must specify key and value types",
                        );
                    };
                    let key_tokens = generate_map_key_field_type_tokens(key, derive_name)?;
                    let value_tokens = generate_message_field_type_tokens(value, derive_name)?;
                    Ok(quote! {
                        ::ros_z::dynamic::FieldType::Map(
                            ::std::boxed::Box::new(#key_tokens),
                            ::std::boxed::Box::new(#value_tokens),
                        )
                    })
                }
                _ if path_matches(&path_idents, &["Option"])
                    || path_matches(&path_idents, &["std", "option", "Option"])
                    || path_matches(&path_idents, &["core", "option", "Option"]) =>
                {
                    let PathArguments::AngleBracketed(args) = &last_segment.arguments else {
                        return unsupported_message_type(
                            ty,
                            "Option fields must specify an inner type",
                        );
                    };
                    let Some(GenericArgument::Type(inner)) = args.args.first() else {
                        return unsupported_message_type(
                            ty,
                            "Option fields must specify an inner type",
                        );
                    };
                    let inner_tokens = generate_message_field_type_tokens(inner, derive_name)?;
                    Ok(quote! {
                        ::ros_z::dynamic::FieldType::Optional(::std::boxed::Box::new(#inner_tokens))
                    })
                }
                _ if path_matches(&path_idents, &["Vec"])
                    || path_matches(&path_idents, &["std", "vec", "Vec"])
                    || path_matches(&path_idents, &["alloc", "vec", "Vec"]) =>
                {
                    let PathArguments::AngleBracketed(args) = &last_segment.arguments else {
                        return unsupported_message_type(
                            ty,
                            "Vec fields must specify an element type",
                        );
                    };
                    let Some(GenericArgument::Type(inner)) = args.args.first() else {
                        return unsupported_message_type(
                            ty,
                            "Vec fields must specify an element type",
                        );
                    };
                    let inner_tokens = generate_message_field_type_tokens(inner, derive_name)?;
                    Ok(quote! {
                        ::ros_z::dynamic::FieldType::Sequence(::std::boxed::Box::new(#inner_tokens))
                    })
                }
                _ => Ok(quote! {
                    <#ty as ::ros_z::FieldTypeInfo>::field_type()
                }),
            }
        }
        Type::Array(array) => {
            let len = match &array.len {
                Expr::Lit(expr_lit) => match &expr_lit.lit {
                    syn::Lit::Int(value) => value.base10_parse::<usize>()?,
                    _ => {
                        return unsupported_message_type(
                            ty,
                            "array lengths must be integer literals for Message derive",
                        );
                    }
                },
                _ => {
                    return unsupported_message_type(
                        ty,
                        "array lengths must be integer literals for Message derive",
                    );
                }
            };

            let inner_tokens = generate_message_field_type_tokens(&array.elem, derive_name)?;
            Ok(quote! {
                ::ros_z::dynamic::FieldType::Array(::std::boxed::Box::new(#inner_tokens), #len)
            })
        }
        Type::Tuple(_) => unsupported_message_type(
            ty,
            &format!("tuple fields are not supported by {derive_name} derive in v1"),
        ),
        _ => unsupported_message_type(
            ty,
            &format!("unsupported field type for {derive_name} derive in v1"),
        ),
    }
}

fn generate_map_key_field_type_tokens(ty: &Type, derive_name: &str) -> syn::Result<TokenStream2> {
    match ty {
        Type::Path(type_path) => {
            if type_path.qself.is_some() {
                return unsupported_message_type(
                    ty,
                    &format!(
                        "qualified self map key types are not supported by {derive_name} derive in v1"
                    ),
                );
            }

            let path_idents = type_path
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>();

            match () {
                _ if path_matches(&path_idents, &["bool"]) => {
                    Ok(quote! { ::ros_z::dynamic::FieldType::Bool })
                }
                _ if path_matches(&path_idents, &["i8"])
                    || path_matches(&path_idents, &["std", "primitive", "i8"])
                    || path_matches(&path_idents, &["core", "primitive", "i8"]) =>
                {
                    Ok(quote! { ::ros_z::dynamic::FieldType::Int8 })
                }
                _ if path_matches(&path_idents, &["u8"])
                    || path_matches(&path_idents, &["std", "primitive", "u8"])
                    || path_matches(&path_idents, &["core", "primitive", "u8"]) =>
                {
                    Ok(quote! { ::ros_z::dynamic::FieldType::Uint8 })
                }
                _ if path_matches(&path_idents, &["i16"])
                    || path_matches(&path_idents, &["std", "primitive", "i16"])
                    || path_matches(&path_idents, &["core", "primitive", "i16"]) =>
                {
                    Ok(quote! { ::ros_z::dynamic::FieldType::Int16 })
                }
                _ if path_matches(&path_idents, &["u16"])
                    || path_matches(&path_idents, &["std", "primitive", "u16"])
                    || path_matches(&path_idents, &["core", "primitive", "u16"]) =>
                {
                    Ok(quote! { ::ros_z::dynamic::FieldType::Uint16 })
                }
                _ if path_matches(&path_idents, &["i32"])
                    || path_matches(&path_idents, &["std", "primitive", "i32"])
                    || path_matches(&path_idents, &["core", "primitive", "i32"]) =>
                {
                    Ok(quote! { ::ros_z::dynamic::FieldType::Int32 })
                }
                _ if path_matches(&path_idents, &["u32"])
                    || path_matches(&path_idents, &["std", "primitive", "u32"])
                    || path_matches(&path_idents, &["core", "primitive", "u32"]) =>
                {
                    Ok(quote! { ::ros_z::dynamic::FieldType::Uint32 })
                }
                _ if path_matches(&path_idents, &["i64"])
                    || path_matches(&path_idents, &["std", "primitive", "i64"])
                    || path_matches(&path_idents, &["core", "primitive", "i64"]) =>
                {
                    Ok(quote! { ::ros_z::dynamic::FieldType::Int64 })
                }
                _ if path_matches(&path_idents, &["u64"])
                    || path_matches(&path_idents, &["std", "primitive", "u64"])
                    || path_matches(&path_idents, &["core", "primitive", "u64"]) =>
                {
                    Ok(quote! { ::ros_z::dynamic::FieldType::Uint64 })
                }
                _ if path_matches(&path_idents, &["String"])
                    || path_matches(&path_idents, &["std", "string", "String"])
                    || path_matches(&path_idents, &["alloc", "string", "String"]) =>
                {
                    Ok(quote! { ::ros_z::dynamic::FieldType::String })
                }
                _ => unsupported_message_type(
                    ty,
                    &format!(
                        "map keys for {derive_name} derive must be bool, an integer, or String"
                    ),
                ),
            }
        }
        _ => unsupported_message_type(
            ty,
            &format!("unsupported map key type for {derive_name} derive in v1"),
        ),
    }
}

fn path_matches(path_idents: &[String], expected: &[&str]) -> bool {
    path_idents.len() == expected.len()
        && path_idents
            .iter()
            .map(String::as_str)
            .zip(expected.iter().copied())
            .all(|(actual, expected)| actual == expected)
}

fn generate_enum_variant_schema_tokens(
    variant: &syn::Variant,
    derive_name: &str,
) -> syn::Result<TokenStream2> {
    let variant_name = variant.ident.to_string();
    let payload = match &variant.fields {
        Fields::Unit => quote! { ::ros_z::dynamic::EnumPayloadSchema::Unit },
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            let field_type =
                generate_message_field_type_tokens(&fields.unnamed[0].ty, derive_name)?;
            quote! {
                ::ros_z::dynamic::EnumPayloadSchema::Newtype(::std::boxed::Box::new(#field_type))
            }
        }
        Fields::Unnamed(fields) => {
            let field_types = fields
                .unnamed
                .iter()
                .map(|field| generate_message_field_type_tokens(&field.ty, derive_name))
                .collect::<syn::Result<Vec<_>>>()?;
            quote! {
                ::ros_z::dynamic::EnumPayloadSchema::Tuple(::std::vec![#(#field_types),*])
            }
        }
        Fields::Named(fields) => {
            let field_schemas = fields
                .named
                .iter()
                .map(|field| generate_message_field_schema_tokens(field, derive_name))
                .collect::<syn::Result<Vec<_>>>()?;
            quote! {
                ::ros_z::dynamic::EnumPayloadSchema::Struct(::std::vec![#(#field_schemas),*])
            }
        }
    };

    Ok(quote! {
        ::ros_z::dynamic::EnumVariantSchema::new(#variant_name, #payload)
    })
}

fn unsupported_message_type<T>(node: &T, message: &str) -> syn::Result<TokenStream2>
where
    T: quote::ToTokens,
{
    Err(syn::Error::new_spanned(node, message))
}

fn is_valid_native_type_path(value: &str) -> bool {
    !value.is_empty()
        && !value.contains(['/', '<', '>'])
        && !value.chars().any(char::is_whitespace)
        && value.split("::").all(is_valid_rust_identifier)
}

fn is_valid_rust_identifier(value: &str) -> bool {
    let (value, is_raw) = value
        .strip_prefix("r#")
        .map_or((value, false), |value| (value, true));
    if value == "_" {
        return false;
    }
    let mut chars = value.chars();
    matches!(chars.next(), Some(ch) if ch == '_' || ch.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        && if is_raw {
            !is_forbidden_raw_identifier(value)
        } else {
            !is_rust_keyword(value)
        }
}

fn is_forbidden_raw_identifier(value: &str) -> bool {
    matches!(value, "Self" | "self" | "super" | "crate")
}

fn is_rust_keyword(value: &str) -> bool {
    matches!(
        value,
        "as" | "async"
            | "await"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "try"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "macro_rules"
            | "union"
    )
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
