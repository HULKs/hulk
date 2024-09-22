use std::{collections::BTreeSet, iter::once};

use convert_case::{Case, Casing};
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use source_analyzer::{
    contexts::Field,
    cyclers::{Cycler, CyclerKind, Cyclers},
    node::Node,
    path::Path,
};
use syn::{Path as SynPath, Type, TypePath};

use crate::{
    accessor::{path_to_accessor_token_stream, ReferenceKind},
    CyclerMode,
};

pub fn generate_cyclers(cyclers: &Cyclers, mode: CyclerMode) -> TokenStream {
    let recording_frame = if mode == CyclerMode::Run {
        let recording_frame_variants = cyclers.instances().map(|(_cycler, instance)| {
            let instance_name = format_ident!("{}", instance);
            quote! {
                #instance_name {
                    timestamp: std::time::SystemTime,
                    duration: std::time::Duration,
                    data: std::vec::Vec<u8>,
                },
            }
        });
        quote! {
            pub enum RecordingFrame {
                #(#recording_frame_variants)*
            }
        }
    } else {
        Default::default()
    };
    let cyclers: Vec<_> = cyclers
        .cyclers
        .iter()
        .map(|cycler| generate_module(cycler, cyclers, mode))
        .collect();

    quote! {
        #recording_frame

        #(#cyclers)*
    }
}

fn generate_module(cycler: &Cycler, cyclers: &Cyclers, mode: CyclerMode) -> TokenStream {
    let module_name = format_ident!("{}", cycler.name.to_case(Case::Snake));
    let cycler_instance = generate_cycler_instance(cycler);
    let database_struct = generate_database_struct();
    let cycler_struct = generate_struct(cycler, cyclers, mode);
    let cycler_implementation = generate_implementation(cycler, cyclers, mode);

    quote! {
        #[allow(dead_code, unused_mut, unused_variables, clippy::too_many_arguments, clippy::needless_question_mark, clippy::borrow_deref_ref)]
        pub(crate) mod #module_name {
            use color_eyre::eyre::WrapErr;
            use crate::structs::#module_name::{MainOutputs, AdditionalOutputs};

            #cycler_instance
            #database_struct
            #cycler_struct
            #cycler_implementation
        }
    }
}

fn generate_cycler_instance(cycler: &Cycler) -> TokenStream {
    let instances = cycler
        .instances
        .iter()
        .map(|instance| format_ident!("{}", instance));
    quote! {
        #[derive(Clone, Copy, Debug)]
        pub(crate) enum CyclerInstance {
            #(#instances,)*
        }
    }
}

fn generate_database_struct() -> TokenStream {
    quote! {
        #[derive(
            Default,
            Clone,
            serde::Serialize,
            serde::Deserialize,
            path_serde::PathSerialize,
            path_serde::PathIntrospect,
            Debug,
        )]
        pub struct Database {
            pub main_outputs: MainOutputs,
            pub additional_outputs: AdditionalOutputs,
        }
    }
}

fn generate_struct(cycler: &Cycler, cyclers: &Cyclers, mode: CyclerMode) -> TokenStream {
    let module_name = format_ident!("{}", cycler.name.to_case(Case::Snake));
    let input_output_fields = generate_input_output_fields(cycler, cyclers);
    let realtime_inputs = match cycler.kind {
        CyclerKind::Perception => quote! {},
        CyclerKind::RealTime => {
            quote! {
                historic_databases: framework::HistoricDatabases<MainOutputs>,
                perception_databases: framework::PerceptionDatabases<crate::perception_databases::Databases>,
            }
        }
    };
    let node_fields = generate_node_fields(cycler);
    let recording_fields = if mode == CyclerMode::Run {
        quote! {
            recording_sender: std::sync::mpsc::SyncSender<crate::cyclers::RecordingFrame>,
            recording_trigger: framework::RecordingTrigger,
        }
    } else {
        Default::default()
    };

    quote! {
        pub struct Cycler<HardwareInterface>  {
            instance: CyclerInstance,
            hardware_interface: std::sync::Arc<HardwareInterface>,
            own_sender: buffered_watch::Sender<(std::time::SystemTime, Database)>,
            own_subscribed_outputs_receiver: buffered_watch::Receiver<std::collections::HashSet<String>>,
            parameters_receiver: buffered_watch::Receiver<(std::time::SystemTime, crate::structs::Parameters)>,
            pub cycler_state: crate::structs::#module_name::CyclerState,
            #realtime_inputs
            #input_output_fields
            #node_fields
            #recording_fields
        }
    }
}

fn generate_input_output_fields(cycler: &Cycler, cyclers: &Cyclers) -> TokenStream {
    match cycler.kind {
        CyclerKind::Perception => {
            let receivers = generate_receiver_fields(cyclers);
            quote! {
                own_producer: framework::Producer<MainOutputs>,
                #receivers
            }
        }
        CyclerKind::RealTime => {
            let consumers = generate_consumer_fields(cyclers);
            quote! {
                #consumers
            }
        }
    }
}

fn generate_receiver_fields(cyclers: &Cyclers) -> TokenStream {
    cyclers
        .instances_with(CyclerKind::RealTime)
        .map(|(cycler, instance)| {
            let field_name = format_ident!("{}_receiver", instance.to_case(Case::Snake));
            let cycler_module_name = format_ident!("{}", cycler.name.to_case(Case::Snake));

            quote! {
                #field_name: buffered_watch::Receiver<(std::time::SystemTime, crate::cyclers::#cycler_module_name::Database)>,
            }
        })
        .collect()
}

