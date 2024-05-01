use syn::Generics;

use crate::container::Container;

pub trait ExtendGenerics {
    fn extend_from_attributes(&mut self, container: &Container);
}

impl ExtendGenerics for Generics {
    fn extend_from_attributes(&mut self, container: &Container) {
        if let Some(bounds) = &container.bounds {
            self.make_where_clause().predicates.extend(bounds.clone());
        }
    }
}
