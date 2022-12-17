use proc_macro2::TokenStream;
use proc_macro_error::abort_call_site;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, token::Colon2, Data, DeriveInput, Fields, PathArguments, Type};

pub fn process_serialize_hierarchy_implementation(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let fields = match input.data {
        Data::Struct(data) => data.fields,
        _ => abort_call_site!("`SerializeHierarchy` can only be derived for `struct`"),
    };
    let serde_serialization = generate_serde_serialization(&fields);
    let path_serialization = generate_path_serialization(&fields);
    let serde_deserialization = generate_serde_deserialization(&fields);
    let path_deserialization = generate_path_deserialization(&fields);
    let path_exists_getter = generate_path_exists_getter(&fields);
    let field_exists_getter = generate_field_exists_getter(&fields);
    let hierarchy_insertions = generate_hierarchy_insertions(&fields);

    let expanded = quote! {
        impl #impl_generics serialize_hierarchy::SerializeHierarchy for #name #ty_generics #where_clause {
            fn serialize_hierarchy(&self, field_path: &str) -> color_eyre::eyre::Result<serde_json::Value> {
                use color_eyre::eyre::WrapErr;
                let split = field_path.split_once(".");
                match split {
                    Some((field_name, suffix)) => match field_name {
                        #(#path_serialization,)*
                        _ => color_eyre::eyre::bail!("no such field in type: `{}`", field_path),
                    },
                    None => match field_path {
                        #(#serde_serialization,)*
                        _ => color_eyre::eyre::bail!("no such field in type: `{}`", field_path),
                    },
                }
            }

            fn deserialize_hierarchy(&mut self, field_path: &str, data: serde_json::Value) -> color_eyre::eyre::Result<()> {
                use color_eyre::eyre::WrapErr;
                let split = field_path.split_once(".");
                match split {
                    Some((field_name, suffix)) => match field_name {
                        #(#path_deserialization,)*
                        _ => color_eyre::eyre::bail!("no such field in type: `{}`", field_path),
                    },
                    None => match field_path {
                        #(#serde_deserialization,)*
                        _ => color_eyre::eyre::bail!("no such field in type: `{}`", field_path),
                    },
                }
            }

            fn exists(field_path: &str) -> bool {
                let split = field_path.split_once(".");
                match split {
                    Some((field_name, suffix)) => match field_name {
                        #(#path_exists_getter,)*
                        _ => false,
                    },
                    None => match field_path {
                        #(#field_exists_getter,)*
                        _ => false,
                    },
                }
            }

            fn get_hierarchy() -> serialize_hierarchy::HierarchyType {
                let mut fields = std::collections::BTreeMap::new();
                #(#hierarchy_insertions;)*
                serialize_hierarchy::HierarchyType::Struct {
                    fields,
                }
            }
        }
    };

    expanded.into()
}

fn generate_serde_serialization(fields: &Fields) -> Vec<TokenStream> {
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
            let error_message = format!("failed to serialize field `{name}`");
            Some(quote! {
                #pattern => serde_json::to_value(&self.#name).wrap_err(#error_message)
            })
        })
        .collect()
}

fn generate_path_serialization(fields: &Fields) -> Vec<TokenStream> {
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
            let is_leaf = field
                .attrs
                .iter()
                .any(|attribute| attribute.path.is_ident("leaf"));
            let name = field.ident.as_ref().unwrap();
            let pattern = name.to_string();
            let code = if is_leaf {
                quote! {
                    #pattern => color_eyre::eyre::bail!("cannot access leaf node with path `{}`", suffix)
                }
            } else {
                let error_message = format!("failed to serialize field `{name}`");
                quote! {
                    #pattern => self.#name.serialize_hierarchy(suffix).wrap_err(#error_message)
                }
            };
            Some(code)
        })
        .collect()
}

fn generate_serde_deserialization(fields: &Fields) -> Vec<TokenStream> {
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
            let error_message = format!("failed to deserialize field `{name}`");
            Some(quote! {
                #pattern => {
                    self.#name = serde_json::from_value(data).wrap_err(#error_message)?;
                    Ok(())
                }
            })
        })
        .collect()
}

fn generate_path_deserialization(fields: &Fields) -> Vec<TokenStream> {
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
            let is_leaf = field
                .attrs
                .iter()
                .any(|attribute| attribute.path.is_ident("leaf"));
            let name = field.ident.as_ref().unwrap();
            let pattern = name.to_string();
            let error_message = format!("failed to deserialize field `{name}`");
            let code = if is_leaf {
                quote! {
                    #pattern => color_eyre::eyre::bail!("cannot access leaf node with path `{}`", suffix)
                }
            } else {
                quote! {
                    #pattern => self.#name.deserialize_hierarchy(suffix, data).wrap_err(#error_message)
                }
            };
            Some(code)
        })
        .collect()
}

fn generate_hierarchy_insertions(fields: &Fields) -> Vec<TokenStream> {
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
            let is_leaf = field
                .attrs
                .iter()
                .any(|attribute| attribute.path.is_ident("leaf"));
            let name = field.ident.as_ref().unwrap();
            let pattern = name.to_string();
            let code = if is_leaf {
                quote! {
                    fields.insert(#pattern.to_string(), serialize_hierarchy::HierarchyType::GenericStruct)
                }
            } else {
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
                quote! {
                    fields.insert(#pattern.to_string(), #field_type::get_hierarchy())
                }
            };
            Some(code)
        })
        .collect()
}

fn generate_field_exists_getter(fields: &Fields) -> Vec<TokenStream> {
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

fn generate_path_exists_getter(fields: &Fields) -> Vec<TokenStream> {
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
            let is_leaf = field
                .attrs
                .iter()
                .any(|attribute| attribute.path.is_ident("leaf"));
            if is_leaf {
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
