use std::{cmp::Ordering, path::Path};

use proc_macro2::LineColumn;
use syn::{spanned::Spanned, File, Item, ItemMod, ItemUse, UsePath, UseTree, Visibility};

use crate::output::output_diagnostic;

pub fn check<P>(file_path: P, buffer: &str, file: &File) -> bool
where
    P: AsRef<Path>,
{
    let mut state = None;
    for item in file.items.iter() {
        match item {
            Item::Mod(ItemMod {
                vis, content: None, ..
            }) => {
                let end = item.span().end();
                let new_state = Order::Modules {
                    visibility: vis.clone(),
                    end,
                };
                if let Some(state) = state {
                    if new_state != state && state.end().line + 1 == end.line {
                        output_diagnostic(
                            file_path,
                            buffer,
                            state.end().line..end.line+1,
                            "error: different categories of mods and uses must be separated by empty lines",
                        );
                        return false;
                    }
                    if new_state < state {
                        output_diagnostic(
                            file_path,
                            buffer,
                            state.end().line..end.line+1,
                            "error: mods and uses are out of order (must be ordered `mod ...`, `use std...`, `use ...`, `use crate...`; and `pub`, `crate`, `pub(...)`, `` within these categories)",
                        );
                        return false;
                    }
                }
                state = Some(new_state);
            }
            Item::Use(ItemUse {
                vis,
                tree: UseTree::Path(UsePath { ident, .. }),
                ..
            }) => {
                let end = item.span().end();
                let new_state = if ident == "std" {
                    Order::StandardUses {
                        visibility: vis.clone(),
                        end,
                    }
                } else if ident == "crate" {
                    Order::CrateUses {
                        visibility: vis.clone(),
                        end,
                    }
                } else {
                    Order::ExternUses {
                        visibility: vis.clone(),
                        end,
                    }
                };
                if let Some(state) = state {
                    if new_state != state && state.end().line + 1 == end.line {
                        output_diagnostic(
                            file_path,
                            buffer,
                            state.end().line..end.line+1,
                            "error: different categories of mods and uses must be separated by empty lines",
                        );
                        return false;
                    }
                    if new_state < state {
                        output_diagnostic(
                            file_path,
                            buffer,
                            state.end().line..end.line+1,
                            "error: mods and uses are out of order (must be ordered `mod ...`, `use std...`, `use ...`, `use crate...`; and `pub`, `crate`, `pub(...)`, `` within these categories)",
                        );
                        return false;
                    }
                }
                state = Some(new_state);
            }
            _ => {}
        }
    }
    true
}

#[derive(Debug)]
enum Order {
    Modules {
        visibility: Visibility,
        end: LineColumn,
    },
    StandardUses {
        visibility: Visibility,
        end: LineColumn,
    },
    ExternUses {
        visibility: Visibility,
        end: LineColumn,
    },
    CrateUses {
        visibility: Visibility,
        end: LineColumn,
    },
}

impl PartialEq for Order {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Modules { .. }, Self::Modules { .. }) => self.visibility() == other.visibility(),
            (Self::StandardUses { .. }, Self::StandardUses { .. }) => {
                self.visibility() == other.visibility()
            }
            (Self::ExternUses { .. }, Self::ExternUses { .. }) => {
                self.visibility() == other.visibility()
            }
            (Self::CrateUses { .. }, Self::CrateUses { .. }) => {
                self.visibility() == other.visibility()
            }
            _ => false,
        }
    }
}

impl Order {
    fn visibility(&self) -> &Visibility {
        match self {
            Order::Modules { visibility, .. } => visibility,
            Order::StandardUses { visibility, .. } => visibility,
            Order::ExternUses { visibility, .. } => visibility,
            Order::CrateUses { visibility, .. } => visibility,
        }
    }

    fn end(&self) -> &LineColumn {
        match self {
            Order::Modules { end, .. } => end,
            Order::StandardUses { end, .. } => end,
            Order::ExternUses { end, .. } => end,
            Order::CrateUses { end, .. } => end,
        }
    }
}

impl PartialOrd for Order {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let order_index = order_to_index(self);
        let other_order_index = order_to_index(other);
        match order_index.cmp(&other_order_index) {
            Ordering::Less => Some(Ordering::Less),
            Ordering::Equal => Some(
                visibility_to_index(self.visibility())
                    .cmp(&visibility_to_index(other.visibility())),
            ),
            Ordering::Greater => Some(Ordering::Greater),
        }
    }
}

fn order_to_index(order: &Order) -> usize {
    match order {
        Order::Modules { .. } => 0,
        Order::StandardUses { .. } => 1,
        Order::ExternUses { .. } => 2,
        Order::CrateUses { .. } => 3,
    }
}

fn visibility_to_index(visibility: &Visibility) -> usize {
    match visibility {
        Visibility::Public(_) => 0,
        Visibility::Crate(_) => 1,
        Visibility::Restricted(_) => 2,
        Visibility::Inherited => 3,
    }
}
