use quote::quote;
use syn::{DataEnum, DeriveInput};

pub fn process_enum(input: &DeriveInput, _data: &DataEnum) -> proc_macro::TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics serialize_hierarchy::SerializeHierarchy for #name #ty_generics #where_clause {
            fn serialize_hierarchy(
                &self,
                field_path: &str
            ) -> color_eyre::Result<serialize_hierarchy::serde_json::Value> {
                color_eyre::eyre::bail!("Cannot access enum with path `{}`", field_path)
            }

            fn deserialize_hierarchy(
                &mut self,
                field_path: &str,
                data: serialize_hierarchy::serde_json::Value
            ) -> color_eyre::Result<()> {
                color_eyre::eyre::bail!("Cannot access enum with path `{}`", field_path)
            }

            fn exists(field_path: &str) -> bool {
                false
            }

            fn get_hierarchy() -> serialize_hierarchy::HierarchyType {
                serialize_hierarchy::HierarchyType::GenericEnum
            }
        }
    };

    expanded.into()
}
