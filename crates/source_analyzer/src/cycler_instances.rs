use std::{collections::BTreeMap, path::Path};

use anyhow::{anyhow, Context};
use proc_macro2::Span;
use syn::Item;

use crate::{
    cycler_crates::cycler_crates_from_crates_directory,
    into_anyhow_result::new_syn_error_as_anyhow_result, parse_rust_file,
};

pub struct CyclerInstances {
    pub instances_to_modules: BTreeMap<String, String>,
    pub modules_to_instances: BTreeMap<String, Vec<String>>,
}

impl CyclerInstances {
    pub fn try_from_crates_directory<P>(crates_directory: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut instances_to_modules = BTreeMap::new();
        let mut modules_to_instances: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for crate_directory in
            cycler_crates_from_crates_directory(&crates_directory).with_context(|| {
                anyhow!(
                    "Failed to get cycler crates from crates directory {:?}",
                    crates_directory.as_ref()
                )
            })?
        {
            let module = crate_directory
                .file_name()
                .context("Failed to get file name from crate directory")?
                .to_str()
                .context("Failed to interpret file name of crate directory as Unicode")?;
            let rust_file_path = crate_directory.join("src/lib.rs");
            let rust_file = parse_rust_file(&rust_file_path)
                .with_context(|| anyhow!("Failed to parse file {rust_file_path:?}"))?;
            let enum_item = rust_file.items.iter().find_map(|item| match item {
                Item::Enum(enum_item) if enum_item.ident == "CyclerInstance" => Some(enum_item),
                _ => None,
            });
            let enum_item = match enum_item {
                Some(enum_item) => enum_item,
                None => {
                    return new_syn_error_as_anyhow_result(
                        Span::call_site(),
                        "expected `CyclerInstances` enum",
                        rust_file_path,
                    )
                }
            };
            for variant in enum_item.variants.iter() {
                instances_to_modules.insert(variant.ident.to_string(), module.to_string());
                modules_to_instances
                    .entry(module.to_string())
                    .or_default()
                    .push(variant.ident.to_string());
            }
        }

        Ok(Self {
            instances_to_modules,
            modules_to_instances,
        })
    }
}
