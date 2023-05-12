use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use source_analyzer::cycler::{CyclerKind, Cyclers};

fn generate_perception_updates(cyclers: &Cyclers) -> TokenStream {
    let updates_fields = cyclers.instances_with(CyclerKind::Perception).map(
        |(cycler, instance)| {
            let field_name_identifier = format_ident!("{}", instance.name.to_case(Case::Snake));
            let cycler_module_name = format_ident!("{}", cycler.name.to_case(Case::Snake));
            quote! {
                pub #field_name_identifier: framework::Update<crate::structs::#cycler_module_name::MainOutputs>
            }
        },
    );
    let mut timestamp_array_items = cyclers
        .instances_with(CyclerKind::Perception)
        .map(|(_cycler, instance)| {
            let field_name_identifier = format_ident!("{}", instance.name.to_case(Case::Snake));
            quote! {
                self.#field_name_identifier.first_timestamp_of_non_finalized_database
            }
        })
        .peekable();
    let find_min_timestamp = if timestamp_array_items.peek().is_some() {
        quote! {
                [
                    #(#timestamp_array_items,)*
                ]
                .iter()
                .copied()
                .flatten()
                .min()
        }
    } else {
        quote! {
            None
        }
    };
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

    quote! {
        pub struct Updates {
            #(#updates_fields,)*
        }

        impl framework::Updates<Databases> for Updates {
            fn first_timestamp_of_temporary_databases(&self) -> Option<std::time::SystemTime> {
                #find_min_timestamp
            }

            fn push_to_databases(self, databases: &mut std::collections::BTreeMap<std::time::SystemTime, Databases>) {
                #(#push_loops)*
            }
        }
    }
}

pub fn generate_perception_databases(cyclers: &Cyclers) -> TokenStream {
    let perception_updates = generate_perception_updates(cyclers);
    let databases_fields = cyclers.instances_with(CyclerKind::Perception).map(
        |(cycler, instance)| {
            let field_name_identifier = format_ident!("{}", instance.name.to_case(Case::Snake));
            let cycler_module_name = format_ident!("{}", cycler.name.to_case(Case::Snake));
            quote! {
                pub #field_name_identifier: Vec<crate::structs::#cycler_module_name::MainOutputs>
            }
        },
    );

    quote! {
        #perception_updates

        #[derive(Default)]
        pub struct Databases {
            #(#databases_fields,)*
        }
    }
}
