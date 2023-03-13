use std::collections::HashSet;

use proc_macro2::TokenStream;
use proc_macro_error::proc_macro_error;
use quote::quote;
use quote::ToTokens;
use syn::Type;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Ident, Meta, NestedMeta};

// mod process_enum;
// mod process_struct;

const SERIALIZE_HIERARCHY: &str = "serialize_hierarchy";
const SKIP: &str = "skip";
const AS_JPEG: &str = "as_jpeg";

#[proc_macro_derive(SerializeHierarchy, attributes(serialize_hierarchy))]
#[proc_macro_error]
pub fn serialize_hierarchy(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    process_input(input)
        .unwrap_or_else(|error| error.into_compile_error())
        .into()
}

fn process_input(input: DeriveInput) -> syn::Result<TokenStream> {
    let children = match &input.data {
        Data::Struct(data) => read_children(data)?,
        Data::Enum(..) => Vec::new(),
        Data::Union(data) => {
            return Err(syn::Error::new_spanned(
                data.union_token,
                "`SerializeHierarchy` can only be derived for `struct` or `enum`",
            ))
        }
    };
    let type_attributes = parse_attributes(&input.attrs)?;
    let as_jpeg = type_attributes.contains(&TypeAttribute::AsJpeg);

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let serializable_children = children
        .iter()
        .filter(|child| !child.attributes.contains(&ChildAttribute::Skip));
    let path_serializations = generate_path_serializations(serializable_children.clone());
    let serde_serializations = generate_serde_serializations(serializable_children.clone());
    let path_deserializations = generate_path_deserializations(serializable_children.clone());
    let serde_deserializations = generate_serde_deserializations(serializable_children.clone());
    let path_exists_getters = generate_path_exists_getters(serializable_children.clone());
    let field_exists_getters = generate_field_exists_getters(serializable_children.clone());
    let field_chains = generate_field_chains(serializable_children.clone());

    let implementation = quote! {
        impl #impl_generics serialize_hierarchy::SerializeHierarchy for #name #ty_generics #where_clause {
            fn serialize_path<S>(
                &self,
                path: &str,
                serializer: S,
            ) -> Result<S::Ok, serialize_hierarchy::Error<S::Error>>
            where
                S: serde::Serializer,
            {
                use serde::Serialize;
                let split = path.split_once('.');
                match split {
                    Some((name, suffix)) => match name {
                        #(#path_serializations,)*
                        segment => Err(serialize_hierarchy::Error::UnexpectedPathSegment {
                            segment: segment.to_string(),
                        }),
                    },
                    None => {
                        match path {
                            #(#serde_serializations,)*
                            segment => Err(serialize_hierarchy::Error::UnexpectedPathSegment {
                                segment: segment.to_string(),
                            }),
                        }
                    }
                }
            }

            fn deserialize_path<'de, D>(
                &mut self,
                path: &str,
                deserializer: D,
            ) -> Result<(), serialize_hierarchy::Error<D::Error>>
            where
                D: serde::Deserializer<'de>,
            {
                use serde::Deserialize;
                let split = path.split_once('.');
                match split {
                    Some((name, suffix)) => match name {
                        #(#path_deserializations,)*
                        name => Err(serialize_hierarchy::Error::UnexpectedPathSegment {
                            segment: name.to_string(),
                        }),
                    },
                    None => match path {
                        #(#serde_deserializations,)*
                        name => Err(serialize_hierarchy::Error::UnexpectedPathSegment {
                            segment: name.to_string(),
                        }),
                    },
                }
            }

            fn exists(path: &str) -> bool {
                let split = path.split_once('.');
                match split {
                    Some((name, suffix)) => match name {
                        #(#path_exists_getters,)*
                        _ => false,
                    },
                    None => match path {
                        #(#field_exists_getters,)*
                        _ => false,
                    },
                }
            }

            fn get_fields() -> std::collections::BTreeSet<String> {
                std::iter::empty::<std::string::String>()
                    #(#field_chains)*
                    .collect()
            }
        }
    };
    Ok(implementation)
}

fn generate_path_serializations<'a>(
    children: impl 'a + IntoIterator<Item = &'a Child>,
) -> impl 'a + Iterator<Item = TokenStream> {
    children.into_iter().map(|child| {
        let identifier = &child.identifier;
        let pattern = identifier.to_string();
        quote! {
            #pattern => self.#identifier.serialize_path(suffix, serializer)
        }
    })
}

fn generate_serde_serializations<'a>(
    children: impl 'a + IntoIterator<Item = &'a Child>,
) -> impl 'a + Iterator<Item = TokenStream> {
    children.into_iter().map(|child| {
        let identifier = &child.identifier;
        let pattern = identifier.to_string();
        quote! {
            #pattern => self.#identifier.serialize(serializer).map_err(serialize_hierarchy::Error::SerializationFailed)
        }
    })
}

fn generate_path_deserializations<'a>(
    children: impl 'a + IntoIterator<Item = &'a Child>,
) -> impl 'a + Iterator<Item = TokenStream> {
    children.into_iter().map(|child| {
        let identifier = &child.identifier;
        let pattern = identifier.to_string();
        quote! {
            #pattern => self.#identifier.deserialize_path(suffix, deserializer)
        }
    })
}

