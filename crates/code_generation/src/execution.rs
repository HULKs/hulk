use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use source_analyzer::cyclers::{CyclerKind, Cyclers};

use crate::{accessor::ReferenceKind, CyclerMode};

pub fn generate_run_function(cyclers: &Cyclers) -> TokenStream {
    let construct_buffered_watch_channels = generate_buffered_watch_channels(cyclers);
    let construct_future_queues = generate_future_queues(cyclers);
    let recording_thread = generate_recording_thread(cyclers);
    let construct_cyclers = generate_cycler_constructors(cyclers, CyclerMode::Run);
    let communication_registrations = generate_communication_registrations(cyclers);
    let start_cyclers = generate_cycler_starts(cyclers);
    let join_cyclers = generate_cycler_joins(cyclers);

    quote! {
        #[allow(clippy::redundant_clone)]
        #[allow(clippy::too_many_arguments)]
        pub fn run(
            hardware_interface: std::sync::Arc<impl crate::HardwareInterface + Send + Sync + 'static>,
            addresses: Option<impl tokio::net::ToSocketAddrs + std::marker::Send + std::marker::Sync + 'static>,
            parameters_directory: impl std::convert::AsRef<std::path::Path> + std::marker::Send + std::marker::Sync + 'static,
            log_path: impl std::convert::AsRef<std::path::Path> + std::marker::Send + std::marker::Sync + 'static,
            hardware_ids: types::hardware::Ids,
            keep_running: tokio_util::sync::CancellationToken,
            recording_intervals: std::collections::HashMap<String, usize>,
        ) -> color_eyre::Result<()>
        {
            use color_eyre::eyre::WrapErr;

            {
                let keep_running = keep_running.clone();
                std::panic::set_hook(Box::new(move |panic_info| {
                    eprintln!("Full backtrace: {:#?}", std::backtrace::Backtrace::capture());
                    keep_running.cancel();
                    eprintln!("{panic_info}");
                }));
            }

            #construct_buffered_watch_channels
            #construct_future_queues

            let parameters_from_disk: crate::structs::Parameters =
                parameters::directory::deserialize(
                    parameters_directory,
                    &hardware_ids,
                    false,
                ).wrap_err("failed to parse initial parameters")?;
            let initial_parameters = parameters_from_disk;
            let (parameters_sender, parameters_receiver) =
                buffered_watch::channel((std::time::SystemTime::now(), initial_parameters));

            let (recording_sender, recording_receiver) = std::sync::mpsc::sync_channel(420);
            let recording_thread = #recording_thread;

            #construct_cyclers
            // Drop sender to cause channel to close once all cyclers exit,
            // otherwise the recording thread waits forever
            drop(recording_sender);

            let communication_thread = addresses.map(|addresses| {
                let keep_running = keep_running.clone();
                std::thread::Builder::new()
                    .name("Communication".to_string())
                    .spawn(move || -> color_eyre::Result<()> {
                        let async_runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()
                            .wrap_err("failed to create async runtime")?;
                        async_runtime.block_on(async move {
                            let mut communication_server = communication::server::Server::default();
                            #communication_registrations
                            let (parameters_subscriptions, _) = buffered_watch::channel(Default::default());
                            communication_server.expose_source("parameters", parameters_receiver, parameters_subscriptions)?;
                            communication_server.expose_sink("parameters", parameters_sender)?;
                            let (_, ids_receiver) = buffered_watch::channel((std::time::SystemTime::now(), hardware_ids));
                            let (ids_subscriptions, _) = buffered_watch::channel(Default::default());
                            communication_server.expose_source("hardware_ids", ids_receiver, ids_subscriptions)?;
                            communication_server.serve(addresses, keep_running).await?;
                            Ok(())
                        })
                    })
                    .expect("failed to spawn communication thread")
            });

            #start_cyclers

            #[cfg(feature = "systemd")]
            systemd::daemon::notify(false, std::iter::once(&(systemd::daemon::STATE_READY, "1")))
                .wrap_err("failed to contact SystemD for ready notification")?;

            let mut encountered_error = false;
            #join_cyclers
            match recording_thread.join() {
                Ok(Err(error)) => {
                    encountered_error = true;
                    eprintln!("recording thread returned error: {error:?}");
                },
                Err(error) => {
                    encountered_error = true;
                    eprintln!("failed to join recording thread: {error:?}");
                },
                _ => {},
            }
            if let Some(thread) = communication_thread {
                match thread.join() {
                    Ok(Err(error)) => {
                        encountered_error = true;
                        eprintln!("communication thread returned error: {error:?}");
                    },
                    Err(error) => {
                        encountered_error = true;
                        eprintln!("failed to join communication thread: {error:?}");
                    },
                    _ => {},
                }
            }

            if encountered_error {
                color_eyre::eyre::bail!("at least one cycler exited with error");
            }
            Ok(())
        }
    }
}

