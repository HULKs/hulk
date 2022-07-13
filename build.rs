use std::{
    collections::HashMap,
    env::var,
    fs::{read_to_string, write, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
};

use module_attributes::ModuleInformation;
use petgraph::{algo::toposort, Graph};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_file, Ident, Item};
use walkdir::WalkDir;

fn main() {
    let out_path = PathBuf::from(var("OUT_DIR").unwrap());
    write_files_for_cycler_name(&out_path, Cycler::Control);
    write_files_for_cycler_name(&out_path, Cycler::Vision);
}

#[derive(Clone, Copy, Debug)]
enum Cycler {
    Control,
    Vision,
}

fn write_files_for_cycler_name(out_path: &Path, cycler: Cycler) {
    let modules = modules_from_cycler(cycler);
    let sorted_modules = sort_modules(&modules);
    let cycler_modules_struct = cycler_modules_struct_from_sorted_modules(&sorted_modules, cycler);
    let cycler_modules_initializer =
        cycler_modules_initializer_from_sorted_modules(&sorted_modules, cycler);
    let cycler_run_cycles = cycler_run_cycles_from_sorted_modules(&sorted_modules, cycler);

    write_token_stream_to_file_for_cycler(
        out_path,
        TokenStreamWithKind::CyclerModulesStruct(cycler_modules_struct),
        cycler,
    );
    write_token_stream_to_file_for_cycler(
        out_path,
        TokenStreamWithKind::CyclerModulesInitializer(cycler_modules_initializer),
        cycler,
    );
    write_token_stream_to_file_for_cycler(
        out_path,
        TokenStreamWithKind::CyclerRunCycles(cycler_run_cycles),
        cycler,
    );
}

fn modules_from_cycler(cycler: Cycler) -> HashMap<String, Module> {
    let mut modules = HashMap::new();
    let cycler_name_lowercase = match cycler {
        Cycler::Control => "control",
        Cycler::Vision => "vision",
    };
    let modules_directory_path = PathBuf::from("src")
        .join(cycler_name_lowercase)
        .join("modules");
    println!(
        "cargo:rerun-if-changed={}",
        modules_directory_path.display()
    );
    let all_entries = WalkDir::new(&modules_directory_path).into_iter();
    let only_ok_entries = all_entries.filter_map(|entry| entry.ok());
    let only_files = only_ok_entries.filter(|entry| {
        matches!(entry.metadata().ok(),
        Some(metadata) if metadata.is_file())
    });
    let only_rs_files = only_files.filter(|entry| {
        entry
            .path()
            .extension()
            .map_or(false, |extension| extension == "rs")
    });
    for entry in only_rs_files {
        let file_path = entry.path();
        let module_information = match module_information_from_file(file_path) {
            Some(module_information) => module_information,
            None => {
                continue;
            }
        };
        let module_name = module_information.module_identifier.to_string();
        let module_path_components = get_module_path_components_from(
            &modules_directory_path,
            file_path,
            module_name.clone(),
        );
        let inputs = inputs_from_module_information(&module_information);
        let historic_inputs = historic_inputs_from_module_information(&module_information);
        let main_outputs = main_outputs_from_module_information(&module_information);
        modules.insert(
            module_name,
            Module {
                module_information,
                module_path_components,
                inputs: inputs
                    .into_iter()
                    .chain(historic_inputs)
                    .filter(|input| input != "sensor_data")
                    .collect(),
                main_outputs,
            },
        );
    }
    modules
}

fn get_module_path_components_from(
    modules_directory_path: &Path,
    file_path: &Path,
    module_name: String,
) -> Vec<Ident> {
    let mut module_path_components = file_path
        .strip_prefix(modules_directory_path)
        .unwrap()
        .components()
        .map(|component| component.as_os_str().to_str().unwrap().to_string())
        .collect::<Vec<_>>();
    *module_path_components.last_mut().unwrap() = module_path_components
        .last()
        .unwrap()
        .strip_suffix(".rs")
        .unwrap()
        .to_string();
    module_path_components.push(module_name);
    module_path_components
        .into_iter()
        .map(|component| format_ident!("{}", component))
        .collect()
}

#[derive(Debug)]
struct Module {
    module_information: ModuleInformation,
    module_path_components: Vec<Ident>,
    inputs: Vec<String>,
    main_outputs: Vec<String>,
}

