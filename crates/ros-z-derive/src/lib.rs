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
    ensure_supported_struct_generics(input, "Message")?;
    let name = &input.ident;
    let (native_methods, serde_bridge) = native_cdr_tokens(input)?;
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
        Data::Struct(data) => {
            impl_message_for_struct(input, data, &type_name, &native_methods, serde_bridge)
        }
        Data::Enum(data) => {
            impl_message_for_enum(input, data, &type_name, &native_methods, serde_bridge)
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
    native_methods: &TokenStream2,
    serde_bridge: bool,
) -> syn::Result<TokenStream2> {
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
    let bounded_generics = add_message_bounds(input, serde_bridge)?;
    let (impl_generics, ty_generics, where_clause) = bounded_generics.split_for_impl();
    let type_name_body = message_type_name(input, type_name, serde_bridge);

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
            type Codec = ::ros_z::CdrCodec<Self>;
            #native_methods

            fn type_name() -> String {
                Self::__ros_z_type_name()
            }
        }
    })
}

fn impl_message_for_enum(
    input: &DeriveInput,
    data: &syn::DataEnum,
    type_name: &TokenStream2,
    native_methods: &TokenStream2,
    serde_bridge: bool,
) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let generics = add_message_bounds(input, serde_bridge)?;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let type_name_body = message_type_name(input, type_name, serde_bridge);
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
                builder.define_message_enum::<Self>(|variants| {
                    #(#variant_tokens)*
                    Ok(())
                })
            }
        }

        impl #impl_generics ::ros_z::Message for #name #ty_generics #where_clause {
            type Codec = ::ros_z::CdrCodec<Self>;
            #native_methods

            fn type_name() -> String {
                Self::__ros_z_type_name()
            }
        }
    })
}

fn serde_cdr_tokens() -> TokenStream2 {
    quote! {
        fn encode_cdr<__RosZBuffer: ::ros_z::__private::ros_z_cdr::CdrBuffer>(
            &self,
            writer: &mut ::ros_z::__private::ros_z_cdr::CdrWriter<'_, ::ros_z::__private::ros_z_cdr::LittleEndian, __RosZBuffer>,
        ) -> ::ros_z::__private::ros_z_cdr::Result<()> {
            ::ros_z::__private::encode_serde(self, writer)
        }

        fn decode_cdr(
            reader: &mut ::ros_z::__private::ros_z_cdr::CdrReader<'_, ::ros_z::__private::ros_z_cdr::LittleEndian>,
        ) -> ::ros_z::__private::ros_z_cdr::Result<Self> {
            ::ros_z::__private::decode_serde(reader)
        }
    }
}

