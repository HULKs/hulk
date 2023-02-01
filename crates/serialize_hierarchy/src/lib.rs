use std::{
    collections::{BTreeSet, HashSet},
    error,
    ops::{Deref, Range},
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime},
};

pub use bincode;
use bincode::{deserialize, serialize};
use nalgebra::{
    ArrayStorage, Const, Isometry2, Isometry3, Matrix, Point, SMatrix, Scalar, UnitComplex, U1,
};
use serde::{de::DeserializeOwned, Serialize};
pub use serde_json;
use serde_json::{from_value, to_value, Value};
pub use serialize_hierarchy_derive::SerializeHierarchy;

pub trait SerializeHierarchy {
    fn serialize_path<S>(&self, path: &str) -> Result<S::Serialized, Error<S::Error>>
    where
        S: Serializer,
        S::Error: error::Error;

    fn deserialize_path<S>(
        &mut self,
        path: &str,
        data: S::Serialized,
    ) -> Result<(), Error<S::Error>>
    where
        S: Serializer,
        S::Error: error::Error;

    fn exists(path: &str) -> bool;

    fn get_fields() -> BTreeSet<String>;
}

pub trait Serializer {
    type Serialized;
    type Error;

    fn serialize<T>(value: &T) -> Result<Self::Serialized, Self::Error>
    where
        T: Serialize;

    fn deserialize<T>(value: Self::Serialized) -> Result<T, Self::Error>
    where
        T: DeserializeOwned;
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

pub struct BinarySerializer;

impl Serializer for BinarySerializer {
    type Serialized = Vec<u8>;
    type Error = bincode::Error;

    fn serialize<T>(value: &T) -> Result<Self::Serialized, Self::Error>
    where
        T: Serialize,
    {
        serialize(value)
    }

    fn deserialize<T>(value: Self::Serialized) -> Result<T, Self::Error>
    where
        T: DeserializeOwned,
    {
        deserialize(value.as_slice())
    }
}

pub struct TextualSerializer;

impl Serializer for TextualSerializer {
    type Serialized = Value;
    type Error = serde_json::Error;

    fn serialize<T>(value: &T) -> Result<Self::Serialized, Self::Error>
    where
        T: Serialize,
    {
        to_value(value)
    }

    fn deserialize<T>(value: Self::Serialized) -> Result<T, Self::Error>
    where
        T: DeserializeOwned,
    {
        from_value(value)
    }
}

impl<T> SerializeHierarchy for Arc<T>
where
    T: SerializeHierarchy,
{
    fn serialize_path<S>(&self, path: &str) -> Result<S::Serialized, Error<S::Error>>
    where
        S: Serializer,
        S::Error: error::Error,
    {
        self.deref().serialize_path::<S>(path)
    }

    fn deserialize_path<S>(
        &mut self,
        path: &str,
        _data: S::Serialized,
    ) -> Result<(), Error<S::Error>>
    where
        S: Serializer,
        S::Error: error::Error,
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
    fn serialize_path<S>(&self, path: &str) -> Result<S::Serialized, Error<S::Error>>
    where
        S: Serializer,
        S::Error: error::Error,
    {
        match self {
            Some(some) => some.serialize_path::<S>(path),
            None => S::serialize(&(None as Option<()>)).map_err(Error::SerializationFailed),
        }
    }

    fn deserialize_path<S>(
        &mut self,
        path: &str,
        _data: S::Serialized,
    ) -> Result<(), Error<S::Error>>
    where
        S: Serializer,
        S::Error: error::Error,
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
    fn serialize_path<S>(&self, path: &str) -> Result<S::Serialized, Error<S::Error>>
    where
        S: Serializer,
        S::Error: error::Error,
    {
        Err(Error::TypeDoesNotSupportSerialization {
            type_name: "HashSet",
            path: path.to_string(),
        })
    }

    fn deserialize_path<S>(
        &mut self,
        path: &str,
        _data: S::Serialized,
    ) -> Result<(), Error<S::Error>>
    where
        S: Serializer,
        S::Error: error::Error,
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
    fn serialize_path<S>(&self, path: &str) -> Result<S::Serialized, Error<S::Error>>
    where
        S: Serializer,
        S::Error: error::Error,
    {
        Err(Error::TypeDoesNotSupportSerialization {
            type_name: "Vec",
            path: path.to_string(),
        })
    }

    fn deserialize_path<S>(
        &mut self,
        path: &str,
        _data: S::Serialized,
    ) -> Result<(), Error<S::Error>>
    where
        S: Serializer,
        S::Error: error::Error,
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
    fn serialize_path<S>(&self, path: &str) -> Result<S::Serialized, Error<S::Error>>
    where
        S: Serializer,
        S::Error: error::Error,
    {
        let index = ["x", "y", "z", "w", "v", "u"][0..N]
            .iter()
            .position(|name| name == &path);
        match index {
            Some(index) => S::serialize(&self[index]).map_err(Error::SerializationFailed),
            _ => Err(Error::UnexpectedPathSegment {
                segment: String::from(path),
            }),
        }
    }

    fn deserialize_path<S>(
        &mut self,
        path: &str,
        data: S::Serialized,
    ) -> Result<(), Error<S::Error>>
    where
        S: Serializer,
        S::Error: error::Error,
    {
        let index = ["x", "y", "z", "w", "v", "u"][0..N]
            .iter()
            .position(|name| name == &path);
        match index {
            Some(index) => {
                let deserialized = S::deserialize(data).map_err(Error::DeserializationFailed)?;
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
    fn serialize_path<S>(&self, path: &str) -> Result<S::Serialized, Error<S::Error>>
    where
        S: Serializer,
        S::Error: error::Error,
    {
        self.coords.serialize_path::<S>(path)
    }

    fn deserialize_path<S>(
        &mut self,
        path: &str,
        data: S::Serialized,
    ) -> Result<(), Error<S::Error>>
    where
        S: Serializer,
        S::Error: error::Error,
    {
        self.coords.deserialize_path::<S>(path, data)
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
            fn serialize_path<S>(&self, path: &str) -> Result<S::Serialized, Error<S::Error>>
            where
                S: Serializer,
                S::Error: error::Error,
            {
                Err(Error::TypeDoesNotSupportSerialization {
                    type_name: stringify!($type),
                    path: path.to_string(),
                })
            }

            fn deserialize_path<S>(
                &mut self,
                path: &str,
                _data: S::Serialized,
            ) -> Result<(), Error<S::Error>>
            where
                S: Serializer,
                S::Error: error::Error,
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