fn module_information_from_file<P>(file_path: P) -> Option<ModuleInformation>
where
    P: AsRef<Path>,
{
    let mut file = File::open(file_path).unwrap();
    let mut buffer = String::new();
    file.read_to_string(&mut buffer).unwrap();
    let ast = parse_file(&buffer).unwrap();
    let mut resulting_module_information = None;
    for item in ast.items {
        let impl_item = match item {
            Item::Impl(impl_item) => impl_item,
            _ => continue,
        };
        if impl_item.attrs.is_empty() {
            continue;
        }
        let first_is_module_identifier = impl_item
            .attrs
            .first()
            .unwrap()
            .path
            .get_ident()
            .map_or(false, |identifier| identifier == "module");
        if !first_is_module_identifier {
            continue;
        }
        let module_information = ModuleInformation::from_module_implementation(impl_item);
        assert!(resulting_module_information.is_none());
        resulting_module_information = Some(module_information);
    }
    resulting_module_information
}

fn inputs_from_module_information(module_information: &ModuleInformation) -> Vec<String> {
    module_information
        .inputs
        .iter()
        .filter_map(|input| {
            assert_eq!(input.path.len(), 1);
            if input.cycler.is_some() {
                return None;
            }
            Some(input.path.first().unwrap().to_string())
        })
        .collect()
}

fn historic_inputs_from_module_information(module_information: &ModuleInformation) -> Vec<String> {
    module_information
        .historic_inputs
        .iter()
        .map(|input| {
            assert_eq!(input.path.len(), 1);
            input.path.first().unwrap().to_string()
        })
        .collect()
}

fn main_outputs_from_module_information(module_information: &ModuleInformation) -> Vec<String> {
    module_information
        .main_outputs
        .iter()
        .map(|main_output| main_output.name.to_string())
        .collect()
}

fn sort_modules(modules: &HashMap<String, Module>) -> Vec<&Module> {
    let mut main_output_to_module = HashMap::new();
    for (name, module) in modules.iter() {
        for main_output in module.main_outputs.iter() {
            main_output_to_module.insert(main_output, name);
        }
    }
    let mut module_dependencies = Graph::new();
    let module_to_node_index = modules
        .keys()
        .map(|name| (name, module_dependencies.add_node(name.clone())))
        .collect::<HashMap<_, _>>();
    let node_index_to_module = module_to_node_index
        .iter()
        .map(|(name, node_index)| (node_index, name))
        .collect::<HashMap<_, _>>();
    for (name, module) in modules.iter() {
        for input in module.inputs.iter() {
            let main_output_provider = main_output_to_module[input];
            let from = module_to_node_index[main_output_provider];
            let to = module_to_node_index[name];
            module_dependencies.add_edge(from, to, format!("{} -> {}", main_output_provider, name));
        }
    }
    let sorted_modules = toposort(&module_dependencies, None).expect("Unexpected data type cycle");
    sorted_modules
        .into_iter()
        .map(|node_index| &modules[*node_index_to_module[&node_index]])
        .collect()
}

fn cycler_modules_struct_from_sorted_modules(
    sorted_modules: &[&Module],
    cycler: Cycler,
) -> TokenStream {
    let cycler_modules_identifier = match cycler {
        Cycler::Control => format_ident!("ControlModules"),
        Cycler::Vision => format_ident!("VisionModules"),
    };
    let cycler_fields = sorted_modules.iter().map(|module| {
        let field_name = &module.module_information.module_snake_case_identifier;
        let field_type_path = &module.module_path_components;
        quote! { #field_name: super::modules::#(#field_type_path)::* }
    });
    quote! {
        struct #cycler_modules_identifier {
            #(#cycler_fields,)*
        }
    }
}

fn cycler_modules_initializer_from_sorted_modules(
    sorted_modules: &[&Module],
    cycler: Cycler,
) -> TokenStream {
    let cycler_modules_identifier = match cycler {
        Cycler::Control => format_ident!("ControlModules"),
        Cycler::Vision => format_ident!("VisionModules"),
    };
    let (new_parameters, run_new_parameters) = match cycler {
        Cycler::Control => (
            quote! { configuration: &crate::framework::configuration::Configuration },
            quote! { configuration },
        ),
        Cycler::Vision => (
            quote! { configuration: &crate::framework::configuration::Configuration, cycler_configuration: &crate::framework::configuration::Vision },
            quote! { configuration, cycler_configuration },
        ),
    };
    let cycler_field_constructors = sorted_modules.iter().map(|module| {
        let field_name = &module.module_information.module_snake_case_identifier;
        let field_type_path = &module.module_path_components;
        let error_context = format!("Failed to initialize module {}", module.module_information.module_identifier);
        quote! { #field_name: super::modules::#(#field_type_path)::*::run_new(#run_new_parameters).context(#error_context)? }
    });
    quote! {
        impl #cycler_modules_identifier {
            fn new(#new_parameters) -> anyhow::Result<Self> {
                Ok(Self {
                    #(#cycler_field_constructors,)*
                })
            }
        }
    }
}

