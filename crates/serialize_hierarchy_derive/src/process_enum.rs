use quote::quote;
use syn::{DataEnum, DeriveInput};

pub fn process_enum(input: &DeriveInput, _data: &DataEnum) -> proc_macro::TokenStream {
    let name = &input.ident;
    let name_string = name.to_string();
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics serialize_hierarchy::SerializeHierarchy for #name #ty_generics #where_clause {
            fn serialize_path<S>(
                &self,
                path: &str,
            ) -> Result<S::Serialized, serialize_hierarchy::Error<S::Error>>
            where
                S: serialize_hierarchy::Serializer,
                S::Error: std::error::Error,
            {
                Err(serialize_hierarchy::Error::TypeDoesNotSupportSerialization {
                    type_name: #name_string,
                    path: path.to_string(),
                })
            }

            fn deserialize_path<S>(
                &mut self,
                path: &str,
                _data: S::Serialized,
            ) -> Result<(), serialize_hierarchy::Error<S::Error>>
            where
                S: serialize_hierarchy::Serializer,
                S::Error: std::error::Error,
            {
                Err(serialize_hierarchy::Error::TypeDoesNotSupportDeserialization {
                    type_name: #name_string,
                    path: path.to_string(),
                })
            }

            fn exists(_path: &str) -> bool {
                false
            }

            fn get_fields() -> std::collections::BTreeSet<String> {
                [std::string::String::new()].into()
            }
        }
    }.into()
}
