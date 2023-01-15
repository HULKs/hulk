use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{token::Colon2, DataStruct, DeriveInput, Fields, PathArguments, Type};

pub fn process_struct(input: &DeriveInput, data: &DataStruct) -> proc_macro::TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let path_serializations = generate_path_serializations(&data.fields);
    let serde_serializations = generate_serde_serializations(&data.fields);
    let path_deserializations = generate_path_deserializations(&data.fields);
    let serde_deserializations = generate_serde_deserializations(&data.fields);
    let path_exists_getters = generate_path_exists_getters(&data.fields);
    let field_exists_getters = generate_field_exists_getters(&data.fields);
    let field_chains = generate_field_chains(&data.fields);

    let expanded = quote! {
        impl #impl_generics serialize_hierarchy::SerializeHierarchy for #name #ty_generics #where_clause {
            fn serialize_path<S>(
                &self,
                path: &str,
            ) -> Result<S::Serialized, serialize_hierarchy::Error<S::Error>>
            where
                S: serialize_hierarchy::Serializer,
                S::Error: std::error::Error,
            {
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

            fn deserialize_path<S>(
                &mut self,
                path: &str,
                data: S::Serialized,
            ) -> Result<(), serialize_hierarchy::Error<S::Error>>
            where
                S: serialize_hierarchy::Serializer,
                S::Error: std::error::Error,
            {
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
                std::iter::once(String::new())
                    #(#field_chains)*
                    .collect()
            }
        }
    };

    expanded.into()
}

fn generate_path_serializations(fields: &Fields) -> Vec<TokenStream> {
    fields
        .into_iter()
        .filter_map(|field| {
            let dont_serialize = field
                .attrs
                .iter()
                .any(|attribute| attribute.path.is_ident("dont_serialize"));
            if dont_serialize {
                return None;
            }
            let name = field.ident.as_ref().unwrap();
            let pattern = name.to_string();
            Some(quote! {
                #pattern => self.#name.serialize_path::<S>(suffix)
            })
        })
        .collect()
}

fn generate_serde_serializations(fields: &Fields) -> Vec<TokenStream> {
    fields
        .into_iter()
        .filter_map(|field| {
            let dont_serialize = field
                .attrs
                .iter()
                .any(|attribute| attribute.path.is_ident("dont_serialize"));
            if dont_serialize {
                return None;
            }
            let name = field.ident.as_ref().unwrap();
            let pattern = name.to_string();
            Some(quote! {
                #pattern => S::serialize(&self.#name).map_err(serialize_hierarchy::Error::SerializationFailed)
            })
        })
        .collect()
}

fn generate_path_deserializations(fields: &Fields) -> Vec<TokenStream> {
    fields
        .into_iter()
        .filter_map(|field| {
            let dont_serialize = field
                .attrs
                .iter()
                .any(|attribute| attribute.path.is_ident("dont_serialize"));
            if dont_serialize {
                return None;
            }
            let name = field.ident.as_ref().unwrap();
            let pattern = name.to_string();
            Some(quote! {
                #pattern => self.#name.deserialize_path::<S>(suffix, data)
            })
        })
        .collect()
}

fn generate_serde_deserializations(fields: &Fields) -> Vec<TokenStream> {
    fields
        .into_iter()
        .filter_map(|field| {
            let dont_serialize = field
                .attrs
                .iter()
                .any(|attribute| attribute.path.is_ident("dont_serialize"));
            if dont_serialize {
                return None;
            }
            let name = field.ident.as_ref().unwrap();
            let pattern = name.to_string();
            Some(quote! {
                #pattern => {
                    self.#name = S::deserialize(data).map_err(serialize_hierarchy::Error::DeserializationFailed)?;
                    Ok(())
                }
            })
        })
        .collect()
}

fn generate_path_exists_getters(fields: &Fields) -> Vec<TokenStream> {
    fields
        .into_iter()
        .filter_map(|field| {
            let dont_serialize = field
                .attrs
                .iter()
                .any(|attribute| attribute.path.is_ident("dont_serialize"));
            if dont_serialize {
                return None;
            }
            let name = field.ident.as_ref().unwrap();
            let pattern = name.to_string();
            let field_type = if let Type::Path(type_path) = &field.ty {
                let mut type_path = type_path.clone();
                type_path.path.segments.iter_mut().for_each(|segment| {
                    if let PathArguments::AngleBracketed(arguments) = &mut segment.arguments {
                        arguments.colon2_token = Some(Colon2::default());
                    }
                });
                type_path.into_token_stream()
            } else {
                field.ty.to_token_stream()
            };
            Some(quote! {
                #pattern => #field_type::exists(suffix)
            })
        })
        .collect()
}

fn generate_field_exists_getters(fields: &Fields) -> Vec<TokenStream> {
    fields
        .into_iter()
        .filter_map(|field| {
            let dont_serialize = field
                .attrs
                .iter()
                .any(|attribute| attribute.path.is_ident("dont_serialize"));
            if dont_serialize {
                return None;
            }
            let name = field.ident.as_ref().unwrap();
            let pattern = name.to_string();
            Some(quote! {
                #pattern => true
            })
        })
        .collect()
}

fn generate_field_chains(fields: &Fields) -> Vec<TokenStream> {
    fields
        .into_iter()
        .filter_map(|field| {
            let dont_serialize = field
                .attrs
                .iter()
                .any(|attribute| attribute.path.is_ident("dont_serialize"));
            if dont_serialize {
                return None;
            }
            let name = field.ident.as_ref().unwrap();
            let pattern = format!("{}.{{}}", name);
            let field_type = if let Type::Path(type_path) = &field.ty {
                let mut type_path = type_path.clone();
                type_path.path.segments.iter_mut().for_each(|segment| {
                    if let PathArguments::AngleBracketed(arguments) = &mut segment.arguments {
                            arguments.colon2_token = Some(Colon2::default());
                    }
                });
                type_path.into_token_stream()
            } else {
                field.ty.to_token_stream()
            };
            Some(quote! {
                .chain(
                    #field_type::get_fields()
                        .into_iter()
                        .map(|name| format!(#pattern, name))
                )
            })
        })
        .collect()
}
