use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use source_analyzer::struct_hierarchy::StructHierarchy;

pub fn hierarchy_to_token_stream(
    hierarchy: &StructHierarchy,
    struct_name: Ident,
    derives: TokenStream,
) -> TokenStream {
    let fields = match hierarchy {
        StructHierarchy::Struct { fields } => fields,
        StructHierarchy::Optional { .. } => panic!("option instead of struct"),
        StructHierarchy::Field { .. } => panic!("field instead of struct"),
    };
    let struct_fields = fields.iter().map(|(name, struct_hierarchy)| {
        let name_identifier = format_ident!("{}", name);
        match struct_hierarchy {
            StructHierarchy::Struct { .. } => {
                let struct_name_identifier =
                    format_ident!("{}{}", struct_name, name.to_case(Case::Pascal));
                quote! { pub #name_identifier: #struct_name_identifier }
            }
            StructHierarchy::Optional { child } => match &**child {
                StructHierarchy::Struct { .. } => {
                    let struct_name_identifier =
                        format_ident!("{}{}", struct_name, name.to_case(Case::Pascal));
                    quote! { pub #name_identifier: Option<#struct_name_identifier> }
                }
                StructHierarchy::Optional { .. } => {
                    panic!("unexpected optional in an optional struct")
                }
                StructHierarchy::Field { data_type } => {
                    quote! { pub #name_identifier: Option<#data_type> }
                }
            },
            StructHierarchy::Field { data_type } => {
                quote! { pub #name_identifier: #data_type }
            }
        }
    });
    let child_structs = fields.iter().map(|(name, struct_hierarchy)| {
        let struct_name = format_ident!("{}{}", struct_name, name.to_case(Case::Pascal));
        match struct_hierarchy {
            StructHierarchy::Struct { .. } => {
                hierarchy_to_token_stream(struct_hierarchy, struct_name, derives.clone())
            }
            StructHierarchy::Optional { child } => match &**child {
                StructHierarchy::Struct { .. } => {
                    hierarchy_to_token_stream(struct_hierarchy, struct_name, derives.clone())
                }
                StructHierarchy::Optional { .. } => {
                    panic!("unexpected optional in an optional struct")
                }
                StructHierarchy::Field { .. } => quote! {},
            },
            StructHierarchy::Field { .. } => quote! {},
        }
    });
    quote! {
        #derives
        pub struct #struct_name {
            #(#struct_fields,)*
        }
        #(#child_structs)*
    }
}