fn generate_serde_deserializations<'a>(
    children: impl 'a + IntoIterator<Item = &'a Child>,
) -> impl 'a + Iterator<Item = TokenStream> {
    children.into_iter().map(|child| {
        let identifier = &child.identifier;
        let pattern = identifier.to_string();
        let ty = &child.ty;
        quote! {
            #pattern => {
                self.#identifier = <#ty as Deserialize>::deserialize(deserializer).map_err(serialize_hierarchy::Error::DeserializationFailed)?;
                Ok(())
            }

        }
    })
}

fn generate_path_exists_getters<'a>(
    children: impl 'a + IntoIterator<Item = &'a Child>,
) -> impl 'a + Iterator<Item = TokenStream> {
    children.into_iter().map(|child| {
        let pattern = child.identifier.to_string();
        let ty = &child.ty;
        quote! {
            #pattern => <#ty as serialize_hierarchy::SerializeHierarchy>::exists(suffix)
        }
    })
}

fn generate_field_exists_getters<'a>(
    children: impl 'a + IntoIterator<Item = &'a Child>,
) -> impl 'a + Iterator<Item = TokenStream> {
    children.into_iter().map(|child| {
        let pattern = child.identifier.to_string();
        quote! {
            #pattern => true
        }
    })
}

fn generate_field_chains<'a>(
    children: impl 'a + IntoIterator<Item = &'a Child>,
) -> impl 'a + Iterator<Item = TokenStream> {
    children.into_iter().map(|child| {
        let identifier = &child.identifier;
        let name_string = identifier.to_string();
        let pattern = format!("{}.{{}}", identifier);
        let ty = &child.ty;
        quote! {
            .chain(std::iter::once(#name_string.to_string()))
            .chain(
                <#ty as serialize_hierarchy::SerializeHierarchy>::get_fields()
                    .into_iter()
                    .map(|name| format!(#pattern, name))
            )
        }
    })
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum TypeAttribute {
    AsJpeg,
}

fn parse_attributes(attrs: &[syn::Attribute]) -> syn::Result<HashSet<TypeAttribute>> {
    let meta_items = attrs
        .iter()
        .map(parse_meta_items)
        .collect::<Result<Vec<_>, _>>()?;

    meta_items
        .into_iter()
        .flatten()
        .map(|meta| match meta {
            NestedMeta::Meta(Meta::Path(word)) if word.is_ident(AS_JPEG) => {
                Ok(TypeAttribute::AsJpeg)
            }
            NestedMeta::Meta(meta_item) => {
                let path = meta_item
                    .path()
                    .into_token_stream()
                    .to_string()
                    .replace(' ', "");
                let message = format!("unknown attribute `{}`", path);
                Err(syn::Error::new_spanned(meta_item.path(), message))
            }

            NestedMeta::Lit(lit) => {
                let message = "unexpected literal in attribute";
                Err(syn::Error::new_spanned(lit, message))
            }
        })
        .collect::<Result<HashSet<_>, _>>()
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum ChildAttribute {
    Skip,
}

#[derive(Debug)]
struct Child {
    attributes: HashSet<ChildAttribute>,
    identifier: Ident,
    ty: Type,
}

fn parse_meta_items(attribute: &syn::Attribute) -> syn::Result<Vec<NestedMeta>> {
    if !attribute.path.is_ident(SERIALIZE_HIERARCHY) {
        return Ok(Vec::new());
    }
    match attribute.parse_meta() {
        Ok(Meta::List(meta)) => Ok(meta.nested.into_iter().collect()),
        Ok(other) => Err(syn::Error::new_spanned(
            other,
            "expected #[serialize_hierarchy(...)]",
        )),
        Err(error) => Err(error),
    }
}

fn read_children(input: &DataStruct) -> syn::Result<Vec<Child>> {
    input
        .fields
        .iter()
        .map(|field| {
            let meta_items = field
                .attrs
                .iter()
                .map(parse_meta_items)
                .collect::<Result<Vec<_>, _>>()?;

            let attributes = meta_items
                .into_iter()
                .flatten()
                .map(|meta| match meta {
                    NestedMeta::Meta(Meta::Path(word)) if word.is_ident(SKIP) => {
                        Ok(ChildAttribute::Skip)
                    }
                    NestedMeta::Meta(meta_item) => {
                        let path = meta_item
                            .path()
                            .into_token_stream()
                            .to_string()
                            .replace(' ', "");
                        let message = format!("unknown attribute `{}`", path);
                        Err(syn::Error::new_spanned(meta_item.path(), message))
                    }

                    NestedMeta::Lit(lit) => {
                        let message = "unexpected literal in attribute";
                        Err(syn::Error::new_spanned(lit, message))
                    }
                })
                .collect::<Result<HashSet<_>, _>>()?;
            let identifier = field
                .ident
                .clone()
                .ok_or_else(|| syn::Error::new_spanned(field, "field has to be named"))?;
            let ty = field.ty.clone();
            Ok(Child {
                attributes,
                identifier,
                ty,
            })
        })
        .collect()
}