pub fn generate_replayer_struct(cyclers: &Cyclers, with_communication: bool) -> TokenStream {
    let cycler_fields = generate_cycler_fields(cyclers);
    let construct_buffered_watch_channels = generate_buffered_watch_channels(cyclers);
    let construct_future_queues = generate_future_queues(cyclers);
    let construct_cyclers = generate_cycler_constructors(cyclers, CyclerMode::Replay);
    let ReplayerTokenStreams {
        fields,
        parameters: identifiers,
        accessors,
    } = generate_replayer_token_streams(cyclers, with_communication);
    let (replayer_arguments, communication_thread) = if with_communication {
        (
            quote! {
                addresses: Option<impl tokio::net::ToSocketAddrs + std::marker::Send + std::marker::Sync + 'static>,
                keep_running: tokio_util::sync::CancellationToken,
            },
            {
                let communication_registrations = generate_communication_registrations(cyclers);
                quote! {
                    let _communication_thread = addresses.map(|addresses| {
                        let keep_running = keep_running.clone();
                        let parameters_receiver = parameters_receiver.clone();
                        std::thread::Builder::new()
                            .name("Communication".to_string())
                            .spawn(move || -> color_eyre::Result<()> {
                                let async_runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()
                                    .wrap_err("failed to create async runtime")?;
                                async_runtime.block_on(async move {
                                    let mut communication_server = communication::server::Server::default();
                                    #communication_registrations
                                    let (parameters_subscriptions, _) = buffered_watch::channel(Default::default());
                                    communication_server.expose_source("parameters", parameters_receiver, parameters_subscriptions)?;
                                    communication_server.expose_sink("parameters", parameters_sender)?;
                                    communication_server.serve(addresses, keep_running).await?;
                                    Ok(())
                                })
                            })
                            .expect("failed to spawn communication thread")
                    });
                }
            },
        )
    } else {
        (Default::default(), Default::default())
    };
    let cycler_parameters = generate_cycler_parameters(cyclers);
    let recording_index_entries =
        generate_recording_index_entries(cyclers, ReferenceKind::Immutable);
    let recording_index_entries_mut =
        generate_recording_index_entries(cyclers, ReferenceKind::Mutable);
    let cycler_replays = generate_cycler_replays(cyclers);

    quote! {
        pub struct Replayer<Hardware> {
            #fields
            #cycler_fields
        }

        impl<Hardware: crate::HardwareInterface + Send + Sync + 'static> Replayer<Hardware> {
            #[allow(clippy::redundant_clone)]
            pub fn new(
                hardware_interface: std::sync::Arc<Hardware>,
                parameters_directory: impl std::convert::AsRef<std::path::Path> + std::marker::Send + std::marker::Sync + 'static,
                hardware_ids: types::hardware::Ids,
                recordings_file_path: impl std::convert::AsRef<std::path::Path>,
                #replayer_arguments
            ) -> color_eyre::Result<Self>
            {
                use color_eyre::eyre::WrapErr;

                #construct_buffered_watch_channels
                #construct_future_queues

                let parameters_from_disk: crate::structs::Parameters =
                    parameters::directory::deserialize(
                        parameters_directory,
                        &hardware_ids,
                        true,
                    ).wrap_err("failed to parse initial parameters")?;
                let initial_parameters = parameters_from_disk;
                #[allow(unused)]
                let (parameters_sender, parameters_receiver) =
                    buffered_watch::channel((std::time::SystemTime::now(), initial_parameters));

                #construct_cyclers

                #communication_thread

                Ok(Self {
                    #identifiers
                    #cycler_parameters
                })
            }

            pub fn get_recording_indices(&self) -> std::collections::BTreeMap<String, &framework::RecordingIndex> {
                std::collections::BTreeMap::from([
                    #recording_index_entries
                ])
            }

            pub fn get_recording_indices_mut(&mut self) -> std::collections::BTreeMap<String, &mut framework::RecordingIndex> {
                std::collections::BTreeMap::from([
                    #recording_index_entries_mut
                ])
            }

            pub fn replay(&mut self, cycler_instance_name: &str, timestamp: std::time::SystemTime, data: &[u8]) -> color_eyre::Result<()> {
                use color_eyre::eyre::{bail, WrapErr};

                match cycler_instance_name {
                    #cycler_replays
                    _ => bail!("unexpected cycler instance name {cycler_instance_name}"),
                }
            }

            pub fn replay_at(&mut self, timestamp: std::time::SystemTime) -> color_eyre::Result<()> {
                use color_eyre::eyre::WrapErr;

                let recording_indices = self.get_recording_indices_mut();
                let frames = recording_indices
                    .into_iter()
                    .map(|(name, index)| {
                        (
                            name,
                            index
                                .find_latest_frame_up_to(timestamp)
                                .expect("failed to find latest frame"),
                        )
                    })
                    .collect::<std::collections::BTreeMap<_, _>>();
                for (name, frame) in frames {
                    if let Some(frame) = frame {
                        self.replay(&name, frame.timing.timestamp, &frame.data)
                            .wrap_err("failed to replay frame")?;
                    }
                }

                Ok(())
            }

            #accessors
        }
    }
}

