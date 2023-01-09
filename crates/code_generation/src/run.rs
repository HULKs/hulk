use std::iter::repeat;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::{cycler::Cycler, other_cycler::OtherCycler};

pub fn generate_run(cyclers: &[Cycler]) -> TokenStream {
    let cycler_initializations: Vec<_> = cyclers
        .iter()
        .flat_map(|cycler| {
            cycler.get_cycler_instances().modules_to_instances[cycler.get_cycler_module_name()]
                .iter()
                .map(|cycler_instance| {
                    let cycler_instance_snake_case = cycler_instance.to_case(Case::Snake);
                    let cycler_database_changed_identifier = format_ident!("{}_changed", cycler_instance_snake_case);
                    let cycler_variable_identifier = format_ident!("{}_cycler", cycler_instance_snake_case);
                    let cycler_module_name_identifier = cycler.get_cycler_module_name_identifier();
                    let cycler_instance_identifier = format_ident!("{}", cycler_instance);
                    let own_writer_identifier = format_ident!("{}_writer", cycler_instance_snake_case);
                    let own_reader_identifier = format_ident!("{}_reader", cycler_instance_snake_case);
                    let own_subscribed_outputs_writer_identifier = format_ident!("{}_subscribed_outputs_writer", cycler_instance_snake_case);
                    let own_subscribed_outputs_reader_identifier = format_ident!("{}_subscribed_outputs_reader", cycler_instance_snake_case);
                    let own_producer_identifier = match cycler {
                        Cycler::Perception { .. } => {
                            let own_producer_identifier = format_ident!("{}_producer", cycler_instance_snake_case);
                            quote! { #own_producer_identifier, }
                        },
                        Cycler::RealTime { .. } => Default::default(),
                    };
                    let other_cycler_identifiers: Vec<_> = cycler
                        .get_other_cyclers()
                        .into_iter()
                        .map(|other_cycler| match other_cycler {
                            OtherCycler::Consumer {
                                cycler_instance_name,
                                ..
                            } => {
                                let identifier = format_ident!("{}_consumer", cycler_instance_name.to_case(Case::Snake));
                                quote! { #identifier }
                            },
                            OtherCycler::Reader {
                                cycler_instance_name,
                                ..
                            } => {
                                let identifier = format_ident!("{}_reader", cycler_instance_name.to_case(Case::Snake));
                                quote! { #identifier.clone() }
                            },
                        })
                        .collect();
                    let error_message = format!("failed to create cycler `{cycler_instance}`");
                    quote! {
                        let #cycler_database_changed_identifier = std::sync::Arc::new(tokio::sync::Notify::new());
                        let (#own_subscribed_outputs_writer_identifier, #own_subscribed_outputs_reader_identifier) = framework::multiple_buffer_with_slots([
                            Default::default(),
                            Default::default(),
                            Default::default(),
                        ]);
                        let #cycler_variable_identifier = #cycler_module_name_identifier::Cycler::new(
                            ::#cycler_module_name_identifier::CyclerInstance::#cycler_instance_identifier,
                            hardware_interface.clone(),
                            #own_writer_identifier,
                            #own_producer_identifier
                            #(#other_cycler_identifiers,)*
                            #cycler_database_changed_identifier.clone(),
                            #own_subscribed_outputs_reader_identifier,
                            configuration_reader.clone(),
                        )
                        .wrap_err(#error_message)?;
                        communication_server.register_cycler_instance(
                            #cycler_instance,
                            #cycler_database_changed_identifier,
                            #own_reader_identifier.clone(),
                            #own_subscribed_outputs_writer_identifier,
                        );
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let configuration_slot_initializers_for_all_cyclers: Vec<_> = repeat(quote! { initial_configuration.clone() })
        .take(2 + cycler_initializations.len() /* 2 writer slots + n-1 reader slots for other cyclers + 1 reader slot for communication */)
        .collect();
    let default_slot_initializers_for_all_cyclers: Vec<_> = repeat(quote! { Default::default() })
        .take(2 + cycler_initializations.len() /* 2 writer slots + n-1 reader slots for other cyclers + 1 reader slot for communication */)
        .collect();
    let default_slot_initializers_for_communication: Vec<_> = repeat(quote! { Default::default() })
        .take(
            2 + 1, /* 2 writer slots + 1 reader slot for communication */
        )
        .collect();
    let multiple_buffer_initializers: Vec<_> = cyclers
        .iter()
        .flat_map(|cycler| {
            cycler.get_cycler_instances().modules_to_instances[cycler.get_cycler_module_name()]
                .iter()
                .map(|cycler_instance| {
                    let cycler_instance_snake_case = cycler_instance.to_case(Case::Snake);
                    let writer_identifier = format_ident!("{}_writer", cycler_instance_snake_case);
                    let reader_identifier = format_ident!("{}_reader", cycler_instance_snake_case);
                    let slot_initializers = match cycler {
                        Cycler::Perception { .. } => &default_slot_initializers_for_communication,
                        Cycler::RealTime { .. } => &default_slot_initializers_for_all_cyclers,
                    };
                    quote! {
                        let (#writer_identifier, #reader_identifier) = framework::multiple_buffer_with_slots([
                            #(#slot_initializers,)*
                        ]);
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let future_queue_initializers: Vec<_> = cyclers
        .iter()
        .filter_map(|cycler| {
            if let Cycler::Perception {..} = cycler {
                Some(cycler.get_cycler_instances().modules_to_instances[cycler.get_cycler_module_name()]
                    .iter()
                    .map(|cycler_instance| {
                        let cycler_instance_snake_case = cycler_instance.to_case(Case::Snake);
                        let producer_identifier = format_ident!("{}_producer", cycler_instance_snake_case);
                        let consumer_identifier = format_ident!("{}_consumer", cycler_instance_snake_case);
                        quote! {
                            let (#producer_identifier, #consumer_identifier) = framework::future_queue();
                        }
                    })
                    .collect::<Vec<_>>(),
                )
            } else {
                None
            }
        })
        .flatten()
        .collect();
    let cycler_starts: Vec<_> = cyclers
        .iter()
        .flat_map(|cycler| {
            cycler.get_cycler_instances().modules_to_instances[cycler.get_cycler_module_name()]
                .iter()
                .map(|cycler_instance| {
                    let cycler_instance_snake_case = cycler_instance.to_case(Case::Snake);
                    let cycler_variable_identifier =
                        format_ident!("{}_cycler", cycler_instance_snake_case);
                    let cycler_handle_identifier =
                        format_ident!("{}_handle", cycler_instance_snake_case);
                    let error_message = format!("failed to start cycler `{cycler_instance}`");
                    quote! {
                        let #cycler_handle_identifier = #cycler_variable_identifier
                            .start(keep_running.clone())
                            .wrap_err(#error_message)?;
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let cycler_joins: Vec<_> = cyclers
        .iter()
        .flat_map(|cycler| {
            cycler.get_cycler_instances().modules_to_instances[cycler.get_cycler_module_name()]
                .iter()
                .map(|cycler_instance| {
                    let cycler_instance_snake_case = cycler_instance.to_case(Case::Snake);
                    let cycler_handle_identifier =
                        format_ident!("{}_handle", cycler_instance_snake_case);
                    quote! {
                        match #cycler_handle_identifier.join() {
                            Ok(Err(error)) => {
                                encountered_error = true;
                                println!("{error:?}");
                            },
                            Err(error) => {
                                encountered_error = true;
                                println!("{error:?}");
                            },
                            _ => {},
                        }
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect();
    quote! {
        #[allow(unused_imports, unused_variables)]
        pub fn run<Interface>(
            hardware_interface: std::sync::Arc<Interface>,
            initial_configuration: structs::Configuration,
            keep_running: tokio_util::sync::CancellationToken,
        ) -> color_eyre::Result<()>
        where
            Interface: types::hardware::Interface + Send + Sync + 'static,
        {
            use color_eyre::eyre::WrapErr;

            let (configuration_writer, configuration_reader) = framework::multiple_buffer_with_slots([
                #(#configuration_slot_initializers_for_all_cyclers,)*
            ]);
            #(#multiple_buffer_initializers)*
            #(#future_queue_initializers)*

            let communication_server = communication::server::Runtime::start(keep_running.clone())
                .wrap_err("failed to start communication server")?;

            #(#cycler_initializations)*

            #(#cycler_starts)*

            let mut encountered_error = false;
            #(#cycler_joins)*

            if encountered_error {
                color_eyre::eyre::bail!("at least one cycler exited with error");
            }
            Ok(())
        }
    }
}
