use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};

use anyhow::{anyhow, bail, Context};
use glob::glob;
use quote::ToTokens;
use syn::{ImplItem, Item, Type};
use topological_sort::TopologicalSort;

use crate::{cycler_crates::cycler_crates_from_crates_directory, parse_rust_file, Contexts, Field};

#[derive(Debug)]
pub struct Modules {
    pub modules: BTreeMap<String, Module>,
    pub cycler_modules_to_modules: BTreeMap<String, Vec<String>>,
}

impl Modules {
    pub fn try_from_crates_directory<P>(crates_directory: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut modules = BTreeMap::new();
        let mut cycler_modules_to_modules: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for crate_directory in
            cycler_crates_from_crates_directory(&crates_directory).with_context(|| {
                anyhow!(
                    "Failed to get cycler crates from crates directory {:?}",
                    crates_directory.as_ref()
                )
            })?
        {
            for rust_file_path in glob(crate_directory.join("src/**/*.rs").to_str().unwrap())
                .with_context(|| {
                    anyhow!("Failed to find rust files from crate directory {crate_directory:?}")
                })?
            {
                let cycler_module = crate_directory
                    .file_name()
                    .context("Failed to get file name from crate directory")?
                    .to_str()
                    .context("Failed to interpret file name of crate directory as Unicode")?;
                let rust_file_path = rust_file_path.context("Failed to get rust file path")?;
                let rust_file = parse_rust_file(&rust_file_path)
                    .with_context(|| anyhow!("Failed to parse rust file {rust_file_path:?}"))?;
                let has_at_least_one_struct_with_context_attribute =
                    rust_file.items.iter().any(|item| match item {
                        Item::Struct(struct_item) => struct_item.attrs.iter().any(|attribute| {
                            attribute
                                .path
                                .get_ident()
                                .map(|attribute_name| attribute_name == "context")
                                .unwrap_or(false)
                        }),
                        _ => false,
                    });
                if !has_at_least_one_struct_with_context_attribute {
                    continue;
                }
                let module_name = rust_file
                    .items
                    .iter()
                    .find_map(|item| match item {
                        Item::Impl(implementation)
                            if implementation.items.iter().any(|item| match item {
                                ImplItem::Method(method) if method.sig.ident == "new" => true,
                                _ => false,
                            }) && implementation.items.iter().any(|item| match item {
                                ImplItem::Method(method) if method.sig.ident == "cycle" => true,
                                _ => false,
                            }) =>
                        {
                            match &*implementation.self_ty {
                                Type::Path(path) => path.path.get_ident(),
                                _ => None,
                            }
                        }
                        _ => None,
                    })
                    .with_context(|| anyhow!("Failed to find module name in {rust_file_path:?}"))?;
                let contexts = Contexts::try_from_file(&rust_file_path, &rust_file)
                    .with_context(|| anyhow!("Failed to get contexts in {rust_file_path:?}"))?;
                let module = Module {
                    cycler_module: cycler_module.to_string(),
                    contexts,
                };
                modules.insert(module_name.to_string(), module);
                cycler_modules_to_modules
                    .entry(cycler_module.to_string())
                    .or_default()
                    .push(module_name.to_string());
            }
        }

        Ok(Self {
            modules,
            cycler_modules_to_modules,
        })
    }

    pub fn sort(&mut self) -> anyhow::Result<()> {
        for module_names in self.cycler_modules_to_modules.values_mut() {
            if module_names.len() == 1 {
                continue;
            }

            let mut main_outputs_to_modules = HashMap::new();
            let mut topological_sort: TopologicalSort<String> = TopologicalSort::new();

            for module_name in module_names.iter() {
                for field in self.modules[module_name].contexts.main_outputs.iter() {
                    match field {
                        Field::MainOutput { data_type, name } => {
                            main_outputs_to_modules
                                .insert(name.to_string(), (module_name.clone(), data_type.clone()));
                        }
                        _ => {}
                    }
                }
            }

            for consuming_module_name in module_names.iter() {
                for field in self.modules[consuming_module_name]
                    .contexts
                    .new_context
                    .iter()
                    .chain(
                        self.modules[consuming_module_name]
                            .contexts
                            .cycle_context
                            .iter(),
                    )
                {
                    match field {
                        Field::HistoricInput {
                            data_type, name, ..
                        }
                        | Field::OptionalInput {
                            data_type, name, ..
                        }
                        | Field::RequiredInput {
                            data_type, name, ..
                        } => {
                            let path_segments = field
                                .get_path_segments()
                                .expect("Unexpected missing path in input field");
                            let first_segment = match path_segments.first() {
                                Some(first_segment) => first_segment,
                                None => bail!("Expected at least one path segment for {name} in module {consuming_module_name}"),
                            };
                            let (producing_module_name, main_output_data_type) = match main_outputs_to_modules.get(first_segment) {
                                Some(producing_module) => producing_module,
                                None => bail!("Failed to find producing module for {name} in module {consuming_module_name}"),
                            };
                            if main_output_data_type != data_type {
                                bail!("Expected data type {} but {name} has {} in module {consuming_module_name}", main_output_data_type.to_token_stream(), data_type.to_token_stream());
                            }
                            topological_sort.add_dependency(
                                producing_module_name.clone(),
                                consuming_module_name.clone(),
                            );
                        }
                        _ => {}
                    }
                }
            }

            module_names.clear();
            module_names.extend(topological_sort);
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Module {
    pub cycler_module: String,
    pub contexts: Contexts,
}
