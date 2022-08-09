use std::collections::HashMap;

use syn::{Ident, Item, UseTree};

pub type Uses = HashMap<Ident, Vec<Ident>>;

pub fn uses_from_items(items: &[Item]) -> Uses {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Use(use_item) => Some(use_item.tree.extract_uses(vec![])),
            _ => None,
        })
        .flatten()
        .collect()
}

trait ExtractUses {
    fn extract_uses(&self, prefix: Vec<Ident>) -> Uses;
}

impl ExtractUses for UseTree {
    fn extract_uses(&self, mut prefix: Vec<Ident>) -> Uses {
        match self {
            UseTree::Path(path) => {
                prefix.push(path.ident.clone());
                path.tree.extract_uses(prefix)
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
                .map(|tree| tree.extract_uses(prefix.clone()))
                .flatten()
                .collect(),
        }
    }
}