fn cycler_run_cycles_from_sorted_modules(
    sorted_modules: &[&Module],
    cycler: Cycler,
) -> TokenStream {
    let cycler_run_cycles = sorted_modules.iter().map(|module| {
        let field_name = &module.module_information.module_snake_case_identifier;
        let field_type = &module.module_information.module_identifier;
        let error_context = format!("Failed to run cycle of module {}", field_type);
        match cycler {
            Cycler::Control => quote! {
                self.modules.#field_name.run_cycle(
                    cycle_start_time,
                    &mut control_database,
                    &self.historic_databases,
                    &self.perception_databases,
                    &configuration,
                    &subscribed_additional_outputs,
                    &changed_parameters,
                    &mut self.persistent_state,
                    &injected_outputs,
                ).context(#error_context)?;
            },
            Cycler::Vision => quote! {
                self.modules.#field_name.run_cycle(
                    &image,
                    self.instance,
                    &mut vision_database,
                    &control_database,
                    &configuration,
                    cycler_configuration,
                    &subscribed_additional_outputs,
                    &changed_parameters,
                    &injected_outputs,
                ).context(#error_context)?;
            },
        }
    });
    quote! {{#(#cycler_run_cycles)*}}
}

#[allow(clippy::enum_variant_names)]
enum TokenStreamWithKind {
    CyclerModulesStruct(TokenStream),
    CyclerModulesInitializer(TokenStream),
    CyclerRunCycles(TokenStream),
}

fn write_token_stream_to_file_for_cycler(
    out_path: &Path,
    token_stream: TokenStreamWithKind,
    cycler: Cycler,
) {
    let cycler_name_lowercase = match cycler {
        Cycler::Control => "control",
        Cycler::Vision => "vision",
    };
    let name_suffix = match token_stream {
        TokenStreamWithKind::CyclerModulesStruct(..) => "cycler_modules_struct",
        TokenStreamWithKind::CyclerModulesInitializer(..) => "cycler_modules_initializer",
        TokenStreamWithKind::CyclerRunCycles(..) => "cycler_run_cycles",
    };
    let file_path = out_path.join(format!("{}_{}.rs", cycler_name_lowercase, name_suffix));
    match token_stream {
        TokenStreamWithKind::CyclerModulesStruct(token_stream) => {
            {
                let mut file = File::create(&file_path)
                    .unwrap_or_else(|_| panic!("Failed create file {:?}", file_path));
                write!(file, "{}", token_stream)
                    .unwrap_or_else(|_| panic!("Failed to write to file {:?}", file_path));
            }

            let status = Command::new("rustfmt")
                .arg(file_path)
                .status()
                .expect("Failed to execute rustfmt");
            if !status.success() {
                panic!("rustfmt did not exit with success");
            }
        }
        TokenStreamWithKind::CyclerModulesInitializer(token_stream) => {
            {
                let mut file = File::create(&file_path)
                    .unwrap_or_else(|_| panic!("Failed create file {:?}", file_path));
                write!(file, "{}", token_stream)
                    .unwrap_or_else(|_| panic!("Failed to write to file {:?}", file_path));
            }

            let status = Command::new("rustfmt")
                .arg(&file_path)
                .status()
                .expect("Failed to execute rustfmt");
            if !status.success() {
                panic!("rustfmt did not exit with success");
            }
        }
        TokenStreamWithKind::CyclerRunCycles(token_stream) => {
            {
                let mut file = File::create(&file_path)
                    .unwrap_or_else(|_| panic!("Failed create file {:?}", file_path));
                // convince rustfmt to format this file by prepending dummy code
                write!(file, "fn x() {}", token_stream)
                    .unwrap_or_else(|_| panic!("Failed to write to file {:?}", file_path));
            }

            let status = Command::new("rustfmt")
                .arg(&file_path)
                .status()
                .expect("Failed to execute rustfmt");
            if !status.success() {
                panic!("rustfmt did not exit with success");
            }

            // remove dummy code again
            let contents = read_to_string(&file_path)
                .unwrap_or_else(|_| panic!("Failed to read file {:?}", file_path));
            let contents = contents.strip_prefix("fn x() ").unwrap();
            write(&file_path, contents)
                .unwrap_or_else(|_| panic!("Failed to write file {:?}", file_path));
        }
    }
}
