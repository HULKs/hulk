use std::{collections::BTreeMap, path::Path};

use anyhow::{anyhow, Context};
use glob::glob;
use syn::{ImplItem, Item, Type};

use crate::{cycler_crates::cycler_crates_from_crates_directory, parse_rust_file, Contexts};

#[derive(Debug)]
pub struct Modules {
    modules: BTreeMap<String, Module>,
    cycler_modules_to_modules: BTreeMap<String, Vec<String>>,
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
}

#[derive(Debug)]
pub struct Module {
    cycler_module: String,
    contexts: Contexts,
}
