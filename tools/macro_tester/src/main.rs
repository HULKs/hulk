use std::{collections::BTreeMap, fs::File, io::Write, path::Path, process::Command};

use anyhow::{anyhow, bail, Context};
use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use source_analyzer::{CyclerInstances, CyclerType, CyclerTypes, Field, Module, Modules};

pub fn write_token_stream<P>(file_path: P, token_stream: TokenStream) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    {
        let mut file = File::create(&file_path)
            .with_context(|| anyhow!("Failed create file {:?}", file_path.as_ref()))?;
        write!(file, "{}", token_stream)
            .with_context(|| anyhow!("Failed to write to file {:?}", file_path.as_ref()))?;
    }

    let status = Command::new("rustfmt")
        .arg(file_path.as_ref())
        .status()
        .context("Failed to execute rustfmt")?;
    if !status.success() {
        bail!("rustfmt did not exit with success");
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cycler_instances = CyclerInstances::try_from_crates_directory("crates")
        .context("Failed to get cycler instances from crates directory")?;
    let mut modules = Modules::try_from_crates_directory("crates")
        .context("Failed to get modules from crates directory")?;
    modules.sort().context("Failed to sort modules")?;
    let cycler_types = CyclerTypes::try_from_crates_directory("crates")
        .context("Failed to get perception cycler instances from crates directory")?;

    let mut file = File::create("build.rs.log")?;

    let cyclers = generate_cyclers(&mut file, &cycler_instances, &modules, &cycler_types)
        .context("Failed to generate cyclers")?;

    writeln!(file, "cyclers: {}", cyclers)?;

    write_token_stream("cyclers.rs", cyclers).context("Failed to write cyclers")?;

    writeln!(file, "cycler_instances: {cycler_instances:#?}")?;
    writeln!(file, "modules: {modules:#?}")?;
    writeln!(file, "cycler_types: {cycler_types:#?}")?;

    Ok(())
}

fn generate_cyclers(
    file: &mut File,
    cycler_instances: &CyclerInstances,
    modules: &Modules,
    cycler_types: &CyclerTypes,
) -> anyhow::Result<TokenStream> {
    let mut cyclers = vec![];
    for cycler_module_name in cycler_instances.modules_to_instances.keys() {
        cyclers.push(
            generate_cycler(
                file,
                cycler_instances,
                modules,
                cycler_types,
                cycler_module_name,
            )
            .with_context(|| {
                anyhow!("Failed to generate cycler for module {cycler_module_name:?}")
            })?,
        );
    }
    Ok(quote! {
        #[derive(Default)]
        pub struct Outputs<MainOutputs, AdditionalOutputs>
        where
            MainOutputs: Default,
            AdditionalOutputs: Default,
        {
            pub main_outputs: MainOutputs,
            pub additional_outputs: AdditionalOutputs,
        }

        #(#cyclers)*
    })
}