fn generate_consumer_fields(cyclers: &Cyclers) -> TokenStream {
    cyclers
        .instances_with(CyclerKind::Perception)
        .map(|(cycler, instance)| {
            let field_name = format_ident!("{}_consumer", instance.to_case(Case::Snake));
            let cycler_module_name = format_ident!("{}", cycler.name.to_case(Case::Snake));

            quote! {
                #field_name: framework::Consumer<crate::structs::#cycler_module_name::MainOutputs>,
            }
        })
        .collect()
}

fn generate_node_fields(cycler: &Cycler) -> TokenStream {
    let fields: Vec<_> = cycler
        .iter_nodes()
        .map(|node| {
            let node_name_snake_case = format_ident!("{}", node.name.to_case(Case::Snake));
            let node_module = &node.module;
            let node_name = format_ident!("{}", node.name);
            quote! {
                #node_name_snake_case: #node_module::#node_name
            }
        })
        .collect();
    quote! {
        #(#fields,)*
    }
}

fn generate_implementation(cycler: &Cycler, cyclers: &Cyclers, mode: CyclerMode) -> TokenStream {
    let new_method = generate_new_method(cycler, cyclers, mode);
    let start_method = match mode {
        CyclerMode::Run => generate_start_method(cycler.kind),
        CyclerMode::Replay => Default::default(),
    };
    let cycle_method = generate_cycle_method(cycler, cyclers, mode);

    quote! {
        impl<HardwareInterface> Cycler<HardwareInterface>
        where
            HardwareInterface: crate::HardwareInterface + Send + Sync + 'static
        {
            #new_method
            #start_method
            #cycle_method
        }
    }
}

fn generate_new_method(cycler: &Cycler, cyclers: &Cyclers, mode: CyclerMode) -> TokenStream {
    let input_output_fields = generate_input_output_fields(cycler, cyclers);
    let cycler_module_name = format_ident!("{}", cycler.name.to_case(Case::Snake));
    let node_initializers = generate_node_initializers(cycler);
    let node_identifiers = cycler
        .iter_nodes()
        .map(|node| format_ident!("{}", node.name.to_case(Case::Snake)));
    let input_output_identifiers = generate_input_output_identifiers(cycler, cyclers);
    let recording_parameter_fields = if mode == CyclerMode::Run {
        quote! {
            recording_sender: std::sync::mpsc::SyncSender<crate::cyclers::RecordingFrame>,
            recording_trigger: framework::RecordingTrigger,
        }
    } else {
        Default::default()
    };
    let recording_initializer_fields = if mode == CyclerMode::Run {
        quote! {
            recording_sender,
            recording_trigger,
        }
    } else {
        Default::default()
    };

    quote! {
        pub(crate) fn new(
            instance: CyclerInstance,
            hardware_interface: std::sync::Arc<HardwareInterface>,
            own_sender: buffered_watch::Sender<(std::time::SystemTime, Database)>,
            own_subscribed_outputs_receiver: buffered_watch::Receiver<std::collections::HashSet<String>>,
            mut parameters_receiver: buffered_watch::Receiver<(std::time::SystemTime, crate::structs::Parameters)>,
            #input_output_fields
            #recording_parameter_fields
        ) -> color_eyre::Result<Self> {
            let parameters_guard = parameters_receiver.borrow_and_mark_as_seen();
            let (_, parameters) = &* parameters_guard;
            let mut cycler_state = crate::structs::#cycler_module_name::CyclerState::default();
            #node_initializers
            drop(parameters_guard);
            Ok(Self {
                instance,
                hardware_interface,
                own_sender,
                own_subscribed_outputs_receiver,
                parameters_receiver,
                cycler_state,
                #input_output_identifiers
                #(#node_identifiers,)*
                #recording_initializer_fields
            })
        }
    }
}

fn generate_node_initializers(cycler: &Cycler) -> TokenStream {
    let initializers = cycler.iter_nodes().map(|node| {
        let node_name_snake_case = format_ident!("{}", node.name.to_case(Case::Snake));
        let node_module = &node.module;
        let node_name = format_ident!("{}", node.name);
        let field_initializers = generate_node_field_initializers(node, cycler);
        let error_message = format!("failed to create node `{}`", node.name);
        quote! {
            let #node_name_snake_case = #node_module::#node_name::new(
                #node_module::CreationContext::new(
                    #field_initializers
                )
            )
            .wrap_err(#error_message)?;
        }
    });
    quote! {
        #(#initializers)*
    }
}

fn generate_node_field_initializers(node: &Node, cycler: &Cycler) -> TokenStream {
    node.contexts
        .creation_context
        .iter()
        .map(|field| match field {
            Field::AdditionalOutput { name, .. } => {
                panic!("unexpected additional output field `{name}` in CreationContext")
            }
            Field::CyclerState { path, .. } => {
                let accessor = path_to_accessor_token_stream(
                    quote! { cycler_state },
                    path,
                    ReferenceKind::Mutable,
                    cycler,
                );
                quote! {
                    #accessor,
                }
            }
            Field::HardwareInterface { .. } => quote! {
                &hardware_interface,
            },
            Field::HistoricInput { name, .. } => {
                panic!("unexpected historic input field `{name}` in new context")
            }
            Field::Input { name, .. } => {
                panic!("unexpected optional input field `{name}` in new context")
            }
            Field::MainOutput { name, .. } => {
                panic!("unexpected main output field `{name}` in new context")
            }
            Field::Parameter { path, .. } => {
                let accessor = path_to_accessor_token_stream(
                    quote! { parameters },
                    path,
                    ReferenceKind::Immutable,
                    cycler,
                );
                quote! {
                    #accessor,
                }
            }
            Field::PerceptionInput { name, .. } => {
                panic!("unexpected perception input field `{name}` in new context")
            }
            Field::RequiredInput { name, .. } => {
                panic!("unexpected required input field `{name}` in new context")
            }
        })
        .collect()
}

