use std::iter::repeat;

use anyhow::{anyhow, bail, Context};
use build_script_helpers::write_token_stream;
use convert_case::{Case, Casing};
use proc_macro2::{Delimiter, Group, Ident, Punct, Spacing, TokenStream, TokenTree};
use quote::{format_ident, quote, TokenStreamExt};
use source_analyzer::{
    cycler_crates_from_crates_directory, CyclerInstances, CyclerType, CyclerTypes, Field, Modules,
    PathSegment,
};

fn main() -> anyhow::Result<()> {
    for crate_directory in cycler_crates_from_crates_directory("..")
        .context("Failed to get cycler crate directories from crates directory")?
    {
        println!("cargo:rerun-if-changed={}", crate_directory.display());
    }

    let cycler_instances = CyclerInstances::try_from_crates_directory("..")
        .context("Failed to get cycler instances from crates directory")?;
    let mut modules = Modules::try_from_crates_directory("..")
        .context("Failed to get modules from crates directory")?;
    modules.sort().context("Failed to sort modules")?;
    let cycler_types = CyclerTypes::try_from_crates_directory("..")
        .context("Failed to get perception cycler instances from crates directory")?;

    for module_names in modules.cycler_modules_to_modules.values() {
        let first_module_name = match module_names.first() {
            Some(first_module_name) => first_module_name,
            None => continue,
        };
        for field in modules.modules[first_module_name]
            .contexts
            .cycle_context
            .iter()
        {
            match field {
                Field::HistoricInput { name, .. } => bail!(
                    "Unexpected historic input for first module `{first_module_name}` in `{}` for `{name}` in cycle context",
                    modules.modules[first_module_name].cycler_module
                ),
                Field::Input { name, .. } => bail!(
                    "Unexpected optional input for first module `{first_module_name}` in `{}` for `{name}` in cycle context",
                    modules.modules[first_module_name].cycler_module
                ),
                Field::PerceptionInput { name, .. } => bail!(
                    "Unexpected perception input for first module `{first_module_name}` in `{}` for `{name}` in cycle context",
                    modules.modules[first_module_name].cycler_module
                ),
                Field::RequiredInput { name, .. } => bail!(
                    "Unexpected required input for first module `{first_module_name}` in `{}` for `{name}` in cycle context",
                    modules.modules[first_module_name].cycler_module
                ),
                _ => {}
            }
        }
    }

    let cyclers = get_cyclers(&cycler_instances, &modules, &cycler_types);

    let cyclers_token_stream = generate_cyclers(&cyclers).context("Failed to generate cyclers")?;
    let runtime_token_stream = generate_run(&cyclers);

    write_token_stream(
        "cyclers.rs",
        quote! {
            #cyclers_token_stream
            #runtime_token_stream
        },
    )
    .context("Failed to write cyclers")?;

    Ok(())
}

fn get_cyclers<'a>(
    cycler_instances: &'a CyclerInstances,
    modules: &'a Modules,
    cycler_types: &'a CyclerTypes,
) -> Vec<Cycler<'a>> {
    cycler_instances
        .modules_to_instances
        .keys()
        .map(|cycler_module_name| {
            match cycler_types.cycler_modules_to_cycler_types[cycler_module_name] {
                CyclerType::Perception => Cycler::Perception {
                    cycler_instances,
                    modules,
                    cycler_types,
                    cycler_module_name,
                },
                CyclerType::RealTime => Cycler::RealTime {
                    cycler_instances,
                    modules,
                    cycler_types,
                    cycler_module_name,
                },
            }
        })
        .collect()
}

fn generate_cyclers(cyclers: &[Cycler]) -> anyhow::Result<TokenStream> {
    let cyclers: Vec<_> = cyclers
        .iter()
        .map(|cycler| {
            cycler.get_module().with_context(|| {
                anyhow!("Failed to get cycler `{}`", cycler.get_cycler_module_name())
            })
        })
        .collect::<Result<_, _>>()
        .context("Failed to get cyclers")?;

    Ok(quote! {
        #(#cyclers)*
    })
}

fn generate_run(cyclers: &[Cycler]) -> TokenStream {
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
    let n_tuple_buffer_initializers: Vec<_> = cyclers
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
                        let (#writer_identifier, #reader_identifier) = framework::n_tuple_buffer_with_slots([
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

            let (configuration_writer, configuration_reader) = framework::n_tuple_buffer_with_slots([
                #(#configuration_slot_initializers_for_all_cyclers,)*
            ]);
            #(#n_tuple_buffer_initializers)*
            #(#future_queue_initializers)*

            #(#cycler_initializations)*

            #(#cycler_starts)*

            #(#cycler_joins)*

            Ok(())
        }
    }
}

