use std::{
    collections::BTreeSet,
    ops::{Deref, DerefMut, Range},
    sync::Arc,
};

use nalgebra::{ArrayStorage, Const, Matrix, Point, Scalar, U1};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{deserialize, serialize, PathDeserialize, PathIntrospect, PathSerialize};

impl<T> PathSerialize for Box<T>
where
    T: PathSerialize,
{
    fn serialize_path<S>(
        &self,
        path: &str,
        serializer: S,
    ) -> Result<S::Ok, serialize::Error<S::Error>>
    where
        S: Serializer,
    {
        self.deref().serialize_path(path, serializer)
    }
}

impl<T> PathDeserialize for Box<T>
where
    T: PathDeserialize,
{
    fn deserialize_path<'de, D>(
        &mut self,
        path: &str,
        data: D,
    ) -> Result<(), deserialize::Error<D::Error>>
    where
        D: Deserializer<'de>,
    {
        self.deref_mut().deserialize_path(path, data)
    }
}

impl<T> PathIntrospect for Box<T>
where
    T: PathIntrospect,
{
    fn extend_with_fields(fields: &mut BTreeSet<String>, prefix: &str) {
        T::extend_with_fields(fields, prefix)
    }
}

impl<T> PathSerialize for Arc<T>
where
    T: PathSerialize,
{
    fn serialize_path<S>(
        &self,
        path: &str,
        serializer: S,
    ) -> Result<S::Ok, serialize::Error<S::Error>>
    where
        S: Serializer,
    {
        self.deref().serialize_path(path, serializer)
    }
}

impl<T> PathDeserialize for Arc<T>
where
    T: PathDeserialize,
{
    fn deserialize_path<'de, D>(
        &mut self,
        path: &str,
        _data: D,
    ) -> Result<(), deserialize::Error<D::Error>>
    where
        D: Deserializer<'de>,
    {
        Err(deserialize::Error::NotSupported {
            type_name: "Arc",
            path: path.to_string(),
        })
    }
}

impl<T> PathIntrospect for Arc<T>
where
    T: PathIntrospect,
{
    fn extend_with_fields(fields: &mut BTreeSet<String>, prefix: &str) {
        T::extend_with_fields(fields, prefix)
    }
}

impl<T> PathSerialize for Option<T>
where
    T: PathSerialize,
{
    fn serialize_path<S>(
        &self,
        path: &str,
        serializer: S,
    ) -> Result<S::Ok, serialize::Error<S::Error>>
    where
        S: Serializer,
    {
        match self {
            Some(some) => some.serialize_path(path, serializer),
            None => (None as Option<()>)
                .serialize(serializer)
                .map_err(serialize::Error::SerializationFailed),
        }
    }
}

impl<T> PathDeserialize for Option<T>
where
    T: PathDeserialize,
{
    fn deserialize_path<'de, D>(
        &mut self,
        path: &str,
        _data: D,
    ) -> Result<(), deserialize::Error<D::Error>>
    where
        D: Deserializer<'de>,
    {
        Err(deserialize::Error::NotSupported {
            type_name: "Option",
            path: path.to_string(),
        })
    }
}

impl<T> PathIntrospect for Option<T>
where
    T: PathIntrospect,
{
    fn extend_with_fields(fields: &mut BTreeSet<String>, prefix: &str) {
        T::extend_with_fields(fields, prefix)
    }
}

impl<T> PathSerialize for Range<T>
where
    T: PathSerialize + Serialize,
{
    fn serialize_path<S>(
        &self,
        path: &str,
        serializer: S,
    ) -> Result<S::Ok, serialize::Error<S::Error>>
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
                .map_err(serialize::Error::SerializationFailed),
            ("end", None) => self
                .end
                .serialize(serializer)
                .map_err(serialize::Error::SerializationFailed),
            _ => Err(serialize::Error::UnexpectedPath {
                path: path.to_owned(),
            }),
        }
    }
}

