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

    fn get_fields() -> BTreeSet<String> {
        let mut fields = BTreeSet::default();
        Self::fill_fields(&mut fields, "");
        fields
    }

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

    #[derive(Deserialize, Serialize, SerializeHierarchy)]
    struct TupleStruct(bool, f32, Inner, Outer);

    #[derive(Deserialize, Serialize, SerializeHierarchy)]
    struct NestedTupleStruct(bool, Inner, TupleStruct);

    #[derive(Deserialize, Serialize, SerializeHierarchy)]
    struct OuterWithTupleStruct {
        tuple_struct: TupleStruct,
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

    #[test]
    fn tuple_struct_fields_contain_fields() {
        assert_eq!(
            TupleStruct::get_fields(),
            ["0", "1", "2", "2.field", "3", "3.inner", "3.inner.field"]
                .map(|s| s.to_string())
                .into()
        );
    }

    #[test]
    fn nested_tuple_struct_fields_contain_fields() {
        assert_eq!(
            NestedTupleStruct::get_fields(),
            [
                "0",
                "1",
                "1.field",
                "2",
                "2.0",
                "2.1",
                "2.2",
                "2.2.field",
                "2.3",
                "2.3.inner",
                "2.3.inner.field"
            ]
            .map(|s| s.to_string())
            .into()
        );
    }

    #[test]
    fn flat_struct_contains_tuple_struct_fields() {
        assert_eq!(
            OuterWithTupleStruct::get_fields(),
            [
                "tuple_struct",
                "tuple_struct.0",
                "tuple_struct.1",
                "tuple_struct.2",
                "tuple_struct.2.field",
                "tuple_struct.3",
                "tuple_struct.3.inner",
                "tuple_struct.3.inner.field"
            ]
            .map(|s| s.to_string())
            .into()
        );
    }
}
