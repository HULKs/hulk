use std::{
    collections::BTreeSet,
    ops::{Deref, Range},
    sync::Arc,
};

use nalgebra::{ArrayStorage, Const, Matrix, Point, Scalar, U1};
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};

use crate::{error::Error, SerializeHierarchy};

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

    fn fill_fields(fields: &mut BTreeSet<String>, prefix: &str) {
        T::fill_fields(fields, prefix)
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

    fn fill_fields(fields: &mut BTreeSet<String>, prefix: &str) {
        T::fill_fields(fields, prefix)
    }
}

impl<T> SerializeHierarchy for Range<T>
where
    T: SerializeHierarchy + Serialize,
    for<'de> T: Deserialize<'de>,
{
    fn serialize_path<S>(&self, path: &str, serializer: S) -> Result<S::Ok, Error<S::Error>>
    where
        S: Serializer,
    {
        let split = path.split_once('.');
        match (path, split) {
            (_, Some(("start", suffix))) => self.start.serialize_path(suffix, serializer),
            (_, Some(("end", suffix))) => self.end.serialize_path(suffix, serializer),
            ("start", None) => self
                .start
                .serialize(serializer)
                .map_err(Error::SerializationFailed),
            ("end", None) => self
                .end
                .serialize(serializer)
                .map_err(Error::SerializationFailed),
            _ => Err(Error::UnexpectedPathSegment {
                segment: path.to_string(),
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
        let split = path.split_once('.');
        match (path, split) {
            (_, Some(("start", suffix))) => self.start.deserialize_path(suffix, deserializer),
            (_, Some(("end", suffix))) => self.end.deserialize_path(suffix, deserializer),
            ("start", None) => {
                self.start = T::deserialize(deserializer).map_err(Error::DeserializationFailed)?;
                Ok(())
            }
            ("end", None) => {
                self.end = T::deserialize(deserializer).map_err(Error::DeserializationFailed)?;
                Ok(())
            }
            _ => Err(Error::UnexpectedPathSegment {
                segment: path.to_string(),
            }),
        }
    }

    fn exists(path: &str) -> bool {
        let split = path.split_once('.');
        match (path, split) {
            (_, Some(("start", suffix))) | (_, Some(("end", suffix))) => T::exists(suffix),
            ("start", None) | ("end", None) => true,
            _ => false,
        }
    }

    fn fill_fields(fields: &mut BTreeSet<String>, prefix: &str) {
        fields.insert(format!("{prefix}start"));
        fields.insert(format!("{prefix}end"));
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

    fn fill_fields(fields: &mut BTreeSet<String>, prefix: &str) {
        for field in &["x", "y", "z", "w", "v", "u"][0..N] {
            fields.insert(format!("{prefix}{field}"));
        }
    }
}

impl<T: Serialize + DeserializeOwned + Clone + Scalar, const N: usize> SerializeHierarchy
    for Point<T, N>
{
    fn serialize_path<S>(&self, path: &str, serializer: S) -> Result<S::Ok, Error<S::Error>>
    where
        S: Serializer,
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

    fn fill_fields(fields: &mut BTreeSet<String>, prefix: &str) {
        Matrix::<T, Const<N>, U1, ArrayStorage<T, N, 1>>::fill_fields(fields, prefix)
    }
}