struct ReplayerTokenStreams {
    fields: TokenStream,
    parameters: TokenStream,
    accessors: TokenStream,
}

fn generate_replayer_token_streams(
    cyclers: &Cyclers,
    with_communication: bool,
) -> ReplayerTokenStreams {
    if with_communication {
        ReplayerTokenStreams {
            fields: quote! {
                parameters_receiver: buffered_watch::Receiver<(std::time::SystemTime, crate::structs::Parameters)>,
            },
            parameters: quote! {
                parameters_receiver,
            },
            accessors: quote! {
                pub fn get_parameters_receiver(&self) -> buffered_watch::Receiver<(std::time::SystemTime, crate::structs::Parameters)> {
                    self.parameters_receiver.clone()
                }
            },
        }
    } else {
        let receiver_tokens: Vec<_> = cyclers
            .instances()
            .map(|(cycler, instance)| {
                (
                    format_ident!("{}_receiver", instance.to_case(Case::Snake)),
                    format_ident!("{}", cycler.name.to_case(Case::Snake)),
                )
            })
            .collect();

        let receiver_identifiers = receiver_tokens.iter().map(|(receiver, _cycler)| receiver);
        let receiver_fields = receiver_tokens.iter().map(|(receiver, cycler)| {
            quote! {
                #receiver: buffered_watch::Receiver<(std::time::SystemTime, crate::cyclers::#cycler::Database)>,
            }
        });
        let receiver_accessors = receiver_tokens.iter().map(|(receiver, cycler)| {
            quote! {
                #[allow(unused)]
                pub(crate) fn #receiver(&self) -> buffered_watch::Receiver<(std::time::SystemTime, crate::cyclers::#cycler::Database)> {
                    self.#receiver.clone()
                }
            }
        });

        ReplayerTokenStreams {
            fields: quote! {
                #(#receiver_fields)*
            },
            parameters: quote! {
                #(#receiver_identifiers,)*
            },
            accessors: quote! {
                #(#receiver_accessors)*
            },
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

fn generate_buffered_watch_channels(cyclers: &Cyclers) -> TokenStream {
    cyclers
        .instances()
        .map(|(_cycler, instance)| {
            let sender_identifier = format_ident!("{}_sender", instance.to_case(Case::Snake));
            let receiver_identifier = format_ident!("{}_receiver", instance.to_case(Case::Snake));
            quote! {
                let (#sender_identifier, #receiver_identifier) = buffered_watch::channel((
                    std::time::UNIX_EPOCH,
                    Default::default()
                 ));
            }
        })
        .collect()
}

fn generate_future_queues(cyclers: &Cyclers) -> TokenStream {
    cyclers
        .instances_with(CyclerKind::Perception)
        .map(|(_cycler, instance)| {
            let producer_identifier = format_ident!("{}_producer", instance.to_case(Case::Snake));
            let consumer_identifier = format_ident!("{}_consumer", instance.to_case(Case::Snake));
            quote! {
                #[allow(unused)]
                let (#producer_identifier, #consumer_identifier) = framework::future_queue();
            }
        })
        .collect()
}

fn generate_recording_thread(cyclers: &Cyclers) -> TokenStream {
    let file_creations = cyclers.instances().map(|(_cycler, instance)| {
        let instance_name_snake_case = format_ident!("{}", instance.to_case(Case::Snake));
        let recording_file_name = format!("{instance}.bincode");
        let error_message_file = format!("failed to create recording file for {instance}");

        quote! {
            let recording_file_path = log_path.as_ref().join(#recording_file_name);
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
            crate::cyclers::RecordingFrame::#instance_name { timestamp, duration, data } => {
                let mut recording_header = Vec::new();
                bincode::serialize_into(&mut recording_header, &timestamp).wrap_err("failed to serialize timestamp")?;
                bincode::serialize_into(&mut recording_header, &duration).wrap_err("failed to serialize duration")?;
                bincode::serialize_into(&mut recording_header, &data.len()).wrap_err("failed to serialize data length")?;
                #instance_name_snake_case.write_all(recording_header.as_slice()).wrap_err(#error_message)?;
                #instance_name_snake_case.write_all(data.as_slice()).wrap_err(#error_message)?;
            },
        }
    });

    quote! {
        {
            let keep_running = keep_running.clone();
            let mut parameters_receiver = parameters_receiver.clone();
            std::thread::Builder::new()
                .name("Recording".to_string())
                .spawn(move || -> color_eyre::Result<()> {
                    let result = (|| {
                        use std::io::Write;
                        {
                            let (_, parameters) = &*parameters_receiver.borrow_and_mark_as_seen();
                            std::fs::write(
                                log_path.as_ref().join("default.json"),
                                serde_json::to_string_pretty(parameters)?,
                            )?;
                        }
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

fn generate_cycler_constructors(cyclers: &Cyclers, mode: CyclerMode) -> TokenStream {
    cyclers.instances().map(|(cycler, instance)| {
        let instance_name_snake_case = instance.to_case(Case::Snake);
        let cycler_variable_identifier = format_ident!("{instance_name_snake_case}_cycler");
        let cycler_index_identifier = format_ident!("{instance_name_snake_case}_index");
        let cycler_module_name = format_ident!("{}", cycler.name.to_case(Case::Snake));
        let cycler_instance_name = &instance;
        let cycler_instance_name_identifier = format_ident!("{cycler_instance_name}");
        let own_sender_identifier = format_ident!("{instance_name_snake_case}_sender");
        let own_subscriptions_sender_identifier = format_ident!("{instance_name_snake_case}_subscriptions_sender");
        let own_subscriptions_receiver_identifier = format_ident!("{instance_name_snake_case}_subscriptions_receiver");
        let recording_trigger = if mode == CyclerMode::Run {
            quote! {
                let recording_trigger = framework::RecordingTrigger::new(
                    recording_intervals.get(#cycler_instance_name).copied().unwrap_or(0)
                );
            }
        } else {
            Default::default()
        };
        let recording_index = if mode == CyclerMode::Replay {
            let recording_file_name = format!("{instance}.bincode");
            quote! {
                let #cycler_index_identifier = framework::RecordingIndex::read_from(
                    recordings_file_path.as_ref().join(#recording_file_name)
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
                    let identifier = format_ident!("{}_receiver", instance.to_case(Case::Snake));
                    quote! { #identifier.clone() }
                },
            });
        let recording_parameters = if mode == CyclerMode::Run {
            quote! {
                recording_sender.clone(),
                recording_trigger,
            }
        } else {
            Default::default()
        };
        let error_message = format!("failed to create cycler `{}`", instance);

        quote! {
            #[allow(unused)]
            #recording_trigger
            #recording_index
            #[allow(unused)]
            let (#own_subscriptions_sender_identifier, #own_subscriptions_receiver_identifier) = buffered_watch::channel(Default::default());
            let #cycler_variable_identifier = crate::cyclers::#cycler_module_name::Cycler::new(
                crate::cyclers::#cycler_module_name::CyclerInstance::#cycler_instance_name_identifier,
                hardware_interface.clone(),
                #own_sender_identifier,
                #own_subscriptions_receiver_identifier,
                parameters_receiver.clone(),
                #own_producer_identifier
                #(#other_cycler_inputs,)*
                #recording_parameters
            )
            .wrap_err(#error_message)?;
        }
    })
    .collect()
}

fn generate_communication_registrations(cyclers: &Cyclers) -> TokenStream {
    cyclers
        .instances()
        .map(|(_cycler, instance)| {
            let instance_name_snake_case = instance.to_case(Case::Snake);
            let own_receiver_identifier = format_ident!("{instance_name_snake_case}_receiver");
            let own_subscriptions_sender_identifier =
                format_ident!("{instance_name_snake_case}_subscriptions_sender");
            quote! {
                communication_server.expose_source(
                    #instance,
                    #own_receiver_identifier,
                    #own_subscriptions_sender_identifier,
                ).wrap_err("failed to expose source in communication")?;
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
            let thread_return_error_message =
                format!("cycler thread {instance} returned error: {{error:?}}");
            let join_error_message =
                format!("failed to join cycler thread {instance}: {{error:?}}");
            quote! {
                match #cycler_handle_identifier.join() {
                    Ok(Err(error)) => {
                        encountered_error = true;
                        eprintln!(#thread_return_error_message);
                    },
                    Err(error) => {
                        encountered_error = true;
                        eprintln!(#join_error_message);
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

fn generate_recording_index_entries(
    cyclers: &Cyclers,
    reference_kind: ReferenceKind,
) -> TokenStream {
    let reference = match reference_kind {
        ReferenceKind::Immutable => quote! {& },
        ReferenceKind::Mutable => quote! {&mut },
    };
    cyclers
        .instances()
        .map(|(_cycler, instance)| {
            let cycler_index_identifier = format_ident!("{}_index", instance.to_case(Case::Snake));
            quote! {
                (#instance.to_string(), #reference self.#cycler_index_identifier),
            }
        })
        .collect()
}

fn generate_cycler_replays(cyclers: &Cyclers) -> TokenStream {
    cyclers
        .instances()
        .map(|(_cycler, instance)| {
            let cycler_variable_identifier =
                format_ident!("{}_cycler", instance.to_case(Case::Snake));
            let error_message = format!("failed to replay {} cycle", instance);
            quote! {
                #instance => self.#cycler_variable_identifier.cycle(timestamp, data).wrap_err(#error_message),
            }
        })
        .collect()
}