fn generate_input_output_identifiers(cycler: &Cycler, cyclers: &Cyclers) -> TokenStream {
    match cycler.kind {
        CyclerKind::Perception => {
            let receivers = generate_receiver_identifiers(cyclers);
            quote! {
                own_producer,
                #(#receivers,)*
            }
        }
        CyclerKind::RealTime => {
            let consumers = generate_consumer_identifiers(cyclers);
            quote! {
                historic_databases: Default::default(),
                perception_databases: Default::default(),
                #(#consumers,)*
            }
        }
    }
}

fn generate_receiver_identifiers(cyclers: &Cyclers) -> Vec<Ident> {
    cyclers
        .instances_with(CyclerKind::RealTime)
        .map(|(_cycler, instance)| format_ident!("{}_receiver", instance.to_case(Case::Snake)))
        .collect()
}

fn generate_consumer_identifiers(cyclers: &Cyclers) -> Vec<Ident> {
    cyclers
        .instances_with(CyclerKind::Perception)
        .map(|(_cycler, instance)| format_ident!("{}_consumer", instance.to_case(Case::Snake)))
        .collect()
}

fn generate_start_method(cycler_kind: CyclerKind) -> TokenStream {
    let scheduler_tokens = match cycler_kind {
        CyclerKind::Perception => TokenStream::new(),
        CyclerKind::RealTime => quote! {
            #[cfg(feature = "realtime")]
            unsafe {
                let priority = libc::sched_param {
                    sched_priority: 5,
                };
                let process_id = libc::getpid();
                assert!(process_id > 0, "failed to get process id");

                let set_scheduler_return_value = libc::sched_setscheduler(
                    process_id,
                    libc::SCHED_FIFO,
                    &priority as *const libc::sched_param,
                );
                assert!(set_scheduler_return_value == 0, "failed to set scheduler");
            }
        },
    };

    quote! {
        pub(crate) fn start(
            mut self,
            keep_running: tokio_util::sync::CancellationToken,
        ) -> color_eyre::Result<std::thread::JoinHandle<color_eyre::Result<()>>> {
            let instance_name = format!("{:?}", self.instance);
            std::thread::Builder::new()
                .name(instance_name.clone())
                .spawn(move || {
                    #scheduler_tokens
                    while !keep_running.is_cancelled() {
                        if let Err(error) = self.cycle() {
                            keep_running.cancel();
                            return Err(error).wrap_err_with(|| {
                                format!("failed to execute cycle of cycler `{:?}`", self.instance)
                            });
                        }
                    }
                    Ok(())
                })
                .wrap_err_with(|| {
                    format!("failed to spawn thread for `{instance_name}`")
                })
        }
    }
}

