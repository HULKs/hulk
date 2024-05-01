use syn::Generics;

use crate::container::Container;

pub trait ExtendGenerics {
    fn extend_generics(&self, generics: &mut Generics);
}

impl ExtendGenerics for Container {
    fn extend_generics(&self, generics: &mut Generics) {
        if let Some(bounds) = &self.bounds {
            generics
                .make_where_clause()
                .predicates
                .extend(bounds.clone());
        }
    }
}
