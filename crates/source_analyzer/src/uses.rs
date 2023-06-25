use std::collections::HashMap;

use syn::{Ident, Item, UseTree};

pub type Uses = HashMap<Ident, Vec<Ident>>;

pub fn uses_from_items(items: &[Item]) -> Uses {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Use(use_item) => Some(extract_uses(&use_item.tree, vec![])),
            _ => None,
        })
        .flatten()
        .collect()
}

fn extract_uses(tree: &UseTree, mut prefix: Vec<Ident>) -> Uses {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.clone());
            extract_uses(&path.tree, prefix)
        }
        UseTree::Name(name) => {
            prefix.push(name.ident.clone());
            HashMap::from([(name.ident.clone(), prefix)])
        }
        UseTree::Rename(rename) => {
            prefix.push(rename.ident.clone());
            HashMap::from([(rename.rename.clone(), prefix)])
        }
        UseTree::Glob(_) => HashMap::new(),
        UseTree::Group(group) => group
            .items
            .iter()
            .flat_map(|tree| extract_uses(tree, prefix.clone()))
            .collect(),
    }
}
