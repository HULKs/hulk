use std::{
    collections::{BTreeSet, HashSet},
    error,
    ops::{Deref, Range},
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime},
};

pub use bincode;
use nalgebra::{
    ArrayStorage, Const, Isometry2, Isometry3, Matrix, Point, SMatrix, Scalar, UnitComplex, U1,
};
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};
pub use serde_json;
pub use serialize_hierarchy_derive::SerializeHierarchy;

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
}

#[derive(Debug, thiserror::Error)]
pub enum Error<E>
where
    E: error::Error,
{
    #[error("failed to serialize")]
    SerializationFailed(E),
    #[error("failed to deserialize")]
    DeserializationFailed(E),
    #[error("type {type_name} does not support serialization for path {path:?}")]
    TypeDoesNotSupportSerialization {
        type_name: &'static str,
        path: String,
    },
    #[error("type {type_name} does not support deserialization for path {path:?}")]
    TypeDoesNotSupportDeserialization {
        type_name: &'static str,
        path: String,
    },
    #[error("unexpected path segment {segment}")]
    UnexpectedPathSegment { segment: String },
}

impl<T> SerializeHierarchy for Arc<T>
where
    T: SerializeHierarchy,
{
    fn serialize_path<S>(&self, path: &str, serializer: S) -> Result<S::Ok, Error<S::Error>>
    where
        S: Serializer,
    {
        self.deref().serialize_path(path, serializer)
    }

    fn deserialize_path<'de, D>(&mut self, path: &str, _data: D) -> Result<(), Error<D::Error>>
    where
        D: Deserializer<'de>,
    {
        Err(Error::TypeDoesNotSupportDeserialization {
            type_name: "Arc",
            path: path.to_string(),
        })
    }

    fn exists(path: &str) -> bool {
        T::exists(path)
    }

    fn get_fields() -> BTreeSet<String> {
        T::get_fields()
    }
}

impl<T> SerializeHierarchy for Option<T>
where
    T: SerializeHierarchy,
{
    fn serialize_path<S>(&self, path: &str, serializer: S) -> Result<S::Ok, Error<S::Error>>
    where
        S: Serializer,
    {
        match self {
            Some(some) => some.serialize_path(path, serializer),
            None => (None as Option<()>)
                .serialize(serializer)
                .map_err(Error::SerializationFailed),
        }
    }

    fn deserialize_path<'de, D>(&mut self, path: &str, _data: D) -> Result<(), Error<D::Error>>
    where
        D: Deserializer<'de>,
    {
        Err(Error::TypeDoesNotSupportDeserialization {
            type_name: "Option",
            path: path.to_string(),
        })
    }

    fn exists(path: &str) -> bool {
        T::exists(path)
    }

    fn get_fields() -> BTreeSet<String> {
        T::get_fields()
    }
}

impl<T> SerializeHierarchy for HashSet<T> {
    fn serialize_path<S>(&self, path: &str, _serializer: S) -> Result<S::Ok, Error<S::Error>>
    where
        S: Serializer,
    {
        Err(Error::TypeDoesNotSupportSerialization {
            type_name: "HashSet",
            path: path.to_string(),
        })
    }

    fn deserialize_path<'de, D>(&mut self, path: &str, _data: D) -> Result<(), Error<D::Error>>
    where
        D: Deserializer<'de>,
    {
        Err(Error::TypeDoesNotSupportDeserialization {
            type_name: "HashSet",
            path: path.to_string(),
        })
    }

    fn exists(_path: &str) -> bool {
        false
    }

    fn get_fields() -> BTreeSet<String> {
        Default::default()
    }
}

impl<T> SerializeHierarchy for Vec<T> {
    fn serialize_path<S>(&self, path: &str, _serializer: S) -> Result<S::Ok, Error<S::Error>>
    where
        S: Serializer,
    {
        Err(Error::TypeDoesNotSupportSerialization {
            type_name: "Vec",
            path: path.to_string(),
        })
    }

    fn deserialize_path<'de, D>(&mut self, path: &str, _data: D) -> Result<(), Error<D::Error>>
    where
        D: Deserializer<'de>,
    {
        Err(Error::TypeDoesNotSupportDeserialization {
            type_name: "Vec",
            path: path.to_string(),
        })
    }

    fn exists(_path: &str) -> bool {
        false
    }

    fn get_fields() -> BTreeSet<String> {
        Default::default()
    }
}

