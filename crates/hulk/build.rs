use std::{
    env::var,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use code_generation::{
    cycler::generate_cyclers, run::generate_run_function, structs::hierarchy_to_token_stream,
};
use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};
use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use source_analyzer::{
    cycler::{CyclerKind, Cyclers},
    structs::Structs,
};

fn main() -> Result<()> {
    let cyclers = Cyclers::try_from_directory(".")?;
    code_cyclers(&cyclers)?;
    code_structs(&cyclers)?;
    code_perception_databases_structs(&cyclers)?;
    Ok(())
}

pub fn write_token_stream(file_name: impl AsRef<Path>, token_stream: TokenStream) -> Result<()> {
    let file_path =
        PathBuf::from(var("OUT_DIR").wrap_err("failed to get environment variable OUT_DIR")?)
            .join(file_name);

    {
        let mut file = File::create(&file_path)
            .wrap_err_with(|| format!("failed create file {file_path:?}"))?;
        write!(file, "{token_stream}")
            .wrap_err_with(|| format!("failed to write to file {file_path:?}"))?;
    }

    let status = Command::new("rustfmt")
        .arg(file_path)
        .status()
        .wrap_err("failed to execute rustfmt")?;
    if !status.success() {
        bail!("rustfmt did not exit with success");
    }

    Ok(())
}

fn code_cyclers(cyclers: &Cyclers) -> Result<()> {
    let cyclers_token_stream = generate_cyclers(cyclers).wrap_err("failed to generate cyclers")?;
    let runtime_token_stream = generate_run_function(cyclers);

    write_token_stream(
        "cyclers.rs",
        quote! {
            #cyclers_token_stream
            #runtime_token_stream
        },
    )
    .wrap_err("failed to write cyclers")?;

    Ok(())
}

fn code_structs(cyclers: &Cyclers) -> Result<()> {
    let structs = Structs::try_from_cyclers(cyclers)?;

    let configuration = hierarchy_to_token_stream(
        &structs.configuration,
        format_ident!("Configuration"),
        quote! { #[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, serialize_hierarchy::SerializeHierarchy)] },
    );
    let cyclers = structs
        .cyclers
        .iter()
        .map(|(cycler_module, cycler_structs)| {
            let cycler_module_identifier = format_ident!("{}", cycler_module);
            let main_outputs = hierarchy_to_token_stream(&cycler_structs.main_outputs,format_ident!("MainOutputs"), quote! { #[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, serialize_hierarchy::SerializeHierarchy)] }, ) ;
            let additional_outputs = hierarchy_to_token_stream(
                    &cycler_structs.additional_outputs,
                    format_ident!("AdditionalOutputs"),
                    quote! { #[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, serialize_hierarchy::SerializeHierarchy)] },
                )
            ;
            let persistent_state = hierarchy_to_token_stream(&cycler_structs.persistent_state,
                    format_ident!("PersistentState"),
                    quote! { #[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, serialize_hierarchy::SerializeHierarchy)] },
                )
            ;

            quote! {
                pub mod #cycler_module_identifier {
                    #main_outputs
                    #additional_outputs
                    #persistent_state
                }
            }
        })
        ;

    let token_stream = quote! {
        #configuration
        #(#cyclers)*
    };

    write_token_stream("structs.rs", token_stream).wrap_err("failed to write structs")?;

    Ok(())
}

fn code_perception_databases_structs(cyclers: &Cyclers) -> Result<()> {
    let updates_fields = cyclers.instances_with(CyclerKind::Perception).map(
        |(cycler, instance)| {
            let field_name_identifier = format_ident!("{}", instance.name.to_case(Case::Snake));
            let module_name_identifier = format_ident!("{}", cycler.module);
            quote! {
                pub #field_name_identifier: framework::Update<crate::structs::#module_name_identifier::MainOutputs>
            }
        },
    );
    let timestamp_array_items =
        cyclers
            .instances_with(CyclerKind::Perception)
            .map(|(_cycler, instance)| {
                let field_name_identifier = format_ident!("{}", instance.name.to_case(Case::Snake));
                quote! {
                    self.#field_name_identifier.first_timestamp_of_non_finalized_database
                }
            });
    let push_loops = cyclers
        .instances_with(CyclerKind::Perception)
        .map(|(_cycler, instance)| {
            let field_name_identifier = format_ident!("{}", instance.name.to_case(Case::Snake));
            quote! {
                for timestamped_database in self.#field_name_identifier.items {
                    databases
                        .get_mut(&timestamped_database.timestamp)
                        .unwrap()
                        .#field_name_identifier
                        .push(timestamped_database.data);
                }
            }
        });
    let databases_fields =
        cyclers
            .instances_with(CyclerKind::Perception)
            .map(|(cycler, instance)| {
                let field_name_identifier = format_ident!("{}", instance.name.to_case(Case::Snake));
                let module_name_identifier = format_ident!("{}", cycler.module);
                quote! {
                    pub #field_name_identifier: Vec<crate::structs::#module_name_identifier::MainOutputs>
                }
            });

    write_token_stream(
        "perception_databases.rs",
        quote! {
            pub struct Updates {
                #(#updates_fields,)*
            }

            impl framework::Updates<Databases> for Updates {
                fn first_timestamp_of_temporary_databases(&self) -> Option<std::time::SystemTime> {
                    [
                        #(#timestamp_array_items,)*
                    ]
                    .iter()
                    .copied()
                    .flatten()
                    .min()
                }

                fn push_to_databases(self, databases: &mut std::collections::BTreeMap<std::time::SystemTime, Databases>) {
                    #(#push_loops)*
                }
            }

            #[derive(Default)]
            pub struct Databases {
                #(#databases_fields,)*
            }
        },
    )
    .wrap_err("failed to write perception databases structs")?;

    Ok(())
}
