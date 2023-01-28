use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};
use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use source_analyzer::{CyclerInstances, CyclerType, CyclerTypes, Nodes};

use super::{node::Node, other_cycler::OtherCycler};

pub fn get_cyclers<'a>(
    cycler_instances: &'a CyclerInstances,
    nodes: &'a Nodes,
    cycler_types: &'a CyclerTypes,
) -> Vec<Cycler<'a>> {
    cycler_instances
        .modules_to_instances
        .keys()
        .map(|cycler_module_name| {
            match cycler_types.cycler_modules_to_cycler_types[cycler_module_name] {
                CyclerType::Perception => Cycler::Perception {
                    cycler_instances,
                    nodes,
                    cycler_types,
                    cycler_module_name,
                },
                CyclerType::RealTime => Cycler::RealTime {
                    cycler_instances,
                    nodes,
                    cycler_types,
                    cycler_module_name,
                },
            }
        })
        .collect()
}

pub fn generate_cyclers(cyclers: &[Cycler]) -> Result<TokenStream> {
    let cyclers: Vec<_> = cyclers
        .iter()
        .map(|cycler| {
            cycler.get_module().wrap_err_with(|| {
                format!("failed to get cycler `{}`", cycler.get_cycler_module_name())
            })
        })
        .collect::<Result<_, _>>()
        .wrap_err("failed to get cyclers")?;

    Ok(quote! {
        #(#cyclers)*
    })
}

#[derive(Debug)]
pub enum Cycler<'a> {
    Perception {
        cycler_instances: &'a CyclerInstances,
        nodes: &'a Nodes,
        cycler_types: &'a CyclerTypes,
        cycler_module_name: &'a str,
    },
    RealTime {
        cycler_instances: &'a CyclerInstances,
        nodes: &'a Nodes,
        cycler_types: &'a CyclerTypes,
        cycler_module_name: &'a str,
    },
}

