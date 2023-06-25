use std::{
    fs::read_to_string,
    hash::Hash,
    path::{Path, PathBuf},
};

use quote::ToTokens;
use syn::{parse_file, ImplItem, Item, ItemImpl, Type};

use crate::{
    contexts::Contexts,
    error::{Error, ParseError},
};

pub type NodeName = String;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Node {
    pub name: NodeName,
    pub module: syn::Path,
    pub file_path: PathBuf,
    pub contexts: Contexts,
}

pub fn parse_rust_file(file_path: impl AsRef<Path>) -> Result<syn::File, Error> {
    let buffer = read_to_string(&file_path).map_err(|source| Error::Io {
        source,
        path: file_path.as_ref().to_path_buf(),
    })?;
    parse_file(&buffer).map_err(|error| Error::RustParse {
        source: error.into(),
        path: file_path.as_ref().to_path_buf(),
    })
}

impl Node {
    pub fn try_from_node_name(node_name: &str, root: &Path) -> Result<Self, Error> {
        let module: syn::Path = syn::parse_str(node_name).map_err(|_| Error::InvalidModulePath)?;
        let file_path = file_path_from_module_path(root, module.clone())?;
        let wrap_error = |source| Error::Node {
            source,
            node: module.to_token_stream().to_string(),
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
            module,
            file_path,
            contexts,
        })
    }
}

fn file_path_from_module_path(root: &Path, module: syn::Path) -> Result<PathBuf, Error> {
    let path_segments: Vec<_> = module
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    let (crate_name, path_segments) = path_segments
        .split_first()
        .ok_or(Error::InvalidModulePath)?;
    let path_to_module = path_segments.join("/");
    Ok(root.join(format!("{crate_name}/src/{path_to_module}.rs")))
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
