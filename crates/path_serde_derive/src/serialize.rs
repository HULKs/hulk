use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Result};

use crate::{bound::ExtendGenerics, container::Container};

pub fn derive_path_serialize(mut input: DeriveInput) -> Result<TokenStream> {
    let container = Container::try_from_ast(&input)?;

    input.generics.remove_defaults();
    input
        .generics
        .extend_with_bounds(container.serialize_bounds.clone());

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let path_serializations = generate_path_serializations(&container);
    let leaf_serializations = generate_leaf_serializations(&container);

    Ok(quote! {
        impl #impl_generics path_serde::PathSerialize for #name #ty_generics #where_clause {
            fn serialize_path<S>(
                &self,
                path: &str,
                serializer: S,
            ) -> Result<S::Ok, path_serde::serialize::Error<S::Error>>
            where
                S: serde::Serializer,
            {
                let split = path.split_once('.');
                match split {
                    Some((name, suffix)) => match name {
                        #(#path_serializations,)*
                        segment => Err(path_serde::serialize::Error::PathDoesNotExist {
                            path: segment.to_string(),
                        }),
                    },
                    None => {
                        match path {
                            #(#leaf_serializations,)*
                            segment => Err(path_serde::serialize::Error::PathDoesNotExist {
                                path: segment.to_string(),
                            }),
                        }
                    }
                }
            }
        }
    })
}

fn generate_path_serializations(container: &Container) -> Vec<TokenStream> {
    container
        .fields
        .iter()
        .filter(|field| !field.skip_serialize && !field.is_leaf)
        .map(|field| {
            let identifier = &field.identifier;
            let pattern = identifier.to_field_name();
            quote! {
                #pattern => self.#identifier.serialize_path(suffix, serializer)
            }
        })
        .collect()
}

fn generate_leaf_serializations(container: &Container) -> Vec<TokenStream> {
    container
        .fields
        .iter()
        .filter(|field| !field.skip_serialize)
        .map(|field| {
            let identifier = &field.identifier;
            let pattern = identifier.to_field_name();
            quote! {
                #pattern => serde::Serialize::serialize(&self.#identifier, serializer)
                    .map_err(path_serde::serialize::Error::SerializationFailed)
            }
        })
        .chain(container.computed_leafs.iter().map(|leaf| {
            let identifier = &leaf.identifier;
            let into_type = &leaf.into_type;
            let pattern = identifier.to_string();
            quote! {
                #pattern => {
                    std::convert::TryInto::<#into_type>::try_into(self)
                        .map_err(serde::ser::Error::custom)
                        .and_then(|leaf| serde::Serialize::serialize(&leaf, serializer))
                        .map_err(path_serde::serialize::Error::SerializationFailed)
                }
            }
        }))
        .collect()
}
