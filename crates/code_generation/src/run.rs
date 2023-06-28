use std::iter::repeat;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use source_analyzer::cycler::{CyclerKind, Cyclers};

pub fn generate_run_function(cyclers: &Cyclers) -> TokenStream {
    let construct_multiple_buffers = generate_multiple_buffers(cyclers);
    let construct_future_queues = generate_future_queues(cyclers);
    // 2 communication writer slots + n reader slots for other cyclers
    let number_of_parameter_slots = 2 + cyclers.number_of_instances();
    let construct_cyclers = generate_cycler_constructors(cyclers);
    let start_cyclers = generate_cycler_starts(cyclers);
    let join_cyclers = generate_cycler_joins(cyclers);

    quote! {
        #[allow(unused_imports, unused_variables)]
        pub fn run<Interface>(
            hardware_interface: std::sync::Arc<Interface>,
            addresses: Option<impl tokio::net::ToSocketAddrs + std::marker::Send + std::marker::Sync + 'static>,
            parameters_directory: impl std::convert::AsRef<std::path::Path> + std::marker::Send + std::marker::Sync + 'static,
            body_id: String,
            head_id: String,
            keep_running: tokio_util::sync::CancellationToken,
        ) -> color_eyre::Result<()>
        where
            Interface: types::hardware::Interface + std::marker::Send + std::marker::Sync + 'static,
        {
            use color_eyre::eyre::WrapErr;

            #construct_multiple_buffers
            #construct_future_queues

            let communication_server = communication::server::Runtime::start(
                addresses, parameters_directory, body_id, head_id, #number_of_parameter_slots, keep_running.clone())
                .wrap_err("failed to start communication server")?;

            #construct_cyclers

            #start_cyclers

            let mut encountered_error = false;
            #join_cyclers
            match communication_server.join() {
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

            if encountered_error {
                color_eyre::eyre::bail!("at least one cycler exited with error");
            }
            Ok(())
        }
    }
}

fn generate_multiple_buffers(cyclers: &Cyclers) -> TokenStream {
    // 2 writer slots + n-1 reader slots for other cyclers + 1 reader slot for communication
    let slots_for_real_time_cyclers: TokenStream = repeat(quote! { Default::default(), })
        .take(2 + cyclers.number_of_instances())
        .collect();
    // 2 writer slots + 1 reader slot for communication
    let slots_for_perception_cyclers: TokenStream =
        repeat(quote! { Default::default(), }).take(2 + 1).collect();

    cyclers.instances().map(|(cycler, instance)| {
        let writer_identifier = format_ident!("{}_writer", instance.name.to_case(Case::Snake));
        let reader_identifier = format_ident!("{}_reader", instance.name.to_case(Case::Snake));
        let slot_initializers = match cycler.kind {
            CyclerKind::Perception => &slots_for_perception_cyclers,
            CyclerKind::RealTime => &slots_for_real_time_cyclers,
        };
        quote! {
            let (#writer_identifier, #reader_identifier) = framework::multiple_buffer_with_slots([
                #slot_initializers
            ]);
        }
    }).collect()
}

fn generate_future_queues(cyclers: &Cyclers) -> TokenStream {
    cyclers
        .instances_with(CyclerKind::Perception)
        .map(|(_cycler, instance)| {
            let producer_identifier =
                format_ident!("{}_producer", instance.name.to_case(Case::Snake));
            let consumer_identifier =
                format_ident!("{}_consumer", instance.name.to_case(Case::Snake));
            quote! {
                let (#producer_identifier, #consumer_identifier) = framework::future_queue();
            }
        })
        .collect()
}

fn generate_cycler_constructors(cyclers: &Cyclers) -> TokenStream {
    cyclers.instances().map(|(cycler, instance)| {
        let instance_name_snake_case = instance.name.to_case(Case::Snake);
        let cycler_database_changed_identifier = format_ident!("{instance_name_snake_case}_changed");
        let cycler_variable_identifier = format_ident!("{instance_name_snake_case}_cycler");
        let cycler_module_name_identifier = format_ident!("{}", cycler.module);
        let cycler_instance_name = &instance.name;
        let cycler_instance_name_identifier = format_ident!("{cycler_instance_name}");
        let own_writer_identifier = format_ident!("{instance_name_snake_case}_writer");
        let own_reader_identifier = format_ident!("{instance_name_snake_case}_reader");
        let own_subscribed_outputs_writer_identifier = format_ident!("{instance_name_snake_case}_subscribed_outputs_writer");
        let own_subscribed_outputs_reader_identifier = format_ident!("{instance_name_snake_case}_subscribed_outputs_reader");
        let own_producer_identifier = match cycler.kind {
            CyclerKind::Perception  => {
                let own_producer_identifier = format_ident!("{instance_name_snake_case}_producer");
                quote! { #own_producer_identifier, }
            },
            CyclerKind::RealTime  => quote!{},
        };
        let other_cycler_inputs = cyclers.instances_with(match cycler.kind {
            CyclerKind::Perception => CyclerKind::RealTime,
            CyclerKind::RealTime => CyclerKind::Perception,
        })
         .map(|(cycler, instance)| match cycler.kind {
                CyclerKind::Perception => {
                    let identifier = format_ident!("{}_consumer", instance.name.to_case(Case::Snake));
                    quote! { #identifier }
                },
                CyclerKind::RealTime => {
                    let identifier = format_ident!("{}_reader", instance.name.to_case(Case::Snake));
                    quote! { #identifier.clone() }
                },
            });
        let error_message = format!("failed to create cycler `{}`", instance.name);
        quote! {
            let #cycler_database_changed_identifier = std::sync::Arc::new(tokio::sync::Notify::new());
            let (#own_subscribed_outputs_writer_identifier, #own_subscribed_outputs_reader_identifier) = framework::multiple_buffer_with_slots([
                Default::default(),
                Default::default(),
                Default::default(),
            ]);
            let #cycler_variable_identifier = crate::cyclers::#cycler_module_name_identifier::Cycler::new(
                crate::cyclers::#cycler_module_name_identifier::CyclerInstance::#cycler_instance_name_identifier,
                hardware_interface.clone(),
                #own_writer_identifier,
                #cycler_database_changed_identifier.clone(),
                #own_subscribed_outputs_reader_identifier,
                communication_server.get_parameters_reader(),
                #own_producer_identifier
                #(#other_cycler_inputs,)*
            )
            .wrap_err(#error_message)?;
            communication_server.register_cycler_instance(
                #cycler_instance_name,
                #cycler_database_changed_identifier,
                #own_reader_identifier.clone(),
                #own_subscribed_outputs_writer_identifier,
            );
        }
    })
    .collect()
}

fn generate_cycler_starts(cyclers: &Cyclers) -> TokenStream {
    cyclers
        .instances()
        .map(|(_cycler, instance)| {
            let cycler_variable_identifier =
                format_ident!("{}_cycler", instance.name.to_case(Case::Snake));
            let cycler_handle_identifier =
                format_ident!("{}_handle", instance.name.to_case(Case::Snake));
            let error_message = format!("failed to start cycler `{}`", instance.name);
            quote! {
                let #cycler_handle_identifier = #cycler_variable_identifier
                    .start(keep_running.clone())
                    .wrap_err(#error_message)?;
            }
        })
        .collect()
}

fn generate_cycler_joins(cyclers: &Cyclers) -> TokenStream {
    cyclers
        .instances()
        .map(|(_cycler, instance)| {
            let cycler_handle_identifier =
                format_ident!("{}_handle", instance.name.to_case(Case::Snake));
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
        .collect()
}