fn generate_cycle_method(cycler: &Cycler, cyclers: &Cyclers, mode: CyclerMode) -> TokenStream {
    let cycle_function_signature = match mode {
        CyclerMode::Run => quote! {
            pub(crate) fn cycle(&mut self) -> color_eyre::Result<()>
        },
        CyclerMode::Replay => quote! {
            pub fn cycle(&mut self, now: std::time::SystemTime, mut recording_frame: &[u8]) -> color_eyre::Result<()>
        },
    };
    let setup_node_executions = cycler
        .setup_nodes
        .iter()
        .map(|node| generate_node_execution(node, cycler, NodeType::Setup, mode));
    let cycle_node_executions = cycler
        .cycle_nodes
        .iter()
        .map(|node| generate_node_execution(node, cycler, NodeType::Cycle, mode));
    let cross_input_fields = get_cross_input_fields(cycler);
    let cross_inputs = match mode {
        CyclerMode::Run => generate_cross_inputs_recording(cycler, cross_input_fields),
        CyclerMode::Replay => generate_cross_inputs_extraction(cross_input_fields),
    };

    let pre_setup = match mode {
        CyclerMode::Run => quote! {
            let enable_recording = self.recording_trigger.should_record() && self.hardware_interface.should_record();
            self.recording_trigger.update();
            let mut recording_frame = Vec::new(); // TODO: possible optimization: cache capacity
        },
        CyclerMode::Replay => Default::default(),
    };
    let post_setup = match mode {
        CyclerMode::Run => quote! {
            let now = <HardwareInterface as hardware::TimeInterface>::get_now(&*self.hardware_interface);
            let recording_timestamp = std::time::SystemTime::now();
            *own_database_timestamp = recording_timestamp;
        },
        CyclerMode::Replay => quote! {
            *own_database_timestamp = now;
        },
    };
    let post_setup = match cycler.kind {
        CyclerKind::Perception => quote! {
            #post_setup
            self.own_producer.announce();
        },
        CyclerKind::RealTime => {
            let perception_cycler_updates = generate_perception_cycler_updates(cyclers);

            quote! {
                #post_setup
                self.perception_databases.update(now, crate::perception_databases::Updates {
                    #perception_cycler_updates
                });
            }
        }
    };
    let borrow_receivers = match cycler.kind {
        CyclerKind::Perception => cyclers
            .instances_with(CyclerKind::RealTime)
            .map(|(_cycler, instance)| {
                let receiver = format_ident!("{}_receiver", instance.to_case(Case::Snake));
                let database = format_ident!("{}_database", instance.to_case(Case::Snake));
                quote! {
                    let (_, #database) = &*self.#receiver.borrow_and_mark_as_seen();
                }
            })
            .collect(),
        CyclerKind::RealTime => quote! {},
    };
    let after_remaining_nodes = match cycler.kind {
        CyclerKind::Perception => quote! {
            self.own_producer.finalize(own_database.main_outputs.clone());
        },
        CyclerKind::RealTime => quote! {
            self.historic_databases.update(
                now,
                self.perception_databases
                    .get_first_timestamp_of_temporary_databases(),
                &own_database.main_outputs,
            );
        },
    };
    let after_remaining_nodes = match mode {
        CyclerMode::Run => {
            let recording_variants = cycler.instances.iter().map(|instance| {
                let instance_name = format_ident!("{}", instance);
                quote! {
                    CyclerInstance::#instance_name => crate::cyclers::RecordingFrame::#instance_name {
                        timestamp: recording_timestamp,
                        duration: recording_duration,
                        data: recording_frame,
                    },
                }
            });

            quote! {
                #after_remaining_nodes
                let recording_duration = recording_timestamp.elapsed().expect("time ran backwards");

                const EXECUTION_TIME_UPPER_BOUND: f32 = 0.4;
                if recording_duration.as_secs_f32() > EXECUTION_TIME_UPPER_BOUND {
                    log::warn!("Cycle took {}s!", recording_duration.as_secs_f32());
                    self
                        .hardware_interface
                        .write_to_speakers(types::audio::SpeakerRequest::PlaySound {
                            sound: types::audio::Sound::Donk,
                        });
                }

                if enable_recording {
                    self.recording_sender.try_send(match instance {
                        #(#recording_variants)*
                    }).wrap_err("failed to send recording frame")?;
                }
            }
        }
        CyclerMode::Replay => after_remaining_nodes,
    };

    quote! {
        #[allow(clippy::nonminimal_bool)]
        #cycle_function_signature {
            let instance = self.instance;
            let instance_name = format!("{instance:?}");
            let itt_domain = ittapi::Domain::new(&instance_name);

            let (own_database_timestamp, own_database) = &mut *self.own_sender.borrow_mut();
            *own_database = Default::default();

            #pre_setup

            {
                let own_subscribed_outputs = self.own_subscribed_outputs_receiver.borrow_and_mark_as_seen();
                let parameters_guard = self.parameters_receiver.borrow_and_mark_as_seen();
                let (_, parameters) = &* parameters_guard;
                #(#setup_node_executions)*
            }

            #post_setup

            {
                let own_subscribed_outputs = self.own_subscribed_outputs_receiver.borrow_and_mark_as_seen();
                let parameters_guard = self.parameters_receiver.borrow_and_mark_as_seen();
                let (_, parameters) = &* parameters_guard;
                #borrow_receivers
                #cross_inputs
                #(#cycle_node_executions)*
            }

            #after_remaining_nodes
            Ok(())
        }
    }
}

fn get_cross_input_fields(cycler: &Cycler) -> BTreeSet<Field> {
    cycler
        .setup_nodes
        .iter()
        .chain(cycler.cycle_nodes.iter())
        .flat_map(|node| {
            node.contexts
                .cycle_context
                .iter()
                .filter(|field| {
                    matches!(
                        field,
                        Field::CyclerState { .. }
                            | Field::HistoricInput { .. }
                            | Field::Input {
                                cycler_instance: Some(_),
                                ..
                            }
                            | Field::PerceptionInput { .. }
                            | Field::RequiredInput {
                                cycler_instance: Some(_),
                                ..
                            }
                    )
                })
                .cloned()
        })
        .collect()
}