fn generate_cycler(
    file: &mut File,
    cycler_instances: &CyclerInstances,
    modules: &Modules,
    cycler_types: &CyclerTypes,
    cycler_module_name: &str,
) -> anyhow::Result<TokenStream> {
    writeln!(file, "generate_cycler({cycler_module_name:?})")?;
    let cycler_type = &cycler_types.cycler_modules_to_cycler_types[cycler_module_name];
    writeln!(file, "  cycler_type: {cycler_type:?}")?;

    let cycler_module_name_identifier = format_ident!("{}", cycler_module_name);
    let own_writer_type = quote! {
        framework::Writer<
            crate::Outputs<
                structs::#cycler_module_name_identifier::MainOutputs,
                structs::#cycler_module_name_identifier::AdditionalOutputs,
            >
        >
    };
    let own_producer_type = quote! {
        framework::Producer<
            structs::#cycler_module_name_identifier::MainOutputs,
        >
    };
    let own_producer_field = match cycler_type {
        CyclerType::Perception => Some(quote! { own_producer: #own_producer_type, }),
        CyclerType::RealTime => None,
    };
    let other_readers_or_consumers: BTreeMap<_, _> = match cycler_type {
        CyclerType::Perception => cycler_types
            .cycler_modules_to_cycler_types
            .iter()
            .filter_map(|(other_cycler_module_name, other_cycler_type)| {
                let other_cycler_module_name_identifier =
                    format_ident!("{}", other_cycler_module_name);
                match other_cycler_type {
                    CyclerType::RealTime => Some(
                        cycler_instances.modules_to_instances[other_cycler_module_name]
                            .iter()
                            .map(move |other_cycler_instance_name| {
                                (
                                    format!(
                                        "{}_reader",
                                        other_cycler_instance_name.to_case(Case::Snake)
                                    ),
                                    quote! {
                                        framework::Reader<
                                            structs::#other_cycler_module_name_identifier::MainOutputs,
                                        >
                                    },
                                )
                            }),
                    ),
                    _ => None,
                }
            })
            .flatten()
            .collect(),
        CyclerType::RealTime => cycler_types
            .cycler_modules_to_cycler_types
            .iter()
            .filter_map(|(other_cycler_module_name, other_cycler_type)| {
                let other_cycler_module_name_identifier =
                    format_ident!("{}", other_cycler_module_name);
                match other_cycler_type {
                    CyclerType::Perception => Some(
                        cycler_instances.modules_to_instances[other_cycler_module_name]
                            .iter()
                            .map(move |other_cycler_instance_name| {
                                (
                                    format!(
                                        "{}_consumer",
                                        other_cycler_instance_name.to_case(Case::Snake)
                                    ),
                                    quote! {
                                        framework::Consumer<
                                            structs::#other_cycler_module_name_identifier::MainOutputs,
                                        >
                                    },
                                )
                            }),
                    ),
                    CyclerType::RealTime => None,
                }
            })
            .flatten()
            .collect(),
    };
    let other_reader_or_consumer_identifiers = other_readers_or_consumers
        .keys()
        .map(|other_reader_or_consumer_name| format_ident!("{}", other_reader_or_consumer_name));
    let other_reader_or_consumer_fields: Vec<_> = other_readers_or_consumers
        .iter()
        .map(
            |(other_reader_or_consumer_name, other_reader_or_consumer_type)| {
                let other_reader_or_consumer_name_identifier =
                    format_ident!("{}", other_reader_or_consumer_name);
                quote! {
                    #other_reader_or_consumer_name_identifier: #other_reader_or_consumer_type
                }
            },
        )
        .collect();
    let module_names = modules.modules.iter().filter_map(|(module_name, module)| {
        if module.cycler_module != cycler_module_name {
            return None;
        }

        let module_name_identifier_snake_case =
            format_ident!("{}", module_name.to_case(Case::Snake));

        Some(quote! {
            #module_name_identifier_snake_case
        })
    });
    let module_fields = modules.modules.iter().filter_map(|(module_name, module)| {
        if module.cycler_module != cycler_module_name {
            return None;
        }

        let module_name_identifier_snake_case = format_ident!("{}", module_name.to_case(Case::Snake));
        let module_name_identifier = format_ident!("{}", module_name);
        let path_segments = module.path_segments.iter().map(|segment| format_ident!("{}", segment));

        Some(quote! {
            #module_name_identifier_snake_case: #cycler_module_name_identifier::#(#path_segments::)*#module_name_identifier
        })
    });
    let module_initializers: Vec<_> = modules.modules.iter().filter_map(|(module_name, module)| {
        if module.cycler_module != cycler_module_name {
            return None;
        }

        let module_name_identifier_snake_case = format_ident!("{}", module_name.to_case(Case::Snake));
        let module_name_identifier = format_ident!("{}", module_name);
        let path_segments: Vec<_> = module.path_segments.iter().map(|segment| format_ident!("{}", segment)).collect();
        let error_message = format!("Failed to create module `{module_name}`");
        let field_initializers: Vec<_> = match module.contexts.new_context.iter().map(|field| {
            match field {
                Field::AdditionalOutput { name, .. } => bail!("Unexpected additional output field `{name}` in NewContext"),
                Field::HardwareInterface { name } => Ok(quote! {
                    #name: framework::HardwareInterface::from(
                        &hardware_interface,
                    )
                }),
                Field::HistoricInput { name, .. } => bail!("Unexpected historic input field `{name}` in NewContext"),
                Field::MainOutput { name, .. } => bail!("Unexpected main output field `{name}` in NewContext"),
                Field::OptionalInput { name, .. } => bail!("Unexpected optional input field `{name}` in NewContext"),
                Field::Parameter { name, .. } => {
                    let segments = field.get_path_segments().unwrap().into_iter().map(|segment| format_ident!("{}", segment));
                    Ok(quote! {
                        #name: framework::Parameter::from(
                            &configuration #(.#segments)*,
                        )
                    })
                },
                Field::PerceptionInput { name, .. } => bail!("Unexpected perception input field `{name}` in NewContext"),
                Field::PersistentState { name, .. } => {
                    let segments = field.get_path_segments().unwrap().into_iter().map(|segment| format_ident!("{}", segment));
                    Ok(quote! {
                        #name: framework::PersistentState::from(
                            &mut persistent_state #(.#segments)*,
                        )
                    })
                },
                Field::RequiredInput { name, .. } => bail!("Unexpected required input field `{name}` in NewContext"),
            }
        }).collect::<Result<_, _>>().context("Failed to generate field initializers") {
            Ok(field_initializers) => field_initializers,
            Err(error) => return Some(Err(error)),
        };

        Some(Ok(quote!{
            let #module_name_identifier_snake_case = #cycler_module_name_identifier::#(#path_segments::)*#module_name_identifier::new(
                #cycler_module_name_identifier::#(#path_segments::)::*NewContext {
                    #(#field_initializers,)*
                },
            )
            .context(#error_message)?;
        }))
    }).collect::<Result<_, _>>().context("Failed to generate module initializers")?;

    let cycle_method = generate_cycle_method(
        cycler_instances,
        modules,
        cycler_types,
        cycler_type,
        cycler_module_name,
        &cycler_module_name_identifier,
    )
    .context("Failed to generate cycle method for cycler")?;

    Ok(quote! {
        pub mod #cycler_module_name_identifier {
            pub struct Cycler<Interface> {
                instance_name: String,
                hardware_interface: std::sync::Arc<Interface>,
                own_writer: #own_writer_type,
                #own_producer_field
                #(#other_reader_or_consumer_fields,)*
                configuration_reader: framework::Reader<structs::Configuration>,
                persistent_state: structs::#cycler_module_name_identifier::PersistentState,
                #(#module_fields,)*
            }

            impl<Interface> Cycler<Interface>
            where
                Interface: hardware::HardwareInterface + Send + Sync + 'static,
            {
                pub fn new(
                    instance_name: String,
                    hardware_interface: std::sync::Arc<Interface>,
                    own_writer: #own_writer_type,
                    #own_producer_field
                    #(#other_reader_or_consumer_fields,)*
                    configuration_reader: framework::Reader<structs::Configuration>,
                    persistent_state: structs::#cycler_module_name_identifier::PersistentState,
                ) -> anyhow::Result<Self> {
                    let configuration = configuration_reader.next().clone();
                    let mut persistent_state = Default::default();
                    #(#module_initializers)*
                    Ok(Self {
                        instance_name,
                        hardware_interface,
                        own_writer,
                        own_producer,
                        #(#other_reader_or_consumer_identifiers,)*
                        configuration_reader,
                        persistent_state,
                        #(#module_names,)*
                    })
                }

                pub fn start(
                    mut self,
                    keep_running: tokio_util::sync::CancellationToken,
                ) -> anyhow::Result<std::thread::JoinHandle<()>> {
                    use anyhow::Context;

                    std::thread::Builder::new()
                        .name(self.instance_name.clone())
                        .spawn(move || {
                            while !keep_running.is_cancelled() {
                                if let Err(error) = self.cycle() {
                                    println!("`cycle` returned error: {error:?}");
                                    keep_running.cancel();
                                }
                            }
                        })
                        .context("Failed to spawn thread")
                }

                #cycle_method
            }
        }
    })
}