fn native_cdr_tokens(input: &DeriveInput) -> syn::Result<(TokenStream2, bool)> {
    if parse_message_args(&input.attrs)?.serde {
        return Ok((serde_cdr_tokens(), true));
    }
    // Keep Serde's custom wire representations explicit rather than guessing their layout.
    let mut attrs: Vec<&Attribute> = input.attrs.iter().collect();
    match &input.data {
        Data::Struct(data) => attrs.extend(data.fields.iter().flat_map(|f| &f.attrs)),
        Data::Enum(data) => {
            for variant in &data.variants {
                attrs.extend(&variant.attrs);
                attrs.extend(variant.fields.iter().flat_map(|f| &f.attrs));
            }
        }
        Data::Union(_) => {}
    }
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let metas = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        )?;
        for meta in metas {
            let path = meta.path();
            if path.is_ident("rename")
                || path.is_ident("rename_all")
                || path.is_ident("default")
                || path.is_ident("deny_unknown_fields")
                || path.is_ident("bound")
                || path.is_ident("transparent")
            {
                continue;
            }
            if path.is_ident("with") {
                if let syn::Meta::NameValue(value) = &meta {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(adapter),
                        ..
                    }) = &value.value
                    {
                        if matches!(
                            adapter.value().as_str(),
                            "ros_z::serde_zslice" | "::ros_z::serde_zslice"
                        ) {
                            continue;
                        }
                    }
                }
            }
            return Ok((serde_cdr_tokens(), true));
        }
    }
    let fields = |fields: &Fields| -> syn::Result<_> {
        let bindings: Vec<_> = fields
            .iter()
            .enumerate()
            .map(|(i, _)| quote::format_ident!("field_{i}"))
            .collect();
        let types: Vec<_> = fields.iter().map(|f| &f.ty).collect();
        let pattern = match fields {
            Fields::Named(named) => {
                let names = named.named.iter().map(|f| &f.ident);
                quote!({ #(#names: #bindings),* })
            }
            Fields::Unnamed(_) => quote!((#(#bindings),*)),
            Fields::Unit => quote!(),
        };
        let bridges = fields
            .iter()
            .map(|field| parse_message_args(&field.attrs).map(|args| args.serde))
            .collect::<syn::Result<Vec<_>>>()?;
        let encode = bindings
            .iter()
            .zip(&types)
            .zip(&bridges)
            .map(|((binding, ty), bridge)| {
                if *bridge {
                    quote!(::ros_z::__private::encode_serde::<#ty, _>(#binding, writer)?;)
                } else {
                    quote!(::ros_z::Message::encode_cdr(#binding, writer)?;)
                }
            })
            .collect::<Vec<_>>();
        let encode = quote!(#(#encode)*);
        let decodes = types
            .iter()
            .zip(&bridges)
            .map(|(ty, bridge)| {
                if *bridge {
                    quote!(::ros_z::__private::decode_serde::<#ty>(reader)?)
                } else {
                    quote!(<#ty as ::ros_z::Message>::decode_cdr(reader)?)
                }
            })
            .collect::<Vec<_>>();
        let decode = match fields {
            Fields::Named(named) => {
                let names = named.named.iter().map(|f| &f.ident);
                quote!({ #(#names: #decodes),* })
            }
            Fields::Unnamed(_) => quote!((#(#decodes),*)),
            Fields::Unit => quote!(),
        };
        Ok((pattern, encode, decode))
    };
    let (encode, decode) = match &input.data {
        Data::Struct(data) => {
            let (pattern, encode, decode) = fields(&data.fields)?;
            (
                quote!(let Self #pattern = self; #encode),
                quote!(Ok(Self #decode)),
            )
        }
        Data::Enum(data) => {
            let mut encodes = Vec::new();
            let mut decodes = Vec::new();
            for (index, variant) in data.variants.iter().enumerate() {
                let index = index as u32;
                let name = &variant.ident;
                let (pattern, encode, decode) = fields(&variant.fields)?;
                encodes.push(quote!(Self::#name #pattern => { writer.write_u32(#index); #encode }));
                decodes.push(quote!(#index => Ok(Self::#name #decode)));
            }
            (
                quote!(match self { #(#encodes),* }),
                quote!(match reader.read_u32()? {
                    #(#decodes),*,
                    tag => Err(::ros_z::__private::ros_z_cdr::Error::Custom(format!("invalid enum tag {tag}"))),
                }),
            )
        }
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                input,
                "Message derive does not support unions",
            ));
        }
    };
    Ok((
        quote! {
            fn encode_cdr<__RosZBuffer: ::ros_z::__private::ros_z_cdr::CdrBuffer>(&self, writer: &mut ::ros_z::__private::ros_z_cdr::CdrWriter<'_, ::ros_z::__private::ros_z_cdr::LittleEndian, __RosZBuffer>) -> ::ros_z::__private::ros_z_cdr::Result<()> {
                #encode
                Ok(())
            }
            fn decode_cdr(reader: &mut ::ros_z::__private::ros_z_cdr::CdrReader<'_, ::ros_z::__private::ros_z_cdr::LittleEndian>) -> ::ros_z::__private::ros_z_cdr::Result<Self> {
                #decode
            }
        },
        false,
    ))
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

fn message_fields(input: &DeriveInput) -> Vec<&syn::Field> {
    match &input.data {
        Data::Struct(data) => data.fields.iter().collect(),
        Data::Enum(data) => data.variants.iter().flat_map(|v| &v.fields).collect(),
        Data::Union(_) => Vec::new(),
    }
}

fn contains_ident(tokens: TokenStream2, ident: &Ident) -> bool {
    tokens.into_iter().any(|token| match token {
        proc_macro2::TokenTree::Ident(candidate) => candidate == *ident,
        proc_macro2::TokenTree::Group(group) => contains_ident(group.stream(), ident),
        _ => false,
    })
}

fn native_generic(input: &DeriveInput, ident: &Ident, serde_bridge: bool) -> bool {
    !serde_bridge
        && message_fields(input).iter().any(|field| {
            let ty = &field.ty;
            !parse_message_args(&field.attrs).is_ok_and(|args| args.serde)
                && contains_ident(quote!(#ty), ident)
        })
}

fn generic_type_name(input: &DeriveInput, ident: &Ident, serde_bridge: bool) -> TokenStream2 {
    if native_generic(input, ident, serde_bridge) {
        quote!(<#ident as ::ros_z::Message>::type_name())
    } else {
        quote!(::std::any::type_name::<#ident>().to_string())
    }
}

fn message_type_name(input: &DeriveInput, name: &TokenStream2, serde_bridge: bool) -> TokenStream2 {
    let args = input
        .generics
        .params
        .iter()
        .filter_map(|param| match param {
            GenericParam::Type(param) => Some(generic_type_name(input, &param.ident, serde_bridge)),
            GenericParam::Const(param) => {
                let ident = &param.ident;
                Some(quote!(::std::format!("{}", #ident)))
            }
            GenericParam::Lifetime(_) => None,
        })
        .collect::<Vec<_>>();
    if args.is_empty() {
        quote!(#name.to_string())
    } else {
        quote!(::std::format!("{}<{}>", #name, ::std::vec![#(#args),*].join(",")))
    }
}

fn add_message_bounds(input: &DeriveInput, serde_bridge: bool) -> syn::Result<Generics> {
    let mut bounded = input.generics.clone();
    for param in &mut bounded.params {
        if let GenericParam::Type(type_param) = param {
            if native_generic(input, &type_param.ident, serde_bridge) {
                type_param.bounds.push(parse_quote!(::ros_z::Message));
            } else {
                type_param.bounds.push(parse_quote!(::std::marker::Send));
                type_param.bounds.push(parse_quote!(::std::marker::Sync));
                type_param.bounds.push(parse_quote!('static));
            }
        }
    }
    for field in message_fields(input) {
        let ty = &field.ty;
        // Recursive fields resolve through this impl, not a self-referential where clause.
        if contains_ident(quote!(#ty), &input.ident)
            || contains_ident(quote!(#ty), &Ident::new("Self", input.ident.span()))
        {
            continue;
        }
        let predicate = if serde_bridge {
            parse_quote!(#ty: ::ros_z::MessageSchema)
        } else if parse_message_args(&field.attrs)?.serde {
            parse_quote!(#ty: ::ros_z::MessageSchema + ::serde::Serialize + ::serde::de::DeserializeOwned)
        } else {
            parse_quote!(#ty: ::ros_z::Message)
        };
        bounded.make_where_clause().predicates.push(predicate);
    }
    if serde_bridge {
        let name = &input.ident;
        let (_, ty_generics, _) = input.generics.split_for_impl();
        bounded.make_where_clause().predicates.push(
            parse_quote!(#name #ty_generics: ::serde::Serialize + ::serde::de::DeserializeOwned),
        );
    }
    Ok(bounded)
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
    serde: bool,
    name: Option<LitStr>,
}

fn parse_message_args(attrs: &[Attribute]) -> syn::Result<MessageArgs> {
    let mut parsed = MessageArgs::default();

    for attr in attrs {
        if !attr.path().is_ident("message") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("serde") {
                parsed.serde = true;
                return Ok(());
            }
            if meta.path.is_ident("name") {
                let value = meta.value()?.parse::<LitStr>()?;
                parsed.name = Some(value);
                return Ok(());
            }

            Err(meta.error("unsupported message attribute, expected: name or serde"))
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