#[derive(Debug)]
enum Cycler<'a> {
    Perception {
        cycler_instances: &'a CyclerInstances,
        modules: &'a Modules,
        cycler_types: &'a CyclerTypes,
        cycler_module_name: &'a str,
    },
    RealTime {
        cycler_instances: &'a CyclerInstances,
        modules: &'a Modules,
        cycler_types: &'a CyclerTypes,
        cycler_module_name: &'a str,
    },
}

impl Cycler<'_> {
    fn get_cycler_instances(&self) -> &CyclerInstances {
        match self {
            Cycler::Perception {
                cycler_instances, ..
            } => cycler_instances,
            Cycler::RealTime {
                cycler_instances, ..
            } => cycler_instances,
        }
    }

    fn get_modules(&self) -> &Modules {
        match self {
            Cycler::Perception { modules, .. } => modules,
            Cycler::RealTime { modules, .. } => modules,
        }
    }

    // TODO: remove?
    // fn get_cycler_types(&self) -> &CyclerTypes {
    //     match self {
    //         Cycler::Perception { cycler_types, .. } => cycler_types,
    //         Cycler::RealTime { cycler_types, .. } => cycler_types,
    //     }
    // }

    fn get_cycler_module_name(&self) -> &str {
        match self {
            Cycler::Perception {
                cycler_module_name, ..
            } => cycler_module_name,
            Cycler::RealTime {
                cycler_module_name, ..
            } => cycler_module_name,
        }
    }

    fn get_cycler_module_name_identifier(&self) -> Ident {
        format_ident!("{}", self.get_cycler_module_name())
    }

    fn get_database_struct(&self) -> TokenStream {
        let cycler_module_name_identifier = self.get_cycler_module_name_identifier();
        quote! {
            #[derive(Default)]
            pub struct Database {
                pub main_outputs: structs::#cycler_module_name_identifier::MainOutputs,
                pub additional_outputs: structs::#cycler_module_name_identifier::AdditionalOutputs,
            }
        }
    }

    fn get_own_producer_identifier(&self) -> TokenStream {
        match self {
            Cycler::Perception { .. } => quote! { own_producer, },
            Cycler::RealTime { .. } => Default::default(),
        }
    }

    fn get_own_producer_type(&self) -> TokenStream {
        let cycler_module_name_identifier = self.get_cycler_module_name_identifier();
        quote! {
            framework::Producer<
                structs::#cycler_module_name_identifier::MainOutputs,
            >
        }
    }

    fn get_own_producer_field(&self) -> TokenStream {
        let own_producer_type = self.get_own_producer_type();
        match self {
            Cycler::Perception { .. } => quote! { own_producer: #own_producer_type, },
            Cycler::RealTime { .. } => Default::default(),
        }
    }

    fn get_other_cyclers(&self) -> Vec<OtherCycler> {
        match self {
            Cycler::Perception {
                cycler_instances,
                cycler_types,
                ..
            } => cycler_types
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
                cycler_instances,
                cycler_types,
                ..
            } => cycler_types
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

    fn get_other_cycler_identifiers(&self) -> Vec<Ident> {
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

    fn get_other_cycler_fields(&self) -> Vec<TokenStream> {
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

    fn get_perception_cycler_updates(&self) -> Vec<TokenStream> {
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

    fn get_perception_cycler_databases(&self) -> Vec<TokenStream> {
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

    fn get_interpreted_modules(&self) -> Vec<Module> {
        self.get_modules()
            .modules
            .iter()
            .filter_map(|(module_name, module)| {
                if module.cycler_module != self.get_cycler_module_name() {
                    return None;
                }

                Some(Module {
                    cycler_instances: self.get_cycler_instances(),
                    module_name,
                    module,
                })
            })
            .collect()
    }

    fn get_module_identifiers(&self) -> Vec<Ident> {
        self.get_interpreted_modules()
            .into_iter()
            .map(|module| module.get_identifier_snake_case())
            .collect()
    }

    fn get_module_fields(&self) -> Vec<TokenStream> {
        self.get_interpreted_modules()
            .into_iter()
            .map(|module| module.get_field())
            .collect()
    }

    fn get_module_initializers(&self) -> anyhow::Result<Vec<TokenStream>> {
        self.get_interpreted_modules()
            .into_iter()
            .map(|module| module.get_initializer())
            .collect()
    }

    fn get_module_executions(&self) -> anyhow::Result<Vec<TokenStream>> {
        self.get_interpreted_modules()
            .into_iter()
            .map(|module| module.get_execution())
            .collect()
    }

    fn get_struct_definition(&self) -> TokenStream {
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
        let module_fields = self.get_module_fields();

        quote! {
            #database_struct

            pub struct Cycler<Interface> {
                instance: #cycler_module_name_identifier::CyclerInstance,
                hardware_interface: std::sync::Arc<Interface>,
                own_writer: framework::Writer<Database>,
                #own_producer_field
                #(#other_cycler_fields,)*
                configuration_reader: framework::Reader<structs::Configuration>,
                #real_time_fields
                persistent_state: structs::#cycler_module_name_identifier::PersistentState,
                #(#module_fields,)*
            }
        }
    }

    fn get_new_method(&self) -> anyhow::Result<TokenStream> {
        let own_producer_field = self.get_own_producer_field();
        let other_cycler_fields = self.get_other_cycler_fields();
        let cycler_module_name_identifier = self.get_cycler_module_name_identifier();
        let module_initializers = self
            .get_module_initializers()
            .context("Failed to get module initializers")?;
        let own_producer_identifier = self.get_own_producer_identifier();
        let other_cycler_identifiers = self.get_other_cycler_identifiers();
        let real_time_initializers = match self {
            Cycler::Perception { .. } => Default::default(),
            Cycler::RealTime { .. } => quote! {
                historic_databases: Default::default(),
                perception_databases: Default::default(),
            },
        };
        let module_identifiers = self.get_module_identifiers();

        Ok(quote! {
            pub fn new(
                instance: #cycler_module_name_identifier::CyclerInstance,
                hardware_interface: std::sync::Arc<Interface>,
                own_writer: framework::Writer<Database>,
                #own_producer_field
                #(#other_cycler_fields,)*
                configuration_reader: framework::Reader<structs::Configuration>,
            ) -> anyhow::Result<Self> {
                use anyhow::Context;
                let configuration = configuration_reader.next().clone();
                let mut persistent_state = structs::#cycler_module_name_identifier::PersistentState::default();
                #(#module_initializers)*
                Ok(Self {
                    instance,
                    hardware_interface,
                    own_writer,
                    #own_producer_identifier
                    #(#other_cycler_identifiers,)*
                    configuration_reader,
                    #real_time_initializers
                    persistent_state,
                    #(#module_identifiers,)*
                })
            }
        })
    }

    fn get_start_method(&self) -> TokenStream {
        quote! {
            pub fn start(
                mut self,
                keep_running: tokio_util::sync::CancellationToken,
            ) -> anyhow::Result<std::thread::JoinHandle<anyhow::Result<()>>> {
                use anyhow::Context;
                let instance_name = format!("{:?}", self.instance);
                std::thread::Builder::new()
                    .name(instance_name.clone())
                    .spawn(move || {
                        while !keep_running.is_cancelled() {
                            if let Err(error) = self.cycle() {
                                keep_running.cancel();
                                return Err(error).context("Failed to execute cycle of cycler");
                            }
                        }
                        Ok(())
                    })
                    .with_context(|| {
                        anyhow::anyhow!("Failed to spawn thread for `{instance_name}`")
                    })
            }
        }
    }

    fn get_cycle_method(&self) -> anyhow::Result<TokenStream> {
        let module_executions = self
            .get_module_executions()
            .context("Failed to get module executions")?;

        if module_executions.is_empty() {
            bail!("Expected at least one module");
        }

        let before_first_module = quote! {
            let mut own_database = self.own_writer.next();
            let own_database_reference = {
                use std::ops::DerefMut;
                own_database.deref_mut()
            };
        };
        let (first_module, remaining_modules) = module_executions.split_at(1);
        let first_module = {
            let first_module = &first_module[0];
            quote! {
                {
                    let configuration = self.configuration_reader.next();
                    #first_module
                }
            }
        };
        let after_first_module = match self {
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
        let remaining_modules = match remaining_modules.is_empty() {
            true => Default::default(),
            false => quote! {
                {
                    let configuration = self.configuration_reader.next();
                    #(#other_cycler_databases)*
                    #(#remaining_modules)*
                }
            },
        };
        let after_remaining_modules = match self {
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
            // todo!("notify communication");
        };

        Ok(quote! {
            fn cycle(&mut self) -> anyhow::Result<()> {
                use anyhow::Context;
                {
                    #before_first_module
                    #first_module
                    #after_first_module
                    #remaining_modules
                    #after_remaining_modules
                }
                #after_dropping_database_writer_guard
                Ok(())
            }
        })
    }

    fn get_struct_implementation(&self) -> anyhow::Result<TokenStream> {
        let new_method = self
            .get_new_method()
            .context("Failed to get `new` method")?;
        let start_method = self.get_start_method();
        let cycle_method = self
            .get_cycle_method()
            .context("Failed to get `cycle` method")?;

        Ok(quote! {
            impl<Interface> Cycler<Interface>
            where
                Interface: hardware::HardwareInterface + Send + Sync + 'static,
            {
                #new_method
                #start_method
                #cycle_method
            }
        })
    }

    fn get_module(&self) -> anyhow::Result<TokenStream> {
        let cycler_module_name_identifier = self.get_cycler_module_name_identifier();
        let struct_definition = self.get_struct_definition();
        let struct_implementation = self
            .get_struct_implementation()
            .context("Failed to get struct implementation")?;

        Ok(quote! {
            #[allow(dead_code, unused_mut, unused_variables)]
            pub mod #cycler_module_name_identifier {
                #struct_definition
                #struct_implementation
            }
        })
    }
}

enum OtherCycler<'a> {
    Consumer {
        cycler_instance_name: &'a str,
        cycler_module_name: &'a str,
    },
    Reader {
        cycler_instance_name: &'a str,
        cycler_module_name: &'a str,
    },
}

struct Module<'a> {
    cycler_instances: &'a CyclerInstances,
    module_name: &'a str,
    module: &'a source_analyzer::Module,
}

impl Module<'_> {
    fn get_identifier(&self) -> Ident {
        format_ident!("{}", self.module_name)
    }

    fn get_identifier_snake_case(&self) -> Ident {
        format_ident!("{}", self.module_name.to_case(Case::Snake))
    }

    fn get_path_segments(&self) -> Vec<Ident> {
        self.module
            .path_segments
            .iter()
            .map(|segment| format_ident!("{}", segment))
            .collect()
    }

    fn get_field(&self) -> TokenStream {
        let module_name_identifier_snake_case = self.get_identifier_snake_case();
        let module_name_identifier = self.get_identifier();
        let path_segments = self.get_path_segments();
        let cycler_module_name_identifier = format_ident!("{}", self.module.cycler_module);

        quote! {
            #module_name_identifier_snake_case:
                #cycler_module_name_identifier::#(#path_segments::)*#module_name_identifier
        }
    }

    fn get_initializer_field_initializers(&self) -> anyhow::Result<Vec<TokenStream>> {
        let cycler_module_name_identifier = format_ident!("{}", self.module.cycler_module);
        self.module
            .contexts
            .new_context
            .iter()
            .map(|field| match field {
                Field::AdditionalOutput { name, .. } => {
                    bail!("Unexpected additional output field `{name}` in new context")
                }
                Field::HardwareInterface { name } => Ok(quote! {
                    #name: &hardware_interface
                }),
                Field::HistoricInput { name, .. } => {
                    bail!("Unexpected historic input field `{name}` in new context")
                }
                Field::Input { name, .. } => {
                    bail!("Unexpected optional input field `{name}` in new context")
                }
                Field::MainOutput { name, .. } => {
                    bail!("Unexpected main output field `{name}` in new context")
                }
                Field::Parameter { name, path, .. } => {
                    let accessor = path_to_accessor_token_stream(
                        quote! { configuration },
                        &path,
                        ReferenceType::Immutable,
                        quote! { instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    Ok(quote! {
                        #name: #accessor
                    })
                }
                Field::PerceptionInput { name, .. } => {
                    bail!("Unexpected perception input field `{name}` in new context")
                }
                Field::PersistentState { name, path, .. } => {
                    let accessor = path_to_accessor_token_stream(
                        quote! { persistent_state },
                        &path,
                        ReferenceType::Mutable,
                        quote! { instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    Ok(quote! {
                        #name: #accessor
                    })
                }
                Field::RequiredInput { name, .. } => {
                    bail!("Unexpected required input field `{name}` in new context")
                }
            })
            .collect()
    }

    fn get_initializer(&self) -> anyhow::Result<TokenStream> {
        let module_name_identifier_snake_case = self.get_identifier_snake_case();
        let module_name_identifier = self.get_identifier();
        let path_segments = self.get_path_segments();
        let cycler_module_name_identifier = format_ident!("{}", self.module.cycler_module);
        let field_initializers = self
            .get_initializer_field_initializers()
            .context("Failed to generate field initializers")?;
        let error_message = format!("Failed to create module `{}`", self.module_name);

        Ok(quote! {
            let #module_name_identifier_snake_case = #cycler_module_name_identifier::#(#path_segments::)*#module_name_identifier::new(
                #cycler_module_name_identifier::#(#path_segments::)*NewContext {
                    #(#field_initializers,)*
                },
            )
            .context(#error_message)?;
        })
    }

    fn get_required_inputs_are_some(&self) -> Option<TokenStream> {
        let cycler_module_name_identifier = format_ident!("{}", self.module.cycler_module);
        let required_inputs_are_some: Vec<_> = self
            .module
            .contexts
            .cycle_context
            .iter()
            .filter_map(|field| match field {
                Field::RequiredInput {
                    path,
                    cycler_instance,
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
                        &path,
                        ReferenceType::Immutable,
                        quote! { self.instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    // TODO: check if required input actually has at least one optional
                    Some(quote! {
                        #accessor .is_some()
                    })
                }
                _ => None,
            })
            .collect();
        match required_inputs_are_some.is_empty() {
            true => None,
            false => Some(quote! {
                #(#required_inputs_are_some)&&*
            }),
        }
    }

    fn get_execution_field_initializers(&self) -> anyhow::Result<Vec<TokenStream>> {
        let cycler_module_name_identifier = format_ident!("{}", self.module.cycler_module);
        self.module
            .contexts
            .cycle_context
            .iter()
            .map(|field| match field {
                Field::AdditionalOutput { name, path, .. } => {
                    let accessor = path_to_accessor_token_stream(
                        quote! { own_database_reference.additional_outputs },
                        &path,
                        ReferenceType::Mutable,
                        quote! { self.instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    // TODO: is_subscribed
                    Ok(quote! {
                        #name: framework::AdditionalOutput::new(
                            false,
                            #accessor,
                        )
                    })
                }
                Field::HardwareInterface { name } => Ok(quote! {
                    #name: &self.hardware_interface
                }),
                Field::HistoricInput { name, path, .. } => {
                    let now_accessor = path_to_accessor_token_stream(
                        quote! { own_database_reference.main_outputs },
                        &path,
                        ReferenceType::Immutable,
                        quote! { self.instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    let historic_accessor = path_to_accessor_token_stream(
                        quote! { database },
                        &path,
                        ReferenceType::Immutable,
                        quote! { self.instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    Ok(quote! {
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
                    })
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
                            quote! { #identifier }
                        }
                        None => {
                            quote! { own_database_reference.main_outputs }
                        }
                    };
                    let accessor = path_to_accessor_token_stream(
                        database_prefix,
                        &path,
                        ReferenceType::Immutable,
                        quote! { self.instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    Ok(quote! {
                        #name: #accessor
                    })
                }
                Field::MainOutput { name, .. } => {
                    bail!("Unexpected main output field `{name}` in cycle context")
                }
                Field::Parameter { name, path, .. } => {
                    let accessor = path_to_accessor_token_stream(
                        quote! { configuration },
                        &path,
                        ReferenceType::Immutable,
                        quote! { self.instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    Ok(quote! {
                        #name: #accessor
                    })
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
                        &path,
                        ReferenceType::Immutable,
                        quote! { self.instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    Ok(quote! {
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
                    })
                }
                Field::PersistentState { name, path, .. } => {
                    let accessor = path_to_accessor_token_stream(
                        quote! { self.persistent_state },
                        &path,
                        ReferenceType::Mutable,
                        quote! { self.instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    Ok(quote! {
                        #name: #accessor
                    })
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
                        &path,
                        ReferenceType::Immutable,
                        quote! { self.instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    Ok(quote! {
                        #name: #accessor .unwrap()
                    })
                }
            })
            .collect()
    }

    fn get_main_output_setters_from_cycle_result(&self) -> Vec<TokenStream> {
        self.module
            .contexts
            .main_outputs
            .iter()
            .filter_map(|field| match field {
                Field::MainOutput { name, .. } => Some(quote! {
                    own_database_reference.main_outputs.#name = main_outputs.#name.value;
                }),
                _ => None,
            })
            .collect()
    }

    fn get_main_output_setters_from_default(&self) -> Vec<TokenStream> {
        self.module
            .contexts
            .main_outputs
            .iter()
            .filter_map(|field| match field {
                Field::MainOutput { name, .. } => Some(quote! {
                    own_database_reference.main_outputs.#name = Default::default();
                }),
                _ => None,
            })
            .collect()
    }

    fn get_execution(&self) -> anyhow::Result<TokenStream> {
        let module_name_identifier_snake_case = self.get_identifier_snake_case();
        let path_segments = self.get_path_segments();
        let cycler_module_name_identifier = format_ident!("{}", self.module.cycler_module);
        let required_inputs_are_some = self.get_required_inputs_are_some();
        let field_initializers = self
            .get_execution_field_initializers()
            .context("Failed to generate field initializers")?;
        let main_output_setters_from_cycle_result =
            self.get_main_output_setters_from_cycle_result();
        let main_output_setters_from_default = self.get_main_output_setters_from_default();
        let error_message = format!("Failed to execute cycle of module `{}`", self.module_name);
        let module_execution = quote! {
            let main_outputs = self.#module_name_identifier_snake_case.cycle(
                #cycler_module_name_identifier::#(#path_segments::)*CycleContext {
                    #(#field_initializers,)*
                },
            )
            .context(#error_message)?;
            #(#main_output_setters_from_cycle_result)*
        };

        match required_inputs_are_some {
            Some(required_inputs_are_some) => Ok(quote! {
                if #required_inputs_are_some {
                    #module_execution
                } else {
                    #(#main_output_setters_from_default)*
                }
            }),
            None => Ok(quote! {
                {
                    #module_execution
                }
            }),
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum ReferenceType {
    Immutable,
    Mutable,
}

fn path_to_accessor_token_stream(
    prefix_token_stream: TokenStream,
    path: &[PathSegment],
    reference_type: ReferenceType,
    instance: TokenStream,
    cycler_instance_prefix: TokenStream,
    cycler_instances: &[String],
) -> TokenStream {
    fn path_to_accessor_token_stream_with_cycler_instance(
        prefix_token_stream: TokenStream,
        path: &[PathSegment],
        reference_type: ReferenceType,
        cycler_instance: Option<&str>,
    ) -> TokenStream {
        let mut token_stream = TokenStream::default();
        let mut token_stream_within_method = None;

        let path_contains_optional = path.iter().any(|segment| segment.is_optional);
        if !path_contains_optional {
            token_stream.append(TokenTree::Punct(Punct::new('&', Spacing::Alone)));
            if let ReferenceType::Mutable = reference_type {
                token_stream.append(TokenTree::Ident(format_ident!("mut")));
            }
        }

        token_stream.extend(prefix_token_stream);

        for (index, segment) in path.iter().enumerate() {
            {
                let token_stream = match &mut token_stream_within_method {
                    Some(token_stream) => token_stream,
                    None => &mut token_stream,
                };

                token_stream.append(TokenTree::Punct(Punct::new('.', Spacing::Alone)));
                match (segment.is_variable, cycler_instance) {
                    (true, Some(cycler_instance)) => {
                        token_stream.append(TokenTree::Ident(format_ident!(
                            "{}",
                            cycler_instance.to_case(Case::Snake)
                        )));
                    }
                    _ => {
                        token_stream.append(TokenTree::Ident(format_ident!("{}", segment.name)));
                    }
                }
            }

            let is_last_segment = index == path.len() - 1;
            if segment.is_optional {
                match token_stream_within_method.take() {
                    Some(mut token_stream_within_method) => {
                        token_stream_within_method
                            .append(TokenTree::Punct(Punct::new('.', Spacing::Alone)));
                        match reference_type {
                            ReferenceType::Immutable => token_stream_within_method
                                .append(TokenTree::Ident(format_ident!("as_ref"))),
                            ReferenceType::Mutable => token_stream_within_method
                                .append(TokenTree::Ident(format_ident!("as_mut"))),
                        }
                        token_stream_within_method.append(TokenTree::Group(Group::new(
                            Delimiter::Parenthesis,
                            TokenStream::default(),
                        )));

                        token_stream.append(TokenTree::Group(Group::new(
                            Delimiter::Parenthesis,
                            token_stream_within_method,
                        )));
                    }
                    None => {
                        token_stream.append(TokenTree::Punct(Punct::new('.', Spacing::Alone)));
                        match reference_type {
                            ReferenceType::Immutable => {
                                token_stream.append(TokenTree::Ident(format_ident!("as_ref")))
                            }
                            ReferenceType::Mutable => {
                                token_stream.append(TokenTree::Ident(format_ident!("as_mut")))
                            }
                        }
                        token_stream.append(TokenTree::Group(Group::new(
                            Delimiter::Parenthesis,
                            TokenStream::default(),
                        )));
                    }
                }

                if !is_last_segment {
                    token_stream.append(TokenTree::Punct(Punct::new('.', Spacing::Alone)));
                    let next_segments_contain_optional = path
                        .iter()
                        .skip(index + 1)
                        .any(|segment| segment.is_optional);
                    let method_name = match next_segments_contain_optional {
                        true => "and_then",
                        false => "map",
                    };
                    token_stream.append(TokenTree::Ident(format_ident!("{}", method_name)));

                    let mut new_token_stream_within_method = TokenStream::default();
                    new_token_stream_within_method
                        .append(TokenTree::Punct(Punct::new('|', Spacing::Alone)));
                    new_token_stream_within_method
                        .append(TokenTree::Ident(format_ident!("{}", segment.name)));
                    new_token_stream_within_method
                        .append(TokenTree::Punct(Punct::new('|', Spacing::Alone)));
                    if !next_segments_contain_optional {
                        new_token_stream_within_method
                            .append(TokenTree::Punct(Punct::new('&', Spacing::Alone)));
                        if let ReferenceType::Mutable = reference_type {
                            new_token_stream_within_method
                                .append(TokenTree::Ident(format_ident!("mut")));
                        }
                    }
                    new_token_stream_within_method
                        .append(TokenTree::Ident(format_ident!("{}", segment.name)));
                    token_stream_within_method = Some(new_token_stream_within_method);
                }
            }
        }

        if let Some(token_stream_within_method) = token_stream_within_method.take() {
            token_stream.append(TokenTree::Group(Group::new(
                Delimiter::Parenthesis,
                token_stream_within_method,
            )));
        }

        token_stream
    }

    let path_contains_variable = path.iter().any(|segment| {
        if segment.is_variable && segment.name != "cycler_instance" {
            unimplemented!("only $cycler_instance is implemented");
        }
        segment.is_variable
    });
    if path_contains_variable {
        let mut token_stream = TokenStream::default();
        token_stream.append(TokenTree::Ident(format_ident!("match")));
        token_stream.extend(instance.clone());
        let mut token_stream_within_match = TokenStream::default();
        for cycler_instance in cycler_instances {
            token_stream_within_match.extend(cycler_instance_prefix.clone());
            token_stream_within_match.append(format_ident!("{}", cycler_instance));
            token_stream_within_match.append(TokenTree::Punct(Punct::new('=', Spacing::Joint)));
            token_stream_within_match.append(TokenTree::Punct(Punct::new('>', Spacing::Alone)));
            token_stream_within_match.extend(path_to_accessor_token_stream_with_cycler_instance(
                prefix_token_stream.clone(),
                path,
                reference_type,
                Some(cycler_instance),
            ));
            token_stream_within_match.append(TokenTree::Punct(Punct::new(',', Spacing::Alone)));
        }
        token_stream.append(TokenTree::Group(Group::new(
            Delimiter::Brace,
            token_stream_within_match,
        )));
        token_stream
    } else {
        path_to_accessor_token_stream_with_cycler_instance(
            prefix_token_stream,
            path,
            reference_type,
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_with_optionals_result_in_correct_accessor_token_streams() {
        let cases = [
            ("a", ReferenceType::Immutable, quote! { &prefix.a }),
            (
                "$cycler_instance",
                ReferenceType::Immutable,
                quote! { match self.instance_name { CyclerInstance::InstanceA => &prefix.InstanceA, CyclerInstance::InstanceB => &prefix.InstanceB, } },
            ),
            ("a", ReferenceType::Mutable, quote! { &mut prefix.a }),
            (
                "$cycler_instance",
                ReferenceType::Mutable,
                quote! { match self.instance_name { CyclerInstance::InstanceA => &mut prefix.InstanceA, CyclerInstance::InstanceB => &mut prefix.InstanceB, } },
            ),
            ("a/b", ReferenceType::Immutable, quote! { &prefix.a.b }),
            (
                "a/$cycler_instance",
                ReferenceType::Immutable,
                quote! { match self.instance_name { CyclerInstance::InstanceA => &prefix.a.InstanceA, CyclerInstance::InstanceB => &prefix.a.InstanceB, } },
            ),
            ("a/b", ReferenceType::Mutable, quote! { &mut prefix.a.b }),
            (
                "a/$cycler_instance",
                ReferenceType::Mutable,
                quote! { match self.instance_name { CyclerInstance::InstanceA => &mut prefix.a.InstanceA, CyclerInstance::InstanceB => &mut prefix.a.InstanceB, } },
            ),
            ("a/b/c", ReferenceType::Immutable, quote! { &prefix.a.b.c }),
            (
                "a/b/c",
                ReferenceType::Mutable,
                quote! { &mut prefix.a.b.c },
            ),
            (
                "a?/b/c",
                ReferenceType::Immutable,
                quote! { prefix.a.as_ref().map(|a| &a.b.c) },
            ),
            (
                "a?/b/c",
                ReferenceType::Mutable,
                quote! { prefix.a.as_mut().map(|a| &mut a.b.c) },
            ),
            ("a?", ReferenceType::Immutable, quote! { prefix.a.as_ref() }),
            (
                "$cycler_instance?",
                ReferenceType::Immutable,
                quote! { match self.instance_name { CyclerInstance::InstanceA => prefix.InstanceA.as_ref(), CyclerInstance::InstanceB => prefix.InstanceB.as_ref(), } },
            ),
            ("a?", ReferenceType::Mutable, quote! { prefix.a.as_mut() }),
            (
                "$cycler_instance?",
                ReferenceType::Mutable,
                quote! { match self.instance_name { CyclerInstance::InstanceA => prefix.InstanceA.as_mut(), CyclerInstance::InstanceB => prefix.InstanceB.as_mut(), } },
            ),
            (
                "a?/b?/c",
                ReferenceType::Immutable,
                quote! { prefix.a.as_ref().and_then(|a| a.b.as_ref()).map(|b| &b.c) },
            ),
            (
                "a?/b?/c",
                ReferenceType::Mutable,
                quote! { prefix.a.as_mut().and_then(|a| a.b.as_mut()).map(|b| &mut b.c) },
            ),
            (
                "a?/b?/c?",
                ReferenceType::Immutable,
                quote! { prefix.a.as_ref().and_then(|a| a.b.as_ref()).and_then(|b| b.c.as_ref()) },
            ),
            (
                "a?/b?/c?",
                ReferenceType::Mutable,
                quote! { prefix.a.as_mut().and_then(|a| a.b.as_mut()).and_then(|b| b.c.as_mut()) },
            ),
            (
                "a?/b?/c?/d",
                ReferenceType::Immutable,
                quote! { prefix.a.as_ref().and_then(|a| a.b.as_ref()).and_then(|b| b.c.as_ref()).map(|c| &c.d) },
            ),
            (
                "a?/b?/c?/d",
                ReferenceType::Mutable,
                quote! { prefix.a.as_mut().and_then(|a| a.b.as_mut()).and_then(|b| b.c.as_mut()).map(|c| &mut c.d) },
            ),
            (
                "a?/b?/c?/d?",
                ReferenceType::Immutable,
                quote! { prefix.a.as_ref().and_then(|a| a.b.as_ref()).and_then(|b| b.c.as_ref()).and_then(|c| c.d.as_ref()) },
            ),
            (
                "a?/b?/c?/d?",
                ReferenceType::Mutable,
                quote! { prefix.a.as_mut().and_then(|a| a.b.as_mut()).and_then(|b| b.c.as_mut()).and_then(|c| c.d.as_mut()) },
            ),
            (
                "a?/b/c/d?",
                ReferenceType::Immutable,
                quote! { prefix.a.as_ref().and_then(|a| a.b.c.d.as_ref()) },
            ),
            (
                "a?/b/c/d?",
                ReferenceType::Mutable,
                quote! { prefix.a.as_mut().and_then(|a| a.b.c.d.as_mut()) },
            ),
            (
                "a?/b/c/d",
                ReferenceType::Immutable,
                quote! { prefix.a.as_ref().map(|a| &a.b.c.d) },
            ),
            (
                "a?/b/c/d",
                ReferenceType::Mutable,
                quote! { prefix.a.as_mut().map(|a| &mut a.b.c.d) },
            ),
            (
                "a?/b/c?/d",
                ReferenceType::Immutable,
                quote! { prefix.a.as_ref().and_then(|a| a.b.c.as_ref()).map(|c| &c.d) },
            ),
            (
                "a?/b/c?/d",
                ReferenceType::Mutable,
                quote! { prefix.a.as_mut().and_then(|a| a.b.c.as_mut()).map(|c| &mut c.d) },
            ),
            (
                "a/b/c?/d",
                ReferenceType::Immutable,
                quote! { prefix.a.b.c.as_ref().map(|c| &c.d) },
            ),
            (
                "a/b/c?/d",
                ReferenceType::Mutable,
                quote! { prefix.a.b.c.as_mut().map(|c| &mut c.d) },
            ),
            (
                "a/b/c/d",
                ReferenceType::Immutable,
                quote! { &prefix.a.b.c.d },
            ),
            (
                "a/b/c/d",
                ReferenceType::Mutable,
                quote! { &mut prefix.a.b.c.d },
            ),
            (
                "a/b?/c?/d",
                ReferenceType::Immutable,
                quote! { prefix.a.b.as_ref().and_then(|b| b.c.as_ref()).map(|c| &c.d) },
            ),
            (
                "a/b?/c?/d",
                ReferenceType::Mutable,
                quote! { prefix.a.b.as_mut().and_then(|b| b.c.as_mut()).map(|c| &mut c.d) },
            ),
            (
                "a/b?/c?/d?",
                ReferenceType::Immutable,
                quote! { prefix.a.b.as_ref().and_then(|b| b.c.as_ref()).and_then(|c| c.d.as_ref()) },
            ),
            (
                "a/b?/c?/d?",
                ReferenceType::Mutable,
                quote! { prefix.a.b.as_mut().and_then(|b| b.c.as_mut()).and_then(|c| c.d.as_mut()) },
            ),
            (
                "a/b/c/d/e/f?/g/i/j/k/l/m/n",
                ReferenceType::Immutable,
                quote! { prefix.a.b.c.d.e.f.as_ref().map(|f| &f.g.i.j.k.l.m.n) },
            ),
            (
                "a/b/c/d/e/f?/g/i/j/k/l/m/n",
                ReferenceType::Mutable,
                quote! { prefix.a.b.c.d.e.f.as_mut().map(|f| &mut f.g.i.j.k.l.m.n) },
            ),
            (
                "a/b/c/d/e/f?/g/i/j/k/l/m/n?",
                ReferenceType::Immutable,
                quote! { prefix.a.b.c.d.e.f.as_ref().and_then(|f| f.g.i.j.k.l.m.n.as_ref()) },
            ),
            (
                "a/b/c/d/e/f?/g/i/j/k/l/m/n?",
                ReferenceType::Mutable,
                quote! { prefix.a.b.c.d.e.f.as_mut().and_then(|f| f.g.i.j.k.l.m.n.as_mut()) },
            ),
        ];

        for (path, reference_type, expected_token_stream) in cases {
            let path_segments: Vec<_> = path.split('/').map(PathSegment::from).collect();

            let token_stream = path_to_accessor_token_stream(
                quote! { prefix },
                &path_segments,
                reference_type,
                quote! { self.instance_name },
                quote! { CyclerInstance:: },
                &["InstanceA".to_string(), "InstanceB".to_string()],
            );
            assert_eq!(
                token_stream.to_string(),
                expected_token_stream.to_string(),
                "path: {path:?}"
            );
        }
    }
}
