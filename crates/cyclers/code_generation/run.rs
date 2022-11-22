use std::iter::repeat;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::{cycler::Cycler, other_cycler::OtherCycler};

pub fn generate_run(cyclers: &[Cycler]) -> TokenStream {
    let cycler_initializations: Vec<_> = cyclers
      .iter()
      .map(|cycler| {
          cycler.get_cycler_instances().modules_to_instances[cycler.get_cycler_module_name()]
              .iter()
              .map(|cycler_instance| {
                  let cycler_instance_snake_case = cycler_instance.to_case(Case::Snake);
                  let cycler_variable_identifier = format_ident!("{}_cycler", cycler_instance_snake_case);
                  let cycler_module_name_identifier = cycler.get_cycler_module_name_identifier();
                  let cycler_instance_identifier = format_ident!("{}", cycler_instance);
                  let own_writer_identifier = format_ident!("{}_writer", cycler_instance_snake_case);
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
                  let error_message = format!("Failed to create cycler `{}`", cycler_instance);
                  quote! {
                      let #cycler_variable_identifier = #cycler_module_name_identifier::Cycler::new(
                          ::#cycler_module_name_identifier::CyclerInstance::#cycler_instance_identifier,
                          hardware_interface.clone(),
                          #own_writer_identifier,
                          #own_producer_identifier
                          #(#other_cycler_identifiers,)*
                          configuration_reader.clone(),
                      )
                      .context(#error_message)?;
                  }
              })
              .collect::<Vec<_>>()
      })
      .flatten()
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
      .map(|cycler| {
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
      .flatten()
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
        .map(|cycler| {
            cycler.get_cycler_instances().modules_to_instances[cycler.get_cycler_module_name()]
                .iter()
                .map(|cycler_instance| {
                    let cycler_instance_snake_case = cycler_instance.to_case(Case::Snake);
                    let cycler_variable_identifier =
                        format_ident!("{}_cycler", cycler_instance_snake_case);
                    let cycler_handle_identifier =
                        format_ident!("{}_handle", cycler_instance_snake_case);
                    let error_message = format!("Failed to start cycler `{}`", cycler_instance);
                    quote! {
                        let #cycler_handle_identifier = #cycler_variable_identifier
                            .start(keep_running.clone())
                            .context(#error_message)?;
                    }
                })
                .collect::<Vec<_>>()
        })
        .flatten()
        .collect();
    let cycler_joins: Vec<_> = cyclers
        .iter()
        .map(|cycler| {
            cycler.get_cycler_instances().modules_to_instances[cycler.get_cycler_module_name()]
                .iter()
                .map(|cycler_instance| {
                    let cycler_instance_snake_case = cycler_instance.to_case(Case::Snake);
                    let cycler_handle_identifier =
                        format_ident!("{}_handle", cycler_instance_snake_case);
                    quote! {
                        if let Err(error) = #cycler_handle_identifier.join() {
                            std::panic::resume_unwind(error)
                        }
                    }
                })
                .collect::<Vec<_>>()
        })
        .flatten()
        .collect();
    quote! {
        #[allow(unused_imports, unused_variables)]
        pub fn run<Interface>(
            hardware_interface: std::sync::Arc<Interface>,
            initial_configuration: structs::Configuration,
            keep_running: tokio_util::sync::CancellationToken,
        ) -> anyhow::Result<()>
        where
            Interface: hardware::HardwareInterface + Send + Sync + 'static,
        {
            use anyhow::Context;

            let (configuration_writer, configuration_reader) = framework::multiple_buffer_with_slots([
                #(#configuration_slot_initializers_for_all_cyclers,)*
            ]);
            #(#multiple_buffer_initializers)*
            #(#future_queue_initializers)*

            #(#cycler_initializations)*

            #(#cycler_starts)*

            #(#cycler_joins)*

            Ok(())
        }
    }
}
