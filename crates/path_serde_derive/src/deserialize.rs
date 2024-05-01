use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Result};

use crate::{container::Container, extend_generics::ExtendGenerics as _};

pub fn derive_path_deserialize(mut input: DeriveInput) -> Result<TokenStream> {
    let container = Container::try_from_ast(&input)?;

    input.generics.extend_from_attributes(&container);

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
                        name => Err(path_serde::deserialize::Error::UnexpectedPath {
                            path: name.to_string(),
                        }),
                    },
                    None => match path {
                        #(#leaf_deserializations,)*
                        name => Err(path_serde::deserialize::Error::UnexpectedPath {
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
        .filter(|field| !field.skip && !field.leaf)
        .map(|field| {
            let identifier = &field.identifier;
            let pattern = identifier.to_string();
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
        .filter(|field| !field.skip)
        .map(|field| {
            let identifier = &field.identifier;
            let pattern = identifier.to_string();
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
