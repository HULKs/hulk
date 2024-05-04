use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Result};

use crate::{bound::ExtendGenerics, container::Container};

pub fn derive_path_deserialize(mut input: DeriveInput) -> Result<TokenStream> {
    let container = Container::try_from_ast(&input)?;

    input.generics.remove_defaults();
    input
        .generics
        .extend_with_bounds(container.deserialize_bounds.clone());

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let path_deserializations = generate_path_deserializations(&container);
    let leaf_deserializations = generate_leaf_deserializations(&container);

    Ok(quote! {
        impl #impl_generics path_serde::PathDeserialize for #name #ty_generics #where_clause {
            fn deserialize_path<'de, D>(
                &mut self,
                path: &str,
                deserializer: D,
            ) -> Result<(), path_serde::deserialize::Error<D::Error>>
            where
                D: serde::Deserializer<'de>,
            {
                let split = path.split_once('.');
                match split {
                    Some((name, suffix)) => match name {
                        #(#path_deserializations,)*
                        name => Err(path_serde::deserialize::Error::PathDoesNotExist {
                            path: name.to_string(),
                        }),
                    },
                    None => match path {
                        #(#leaf_deserializations,)*
                        name => Err(path_serde::deserialize::Error::PathDoesNotExist {
                            path: name.to_string(),
                        }),
                    },
                }
            }
        }
    })
}

fn generate_path_deserializations(container: &Container) -> Vec<TokenStream> {
    container
        .fields
        .iter()
        .filter(|field| !field.skip_deserialize && !field.is_leaf)
        .map(|field| {
            let identifier = &field.identifier;
            let pattern = identifier.to_field_name();
            quote! {
                #pattern => self.#identifier.deserialize_path(suffix, deserializer)
            }
        })
        .collect()
}

fn generate_leaf_deserializations(container: &Container) -> Vec<TokenStream> {
    container
        .fields
        .iter()
        .filter(|field| !field.skip_deserialize)
        .map(|field| {
            let identifier = &field.identifier;
            let pattern = identifier.to_field_name();
            let ty = &field.ty;
            quote! {
                #pattern => {
                    self.#identifier =
                        <#ty as serde::Deserialize>::deserialize(deserializer)
                            .map_err(path_serde::deserialize::Error::DeserializationFailed)?;
                    Ok(())
                }
            }
        })
        .collect()
}
