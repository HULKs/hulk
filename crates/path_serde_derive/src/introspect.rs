use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Result};

use crate::{bound::ExtendGenerics, container::Container};

pub fn derive_path_introspect(mut input: DeriveInput) -> Result<TokenStream> {
    let container = Container::try_from_ast(&input)?;

    input.generics.remove_defaults();
    input
        .generics
        .extend_with_bounds(container.introspect_bounds.clone());

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let extend_with_fields = generate_extend_with_fields(&container);

    Ok(quote! {
        impl #impl_generics path_serde::PathIntrospect for #name #ty_generics #where_clause {
            fn extend_with_fields(fields: &mut std::collections::HashSet<String>, prefix: &str)  {
                #(#extend_with_fields)*
            }
        }
    })
}

fn generate_extend_with_fields(container: &Container) -> Vec<TokenStream> {
    let leafs = container
        .fields
        .iter()
        .filter(|field| !field.skip_introspect);
    let children = container
        .fields
        .iter()
        .filter(|field| !field.skip_introspect && !field.is_leaf);
    let computed_leafs = container.computed_leaves.iter();

    leafs.map(|field| {
            let field_name = &field.identifier.to_field_name();
            quote! {
                fields.insert(format!("{prefix}{}", #field_name));
            }
        })
    .chain(children.map(|field| {
            let field_name = &field.identifier.to_field_name();
            let ty = &field.ty;
            quote! {
                <#ty as path_serde::PathIntrospect>::extend_with_fields(fields, &format!("{prefix}{}.", #field_name));
            }
    }))
        .chain(computed_leafs.map(|leaf| {
            let field_name = &leaf.identifier.to_string();
            quote! {
                fields.insert(format!("{prefix}{}", #field_name));
            }
        }))
        .collect()
}
