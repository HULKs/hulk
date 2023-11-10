use std::collections::BTreeSet;

pub use bincode;
pub use error::Error;

pub use jpeg::{DecodeJpeg, EncodeJpeg};
use serde::{Deserializer, Serializer};
pub use serde_json;
pub use serialize_hierarchy_derive::SerializeHierarchy;

pub mod error;
mod implementation;
mod jpeg;
mod not_supported;

pub trait SerializeHierarchy {
    fn serialize_path<S>(&self, path: &str, serializer: S) -> Result<S::Ok, Error<S::Error>>
    where
        S: Serializer;

    fn deserialize_path<'de, D>(
        &mut self,
        path: &str,
        deserializer: D,
    ) -> Result<(), Error<D::Error>>
    where
        D: Deserializer<'de>;

    fn exists(path: &str) -> bool;

    fn get_fields() -> BTreeSet<String>;

    fn fill_fields(fields: &mut BTreeSet<String>, prefix: &str);
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use crate as serialize_hierarchy;

    use super::*;

    #[derive(Deserialize, Serialize, SerializeHierarchy)]
    struct Outer {
        inner: Inner,
    }

    #[derive(Deserialize, Serialize, SerializeHierarchy)]
    struct Inner {
        field: bool,
    }

    #[test]
    fn primitive_fields_are_empty() {
        assert_eq!(bool::get_fields(), Default::default());
    }

    #[test]
    fn flat_struct_fields_contain_fields() {
        assert_eq!(Inner::get_fields(), ["field".to_string()].into());
    }

    #[test]
    fn nested_struct_fields_contain_fields() {
        assert_eq!(
            Outer::get_fields(),
            ["inner".to_string(), "inner.field".to_string()].into()
        );
    }
}