impl<T: Serialize + DeserializeOwned, const N: usize> SerializeHierarchy
    for Matrix<T, Const<N>, U1, ArrayStorage<T, N, 1>>
{
    fn serialize_path<S>(&self, path: &str, serializer: S) -> Result<S::Ok, Error<S::Error>>
    where
        S: Serializer,
    {
        let index = ["x", "y", "z", "w", "v", "u"][0..N]
            .iter()
            .position(|name| name == &path);
        match index {
            Some(index) => self[index]
                .serialize(serializer)
                .map_err(Error::SerializationFailed),
            _ => Err(Error::UnexpectedPathSegment {
                segment: String::from(path),
            }),
        }
    }

    fn deserialize_path<'de, D>(
        &mut self,
        path: &str,
        deserializer: D,
    ) -> Result<(), Error<D::Error>>
    where
        D: Deserializer<'de>,
    {
        let index = ["x", "y", "z", "w", "v", "u"][0..N]
            .iter()
            .position(|name| name == &path);
        match index {
            Some(index) => {
                let deserialized = <T as Deserialize>::deserialize(deserializer)
                    .map_err(Error::DeserializationFailed)?;
                self[index] = deserialized;
                Ok(())
            }
            None => Err(Error::UnexpectedPathSegment {
                segment: String::from(path),
            }),
        }
    }

    fn exists(path: &str) -> bool {
        Matrix::<T, Const<N>, U1, ArrayStorage<T, N, 1>>::get_fields().contains(path)
    }

    fn get_fields() -> BTreeSet<String> {
        ["x", "y", "z", "w", "v", "u"][0..N]
            .iter()
            .map(|path| String::from(*path))
            .collect()
    }
}

impl<T: Serialize + DeserializeOwned + Clone + Scalar, const N: usize> SerializeHierarchy
    for Point<T, N>
{
    fn serialize_path<S>(&self, path: &str, serializer: S) -> Result<S::Ok, Error<S::Error>>
    where
        S: Serializer,
        S::Error: error::Error,
    {
        self.coords.serialize_path(path, serializer)
    }

    fn deserialize_path<'de, D>(
        &mut self,
        path: &str,
        deserializer: D,
    ) -> Result<(), Error<D::Error>>
    where
        D: Deserializer<'de>,
    {
        self.coords.deserialize_path(path, deserializer)
    }

    fn exists(path: &str) -> bool {
        Matrix::<T, Const<N>, U1, ArrayStorage<T, N, 1>>::exists(path)
    }

    fn get_fields() -> BTreeSet<String> {
        Matrix::<T, Const<N>, U1, ArrayStorage<T, N, 1>>::get_fields()
    }
}

macro_rules! serialize_hierarchy_primary_impl {
    ($type:ty) => {
        impl SerializeHierarchy for $type {
            fn serialize_path<S>(
                &self,
                path: &str,
                _serializer: S,
            ) -> Result<S::Ok, Error<S::Error>>
            where
                S: Serializer,
                S::Error: error::Error,
            {
                Err(Error::TypeDoesNotSupportSerialization {
                    type_name: stringify!($type),
                    path: path.to_string(),
                })
            }

            fn deserialize_path<'de, D>(
                &mut self,
                path: &str,
                _data: D,
            ) -> Result<(), Error<D::Error>>
            where
                D: Deserializer<'de>,
            {
                Err(Error::TypeDoesNotSupportDeserialization {
                    type_name: stringify!($type),
                    path: path.to_string(),
                })
            }

            fn exists(_path: &str) -> bool {
                false
            }

            fn get_fields() -> BTreeSet<String> {
                Default::default()
            }
        }
    };
}

serialize_hierarchy_primary_impl!(bool);
serialize_hierarchy_primary_impl!(f32);
serialize_hierarchy_primary_impl!(i16);
serialize_hierarchy_primary_impl!(i32);
serialize_hierarchy_primary_impl!(u8);
serialize_hierarchy_primary_impl!(u16);
serialize_hierarchy_primary_impl!(u32);
serialize_hierarchy_primary_impl!(u64);
serialize_hierarchy_primary_impl!(usize);
// nalgebra
serialize_hierarchy_primary_impl!(SMatrix<f32, 3, 3>);
serialize_hierarchy_primary_impl!(Isometry2<f32>);
serialize_hierarchy_primary_impl!(Isometry3<f32>);
serialize_hierarchy_primary_impl!(UnitComplex<f32>);
// stdlib
serialize_hierarchy_primary_impl!(SystemTime);
serialize_hierarchy_primary_impl!(Duration);
serialize_hierarchy_primary_impl!(String);
serialize_hierarchy_primary_impl!(Range<f32>);
serialize_hierarchy_primary_impl!(Range<Duration>);
serialize_hierarchy_primary_impl!(PathBuf);

#[cfg(test)]
mod tests {
    use serde::Deserialize;

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