fn generate_cross_inputs_recording(
    cycler: &Cycler,
    cross_inputs: impl IntoIterator<Item = Field>,
) -> TokenStream {
    let recordings = cross_inputs.into_iter().map(|field| {
        let error_message = match &field {
            Field::CyclerState { name, .. } => format!("failed to record cycler state {name}"),
            Field::HistoricInput { name, .. } => format!("failed to record historic input {name}"),
            Field::Input { cycler_instance: Some(_), name, .. } => format!("failed to record input {name}"),
            Field::PerceptionInput { name, .. } => format!("failed to record perception input {name}"),
            Field::RequiredInput { cycler_instance: Some(_), name, .. } => format!("failed to record required input {name}"),
            _ => panic!("unexpected field {field:?}"),
        };
        let value_to_be_recorded = match field {
            Field::CyclerState { path, .. } => {
                let accessor = path_to_accessor_token_stream(
                    quote! { self.cycler_state },
                    &path,
                    ReferenceKind::Immutable,
                    cycler,
                );
                quote! {
                    #accessor
                }
            }
            Field::HistoricInput { path, .. } => {
                let historic_accessor = path_to_accessor_token_stream(
                    quote!{ database },
                    &path,
                    ReferenceKind::Immutable,
                    cycler,
                );
                quote! {
                    &self
                        .historic_databases
                        .databases
                        .iter()
                        .map(|(system_time, database)| (
                            *system_time,
                            #historic_accessor,
                        ))
                        .collect::<std::collections::BTreeMap<_, _>>()
                }
            }
            Field::Input {
                cycler_instance: Some(cycler_instance),
                path,
                ..
            } => {
                let identifier = format_ident!("{}_database", cycler_instance.to_case(Case::Snake));
                let database_prefix = quote! { #identifier.main_outputs };
                let accessor = path_to_accessor_token_stream(
                    database_prefix,
                    &path,
                    ReferenceKind::Immutable,
                    cycler,
                );
                quote! {
                    &#accessor
                }
            }
            Field::PerceptionInput { cycler_instance, path, .. } => {
                let cycler_instance_identifier =
                    format_ident!("{}", cycler_instance.to_case(Case::Snake));
                let accessor = path_to_accessor_token_stream(
                    quote! { database },
                    &path,
                    ReferenceKind::Immutable,
                    cycler,
                );
                quote! {
                    &[
                        self
                            .perception_databases
                            .persistent()
                            .map(|(system_time, databases)| (
                                *system_time,
                                databases
                                    .#cycler_instance_identifier
                                    .iter()
                                    .map(|database| #accessor)
                                    .collect::<Vec<_>>()
                                ,
                            ))
                            .collect::<std::collections::BTreeMap<_, _>>(),
                        self
                            .perception_databases
                            .temporary()
                            .map(|(system_time, databases)| (
                                *system_time,
                                databases
                                    .#cycler_instance_identifier
                                    .iter()
                                    .map(|database| #accessor)
                                    .collect::<Vec<_>>()
                                ,
                            ))
                            .collect::<std::collections::BTreeMap<_, _>>(),
                    ]
                }
            }
            Field::RequiredInput {
                cycler_instance: Some(cycler_instance),
                path,
                ..
            } => {
                let identifier = format_ident!("{}_database", cycler_instance.to_case(Case::Snake));
                let database_prefix = quote! { #identifier.main_outputs };
                let accessor = path_to_accessor_token_stream(
                    database_prefix,
                    &path,
                    ReferenceKind::Immutable,
                    cycler,
                );
                quote! {
                    &#accessor
                }
            }
            _ => panic!("unexpected field {field:?}"),
        };
        quote! {
            bincode::serialize_into(&mut recording_frame, #value_to_be_recorded).wrap_err(#error_message)?;
        }
    }).collect::<Vec<_>>();

    if recordings.is_empty() {
        return Default::default();
    }

    quote! {
        if enable_recording {
            #(#recordings)*
        }
    }
}

fn generate_cross_inputs_extraction(cross_inputs: impl IntoIterator<Item = Field>) -> TokenStream {
    let extractions = cross_inputs.into_iter().map(|field| {
        let error_message = match &field {
            Field::CyclerState { name, .. } => format!("failed to record cycler state {name}"),
            Field::HistoricInput { name, .. } => format!("failed to record historic input {name}"),
            Field::Input { cycler_instance: Some(_), name, .. } => format!("failed to record input {name}"),
            Field::PerceptionInput { name, .. } => format!("failed to record perception input {name}"),
            Field::RequiredInput { cycler_instance: Some(_), name, .. } => format!("failed to record required input {name}"),
            _ => panic!("unexpected field {field:?}"),
        };
        match field {
            Field::CyclerState { path, .. } => {
                let name = path_to_extraction_variable_name("own", &path, "cycler_state");
                quote! {
                    #[allow(non_snake_case)]
                    let mut #name = bincode::deserialize_from(&mut recording_frame).wrap_err(#error_message)?;
                }
            }
            Field::HistoricInput { path, data_type, .. } => {
                let name = path_to_extraction_variable_name("own", &path, "historic_input");
                quote! {
                    #[allow(non_snake_case)]
                    let #name: std::collections::BTreeMap<std::time::SystemTime, #data_type> = bincode::deserialize_from(&mut recording_frame).wrap_err(#error_message)?;
                }
            }
            Field::Input {
                cycler_instance: Some(cycler_instance),
                path,
                data_type,
                ..
            } => {
                let name = path_to_extraction_variable_name(&cycler_instance, &path, "input");
                quote! {
                    #[allow(non_snake_case)]
                    let #name: #data_type = bincode::deserialize_from(&mut recording_frame).wrap_err(#error_message)?;
                }
            }
            Field::PerceptionInput { cycler_instance, path, data_type, .. } => {
                let name = path_to_extraction_variable_name(&cycler_instance, &path, "perception_input");
                quote! {
                    #[allow(non_snake_case)]
                    let #name: [std::collections::BTreeMap<std::time::SystemTime, Vec<#data_type>>; 2] = bincode::deserialize_from(&mut recording_frame).wrap_err(#error_message)?;
                }
            }
            Field::RequiredInput {
                cycler_instance: Some(cycler_instance),
                path,
                data_type,
                ..
            } => {
                let name = path_to_extraction_variable_name(&cycler_instance, &path, "required_input");
                quote! {
                    #[allow(non_snake_case)]
                    let #name: #data_type = bincode::deserialize_from(&mut recording_frame).wrap_err(#error_message)?;
                }
            }
            _ => panic!("unexpected field {field:?}"),
        }
    }).collect::<Vec<_>>();

    quote! {
        #(#extractions)*
    }
}

fn path_to_extraction_variable_name(cycler_instance: &str, path: &Path, suffix: &str) -> Ident {
    format_ident!(
        "replay_extraction_{}_{}_{}",
        cycler_instance,
        path.to_segments().join("_"),
        suffix,
    )
}

fn generate_perception_cycler_updates(cyclers: &Cyclers) -> TokenStream {
    cyclers
        .instances_with(CyclerKind::Perception)
        .map(|(_cycler, instance)| {
            let identifier = format_ident!("{}", instance.to_case(Case::Snake));
            let consumer = format_ident!("{}_consumer", identifier);
            quote! {
                #identifier: self.#consumer.consume(now),
            }
        })
        .collect()
}

fn generate_node_execution(
    node: &Node,
    cycler: &Cycler,
    node_type: NodeType,
    mode: CyclerMode,
) -> TokenStream {
    match (node_type, mode) {
        (NodeType::Setup, CyclerMode::Run) => {
            let execute_node_and_write_main_outputs =
                generate_execute_node_and_write_main_outputs(node, cycler, mode);
            let record_main_outputs = generate_record_main_outputs(node);
            quote! {
                #execute_node_and_write_main_outputs
                #record_main_outputs
            }
        }
        (NodeType::Cycle, CyclerMode::Run) => {
            let record_node_state = generate_record_node_state(node);
            let execute_node_and_write_main_outputs =
                generate_execute_node_and_write_main_outputs(node, cycler, mode);
            quote! {
                #record_node_state
                #execute_node_and_write_main_outputs
            }
        }
        (NodeType::Setup, CyclerMode::Replay) => {
            let deserialize_frame_and_write_main_outputs =
                generate_deserialize_frame_and_write_main_outputs(node);
            quote! {
                #deserialize_frame_and_write_main_outputs
            }
        }
        (NodeType::Cycle, CyclerMode::Replay) => {
            let restore_node_state = generate_restore_node_state(node);
            let execute_node_and_write_main_outputs =
                generate_execute_node_and_write_main_outputs(node, cycler, mode);
            quote! {
                #restore_node_state
                #execute_node_and_write_main_outputs
            }
        }
    }
}

fn generate_execute_node_and_write_main_outputs(
    node: &Node,
    cycler: &Cycler,
    mode: CyclerMode,
) -> TokenStream {
    let are_required_inputs_some = generate_required_input_condition(node, cycler, mode);
    let node_name = &node.name;
    let node_member = format_ident!("{}", node.name.to_case(Case::Snake));
    let node_module = &node.module;
    let context_initializers = generate_context_initializers(node, cycler, mode);
    let cycle_error_message = format!("failed to execute cycle of `{}`", node.name);
    let write_main_outputs = generate_write_main_outputs(node);
    let write_main_outputs_from_defaults = generate_write_main_outputs_from_defaults(node);

    quote! {
        {
            #[allow(clippy::needless_else)]
            if #are_required_inputs_some {
                let main_outputs = {
                    let _task = ittapi::Task::begin(&itt_domain, #node_name);
                    self.#node_member.cycle(
                        #node_module::CycleContext::new(
                            #context_initializers
                        ),
                    )
                    .wrap_err(#cycle_error_message)?
                };
                #write_main_outputs
            }
            else {
                #write_main_outputs_from_defaults
            }
        }
    }
}

fn generate_record_main_outputs(node: &Node) -> TokenStream {
    node.contexts
        .main_outputs
        .iter()
        .filter_map(|field| match field {
            Field::MainOutput { name, .. } => {
                let error_message = format!("failed to record {name}");
                Some(quote! {
                    if enable_recording {
                        bincode::serialize_into(&mut recording_frame, &own_database.main_outputs.#name).wrap_err(#error_message)?;
                    }
                })
            },
            _ => None,
        })
        .collect()
}

fn generate_record_node_state(node: &Node) -> TokenStream {
    let node_member = format_ident!("{}", node.name.to_case(Case::Snake));
    let error_message = format!("failed to record `{}`", node.name);
    quote! {
        if enable_recording {
            bincode::serialize_into(&mut recording_frame, &self.#node_member).wrap_err(#error_message)?;
        }
    }
}

fn generate_deserialize_frame_and_write_main_outputs(node: &Node) -> TokenStream {
    node.contexts
        .main_outputs
        .iter()
        .filter_map(|field| match field {
            Field::MainOutput { name, .. } => {
                let error_message = format!("failed to extract {name}");
                Some(quote! {
                    own_database.main_outputs.#name = bincode::deserialize_from(&mut recording_frame).wrap_err(#error_message)?;
                })
            }
            _ => None,
        })
        .collect()
}

fn generate_restore_node_state(node: &Node) -> TokenStream {
    let node_member = format_ident!("{}", node.name.to_case(Case::Snake));
    let error_message = format!("failed to extract `{}`", node.name);
    quote! {
        {
            use bincode::Options;
            let mut deserializer = bincode::Deserializer::with_reader(
                &mut recording_frame,
                bincode::options()
                    .with_fixint_encoding()
                    .allow_trailing_bytes(),
            );
            serde::Deserialize::deserialize_in_place(
                &mut deserializer,
                &mut self.#node_member,
            ).wrap_err(#error_message)?;
        }
    }
}

enum NodeType {
    Setup,
    Cycle,
}

fn generate_required_input_condition(
    node: &Node,
    cycler: &Cycler,
    mode: CyclerMode,
) -> TokenStream {
    let conditions = node
        .contexts
        .cycle_context
        .iter()
        .filter_map(|field| match field {
            Field::RequiredInput {
                cycler_instance,
                path,
                ..
            } => match mode {
                CyclerMode::Run => {
                    let database_prefix = match cycler_instance {
                        Some(cycler_instance) => {
                            let identifier =
                                format_ident!("{}_database", cycler_instance.to_case(Case::Snake));
                            quote! { #identifier.main_outputs }
                        }
                        None => {
                            quote! { own_database.main_outputs }
                        }
                    };
                    let accessor = path_to_accessor_token_stream(
                        database_prefix,
                        path,
                        ReferenceKind::Immutable,
                        cycler,
                    );
                    Some(quote! {
                        #accessor .is_some()
                    })
                }
                CyclerMode::Replay => match cycler_instance {
                    Some(cycler_instance) => {
                        let name = path_to_extraction_variable_name(
                            cycler_instance,
                            path,
                            "required_input",
                        );
                        Some(quote! {
                            #name .is_some()
                        })
                    }
                    None => {
                        let accessor = path_to_accessor_token_stream(
                            quote! { own_database.main_outputs },
                            path,
                            ReferenceKind::Immutable,
                            cycler,
                        );
                        Some(quote! {
                            #accessor .is_some()
                        })
                    }
                },
            },
            _ => None,
        })
        .chain(once(quote! {true}));
    quote! {
        #(#conditions)&&*
    }
}

fn generate_context_initializers(node: &Node, cycler: &Cycler, mode: CyclerMode) -> TokenStream {
    let initializers = node
            .contexts
            .cycle_context
            .iter()
            .map(|field| match field {
                Field::AdditionalOutput {  path, .. } => {
                    let accessor = path_to_accessor_token_stream(
                        quote!{ own_database.additional_outputs },
                        path,
                        ReferenceKind::Mutable,
                        cycler,
                    );
                    let path_string = once("additional_outputs").chain(
                            path.segments.iter().map(|segment| segment.name.as_str())
                        ).join(".");
                    quote! {
                        framework::AdditionalOutput::new(
                            own_subscribed_outputs
                                .iter()
                                .any(|subscribed_output| framework::should_be_filled(subscribed_output, #path_string)),
                            #accessor,
                        )
                    }
                }
                Field::CyclerState { path, .. } => {
                    match mode {
                        CyclerMode::Run => {
                            let accessor = path_to_accessor_token_stream(
                                quote! { self.cycler_state },
                                path,
                                ReferenceKind::Mutable,
                                cycler,
                            );
                            quote! {
                                #accessor
                            }
                        },
                        CyclerMode::Replay => {
                            let name = path_to_extraction_variable_name("own", path, "cycler_state");
                            quote! {
                                &mut #name
                            }
                        },
                    }
                }
                Field::HardwareInterface { .. } => quote! {
                    &self.hardware_interface
                },
                Field::HistoricInput { path, data_type, .. } => {
                    match mode {
                        CyclerMode::Run => {
                            let now_accessor = path_to_accessor_token_stream(
                                quote!{ own_database.main_outputs },
                                path,
                                ReferenceKind::Immutable,
                                cycler,
                            );
                            let historic_accessor = path_to_accessor_token_stream(
                                quote!{ database },
                                path,
                                ReferenceKind::Immutable,
                                cycler,
                            );
                            quote! {
                                [(now, #now_accessor)]
                                    .into_iter()
                                    .chain(
                                        self
                                            .historic_databases
                                            .databases
                                            .iter()
                                            .map(|(system_time, database)| (
                                                *system_time,
                                                #historic_accessor,
                                            ))
                                    )
                                    .collect::<std::collections::BTreeMap<_, _>>()
                                    .into()
                            }
                        },
                        CyclerMode::Replay => {
                            let name = path_to_extraction_variable_name("own", path, "historic_input");
                            let is_option = match data_type {
                                Type::Path(TypePath {
                                    path: SynPath { segments, .. },
                                    ..
                                }) => segments.last().is_some_and(|segment| segment.ident == "Option"),
                                _ => false,
                            };

                            let now_accessor = path_to_accessor_token_stream(
                                quote!{ own_database.main_outputs },
                                path,
                                ReferenceKind::Immutable,
                                cycler,
                            );

                            if is_option {
                                quote! {
                                    [(own_database.main_outputs.cycle_time.start_time, #now_accessor)]
                                        .into_iter()
                                        .chain(
                                            #name.iter().map(|(key, option_value)| (*key, option_value.as_ref()))
                                        ).collect::<std::collections::BTreeMap<_, _>>().into()
                                }
                            } else {
                                quote! {
                                    [(own_database.main_outputs.cycle_time.start_time, #now_accessor)]
                                        .into_iter()
                                        .chain(
                                            #name.iter().map(|(key, option_value)| (*key, option_value))
                                        ).collect::<std::collections::BTreeMap<_, _>>().into()
                                }
                            }
                        },
                    }
                }
                Field::Input {
                    cycler_instance,
                    path,
                    data_type,
                    ..
                } => {
                    match cycler_instance {
                        Some(cycler_instance) => {
                            match mode {
                                CyclerMode::Run => {
                                    let identifier =
                                        format_ident!("{}_database", cycler_instance.to_case(Case::Snake));
                                    let database_prefix = quote! { #identifier.main_outputs };
                                    let accessor = path_to_accessor_token_stream(
                                        database_prefix,
                                        path,
                                        ReferenceKind::Immutable,
                                        cycler,
                                    );
                                    quote! {
                                        #accessor
                                    }
                                },
                                CyclerMode::Replay=> {
                                    let name = path_to_extraction_variable_name(cycler_instance, path, "input");
                                    let is_option = match data_type {
                                        Type::Path(TypePath {
                                            path: SynPath { segments, .. },
                                            ..
                                        }) => segments.last().is_some_and(|segment| segment.ident == "Option"),
                                        _ => false,
                                    };
                                    if is_option {
                                        quote! {
                                            #name.as_ref()
                                        }
                                    } else {
                                        quote! {
                                            &#name
                                        }
                                    }
                                },
                            }
                        }
                        None => {
                            let database_prefix = quote! { own_database.main_outputs };
                            let accessor = path_to_accessor_token_stream(
                                database_prefix,
                                path,
                                ReferenceKind::Immutable,
                                cycler,
                            );
                            quote! {
                                #accessor
                            }
                        }
                    }
                }
                Field::MainOutput { name, .. } => {
                    panic!("unexpected MainOutput `{name}` in cycle context")
                }
                Field::Parameter { path, .. } => {
                    let accessor = path_to_accessor_token_stream(
                        quote! { parameters },
                        path,
                        ReferenceKind::Immutable,
                        cycler,
                    );
                    quote! {
                        #accessor
                    }
                }
                Field::PerceptionInput {
                    cycler_instance,
                    path,
                    data_type,
                    ..
                } => {
                    match mode {
                        CyclerMode::Run => {
                            let cycler_instance_identifier =
                                format_ident!("{}", cycler_instance.to_case(Case::Snake));
                            let accessor = path_to_accessor_token_stream(
                                quote! { database },
                                path,
                                ReferenceKind::Immutable,
                                cycler,
                            );
                            quote! {
                                framework::PerceptionInput {
                                    persistent: self
                                        .perception_databases
                                        .persistent()
                                        .map(|(system_time, databases)| (
                                            *system_time,
                                            databases
                                                .#cycler_instance_identifier
                                                .iter()
                                                .map(|database| #accessor)
                                                .collect()
                                            ,
                                        ))
                                        .collect(),
                                    temporary: self
                                        .perception_databases
                                        .temporary()
                                        .map(|(system_time, databases)| (
                                            *system_time,
                                            databases
                                                .#cycler_instance_identifier
                                                .iter()
                                                .map(|database| #accessor)
                                                .collect()
                                            ,
                                        ))
                                        .collect(),
                                }
                            }
                        },
                        CyclerMode::Replay => {
                            let name = path_to_extraction_variable_name(cycler_instance, path, "perception_input");
                            let is_option = match data_type {
                                Type::Path(TypePath {
                                    path: SynPath { segments, .. },
                                    ..
                                }) => segments.last().is_some_and(|segment| segment.ident == "Option"),
                                _ => false,
                            };
                            let map_operation = if is_option {
                                quote! {
                                    values.iter().map(|option_value| option_value.as_ref()).collect()
                                }
                            } else {
                                quote! {
                                    values.iter().collect()
                                }
                            };
                            quote! {
                                framework::PerceptionInput {
                                    persistent: #name[0].iter().map(|(system_time, values)| (
                                        *system_time,
                                        #map_operation,
                                    )).collect(),
                                    temporary: #name[1].iter().map(|(system_time, values)| (
                                        *system_time,
                                        #map_operation,
                                    )).collect(),
                                }
                            }
                        },
                    }
                }
                Field::RequiredInput {
                    cycler_instance,
                    path,
                    ..
                } => {
                    match cycler_instance {
                        Some(cycler_instance) => {
                            match mode {
                                CyclerMode::Run => {
                                    let identifier =
                                        format_ident!("{}_database", cycler_instance.to_case(Case::Snake));
                                    let database_prefix = quote! { #identifier.main_outputs };
                                    let accessor = path_to_accessor_token_stream(
                                        database_prefix,
                                        path,
                                        ReferenceKind::Immutable,
                                        cycler,
                                    );
                                    quote! {
                                        #accessor .unwrap()
                                    }
                                },
                                CyclerMode::Replay => {
                                    let name = path_to_extraction_variable_name(cycler_instance, path, "required_input");
                                    quote! {
                                        &#name .unwrap()
                                    }
                                },
                            }
                        }
                        None => {
                            let database_prefix = quote! { own_database.main_outputs };
                            let accessor = path_to_accessor_token_stream(
                                database_prefix,
                                path,
                                ReferenceKind::Immutable,
                                cycler,
                            );
                            quote! {
                                #accessor .unwrap()
                            }
                        }
                    }
                }
            });
    quote! {
        #(#initializers,)*
    }
}

fn generate_write_main_outputs(node: &Node) -> TokenStream {
    node.contexts
        .main_outputs
        .iter()
        .filter_map(|field| match field {
            Field::MainOutput { name, .. } => Some(quote! {
                own_database.main_outputs.#name = main_outputs.#name.value;
            }),
            _ => None,
        })
        .collect()
}

fn generate_write_main_outputs_from_defaults(node: &Node) -> TokenStream {
    node.contexts
        .main_outputs
        .iter()
        .filter_map(|field| match field {
            Field::MainOutput { name, .. } => {
                let setter = quote! {
                    own_database.main_outputs.#name = Default::default();
                };
                Some(setter)
            }
            _ => None,
        })
        .collect()
}
