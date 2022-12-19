use std::{
    collections::{BTreeMap, HashMap},
    path::{Component, Path},
};

use color_eyre::{
    eyre::{bail, eyre, WrapErr},
    Result,
};
use glob::glob;
use quote::ToTokens;
use syn::{ImplItem, Item, Type};
use topological_sort::TopologicalSort;

use crate::{
    cycler_crates::cycler_crates_from_crates_directory, parse::parse_rust_file, Contexts, Field,
    PathSegment,
};

#[derive(Debug)]
pub struct Nodes {
    pub nodes: BTreeMap<String, Node>,
    pub cycler_modules_to_nodes: BTreeMap<String, Vec<String>>,
}

impl Nodes {
    pub fn try_from_crates_directory(crates_directory: impl AsRef<Path>) -> Result<Self> {
        let mut nodes = BTreeMap::new();
        let mut cycler_modules_to_nodes: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for crate_directory in cycler_crates_from_crates_directory(&crates_directory)
            .wrap_err_with(|| {
                format!(
                    "failed to get cycler crates from crates directory {:?}",
                    crates_directory.as_ref()
                )
            })?
        {
            for rust_file_path in glob(crate_directory.join("src/**/*.rs").to_str().unwrap())
                .wrap_err_with(|| {
                    format!("failed to find rust files from crate directory {crate_directory:?}")
                })?
            {
                let cycler_module = crate_directory
                    .file_name()
                    .ok_or_else(|| eyre!("failed to get file name from crate directory"))?
                    .to_str()
                    .ok_or_else(|| {
                        eyre!("failed to interpret file name of crate directory as Unicode")
                    })?;
                let rust_file_path = rust_file_path.wrap_err("failed to get rust file path")?;
                let rust_file = parse_rust_file(&rust_file_path)
                    .wrap_err_with(|| format!("failed to parse rust file {rust_file_path:?}"))?;
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
                let node_name = rust_file
                    .items
                    .iter()
                    .find_map(|item| match item {
                        Item::Impl(implementation)
                            if implementation.items.iter().any(|item| {
                                matches!(item,
                                ImplItem::Method(method) if method.sig.ident == "new")
                            }) && implementation.items.iter().any(|item| {
                                matches!(item,
                                ImplItem::Method(method) if method.sig.ident == "cycle")
                            }) =>
                        {
                            match &*implementation.self_ty {
                                Type::Path(path) => path.path.get_ident(),
                                _ => None,
                            }
                        }
                        _ => None,
                    })
                    .ok_or_else(|| eyre!("failed to find node name in {rust_file_path:?}"))?;
                let contexts = Contexts::try_from_file(&rust_file_path, &rust_file)
                    .wrap_err_with(|| format!("failed to get contexts in {rust_file_path:?}"))?;
                let path_segments: Vec<_> = rust_file_path
                    .strip_prefix(crate_directory.join("src"))
                    .wrap_err("failed to strip prefix of node's rust file path")?
                    .with_extension("")
                    .components()
                    .map(|component| match component {
                        Component::Normal(component) => component
                            .to_str()
                            .ok_or_else(|| eyre!("failed to interpret path component as Unicode"))
                            .map(ToString::to_string),
                        _ => bail!("unexpected path component"),
                    })
                    .collect::<Result<_, _>>()
                    .wrap_err("failed to generate node's path")?;
                let node = Node {
                    cycler_module: cycler_module.to_string(),
                    path_segments,
                    contexts,
                };
                if let Some(overwritten_node) = nodes.insert(node_name.to_string(), node) {
                    bail!(
                        "node `{}` is not allowed to exist in multiple cyclers `{}`, `{}`, and maybe more",
                        node_name.to_string(),
                        cycler_module.to_string(),
                        overwritten_node.cycler_module,
                    );
                }
                cycler_modules_to_nodes
                    .entry(cycler_module.to_string())
                    .or_default()
                    .push(node_name.to_string());
            }
        }

        Ok(Self {
            nodes,
            cycler_modules_to_nodes,
        })
    }

    pub fn sort(&mut self) -> Result<()> {
        for node_names in self.cycler_modules_to_nodes.values_mut() {
            if node_names.len() == 1 {
                continue;
            }

            let mut main_outputs_to_nodes = HashMap::new();
            let mut topological_sort: TopologicalSort<String> = TopologicalSort::new();

            for node_name in node_names.iter() {
                for field in self.nodes[node_name].contexts.main_outputs.iter() {
                    if let Field::MainOutput { data_type, name } = field {
                        main_outputs_to_nodes
                            .insert(name.to_string(), (node_name.clone(), data_type.clone()));
                    }
                }
            }

            for consuming_node_name in node_names.iter() {
                for field in self.nodes[consuming_node_name]
                    .contexts
                    .creation_context
                    .iter()
                    .chain(
                        self.nodes[consuming_node_name]
                            .contexts
                            .cycle_context
                            .iter(),
                    )
                {
                    match field {
                        Field::HistoricInput {
                            data_type,
                            name,
                            path,
                        }
                        | Field::Input {
                            cycler_instance: None,
                            data_type,
                            name,
                            path,
                            ..
                        }
                        | Field::RequiredInput {
                            cycler_instance: None,
                            data_type,
                            name,
                            path,
                            ..
                        } => {
                            let first_segment = match path.first() {
                                Some(PathSegment { name, is_variable: false, .. }) => name,
                                Some(..) => bail!("unexpected variable segment as first segment for `{name}` in node `{consuming_node_name}` (not implemented)"),
                                None => bail!("expected at least one path segment for `{name}` in node `{consuming_node_name}`"),
                            };
                            let (producing_node_name, main_output_data_type) = match main_outputs_to_nodes.get(first_segment) {
                                Some(producing_node) => producing_node,
                                None => bail!("failed to find producing node for `{name}` in node `{consuming_node_name}`"),
                            };
                            if main_output_data_type != data_type {
                                bail!("expected data type `{}` but `{name}` has `{}` in node `{consuming_node_name}`", main_output_data_type.to_token_stream(), data_type.to_token_stream());
                            }
                            topological_sort.add_dependency(
                                producing_node_name.clone(),
                                consuming_node_name.clone(),
                            );
                        }
                        _ => {}
                    }
                }
            }

            let unsorted_node_names: Vec<_> = node_names.drain(..).collect();
            node_names.extend(topological_sort);
            for node_name in unsorted_node_names {
                if !node_names.contains(&node_name) {
                    node_names.push(node_name);
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Node {
    pub cycler_module: String,
    pub path_segments: Vec<String>,
    pub contexts: Contexts,
}
