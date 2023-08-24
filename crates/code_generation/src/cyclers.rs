use std::iter::once;

use convert_case::{Case, Casing};
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use source_analyzer::{
    contexts::Field,
    cyclers::{Cycler, CyclerKind, Cyclers},
    node::Node,
};

use crate::accessor::{path_to_accessor_token_stream, ReferenceKind};

pub fn generate_cyclers(cyclers: &Cyclers) -> TokenStream {
    let cyclers: Vec<_> = cyclers
        .cyclers
        .iter()
        .map(|cycler| generate_module(cycler, cyclers))
        .collect();

    quote! {
        #(#cyclers)*
    }
}

fn generate_module(cycler: &Cycler, cyclers: &Cyclers) -> TokenStream {
    let module_name = format_ident!("{}", cycler.name.to_case(Case::Snake));
    let cycler_instance = generate_cycler_instance(cycler);
    let database_struct = generate_database_struct();
    let cycler_struct = generate_struct(cycler, cyclers);
    let cycler_implementation = generate_implementation(cycler, cyclers);

    quote! {
        #[allow(dead_code, unused_mut, unused_variables, clippy::too_many_arguments, clippy::needless_question_mark)]
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
        #[derive(Default, serde::Deserialize, serde::Serialize, serialize_hierarchy::SerializeHierarchy)]
        pub(crate) struct Database {
            pub main_outputs: MainOutputs,
            pub additional_outputs: AdditionalOutputs,
        }
    }
}

fn generate_struct(cycler: &Cycler, cyclers: &Cyclers) -> TokenStream {
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

    quote! {
        pub(crate) struct Cycler<HardwareInterface>  {
            instance: CyclerInstance,
            hardware_interface: std::sync::Arc<HardwareInterface>,
            own_writer: framework::Writer<Database>,
            own_changed: std::sync::Arc<tokio::sync::Notify>,
            own_subscribed_outputs_reader: framework::Reader<std::collections::HashSet<String>>,
            parameters_reader: framework::Reader<crate::structs::Parameters>,
            persistent_state: crate::structs::#module_name::PersistentState,
            #realtime_inputs
            #input_output_fields
            #node_fields
        }
    }
}

