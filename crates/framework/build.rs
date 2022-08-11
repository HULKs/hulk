use anyhow::Context;
use build_script_helpers::write_token_stream;
use convert_case::{Case, Casing};
use quote::{format_ident, quote};
use source_analyzer::{CyclerInstances, PerceptionCyclersInstances};

fn main() -> anyhow::Result<()> {
    let cycler_instances = CyclerInstances::try_from_crates_directory("..")
        .context("Failed to get cycler instances from crates directory")?;
    let perception_cycler_instances =
        PerceptionCyclersInstances::try_from_crates_directory("..")
            .context("Failed to get perception cycler instances from crates directory")?;

    let updates_fields = cycler_instances.instances_to_modules.iter().filter_map(|(instance_name, module_name)| {
        match perception_cycler_instances.perception_cycler_instances.contains(instance_name) {
            true => {
                let field_name_identifier = format_ident!("{}", instance_name.to_case(Case::Snake));
                let module_name_identifier = format_ident!("{}", module_name);
                Some(quote! { pub #field_name_identifier: Update<structs::#module_name_identifier::MainOutputs> })
            },
            false => None,
        }
    });
    let timestamp_array_items = cycler_instances
        .instances_to_modules
        .keys()
        .filter_map(|instance_name| {
            match perception_cycler_instances.perception_cycler_instances.contains(instance_name) {
                true => {
                    let field_name_identifier = format_ident!("{}", instance_name.to_case(Case::Snake));
                    Some(quote! { self.#field_name_identifier.first_timestamp_of_non_finalized_database })
                },
                false => None,
            }
        });
    let push_loops = cycler_instances
        .instances_to_modules
        .keys()
        .filter_map(|instance_name| {
            match perception_cycler_instances
                .perception_cycler_instances
                .contains(instance_name)
            {
                true => {
                    let field_name_identifier =
                        format_ident!("{}", instance_name.to_case(Case::Snake));
                    Some(quote! {
                        for timestamped_database in self.#field_name_identifier.items {
                            databases
                                .get_mut(&timestamped_database.timestamp)
                                .unwrap()
                                .#field_name_identifier
                                .push(timestamped_database.data);
                        }
                    })
                }
                false => None,
            }
        });
    let databases_fields = cycler_instances.instances_to_modules.iter().filter_map(|(instance_name, module_name)| {
        match perception_cycler_instances.perception_cycler_instances.contains(instance_name) {
            true => {
                let field_name_identifier = format_ident!("{}", instance_name.to_case(Case::Snake));
                let module_name_identifier = format_ident!("{}", module_name);
                Some(quote! { pub #field_name_identifier: Vec<structs::#module_name_identifier::MainOutputs> })
            },
            false => None,
        }
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