impl Cycler<'_> {
    pub fn get_cycler_instances(&self) -> &CyclerInstances {
        match self {
            Cycler::Perception {
                cycler_instances, ..
            } => cycler_instances,
            Cycler::RealTime {
                cycler_instances, ..
            } => cycler_instances,
        }
    }

    pub fn get_nodes(&self) -> &Nodes {
        match self {
            Cycler::Perception { nodes, .. } => nodes,
            Cycler::RealTime { nodes, .. } => nodes,
        }
    }

    fn get_cycler_types(&self) -> &CyclerTypes {
        match self {
            Cycler::Perception { cycler_types, .. } => cycler_types,
            Cycler::RealTime { cycler_types, .. } => cycler_types,
        }
    }

    pub fn get_cycler_module_name(&self) -> &str {
        match self {
            Cycler::Perception {
                cycler_module_name, ..
            } => cycler_module_name,
            Cycler::RealTime {
                cycler_module_name, ..
            } => cycler_module_name,
        }
    }

    pub fn get_cycler_module_name_identifier(&self) -> Ident {
        format_ident!("{}", self.get_cycler_module_name())
    }

    pub fn get_database_struct(&self) -> TokenStream {
        let cycler_module_name_identifier = self.get_cycler_module_name_identifier();
        quote! {
            #[derive(Default, serde::Deserialize, serde::Serialize, serialize_hierarchy::SerializeHierarchy)]
            pub struct Database {
                pub main_outputs: structs::#cycler_module_name_identifier::MainOutputs,
                pub additional_outputs: structs::#cycler_module_name_identifier::AdditionalOutputs,
            }
        }
    }

    pub fn get_own_producer_identifier(&self) -> TokenStream {
        match self {
            Cycler::Perception { .. } => quote! { own_producer, },
            Cycler::RealTime { .. } => Default::default(),
        }
    }

    pub fn get_own_producer_type(&self) -> TokenStream {
        let cycler_module_name_identifier = self.get_cycler_module_name_identifier();
        quote! {
            framework::Producer<
                structs::#cycler_module_name_identifier::MainOutputs,
            >
        }
    }

    pub fn get_own_producer_field(&self) -> TokenStream {
        let own_producer_type = self.get_own_producer_type();
        match self {
            Cycler::Perception { .. } => quote! { own_producer: #own_producer_type, },
            Cycler::RealTime { .. } => Default::default(),
        }
    }

    pub fn get_other_cyclers(&self) -> Vec<OtherCycler> {
        match self {
            Cycler::Perception {
                cycler_instances, ..
            } => self
                .get_cycler_types()
                .cycler_modules_to_cycler_types
                .iter()
                .filter_map(
                    |(other_cycler_module_name, other_cycler_type)| match other_cycler_type {
                        CyclerType::RealTime => Some(
                            cycler_instances.modules_to_instances[other_cycler_module_name]
                                .iter()
                                .map(|other_cycler_instance_name| OtherCycler::Reader {
                                    cycler_instance_name: other_cycler_instance_name,
                                    cycler_module_name: other_cycler_module_name,
                                }),
                        ),
                        CyclerType::Perception => None,
                    },
                )
                .flatten()
                .collect(),
            Cycler::RealTime {
                cycler_instances, ..
            } => self
                .get_cycler_types()
                .cycler_modules_to_cycler_types
                .iter()
                .filter_map(
                    |(other_cycler_module_name, other_cycler_type)| match other_cycler_type {
                        CyclerType::Perception => Some(
                            cycler_instances.modules_to_instances[other_cycler_module_name]
                                .iter()
                                .map(|other_cycler_instance_name| OtherCycler::Consumer {
                                    cycler_instance_name: other_cycler_instance_name,
                                    cycler_module_name: other_cycler_module_name,
                                }),
                        ),
                        CyclerType::RealTime => None,
                    },
                )
                .flatten()
                .collect(),
        }
    }

    pub fn get_other_cycler_identifiers(&self) -> Vec<Ident> {
        self.get_other_cyclers()
            .into_iter()
            .map(|other_cycler| match other_cycler {
                OtherCycler::Consumer {
                    cycler_instance_name,
                    ..
                } => format_ident!("{}_consumer", cycler_instance_name.to_case(Case::Snake)),
                OtherCycler::Reader {
                    cycler_instance_name,
                    ..
                } => format_ident!("{}_reader", cycler_instance_name.to_case(Case::Snake)),
            })
            .collect()
    }

    pub fn get_other_cycler_fields(&self) -> Vec<TokenStream> {
        self.get_other_cyclers()
            .into_iter()
            .map(|other_cycler| {
                let (field_name, field_type) = match other_cycler {
                    OtherCycler::Consumer {
                        cycler_instance_name,
                        cycler_module_name,
                    } => {
                        let cycler_module_name_identifier = format_ident!("{}", cycler_module_name);
                        (
                            format_ident!("{}_consumer", cycler_instance_name.to_case(Case::Snake)),
                            quote! {
                                framework::Consumer<
                                    structs::#cycler_module_name_identifier::MainOutputs,
                                >
                            },
                        )
                    }
                    OtherCycler::Reader {
                        cycler_instance_name,
                        cycler_module_name,
                    } => {
                        let cycler_module_name_identifier = format_ident!("{}", cycler_module_name);
                        (
                            format_ident!("{}_reader", cycler_instance_name.to_case(Case::Snake)),
                            quote! {
                                framework::Reader<crate::#cycler_module_name_identifier::Database>
                            },
                        )
                    }
                };
                quote! {
                    #field_name: #field_type
                }
            })
            .collect()
    }

    pub fn get_perception_cycler_updates(&self) -> Vec<TokenStream> {
        self.get_other_cyclers()
            .into_iter()
            .filter_map(|other_cycler| match other_cycler {
                OtherCycler::Consumer {
                    cycler_instance_name,
                    ..
                } => {
                    let update_name_identifier =
                        format_ident!("{}", cycler_instance_name.to_case(Case::Snake));
                    let consumer_identifier =
                        format_ident!("{}_consumer", cycler_instance_name.to_case(Case::Snake));

                    Some(quote! {
                        #update_name_identifier: self.#consumer_identifier.consume(now)
                    })
                }
                OtherCycler::Reader { .. } => None,
            })
            .collect()
    }

    pub fn get_perception_cycler_databases(&self) -> Vec<TokenStream> {
        self.get_other_cyclers()
            .into_iter()
            .filter_map(|other_cycler| match other_cycler {
                OtherCycler::Reader {
                    cycler_instance_name,
                    ..
                } => {
                    let reader_identifier =
                        format_ident!("{}_reader", cycler_instance_name.to_case(Case::Snake));
                    let database_identifier =
                        format_ident!("{}_database", cycler_instance_name.to_case(Case::Snake));

                    Some(quote! {
                        let #database_identifier = self.#reader_identifier.next();
                    })
                }
                OtherCycler::Consumer { .. } => None,
            })
            .collect()
    }

    pub fn get_interpreted_nodes(&self) -> Vec<Node> {
        let nodes = self.get_nodes();
        nodes.cycler_modules_to_nodes[self.get_cycler_module_name()]
            .iter()
            .map(|node_name| Node {
                cycler_instances: self.get_cycler_instances(),
                node_name,
                node: nodes.nodes.get(node_name).expect("missing node"),
            })
            .collect()
    }

    pub fn get_node_identifiers(&self) -> Vec<Ident> {
        self.get_interpreted_nodes()
            .into_iter()
            .map(|node| node.get_identifier_snake_case())
            .collect()
    }

    pub fn get_node_fields(&self) -> Vec<TokenStream> {
        self.get_interpreted_nodes()
            .into_iter()
            .map(|node| node.get_field())
            .collect()
    }

    pub fn get_node_initializers(&self) -> Result<Vec<TokenStream>> {
        self.get_interpreted_nodes()
            .into_iter()
            .map(|node| node.get_initializer())
            .collect()
    }

    pub fn get_node_executions(&self) -> Result<Vec<TokenStream>> {
        self.get_interpreted_nodes()
            .into_iter()
            .map(|node| node.get_execution())
            .collect()
    }

    pub fn get_struct_definition(&self) -> TokenStream {
        let database_struct = self.get_database_struct();
        let own_producer_field = self.get_own_producer_field();
        let other_cycler_fields = self.get_other_cycler_fields();
        let cycler_module_name_identifier = self.get_cycler_module_name_identifier();
        let real_time_fields = match self {
            Cycler::Perception { .. } => Default::default(),
            Cycler::RealTime {
                cycler_module_name, ..
            } => {
                let cycler_module_name_identifier = format_ident!("{}", cycler_module_name);

                quote! {
                    historic_databases: framework::HistoricDatabases<
                        structs::#cycler_module_name_identifier::MainOutputs,
                    >,
                    perception_databases: framework::PerceptionDatabases,
                }
            }
        };
        let node_fields = self.get_node_fields();

        quote! {
            #database_struct

            pub struct Cycler<Interface> {
                instance: #cycler_module_name_identifier::CyclerInstance,
                hardware_interface: std::sync::Arc<Interface>,
                own_writer: framework::Writer<Database>,
                #own_producer_field
                #(#other_cycler_fields,)*
                own_changed: std::sync::Arc<tokio::sync::Notify>,
                own_subscribed_outputs_reader: framework::Reader<std::collections::HashSet<String>>,
                configuration_reader: framework::Reader<structs::Configuration>,
                #real_time_fields
                persistent_state: structs::#cycler_module_name_identifier::PersistentState,
                #(#node_fields,)*
            }
        }
    }

    pub fn get_new_method(&self) -> Result<TokenStream> {
        let own_producer_field = self.get_own_producer_field();
        let other_cycler_fields = self.get_other_cycler_fields();
        let cycler_module_name_identifier = self.get_cycler_module_name_identifier();
        let node_initializers = self
            .get_node_initializers()
            .wrap_err("failed to get node initializers")?;
        let own_producer_identifier = self.get_own_producer_identifier();
        let other_cycler_identifiers = self.get_other_cycler_identifiers();
        let real_time_initializers = match self {
            Cycler::Perception { .. } => Default::default(),
            Cycler::RealTime { .. } => quote! {
                historic_databases: Default::default(),
                perception_databases: Default::default(),
            },
        };
        let node_identifiers = self.get_node_identifiers();

        Ok(quote! {
            pub fn new(
                instance: #cycler_module_name_identifier::CyclerInstance,
                hardware_interface: std::sync::Arc<Interface>,
                own_writer: framework::Writer<Database>,
                #own_producer_field
                #(#other_cycler_fields,)*
                own_changed: std::sync::Arc<tokio::sync::Notify>,
                own_subscribed_outputs_reader: framework::Reader<std::collections::HashSet<String>>,
                configuration_reader: framework::Reader<structs::Configuration>,
            ) -> color_eyre::Result<Self> {
                use color_eyre::eyre::WrapErr;
                let configuration = configuration_reader.next().clone();
                let mut persistent_state = structs::#cycler_module_name_identifier::PersistentState::default();
                #(#node_initializers)*
                Ok(Self {
                    instance,
                    hardware_interface,
                    own_writer,
                    #own_producer_identifier
                    #(#other_cycler_identifiers,)*
                    own_changed,
                    own_subscribed_outputs_reader,
                    configuration_reader,
                    #real_time_initializers
                    persistent_state,
                    #(#node_identifiers,)*
                })
            }
        })
    }

    pub fn get_start_method(&self) -> TokenStream {
        quote! {
            pub fn start(
                mut self,
                keep_running: tokio_util::sync::CancellationToken,
            ) -> color_eyre::Result<std::thread::JoinHandle<color_eyre::Result<()>>> {
                use color_eyre::eyre::WrapErr;
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

    pub fn get_cycle_method(&self) -> Result<TokenStream> {
        let node_executions = self
            .get_node_executions()
            .wrap_err("failed to get node executions")?;

        if node_executions.is_empty() {
            bail!("expected at least one node");
        }

        let before_first_node = quote! {
            let mut own_database = self.own_writer.next();
            let own_database_reference = {
                use std::ops::DerefMut;
                own_database.deref_mut()
            };
        };
        let (first_node, remaining_nodes) = node_executions.split_at(1);
        let first_node = {
            let first_node = &first_node[0];
            quote! {
                {
                    let own_subscribed_outputs = self.own_subscribed_outputs_reader.next();
                    let configuration = self.configuration_reader.next();
                    #first_node
                }
            }
        };
        let after_first_node = match self {
            Cycler::Perception { .. } => quote! {
                self.own_producer.announce();
            },
            Cycler::RealTime { .. } => {
                let perception_cycler_updates = self.get_perception_cycler_updates();

                quote! {
                    let now = self.hardware_interface.get_now();
                    self.perception_databases.update(now, framework::Updates {
                        #(#perception_cycler_updates,)*
                    });
                }
            }
        };
        let other_cycler_databases = self.get_perception_cycler_databases();
        let remaining_nodes = match remaining_nodes.is_empty() {
            true => Default::default(),
            false => quote! {
                {
                    let own_subscribed_outputs = self.own_subscribed_outputs_reader.next();
                    let configuration = self.configuration_reader.next();
                    #(#other_cycler_databases)*
                    #(#remaining_nodes)*
                }
            },
        };
        let after_remaining_nodes = match self {
            Cycler::Perception { .. } => quote! {
                self.own_producer.finalize(own_database_reference.main_outputs.clone());
            },
            Cycler::RealTime { .. } => quote! {
                self.historic_databases.update(
                    now,
                    self.perception_databases
                        .get_first_timestamp_of_temporary_databases(),
                    &own_database_reference.main_outputs,
                );
            },
        };
        let after_dropping_database_writer_guard = quote! {
            self.own_changed.notify_one();
        };

        Ok(quote! {
            pub fn cycle(&mut self) -> color_eyre::Result<()> {
                use color_eyre::eyre::WrapErr;
                {
                    let instance_name = format!("{:?}", self.instance);
                    let itt_domain = ittapi::Domain::new(&instance_name);
                    #before_first_node
                    #first_node
                    #after_first_node
                    #remaining_nodes
                    #after_remaining_nodes
                }
                #after_dropping_database_writer_guard
                Ok(())
            }
        })
    }

    pub fn get_struct_implementation(&self) -> Result<TokenStream> {
        let new_method = self
            .get_new_method()
            .wrap_err("failed to get `new` method")?;
        let start_method = self.get_start_method();
        let cycle_method = self
            .get_cycle_method()
            .wrap_err("failed to get `cycle` method")?;

        Ok(quote! {
            impl<Interface> Cycler<Interface>
            where
                Interface: types::hardware::Interface + std::marker::Send + std::marker::Sync + 'static,
            {
                #new_method
                #start_method
                #cycle_method
            }
        })
    }

    pub fn get_module(&self) -> Result<TokenStream> {
        let cycler_module_name_identifier = self.get_cycler_module_name_identifier();
        let struct_definition = self.get_struct_definition();
        let struct_implementation = self
            .get_struct_implementation()
            .wrap_err("failed to get struct implementation")?;

        Ok(quote! {
            #[allow(dead_code, unused_mut, unused_variables)]
            pub mod #cycler_module_name_identifier {
                #struct_definition
                #struct_implementation
            }
        })
    }
}
