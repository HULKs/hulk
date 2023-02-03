use std::collections::BTreeMap;

use build_script_helpers::write_token_stream;
use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};
use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use source_analyzer::{cycler_crates_from_crates_directory, StructHierarchy, Structs};

fn main() -> Result<()> {
    for crate_directory in cycler_crates_from_crates_directory("..")
        .wrap_err("failed to get cycler crate directories from crates directory")?
    {
        println!("cargo:rerun-if-changed={}", crate_directory.display());
    }

    let structs = Structs::try_from_crates_directory("..")
        .wrap_err("failed to get structs from crates directory")?;

    let parameters = match &structs.parameters {
        StructHierarchy::Struct { fields } => {
            let structs = struct_hierarchy_to_token_stream(
                "Parameters",
                fields,
                quote! { #[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, serialize_hierarchy::SerializeHierarchy)] },
            )
            .wrap_err("failed to generate struct `Parameters`")?;
            quote! {
                #structs
            }
        }
        StructHierarchy::Optional { .. } => bail!("unexpected optional variant as root-struct"),
        StructHierarchy::Field { .. } => bail!("unexpected field variant as root-struct"),
    };
    let cyclers = structs
        .cycler_structs
        .iter()
        .map(|(cycler_module, cycler_structs)| {
            let cycler_module_identifier = format_ident!("{}", cycler_module);
            let main_outputs = match &cycler_structs.main_outputs {
                StructHierarchy::Struct { fields } => struct_hierarchy_to_token_stream(
                    "MainOutputs",
                    fields,
                    quote! { #[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, serialize_hierarchy::SerializeHierarchy)] },
                )
                .wrap_err("failed to generate struct `MainOutputs`")?,
                StructHierarchy::Optional { .. } => {
                    bail!("unexpected optional variant as root-struct")
                }
                StructHierarchy::Field { .. } => bail!("unexpected field variant as root-struct"),
            };
            let additional_outputs = match &cycler_structs.additional_outputs {
                StructHierarchy::Struct { fields } => struct_hierarchy_to_token_stream(
                    "AdditionalOutputs",
                    fields,
                    quote! { #[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, serialize_hierarchy::SerializeHierarchy)] },
                )
                .wrap_err("failed to generate struct `AdditionalOutputs`")?,
                StructHierarchy::Optional { .. } => {
                    bail!("unexpected optional variant as root-struct")
                }
                StructHierarchy::Field { .. } => bail!("unexpected field variant as root-struct"),
            };
            let persistent_state = match &cycler_structs.persistent_state {
                StructHierarchy::Struct { fields } => struct_hierarchy_to_token_stream(
                    "PersistentState",
                    fields,
                    quote! { #[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, serialize_hierarchy::SerializeHierarchy)] },
                )
                .wrap_err("failed to generate struct `PersistentState`")?,
                StructHierarchy::Optional { .. } => {
                    bail!("unexpected optional variant as root-struct")
                }
                StructHierarchy::Field { .. } => bail!("unexpected field variant as root-struct"),
            };

            Ok(quote! {
                pub mod #cycler_module_identifier {
                    #main_outputs
                    #additional_outputs
                    #persistent_state
                }
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let token_stream = quote! {
        #parameters
        #(#cyclers)*
    };

    write_token_stream("structs.rs", token_stream)
        .wrap_err("failed to write perception databases structs")?;

    Ok(())
}

fn struct_hierarchy_to_token_stream(
    struct_name: &str,
    fields: &BTreeMap<String, StructHierarchy>,
    derives: TokenStream,
) -> Result<TokenStream> {
    let struct_name_identifier = format_ident!("{}", struct_name);
    let struct_fields: Vec<_> = fields
        .iter()
        .map(|(name, struct_hierarchy)| {
            let name_identifier = format_ident!("{}", name);
            match struct_hierarchy {
                StructHierarchy::Struct { .. } => {
                    let struct_name_identifier =
                        format_ident!("{}{}", struct_name, name.to_case(Case::Pascal));
                    Ok(quote! { pub #name_identifier: #struct_name_identifier })
                }
                StructHierarchy::Optional { child } => match &**child {
                    StructHierarchy::Struct { .. } => {
                        let struct_name_identifier =
                            format_ident!("{}{}", struct_name, name.to_case(Case::Pascal));
                        Ok(quote! { pub #name_identifier: Option<#struct_name_identifier> })
                    }
                    StructHierarchy::Optional { .. } => {
                        bail!("unexpected optional in an optional struct")
                    }
                    StructHierarchy::Field { data_type } => {
                        Ok(quote! { pub #name_identifier: Option<#data_type> })
                    }
                },
                StructHierarchy::Field { data_type } => {
                    Ok(quote! { pub #name_identifier: #data_type })
                }
            }
        })
        .collect::<Result<_, _>>()
        .wrap_err("failed to generate struct fields")?;
    let child_structs: Vec<_> = fields
        .iter()
        .map(|(name, struct_hierarchy)| match struct_hierarchy {
            StructHierarchy::Struct { fields } => {
                let struct_name = format!("{}{}", struct_name, name.to_case(Case::Pascal));
                struct_hierarchy_to_token_stream(&struct_name, fields, derives.clone())
                    .wrap_err_with(|| format!("failed to generate struct `{struct_name}`"))
            }
            StructHierarchy::Optional { child } => match &**child {
                StructHierarchy::Struct { fields } => {
                    let struct_name = format!("{}{}", struct_name, name.to_case(Case::Pascal));
                    struct_hierarchy_to_token_stream(&struct_name, fields, derives.clone())
                        .wrap_err_with(|| format!("failed to generate struct `{struct_name}`"))
                }
                StructHierarchy::Optional { .. } => {
                    bail!("unexpected optional in an optional struct")
                }
                StructHierarchy::Field { .. } => Ok(Default::default()),
            },
            StructHierarchy::Field { .. } => Ok(Default::default()),
        })
        .collect::<Result<_, _>>()
        .wrap_err("failed to generate child structs")?;

    Ok(quote! {
        #derives
        pub struct #struct_name_identifier {
            #(#struct_fields,)*
        }
        #(#child_structs)*
    })
}
