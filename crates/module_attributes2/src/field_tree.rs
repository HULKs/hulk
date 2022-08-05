use std::collections::BTreeMap;

use anyhow::bail;
use syn::{Ident, Type};

use crate::attribute::Path;

pub enum FieldTree {
    Tree {
        children: BTreeMap<Ident, FieldTree>,
    },
    Leaf {
        data_type: Type,
    },
}

impl Default for FieldTree {
    fn default() -> Self {
        FieldTree::Tree {
            children: BTreeMap::new(),
        }
    }
}

impl FieldTree {
    pub fn insert(&mut self, path: &Path, data_type: Type) -> anyhow::Result<()> {
        match self {
            FieldTree::Tree { children } => {
                let should_overwrite_children = path.segments.is_empty();
                if should_overwrite_children {
                    *self = FieldTree::Leaf { data_type };
                } else {
                    let first_segment = path.segments.first().unwrap();
                    let sub_path = Path {
                        segments: path.segments.iter().skip(1).cloned().collect(),
                    };
                    children
                        .entry(first_segment.clone())
                        .or_default()
                        .insert(&sub_path, data_type)?;
                }
            }
            FieldTree::Leaf {
                data_type: stored_data_type,
            } => {
                if &data_type != stored_data_type {
                    bail!("Mismatched data_type of path {path:?}: {data_type:?} != {stored_data_type:?}");
                }
                // ignore insertion otherwise (self.data_type is responsible for defining the sub-path)
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use quote::format_ident;

    use super::*;

    #[test]
    fn create_deep_and_top_level_object() {
        let mut tree = FieldTree::default();

        assert!(matches!(tree, FieldTree::Tree { ref children } if children.is_empty()));

        tree.insert(
            &Path {
                segments: vec![format_ident!("a"), format_ident!("b"), format_ident!("c")],
            },
            Type::Verbatim(Default::default()),
        )
        .unwrap();

        let a = match &tree {
            FieldTree::Tree { children } => match children.get(&format_ident!("a")) {
                Some(a) => a,
                None => panic!("Should exist"),
            },
            FieldTree::Leaf { data_type: _ } => panic!("Should not be a leaf"),
        };
        let b = match a {
            FieldTree::Tree { children } => match children.get(&format_ident!("b")) {
                Some(b) => b,
                None => panic!("Should exist"),
            },
            FieldTree::Leaf { data_type: _ } => panic!("Should not be a leaf"),
        };
        let c = match b {
            FieldTree::Tree { children } => match children.get(&format_ident!("c")) {
                Some(c) => c,
                None => panic!("Should exist"),
            },
            FieldTree::Leaf { data_type: _ } => panic!("Should not be a leaf"),
        };
        match c {
            FieldTree::Tree { children: _ } => panic!("Should not be a tree"),
            FieldTree::Leaf { data_type } => {
                assert_eq!(data_type, &Type::Verbatim(Default::default()))
            }
        }

        tree.insert(
            &Path {
                segments: vec![format_ident!("a"), format_ident!("b"), format_ident!("d")],
            },
            Type::Verbatim(Default::default()),
        )
        .unwrap();

        let a = match &tree {
            FieldTree::Tree { children } => match children.get(&format_ident!("a")) {
                Some(a) => a,
                None => panic!("Should exist"),
            },
            FieldTree::Leaf { data_type: _ } => panic!("Should not be a leaf"),
        };
        let b = match a {
            FieldTree::Tree { children } => match children.get(&format_ident!("b")) {
                Some(b) => b,
                None => panic!("Should exist"),
            },
            FieldTree::Leaf { data_type: _ } => panic!("Should not be a leaf"),
        };
        let c = match b {
            FieldTree::Tree { children } => match children.get(&format_ident!("c")) {
                Some(c) => c,
                None => panic!("Should exist"),
            },
            FieldTree::Leaf { data_type: _ } => panic!("Should not be a leaf"),
        };
        match c {
            FieldTree::Tree { children: _ } => panic!("Should not be a tree"),
            FieldTree::Leaf { data_type } => {
                assert_eq!(data_type, &Type::Verbatim(Default::default()))
            }
        }
        let d = match b {
            FieldTree::Tree { children } => match children.get(&format_ident!("d")) {
                Some(d) => d,
                None => panic!("Should exist"),
            },
            FieldTree::Leaf { data_type: _ } => panic!("Should not be a leaf"),
        };
        match d {
            FieldTree::Tree { children: _ } => panic!("Should not be a tree"),
            FieldTree::Leaf { data_type } => {
                assert_eq!(data_type, &Type::Verbatim(Default::default()))
            }
        }

        tree.insert(
            &Path {
                segments: vec![format_ident!("a")],
            },
            Type::Verbatim(Default::default()),
        )
        .unwrap();

        let a = match &tree {
            FieldTree::Tree { children } => match children.get(&format_ident!("a")) {
                Some(a) => a,
                None => panic!("Should exist"),
            },
            FieldTree::Leaf { data_type: _ } => panic!("Should not be a leaf"),
        };
        match a {
            FieldTree::Tree { children: _ } => panic!("Should not be a tree"),
            FieldTree::Leaf { data_type } => {
                assert_eq!(data_type, &Type::Verbatim(Default::default()))
            }
        }
    }
}