fn generate_cycle_method(
    cycler_instances: &CyclerInstances,
    modules: &Modules,
    cycler_types: &CyclerTypes,
    cycler_type: &CyclerType,
    cycler_module_name: &str,
    cycler_module_name_identifier: &Ident,
) -> anyhow::Result<TokenStream> {
    let module_executions: Vec<_> = modules
        .modules
        .iter()
        .filter_map(|(module_name, module)| {
            if module.cycler_module != cycler_module_name {
                return None;
            }

            Some(
                generate_module_cycle_execution(
                    cycler_instances,
                    modules,
                    cycler_types,
                    cycler_module_name,
                    cycler_module_name_identifier,
                    module_name,
                    module,
                )
                .with_context(|| {
                    anyhow!("Failed to generate module execution for module `{module_name}`")
                }),
            )
        })
        .collect::<Result<_, _>>()
        .context("Failed to generate module executions")?;

    if module_executions.is_empty() {
        bail!("Expected at least one module");
    }

    let (first_module, remaining_modules) = module_executions.split_at(1);
    let first_module = &first_module[0];
    let remaining_module_executions = match remaining_modules.is_empty() {
        true => Default::default(),
        false => quote! {
            {
                let configuration = self.configuration_reader.next();

                #(#remaining_modules)*
            }
        },
    };

    Ok(quote! {
        fn cycle(&mut self) -> anyhow::Result<()> {
            use anyhow::Context;

            {
                let mut own_database = self.own_writer.next();

                {
                    let configuration = self.configuration_reader.next();

                    #first_module
                }

                self.own_producer.announce();

                #remaining_module_executions

                self.own_producer.finalize(own_database.main_outputs.clone());
            }
        }
    })
}

