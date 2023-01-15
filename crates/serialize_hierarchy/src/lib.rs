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
    Isometry2, Isometry3, Point2, Point3, SMatrix, UnitComplex, Vector2, Vector3, Vector4,
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
        [String::new()].into()
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
        [String::new()].into()
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
                [String::new()].into()
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
serialize_hierarchy_primary_impl!(Point2<f32>);
serialize_hierarchy_primary_impl!(Point2<u16>);
serialize_hierarchy_primary_impl!(Point3<f32>);
serialize_hierarchy_primary_impl!(Vector2<f32>);
serialize_hierarchy_primary_impl!(Vector3<f32>);
serialize_hierarchy_primary_impl!(Vector4<f32>);
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
