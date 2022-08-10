use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use glob::glob;
use syn::Item;

use crate::parse::parse_rust_file;

pub fn cycler_crates_from_crates_directory<P>(crates_directory: P) -> anyhow::Result<Vec<PathBuf>>
where
    P: AsRef<Path>,
{
    glob(
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
    })
    .collect()
}
