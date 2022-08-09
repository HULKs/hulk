use std::{
    collections::{BTreeMap, HashMap},
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context};
use glob::glob;
use syn::{parse_file, File, Ident, Item, UseTree};

use crate::{
    into_anyhow_result::{into_anyhow_result, SynContext},
    parse::parse_rust_file,
};

#[derive(Debug)]
pub struct Structs {
    pub configuration: StructHierarchy,
    pub cycler_structs: BTreeMap<String, CyclerStructs>,
}

impl Structs {
    pub fn try_from<P>(crates_directory: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        for crate_directory in iterate_over_cycler_crates(crates_directory)
            .context("Failed to iterate over cycler crates")?
        {
            let crate_directory = crate_directory.context("Failed to analyze crate directory")?;
            for rust_file_path in glob(crate_directory.join("src/**/*.rs").to_str().unwrap())
                .with_context(|| {
                    anyhow!("Failed to find rust files from crate directory {crate_directory:?}")
                })?
            {
                let rust_file_path = rust_file_path.context("Failed to get rust file path")?;
                let rust_file = parse_rust_file(&rust_file_path)
                    .with_context(|| anyhow!("Failed to parse rust file {rust_file_path:?}"))?;
                // let uses = uses_from_items(&rust_file.items);
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
                println!("{rust_file_path:?}: has_at_least_one_struct_with_context_attribute: {has_at_least_one_struct_with_context_attribute}");
                if !has_at_least_one_struct_with_context_attribute {
                    continue;
                }
            }
        }
        Ok(Self {
            configuration: StructHierarchy::Struct {
                fields: BTreeMap::new(),
            },
            cycler_structs: BTreeMap::new(),
        })
    }
}

#[derive(Debug)]
pub struct CyclerStructs {
    pub main_outputs: StructHierarchy,
    pub additional_outputs: StructHierarchy,
    pub persistent_state: StructHierarchy,
}

#[derive(Debug)]
pub enum StructHierarchy {
    Struct {
        fields: BTreeMap<String, StructHierarchy>,
    },
    Field {
        data_type: String,
    },
}

impl Default for StructHierarchy {
    fn default() -> Self {
        Self::Struct {
            fields: Default::default(),
        }
    }
}

impl StructHierarchy {}

fn iterate_over_cycler_crates<P>(
    crates_directory: P,
) -> anyhow::Result<impl Iterator<Item = anyhow::Result<PathBuf>>>
where
    P: AsRef<Path>,
{
    Ok(glob(
        crates_directory
            .as_ref()
            .join("*/src/lib.rs")
            .to_str()
            .unwrap(),
    )
    .with_context(|| {
        anyhow!(
            "Failed to find lib.rs files from crates directory {:?}",
            crates_directory.as_ref()
        )
    })?
    .filter_map(|file_path| {
        let file_path = match file_path {
            Ok(file_path) => file_path,
            Err(error) => return Some(Err(error.into())),
        };
        let file = match parse_rust_file(&file_path) {
            Ok(file) => file,
            Err(error) => return Some(Err(error)),
        };
        match file.items.into_iter().any(|item| match item {
            Item::Enum(enum_item) => enum_item.ident == "CyclerInstance",
            _ => false,
        }) {
            true => file_path
                .parent()
                .and_then(|source_directory| source_directory.parent())
                .map(|crate_directory| Ok(crate_directory.to_path_buf())),
            false => None,
        }
    }))
}
