use std::{
    fmt::{self, Display, Formatter},
    fs::read_to_string,
    hash::Hash,
    path::Path,
};

use itertools::Itertools;
use quote::ToTokens;
use syn::{parse_file, ImplItem, Item, ItemImpl, Type};

use crate::{
    configuration::NodeConfiguration,
    contexts::Contexts,
    error::{Error, ParseError},
};

pub type NodeName = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Node {
    pub name: NodeName,
    pub module: syn::Path,
    pub is_setup: bool,
    pub contexts: Contexts,
}

impl Display for Node {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let name = &self.name;
        write!(formatter, "{name}")
    }
}

pub fn parse_rust_file(file_path: impl AsRef<Path>) -> Result<syn::File, Error> {
    let buffer = read_to_string(&file_path).map_err(|error| Error::Io {
        source: error,
        path: file_path.as_ref().to_path_buf(),
    })?;
    parse_file(&buffer).map_err(|error| Error::RustParse {
        caused_by: error.into(),
        path: file_path.as_ref().to_path_buf(),
    })
}

impl Node {
    pub fn try_from_configuration(
        cycler_module: &str,
        node_config: &NodeConfiguration,
        root: &Path,
    ) -> Result<Self, Error> {
        let path_to_module = node_config
            .module
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .join("/");
        let file_path = root.join(format!("../{cycler_module}/src/{path_to_module}.rs"));
        let wrap_error = |error| Error::Node {
            caused_by: error,
            node: node_config.module.to_token_stream().to_string(),
            path: file_path.clone(),
        };
        let rust_file = parse_rust_file(&file_path)?;
        let name = rust_file
            .items
            .iter()
            .find_map(|item| match item {
                Item::Impl(implementation) if has_new_and_cycle_method(implementation) => {
                    match *implementation.self_ty {
                        Type::Path(ref path) => path.path.get_ident(),
                        _ => None,
                    }
                }
                _ => None,
            })
            .ok_or_else(|| wrap_error(ParseError::new_spanned(&rust_file, "cannot find node declaration, expected a type with new(...) and cycle(...) method")))?
            .to_string();
        let contexts = Contexts::try_from_file(&rust_file).map_err(wrap_error)?;
        Ok(Self {
            name,
            module: node_config.module.clone(),
            is_setup: node_config.is_setup,
            contexts,
        })
    }
}

fn has_new_and_cycle_method(implementation: &ItemImpl) -> bool {
    implementation
        .items
        .iter()
        .any(|item| matches!(item, ImplItem::Method(method) if method.sig.ident == "new"))
        && implementation
            .items
            .iter()
            .any(|item| matches!(item, ImplItem::Method(method) if method.sig.ident == "cycle"))
}
