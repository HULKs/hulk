use std::{fs::File, io::Write};

use anyhow::Context;
use build_script_helpers::write_token_stream;
use convert_case::{Case, Casing};
use quote::{format_ident, quote};
use source_analyzer::CyclerInstances;

fn main() -> anyhow::Result<()> {
    let cycler_instances = CyclerInstances::try_from_crates_directory("..")
        .context("Failed to get cycler instances from crates directory")?;
    let mut file = File::create("build.rs.log").unwrap();
    writeln!(file, "cycler_instances: {cycler_instances:?}").unwrap();

    let updates_fields = cycler_instances.instances_to_modules.iter().map(|(instance_name, module_name)| {
        let field_name_identifier = format_ident!("{}", instance_name.to_case(Case::Snake));
        let module_name_identifier = format_ident!("{}", module_name);
        quote! { pub #field_name_identifier: Update<structs::#module_name_identifier::MainOutputs> }
    });
    let timestamp_array_items = cycler_instances
        .instances_to_modules
        .keys()
        .map(|instance_name| {
            let field_name_identifier = format_ident!("{}", instance_name.to_case(Case::Snake));
            quote! { self.#field_name_identifier.first_timestamp_of_non_finalized_database }
        });
    let push_loops = cycler_instances
        .instances_to_modules
        .keys()
        .map(|instance_name| {
            let field_name_identifier = format_ident!("{}", instance_name.to_case(Case::Snake));
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
    let databases_fields = cycler_instances.instances_to_modules.iter().map(|(instance_name, module_name)| {
        let field_name_identifier = format_ident!("{}", instance_name.to_case(Case::Snake));
        let module_name_identifier = format_ident!("{}", module_name);
        quote! { pub #field_name_identifier: Vec<structs::#module_name_identifier::MainOutputs> }
    });

    write_token_stream(
        "perception_databases_structs.rs",
        quote! {
            pub struct Updates {
                #(#updates_fields,)*
            }

            impl Updates {
                fn first_timestamp_of_temporary_databases(&self) -> Option<SystemTime> {
                    [
                        #(#timestamp_array_items,)*
                    ]
                    .iter()
                    .copied()
                    .flatten()
                    .min()
                }

                fn push_to_databases(self, databases: &mut BTreeMap<SystemTime, Databases>) {
                    #(#push_loops)*
                }
            }

            pub struct Update<MainOutputs> {
                pub items: Vec<Item<MainOutputs>>,
                pub first_timestamp_of_non_finalized_database: Option<SystemTime>,
            }

            #[derive(Default)]
            pub struct Databases {
                #(#databases_fields,)*
            }
        },
    )
    .context("Failed to write perception databases structs")?;

    Ok(())
}
