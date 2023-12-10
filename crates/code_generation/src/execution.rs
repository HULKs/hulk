use std::iter::repeat;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use source_analyzer::cyclers::{CyclerKind, Cyclers};

use super::Execution;

pub fn generate_run_function(cyclers: &Cyclers) -> TokenStream {
    let construct_multiple_buffers = generate_multiple_buffers(cyclers);
    let construct_future_queues = generate_future_queues(cyclers);
    // 2 communication writer slots + n reader slots for other cyclers
    let number_of_parameter_slots = 2 + cyclers.number_of_instances();
    let recording_thread = generate_recording_thread(cyclers);
    let construct_cyclers = generate_cycler_constructors(cyclers, Execution::Run);
    let start_cyclers = generate_cycler_starts(cyclers);
    let join_cyclers = generate_cycler_joins(cyclers);

    quote! {
        #[allow(clippy::redundant_clone)]
        pub fn run(
            hardware_interface: std::sync::Arc<impl crate::HardwareInterface + Send + Sync + 'static>,
            addresses: Option<impl tokio::net::ToSocketAddrs + std::marker::Send + std::marker::Sync + 'static>,
            parameters_directory: impl std::convert::AsRef<std::path::Path> + std::marker::Send + std::marker::Sync + 'static,
            body_id: String,
            head_id: String,
            keep_running: tokio_util::sync::CancellationToken,
            cycler_instances_to_be_recorded: std::collections::HashSet<String>,
        ) -> color_eyre::Result<()>
        {
            use color_eyre::eyre::WrapErr;

            #construct_multiple_buffers
            #construct_future_queues
            let (recording_sender, recording_receiver) = std::sync::mpsc::sync_channel(420);

            let communication_server = communication::server::Runtime::start(
                addresses, parameters_directory, body_id, head_id, #number_of_parameter_slots, keep_running.clone())
                .wrap_err("failed to start communication server")?;

            let recording_thread = #recording_thread;

            #construct_cyclers
            // Drop sender to cause channel to close once all cyclers exit,
            // otherwise the recording thread waits forever
            drop(recording_sender);

            #start_cyclers

            #[cfg(feature = "systemd")]
            systemd::daemon::notify(false, std::iter::once(&(systemd::daemon::STATE_READY, "1")))
                .wrap_err("failed to contact SystemD for ready notification")?;

            let mut encountered_error = false;
            #join_cyclers
            match recording_thread.join() {
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

pub fn generate_replayer_struct(cyclers: &Cyclers) -> TokenStream {
    let cycler_fields = generate_cycler_fields(cyclers);
    let construct_multiple_buffers = generate_multiple_buffers(cyclers);
    let construct_future_queues = generate_future_queues(cyclers);
    // 2 communication writer slots + n reader slots for other cyclers
    let number_of_parameter_slots = 2 + cyclers.number_of_instances();
    let construct_cyclers = generate_cycler_constructors(cyclers, Execution::Replay);
    let cycler_parameters = generate_cycler_parameters(cyclers);
    let cycler_seeks = generate_cycler_seeks(cyclers);
    let cycler_recording_paths = generate_cycler_recording_paths(cyclers);

    quote! {
        pub struct Replayer<Hardware> {
            _communication_server: communication::server::Runtime<crate::structs::Parameters>,
            #cycler_fields
        }

        impl<Hardware: crate::HardwareInterface + Send + Sync + 'static> Replayer<Hardware> {
            #[allow(clippy::redundant_clone)]
            pub fn new(
                hardware_interface: std::sync::Arc<Hardware>,
                addresses: Option<impl tokio::net::ToSocketAddrs + std::marker::Send + std::marker::Sync + 'static>,
                parameters_directory: impl std::convert::AsRef<std::path::Path> + std::marker::Send + std::marker::Sync + 'static,
                body_id: String,
                head_id: String,
                keep_running: tokio_util::sync::CancellationToken,
                recording_file_paths: RecordingFilePaths,
            ) -> color_eyre::Result<Self>
            {
                use color_eyre::eyre::WrapErr;

                #construct_multiple_buffers
                #construct_future_queues

                let communication_server = communication::server::Runtime::start(
                    addresses, parameters_directory, body_id, head_id, #number_of_parameter_slots, keep_running.clone())
                    .wrap_err("failed to start communication server")?;

                #construct_cyclers

                Ok(Self {
                    _communication_server: communication_server,
                    #cycler_parameters
                })
            }

            pub fn seek_before_or_equal_of(&mut self, timestamp: std::time::SystemTime) -> color_eyre::Result<()> {
                use color_eyre::eyre::WrapErr;

                #cycler_seeks

                Ok(())
            }
        }

        pub struct RecordingFilePaths {
            #cycler_recording_paths
        }
    }
}

fn generate_cycler_fields(cyclers: &Cyclers) -> TokenStream {
    cyclers
        .instances()
        .map(|(cycler, instance)| {
            let cycler_variable_identifier =
                format_ident!("{}_cycler", instance.to_case(Case::Snake));
            let cycler_module_name = format_ident!("{}", cycler.name.to_case(Case::Snake));
            let cycler_index_identifier = format_ident!("{}_index", instance.to_case(Case::Snake));
            quote! {
                #cycler_variable_identifier: crate::cyclers::#cycler_module_name::Cycler<Hardware>,
                #cycler_index_identifier: framework::RecordingIndex,
            }
        })
        .collect()
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
        let writer_identifier = format_ident!("{}_writer", instance.to_case(Case::Snake));
        let reader_identifier = format_ident!("{}_reader", instance.to_case(Case::Snake));
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
            let producer_identifier = format_ident!("{}_producer", instance.to_case(Case::Snake));
            let consumer_identifier = format_ident!("{}_consumer", instance.to_case(Case::Snake));
            quote! {
                let (#producer_identifier, #consumer_identifier) = framework::future_queue();
            }
        })
        .collect()
}

fn generate_recording_thread(cyclers: &Cyclers) -> TokenStream {
    let file_creations = cyclers.instances().map(|(_cycler, instance)| {
        let instance_name_snake_case = format_ident!("{}", instance.to_case(Case::Snake));
        let recording_file_name = format!("{instance}.{{seconds}}.bincode");
        let error_message_file = format!("failed to create recording file for {instance}");

        quote! {
            let recording_file_path = std::path::Path::new("logs").join(format!(#recording_file_name));
            std::fs::create_dir_all(
                recording_file_path.parent()
                    .expect("recording file path has no parent directory")
            ).wrap_err("failed to create logs folder")?;

            let mut #instance_name_snake_case = std::io::BufWriter::new(std::fs::File::create(recording_file_path).wrap_err(#error_message_file)?); // TODO: possible optimization: buffer size
        }
    });
    let frame_writes = cyclers.instances().map(|(_cycler, instance)| {
        let instance_name = format_ident!("{}", instance);
        let instance_name_snake_case = format_ident!("{}", instance.to_case(Case::Snake));
        let error_message = format!("failed to write into recording file for {instance}");
        quote! {
            crate::cyclers::RecordingFrame::#instance_name { timestamp, data } => {
                let mut recording_header = Vec::new();
                bincode::serialize_into(&mut recording_header, &timestamp).wrap_err("failed to serialize timestamp")?;
                bincode::serialize_into(&mut recording_header, &data.len()).wrap_err("failed to serialize data length")?;
                #instance_name_snake_case.write_all(recording_header.as_slice()).wrap_err(#error_message)?;
                #instance_name_snake_case.write_all(data.as_slice()).wrap_err(#error_message)?;
            },
        }
    });

    quote! {
        {
            let keep_running = keep_running.clone();
            std::thread::Builder::new()
                .name("Recording".to_string())
                .spawn(move || -> color_eyre::Result<()> {
                    let result = (|| {
                        use std::io::Write;
                        let seconds = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs();
                        #(#file_creations)*
                        for recording_frame in recording_receiver {
                            match recording_frame {
                                #(#frame_writes)*
                            }
                        }
                        Ok(())
                    })();

                    keep_running.cancel();
                    result
                })
                .wrap_err("failed to spawn recording thread")?
        }
    }
}

fn generate_cycler_constructors(cyclers: &Cyclers, mode: Execution) -> TokenStream {
    cyclers.instances().map(|(cycler, instance)| {
        let instance_name_snake_case = instance.to_case(Case::Snake);
        let instance_name_snake_case_identifier = format_ident!("{instance_name_snake_case}");
        let cycler_database_changed_identifier = format_ident!("{instance_name_snake_case}_changed");
        let cycler_variable_identifier = format_ident!("{instance_name_snake_case}_cycler");
        let cycler_index_identifier = format_ident!("{instance_name_snake_case}_index");
        let cycler_module_name = format_ident!("{}", cycler.name.to_case(Case::Snake));
        let cycler_instance_name = &instance;
        let cycler_instance_name_identifier = format_ident!("{cycler_instance_name}");
        let own_writer_identifier = format_ident!("{instance_name_snake_case}_writer");
        let own_reader_identifier = format_ident!("{instance_name_snake_case}_reader");
        let own_subscribed_outputs_writer_identifier = format_ident!("{instance_name_snake_case}_subscribed_outputs_writer");
        let own_subscribed_outputs_reader_identifier = format_ident!("{instance_name_snake_case}_subscribed_outputs_reader");
        let enable_recording = if mode == Execution::Run {
            quote! {
                let enable_recording = cycler_instances_to_be_recorded.contains(#cycler_instance_name);
            }
        } else {
            Default::default()
        };
        let recording_index = if mode == Execution::Replay {
            quote! {
                let #cycler_index_identifier = framework::RecordingIndex::read_from(
                    recording_file_paths.#instance_name_snake_case_identifier
                ).wrap_err("failed to read recording index")?;
            }
        } else {
            Default::default()
        };
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
                    let identifier = format_ident!("{}_consumer", instance.to_case(Case::Snake));
                    quote! { #identifier }
                },
                CyclerKind::RealTime => {
                    let identifier = format_ident!("{}_reader", instance.to_case(Case::Snake));
                    quote! { #identifier.clone() }
                },
            });
        let recording_parameters = if mode == Execution::Run {
            quote! {
                recording_sender.clone(),
                enable_recording,
            }
        } else {
            Default::default()
        };
        let error_message = format!("failed to create cycler `{}`", instance);
        quote! {
            let #cycler_database_changed_identifier = std::sync::Arc::new(tokio::sync::Notify::new());
            let (#own_subscribed_outputs_writer_identifier, #own_subscribed_outputs_reader_identifier) = framework::multiple_buffer_with_slots([
                Default::default(),
                Default::default(),
                Default::default(),
            ]);
            #enable_recording
            #recording_index
            let #cycler_variable_identifier = crate::cyclers::#cycler_module_name::Cycler::new(
                crate::cyclers::#cycler_module_name::CyclerInstance::#cycler_instance_name_identifier,
                hardware_interface.clone(),
                #own_writer_identifier,
                #cycler_database_changed_identifier.clone(),
                #own_subscribed_outputs_reader_identifier,
                communication_server.get_parameters_reader(),
                #own_producer_identifier
                #(#other_cycler_inputs,)*
                #recording_parameters
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
                format_ident!("{}_cycler", instance.to_case(Case::Snake));
            let cycler_handle_identifier =
                format_ident!("{}_handle", instance.to_case(Case::Snake));
            let error_message = format!("failed to start cycler `{}`", instance);
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
                format_ident!("{}_handle", instance.to_case(Case::Snake));
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

fn generate_cycler_parameters(cyclers: &Cyclers) -> TokenStream {
    cyclers
        .instances()
        .map(|(_cycler, instance)| {
            let cycler_variable_identifier =
                format_ident!("{}_cycler", instance.to_case(Case::Snake));
            let cycler_index_identifier = format_ident!("{}_index", instance.to_case(Case::Snake));
            quote! {
                #cycler_variable_identifier,
                #cycler_index_identifier,
            }
        })
        .collect()
}

fn generate_cycler_seeks(cyclers: &Cyclers) -> TokenStream {
    cyclers
        .instances()
        .map(|(_cycler, instance)| {
            let cycler_variable_identifier =
                format_ident!("{}_cycler", instance.to_case(Case::Snake));
            let cycler_index_identifier = format_ident!("{}_index", instance.to_case(Case::Snake));
            quote! {
                let frame = self.#cycler_index_identifier.before_or_equal_of(timestamp).wrap_err("failed to seek")?;
                self.#cycler_variable_identifier.cycle(frame.timestamp, &frame.data).wrap_err("failed to replay cycle")?;
            }
        })
        .collect()
}

fn generate_cycler_recording_paths(cyclers: &Cyclers) -> TokenStream {
    cyclers
        .instances()
        .map(|(_cycler, instance)| {
            let cycler_identifier = format_ident!("{}", instance.to_case(Case::Snake));
            quote! {
                pub #cycler_identifier: std::path::PathBuf,
            }
        })
        .collect()
}
