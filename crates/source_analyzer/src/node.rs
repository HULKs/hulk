use std::{fs::read_to_string, hash::Hash, path::Path};

use quote::ToTokens;
use syn::{parse_file, ImplItem, Item, ItemImpl, Type};

use crate::{
    contexts::Contexts,
    error::{Error, ParseError},
    manifest::NodeSpecification,
};

pub type NodeName = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Node {
    pub name: NodeName,
    pub module: syn::Path,
    pub contexts: Contexts,
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
    pub fn try_from_specification(
        node_specification: &NodeSpecification,
        root: &Path,
    ) -> Result<Self, Error> {
        let path = root.join(&node_specification.path);
        let wrap_error = |error| Error::Node {
            caused_by: error,
            node: node_specification.module.to_token_stream().to_string(),
            path: path.clone(),
        };
        let rust_file = parse_rust_file(&path)?;
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
            module: node_specification.module.clone(),
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