impl<T> PathDeserialize for Range<T>
where
    T: PathDeserialize,
    for<'de> T: Deserialize<'de>,
{
    fn deserialize_path<'de, D>(
        &mut self,
        path: &str,
        deserializer: D,
    ) -> Result<(), deserialize::Error<D::Error>>
    where
        D: Deserializer<'de>,
    {
        let split = path.split_once('.');
        match (path, split) {
            (_, Some(("start", suffix))) => self.start.deserialize_path(suffix, deserializer),
            (_, Some(("end", suffix))) => self.end.deserialize_path(suffix, deserializer),
            ("start", None) => {
                self.start = T::deserialize(deserializer)
                    .map_err(deserialize::Error::DeserializationFailed)?;
                Ok(())
            }
            ("end", None) => {
                self.end = T::deserialize(deserializer)
                    .map_err(deserialize::Error::DeserializationFailed)?;
                Ok(())
            }
            _ => Err(deserialize::Error::UnexpectedPath {
                path: path.to_owned(),
            }),
        }
    }
}

impl<T> PathIntrospect for Range<T>
where
    T: PathIntrospect,
{
    fn extend_with_fields(fields: &mut BTreeSet<String>, prefix: &str) {
        fields.insert(format!("{prefix}start"));
        fields.insert(format!("{prefix}end"));
    }
}

impl<T, const N: usize> PathSerialize for Matrix<T, Const<N>, U1, ArrayStorage<T, N, 1>>
where
    T: PathSerialize + Serialize,
{
    fn serialize_path<S>(
        &self,
        path: &str,
        serializer: S,
    ) -> Result<S::Ok, serialize::Error<S::Error>>
    where
        S: Serializer,
    {
        let index = ["x", "y", "z", "w", "v", "u"][0..N]
            .iter()
            .position(|name| name == &path);
        match index {
            Some(index) => self[index]
                .serialize(serializer)
                .map_err(serialize::Error::SerializationFailed),
            _ => Err(serialize::Error::UnexpectedPath {
                path: path.to_owned(),
            }),
        }
    }
}

impl<T, const N: usize> PathDeserialize for Matrix<T, Const<N>, U1, ArrayStorage<T, N, 1>>
where
    T: PathDeserialize,
    for<'de> T: Deserialize<'de>,
{
    fn deserialize_path<'de, D>(
        &mut self,
        path: &str,
        deserializer: D,
    ) -> Result<(), deserialize::Error<D::Error>>
    where
        D: Deserializer<'de>,
    {
        let index = ["x", "y", "z", "w", "v", "u"][0..N]
            .iter()
            .position(|name| name == &path);
        match index {
            Some(index) => {
                let deserialized = <T as Deserialize>::deserialize(deserializer)
                    .map_err(deserialize::Error::DeserializationFailed)?;
                self[index] = deserialized;
                Ok(())
            }
            None => Err(deserialize::Error::UnexpectedPath {
                path: path.to_owned(),
            }),
        }
    }
}

impl<T, const N: usize> PathIntrospect for Matrix<T, Const<N>, U1, ArrayStorage<T, N, 1>> {
    fn extend_with_fields(fields: &mut BTreeSet<String>, prefix: &str) {
        for field in &["x", "y", "z", "w", "v", "u"][0..N] {
            fields.insert(format!("{prefix}{field}"));
        }
    }
}

impl<T, const N: usize> PathSerialize for Point<T, N>
where
    T: PathSerialize + Scalar + Serialize,
{
    fn serialize_path<S>(
        &self,
        path: &str,
        serializer: S,
    ) -> Result<S::Ok, serialize::Error<S::Error>>
    where
        S: Serializer,
    {
        self.coords.serialize_path(path, serializer)
    }
}

impl<T, const N: usize> PathDeserialize for Point<T, N>
where
    T: PathDeserialize + Scalar,
    for<'de> T: Deserialize<'de>,
{
    fn deserialize_path<'de, D>(
        &mut self,
        path: &str,
        deserializer: D,
    ) -> Result<(), deserialize::Error<D::Error>>
    where
        D: Deserializer<'de>,
    {
        self.coords.deserialize_path(path, deserializer)
    }
}

impl<T, const N: usize> PathIntrospect for Point<T, N>
where
    T: PathIntrospect + Scalar,
{
    fn extend_with_fields(fields: &mut BTreeSet<String>, prefix: &str) {
        Matrix::<T, Const<N>, U1, ArrayStorage<T, N, 1>>::extend_with_fields(fields, prefix)
    }
}
