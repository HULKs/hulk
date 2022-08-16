use std::{collections::BTreeMap, env::var, fs::File, io::Write, path::PathBuf, process::Command};

use anyhow::{anyhow, bail, Context};
use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use source_analyzer::{StructHierarchy, Structs};

fn main() -> anyhow::Result<()> {
    let structs = Structs::try_from_crates_directory("..")
        .context("Failed to get structs from crates directory")?;

    let configuration = match &structs.configuration {
        StructHierarchy::Struct { fields } => {
            struct_hierarchy_to_token_stream("Configuration", fields)
                .context("Failed to generate struct `Configuration`")?
        }
        StructHierarchy::Optional { .. } => bail!("Unexpected optional variant as root-struct"),
        StructHierarchy::Field { .. } => bail!("Unexpected field variant as root-struct"),
    };
    let cyclers = structs
        .cycler_structs
        .iter()
        .map(|(cycler_module, cycler_structs)| {
            let cycler_module_identifier = format_ident!("{}", cycler_module);
            let main_outputs = match &cycler_structs.main_outputs {
                StructHierarchy::Struct { fields } => {
                    struct_hierarchy_to_token_stream("MainOutputs", fields)
                        .context("Failed to generate struct `MainOutputs`")?
                }
                StructHierarchy::Optional { .. } => {
                    bail!("Unexpected optional variant as root-struct")
                }
                StructHierarchy::Field { .. } => bail!("Unexpected field variant as root-struct"),
            };
            let additional_outputs = match &cycler_structs.additional_outputs {
                StructHierarchy::Struct { fields } => {
                    struct_hierarchy_to_token_stream("AdditionalOutputs", fields)
                        .context("Failed to generate struct `AdditionalOutputs`")?
                }
                StructHierarchy::Optional { .. } => {
                    bail!("Unexpected optional variant as root-struct")
                }
                StructHierarchy::Field { .. } => bail!("Unexpected field variant as root-struct"),
            };
            let persistent_state = match &cycler_structs.persistent_state {
                StructHierarchy::Struct { fields } => {
                    struct_hierarchy_to_token_stream("PersistentState", fields)
                        .context("Failed to generate struct `PersistentState`")?
                }
                StructHierarchy::Optional { .. } => {
                    bail!("Unexpected optional variant as root-struct")
                }
                StructHierarchy::Field { .. } => bail!("Unexpected field variant as root-struct"),
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
        #configuration
        #(#cyclers)*
    };

    let file_path =
        PathBuf::from(var("OUT_DIR").context("Failed to get environment variable OUT_DIR")?)
            .join("structs.rs");
    {
        let mut file = File::create(&file_path)
            .with_context(|| anyhow!("Failed create file {file_path:?}"))?;
        write!(file, "{}", token_stream)
            .with_context(|| anyhow!("Failed to write to file {file_path:?}"))?;
    }

    let status = Command::new("rustfmt")
        .arg(file_path)
        .status()
        .context("Failed to execute rustfmt")?;
    if !status.success() {
        bail!("rustfmt did not exit with success");
    }

    Ok(())
}

fn struct_hierarchy_to_token_stream(
    struct_name: &str,
    fields: &BTreeMap<String, StructHierarchy>,
) -> anyhow::Result<TokenStream> {
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
                        bail!("Unexpected optional in an optional struct")
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
        .context("Failed to generate struct fields")?;
    let child_structs: Vec<_> = fields
        .iter()
        .map(|(name, struct_hierarchy)| match struct_hierarchy {
            StructHierarchy::Struct { fields } => {
                let struct_name = format!("{}{}", struct_name, name.to_case(Case::Pascal));
                struct_hierarchy_to_token_stream(&struct_name, &fields)
                    .with_context(|| anyhow!("Failed to generate struct `{struct_name}`"))
            }
            StructHierarchy::Optional { child } => match &**child {
                StructHierarchy::Struct { fields } => {
                    let struct_name = format!("{}{}", struct_name, name.to_case(Case::Pascal));
                    struct_hierarchy_to_token_stream(&struct_name, &fields)
                        .with_context(|| anyhow!("Failed to generate struct `{struct_name}`"))
                }
                StructHierarchy::Optional { .. } => {
                    bail!("Unexpected optional in an optional struct")
                }
                StructHierarchy::Field { .. } => Ok(Default::default()),
            },
            StructHierarchy::Field { .. } => Ok(Default::default()),
        })
        .collect::<Result<_, _>>()
        .context("Failed to generate child structs")?;

    Ok(quote! {
        #[derive(Clone, Debug, Default)]
        pub struct #struct_name_identifier {
            #(#struct_fields,)*
        }
        #(#child_structs)*
    })
}