fn generate_module_cycle_execution(
    cycler_instances: &CyclerInstances,
    modules: &Modules,
    cycler_types: &CyclerTypes,
    cycler_module_name: &str,
    cycler_module_name_identifier: &Ident,
    module_name: &str,
    module: &Module,
) -> anyhow::Result<TokenStream> {
    let module_name_identifier_snake_case = format_ident!("{}", module_name.to_case(Case::Snake));
    let module_name_identifier = format_ident!("{}", module_name);
    let path_segments: Vec<_> = module
        .path_segments
        .iter()
        .map(|segment| format_ident!("{}", segment))
        .collect();
    let required_inputs_are_some =
        module
            .contexts
            .cycle_context
            .iter()
            .filter_map(|field| match field {
                Field::RequiredInput { .. } => {
                    let segments = field
                        .get_path_segments()
                        .unwrap()
                        .into_iter()
                        .map(|segment| format_ident!("{}", segment));
                    Some(quote! {
                        own_database.main_outputs #(.#segments)* .is_some()
                    })
                }
                _ => None,
            });
    let field_initializers: Vec<_> = module
        .contexts
        .cycle_context
        .iter()
        .map(|field| match field {
            Field::AdditionalOutput { name, .. } => {
                let segments = field
                    .get_path_segments()
                    .unwrap()
                    .into_iter()
                    .map(|segment| format_ident!("{}", segment));
                Ok(quote! {
                    #name: framework::AdditionalOutput::new(
                        false,
                        &mut own_database.additional_outputs #(.#segments)*,
                    )
                })
            }
            Field::HardwareInterface { name } => Ok(quote! {
                #name: framework::HardwareInterface::from(
                    &self.hardware_interface,
                )
            }),
            Field::HistoricInput { name, .. } => {
                bail!("Unexpected historic input field `{name}` in `CycleContext`")
            }
            Field::MainOutput { name, .. } => {
                bail!("Unexpected main output field `{name}` in `CycleContext`")
            }
            Field::OptionalInput { name, .. } => {
                let segments = field
                    .get_path_segments()
                    .unwrap()
                    .into_iter()
                    .map(|segment| format_ident!("{}", segment));
                Ok(quote! {
                    #name: framework::OptionalInput::from(
                        &own_database.main_outputs #(.#segments)*,
                    )
                })
            }
            Field::Parameter { name, .. } => {
                let segments = field
                    .get_path_segments()
                    .unwrap()
                    .into_iter()
                    .map(|segment| format_ident!("{}", segment));
                Ok(quote! {
                    #name: framework::Parameter::from(
                        &configuration #(.#segments)*,
                    )
                })
            }
            Field::PerceptionInput { name, .. } => {
                // bail!("Unexpected perception input field `{name}` in `CycleContext`")
                Ok(quote! {
                    #name: todo!()
                })
            }
            Field::PersistentState { name, .. } => {
                let segments = field
                    .get_path_segments()
                    .unwrap()
                    .into_iter()
                    .map(|segment| format_ident!("{}", segment));
                Ok(quote! {
                    #name: framework::PersistentState::from(
                        &mut self.persistent_state #(.#segments)*,
                    )
                })
            }
            Field::RequiredInput { name, .. } => {
                let segments = field
                    .get_path_segments()
                    .unwrap()
                    .into_iter()
                    .map(|segment| format_ident!("{}", segment));
                Ok(quote! {
                    #name: framework::RequiredInput::from(
                        own_database.main_outputs #(.#segments)*.as_ref().unwrap(),
                    )
                })
            }
        })
        .collect::<Result<_, _>>()
        .context("Failed to generate field initializers")?;
    let set_main_outputs_to_cycle_result =
        module
            .contexts
            .main_outputs
            .iter()
            .filter_map(|field| match field {
                Field::MainOutput { name, .. } => Some(quote! {
                    own_database.main_outputs.#name = main_outputs.#name.value;
                }),
                _ => None,
            });
    let set_main_outputs_to_none =
        module
            .contexts
            .main_outputs
            .iter()
            .filter_map(|field| match field {
                Field::MainOutput { name, .. } => Some(quote! {
                    own_database.main_outputs.#name = None;
                }),
                _ => None,
            });
    let error_message = format!("Failed to execute cycle of module `{module_name}`");

    // We are praying to the Rust compiler that it hopefully optimizes this `if true` away. :pray:
    // That way, we don't need to bother with the empty corner case.
    Ok(quote! {
        if true #(&& #required_inputs_are_some)* {
            let main_outputs = self.#module_name_identifier_snake_case.cycle(
                #cycler_module_name_identifier::#(#path_segments::)::*CycleContext {
                    #(#field_initializers,)*
                },
            )
            .context(#error_message)?;
            #(#set_main_outputs_to_cycle_result)*
        } else {
            #(#set_main_outputs_to_none)*
        }
    })
}