fn generate_input_output_fields(cycler: &Cycler, cyclers: &Cyclers) -> TokenStream {
    match cycler.kind {
        CyclerKind::Perception => {
            let readers = generate_reader_fields(cyclers);
            quote! {
                own_producer: framework::Producer<MainOutputs>,
                #readers
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

fn generate_reader_fields(cyclers: &Cyclers) -> TokenStream {
    cyclers
        .instances_with(CyclerKind::RealTime)
        .map(|(cycler, instance)| {
            let field_name = format_ident!("{}_reader", instance.to_case(Case::Snake));
            let cycler_module_name = format_ident!("{}", cycler.name.to_case(Case::Snake));

            quote! {
                #field_name: framework::Reader<crate::cyclers::#cycler_module_name::Database>,
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

fn generate_implementation(cycler: &Cycler, cyclers: &Cyclers) -> TokenStream {
    let new_method = generate_new_method(cycler, cyclers);
    let start_method = generate_start_method();
    let cycle_method = generate_cycle_method(cycler, cyclers);

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

fn generate_new_method(cycler: &Cycler, cyclers: &Cyclers) -> TokenStream {
    let input_output_fields = generate_input_output_fields(cycler, cyclers);
    let cycler_module_name = format_ident!("{}", cycler.name.to_case(Case::Snake));
    let node_initializers = generate_node_initializers(cycler);
    let node_identifiers = cycler
        .iter_nodes()
        .map(|node| format_ident!("{}", node.name.to_case(Case::Snake)));
    let input_output_identifiers = generate_input_output_identifiers(cycler, cyclers);

    quote! {
        pub(crate) fn new(
            instance: CyclerInstance,
            hardware_interface: std::sync::Arc<HardwareInterface>,
            own_writer: framework::Writer<Database>,
            own_changed: std::sync::Arc<tokio::sync::Notify>,
            own_subscribed_outputs_reader: framework::Reader<std::collections::HashSet<String>>,
            parameters_reader: framework::Reader<crate::structs::Parameters>,
            #input_output_fields
        ) -> color_eyre::Result<Self> {
            let parameters = parameters_reader.next().clone();
            let mut persistent_state = crate::structs::#cycler_module_name::PersistentState::default();
            #node_initializers
            Ok(Self {
                instance,
                hardware_interface,
                own_writer,
                own_changed,
                own_subscribed_outputs_reader,
                parameters_reader,
                persistent_state,
                #input_output_identifiers
                #(#node_identifiers,)*
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
                #node_module::CreationContext {
                    #field_initializers
                }
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
            Field::HardwareInterface { name } => quote! {
                #name: &hardware_interface,
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
            Field::Parameter { name, path, .. } => {
                let accessor = path_to_accessor_token_stream(
                    quote! { parameters },
                    path,
                    ReferenceKind::Immutable,
                    cycler,
                );
                quote! {
                    #name: #accessor,
                }
            }
            Field::PerceptionInput { name, .. } => {
                panic!("unexpected perception input field `{name}` in new context")
            }
            Field::PersistentState { name, path, .. } => {
                let accessor = path_to_accessor_token_stream(
                    quote! { persistent_state },
                    path,
                    ReferenceKind::Mutable,
                    cycler,
                );
                quote! {
                    #name: #accessor,
                }
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
            let readers = generate_reader_identifiers(cyclers);
            quote! {
                own_producer,
                #(#readers,)*
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

fn generate_reader_identifiers(cyclers: &Cyclers) -> Vec<Ident> {
    cyclers
        .instances_with(CyclerKind::RealTime)
        .map(|(_cycler, instance)| format_ident!("{}_reader", instance.to_case(Case::Snake)))
        .collect()
}

fn generate_consumer_identifiers(cyclers: &Cyclers) -> Vec<Ident> {
    cyclers
        .instances_with(CyclerKind::Perception)
        .map(|(_cycler, instance)| format_ident!("{}_consumer", instance.to_case(Case::Snake)))
        .collect()
}

fn generate_start_method() -> TokenStream {
    quote! {
        pub(crate) fn start(
            mut self,
            keep_running: tokio_util::sync::CancellationToken,
        ) -> color_eyre::Result<std::thread::JoinHandle<color_eyre::Result<()>>> {
            let instance_name = format!("{:?}", self.instance);
            std::thread::Builder::new()
                .name(instance_name.clone())
                .spawn(move || {
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

fn generate_cycle_method(cycler: &Cycler, cyclers: &Cyclers) -> TokenStream {
    let setup_node_executions = cycler
        .setup_nodes
        .iter()
        .map(|node| generate_node_execution(node, cycler));
    let cycle_node_executions = cycler
        .cycle_nodes
        .iter()
        .map(|node| generate_node_execution(node, cycler));

    let post_setup = match cycler.kind {
        CyclerKind::Perception => quote! {
            self.own_producer.announce();
        },
        CyclerKind::RealTime => {
            let perception_cycler_updates = generate_perception_cycler_updates(cyclers);

            quote! {
                let now = <HardwareInterface as hardware::TimeInterface>::get_now(&*self.hardware_interface);
                self.perception_databases.update(now, crate::perception_databases::Updates {
                    #perception_cycler_updates
                });
            }
        }
    };
    let lock_readers = match cycler.kind {
        CyclerKind::Perception => cyclers
            .instances_with(CyclerKind::RealTime)
            .map(|(_cycler, instance)| {
                let reader = format_ident!("{}_reader", instance.to_case(Case::Snake));
                let database = format_ident!("{}_database", instance.to_case(Case::Snake));
                quote! {
                    let #database = self.#reader.next();
                }
            })
            .collect(),
        CyclerKind::RealTime => quote! {},
    };
    let after_remaining_nodes = match cycler.kind {
        CyclerKind::Perception => quote! {
            self.own_producer.finalize(own_database_reference.main_outputs.clone());
        },
        CyclerKind::RealTime => quote! {
            self.historic_databases.update(
                now,
                self.perception_databases
                    .get_first_timestamp_of_temporary_databases(),
                &own_database_reference.main_outputs,
            );
        },
    };

    quote! {
        #[allow(clippy::nonminimal_bool)]
        pub(crate) fn cycle(&mut self) -> color_eyre::Result<()> {
            {
                let instance = self.instance;
                let instance_name = format!("{instance:?}");
                let itt_domain = ittapi::Domain::new(&instance_name);

                let mut own_database = self.own_writer.next();
                let own_database_reference = {
                    use std::ops::DerefMut;
                    own_database.deref_mut()
                };

                {
                    let own_subscribed_outputs = self.own_subscribed_outputs_reader.next();
                    let parameters = self.parameters_reader.next();
                    #(#setup_node_executions)*
                }

                #post_setup

                {
                    let own_subscribed_outputs = self.own_subscribed_outputs_reader.next();
                    let parameters = self.parameters_reader.next();
                    #lock_readers
                    #(#cycle_node_executions)*
                }

                #after_remaining_nodes
            }
            self.own_changed.notify_one();
            Ok(())
        }
    }
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

fn generate_node_execution(node: &Node, cycler: &Cycler) -> TokenStream {
    let are_required_inputs_some = generate_required_input_condition(node, cycler);
    let node_name = &node.name;
    let node_module = &node.module;
    let node_member = format_ident!("{}", node.name.to_case(Case::Snake));
    let context_initializers = generate_context_initializers(node, cycler);
    let error_message = format!("failed to execute cycle of `{}`", node.name);
    let database_updates = generate_database_updates(node);
    let database_updates_from_defaults = generate_database_updates_from_defaults(node);
    quote! {
        {
            #[allow(clippy::needless_else)]
            if #are_required_inputs_some {
                let main_outputs = {
                    let _task = ittapi::Task::begin(&itt_domain, #node_name);
                    self.#node_member.cycle(
                        #node_module::CycleContext {
                            #context_initializers
                        },
                    )
                    .wrap_err(#error_message)?
                };
                #database_updates
            }
            else {
                #database_updates_from_defaults
            }
        }
    }
}

fn generate_required_input_condition(node: &Node, cycler: &Cycler) -> TokenStream {
    let conditions = node
        .contexts
        .cycle_context
        .iter()
        .filter_map(|field| match field {
            Field::RequiredInput {
                cycler_instance,
                path,
                ..
            } => {
                let database_prefix = match cycler_instance {
                    Some(cycler_instance) => {
                        let identifier =
                            format_ident!("{}_database", cycler_instance.to_case(Case::Snake));
                        quote! { #identifier.main_outputs }
                    }
                    None => {
                        quote! { own_database_reference.main_outputs }
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
            _ => None,
        })
        .chain(once(quote! {true}));
    quote! {
        #(#conditions)&&*
    }
}

fn generate_context_initializers(node: &Node, cycler: &Cycler) -> TokenStream {
    let initializers = node
            .contexts
            .cycle_context
            .iter()
            .map(|field| match field {
                Field::AdditionalOutput { name, path, .. } => {
                    let accessor = path_to_accessor_token_stream(
                        quote!{ own_database_reference.additional_outputs },
                        path,
                        ReferenceKind::Mutable,
                        cycler,
                    );
                    let path_string = once("additional_outputs").chain(
                            path.segments.iter().map(|segment| segment.name.as_str())
                        ).join(".");
                    quote! {
                        #name: framework::AdditionalOutput::new(
                            own_subscribed_outputs
                                .iter()
                                .any(|subscribed_output| framework::should_be_filled(subscribed_output, #path_string)),
                            #accessor,
                        )
                    }
                }
                Field::HardwareInterface { name } => quote! {
                    #name: &self.hardware_interface
                },
                Field::HistoricInput { name, path, .. } => {
                    let now_accessor = path_to_accessor_token_stream(
                        quote!{ own_database_reference.main_outputs },
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
                        #name: [(now, #now_accessor)]
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
                }
                Field::Input {
                    cycler_instance,
                    name,
                    path,
                    ..
                } => {
                    let database_prefix = match cycler_instance {
                        Some(cycler_instance) => {
                            let identifier =
                                format_ident!("{}_database", cycler_instance.to_case(Case::Snake));
                            quote! { #identifier.main_outputs }
                        }
                        None => {
                            quote! { own_database_reference.main_outputs }
                        }
                    };
                    let accessor = path_to_accessor_token_stream(
                        database_prefix,
                        path,
                        ReferenceKind::Immutable,
                        cycler,
                    );
                    quote! {
                        #name: #accessor
                    }
                }
                Field::MainOutput { name, .. } => {
                    panic!("unexpected MainOutput `{name}` in cycle context")
                }
                Field::Parameter { name, path, .. } => {
                    let accessor = path_to_accessor_token_stream(
                        quote! { parameters },
                        path,
                        ReferenceKind::Immutable,
                        cycler,
                    );
                    quote! {
                        #name: #accessor
                    }
                }
                Field::PerceptionInput {
                    cycler_instance,
                    name,
                    path,
                    ..
                } => {
                    let cycler_instance_identifier =
                        format_ident!("{}", cycler_instance.to_case(Case::Snake));
                    let accessor = path_to_accessor_token_stream(
                        quote! { database },
                        path,
                        ReferenceKind::Immutable,
                        cycler,
                    );
                    quote! {
                        #name: framework::PerceptionInput {
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
                }
                Field::PersistentState { name, path, .. } => {
                    let accessor = path_to_accessor_token_stream(
                        quote! { self.persistent_state },
                        path,
                        ReferenceKind::Mutable,
                        cycler,
                    );
                    quote! {
                        #name: #accessor
                    }
                }
                Field::RequiredInput {
                    cycler_instance,
                    name,
                    path,
                    ..
                } => {
                    let database_prefix = match cycler_instance {
                        Some(cycler_instance) => {
                            let identifier =
                                format_ident!("{}_database", cycler_instance.to_case(Case::Snake));
                            quote! { #identifier.main_outputs }
                        }
                        None => {
                            quote! { own_database_reference.main_outputs }
                        }
                    };
                    let accessor = path_to_accessor_token_stream(
                        database_prefix,
                        path,
                        ReferenceKind::Immutable,
                        cycler,
                    );
                    quote! {
                        #name: #accessor .unwrap()
                    }
                }
            })
    ;
    quote! {
        #(#initializers,)*
    }
}

fn generate_database_updates(node: &Node) -> TokenStream {
    node.contexts
        .main_outputs
        .iter()
        .filter_map(|field| match field {
            Field::MainOutput { name, .. } => {
                let setter = quote! {
                    own_database_reference.main_outputs.#name = main_outputs.#name.value;
                };
                Some(setter)
            }
            _ => None,
        })
        .collect()
}

fn generate_database_updates_from_defaults(node: &Node) -> TokenStream {
    node.contexts
        .main_outputs
        .iter()
        .filter_map(|field| match field {
            Field::MainOutput { name, .. } => {
                let setter = quote! {
                    own_database_reference.main_outputs.#name = Default::default();
                };
                Some(setter)
            }
            _ => None,
        })
        .collect()
}
