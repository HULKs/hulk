use std::{fs, io::Read, path::Path};

use anyhow::{anyhow, Context};
use module_attributes2::Module;
use syn::{Ident, Item, ItemEnum, ItemImpl};

pub fn parse_file<P>(file_path: P) -> anyhow::Result<syn::File>
where
    P: AsRef<Path>,
{
    let mut file = fs::File::open(&file_path).context("Failed to open file")?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)
        .context("Failed to read file to string")?;
    syn::parse_file(&buffer).map_err(|error| {
        let start = error.span().start();
        anyhow!(
            "Failed to parse file into abstract syntax tree: {error} at {}:{}:{}",
            file_path.as_ref().display(),
            start.line,
            start.column
        )
    })
}

pub fn get_cycler_instance_enum(file: &syn::File) -> Option<&ItemEnum> {
    file.items.iter().find_map(|item| match item {
        Item::Enum(enum_item) if enum_item.ident == "CyclerInstance" => Some(enum_item),
        _ => None,
    })
}

pub fn get_module_implementation(file: &syn::File) -> Option<&ItemImpl> {
    file.items.iter().find_map(|item| match item {
        Item::Impl(impl_item)
            if impl_item
                .attrs
                .first()
                .and_then(|first_attribute| {
                    first_attribute.path.get_ident().map(|identifier| {
                        identifier == "realtime_module" || identifier == "perception_module"
                    })
                })
                .unwrap_or(false) =>
        {
            Some(impl_item)
        }
        _ => None,
    })
}

pub fn get_cycler_instances(enum_item: &ItemEnum) -> Vec<Ident> {
    enum_item
        .variants
        .iter()
        .map(|variant| variant.ident.clone())
        .collect()
}

pub fn get_module(impl_item: &ItemImpl) -> syn::Result<Module> {
    Module::from_implementation(impl_item.clone())
}
