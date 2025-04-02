use std::{
    collections::HashSet,
    ops::{Deref, DerefMut, Range, RangeInclusive},
    sync::Arc,
    time::Duration,
};

use nalgebra::{
    ArrayStorage, Const, Isometry2, Isometry3, Matrix, Point, RealField, Scalar, SimdRealField,
    UnitComplex, UnitQuaternion, Vector2, Vector3, U1,
};
use num_traits::real::Real;
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
        deserializer: D,
    ) -> Result<(), deserialize::Error<D::Error>>
    where
        D: Deserializer<'de>,
    {
        self.deref_mut().deserialize_path(path, deserializer)
    }
}

impl<T> PathIntrospect for Box<T>
where
    T: PathIntrospect,
{
    fn extend_with_fields(fields: &mut HashSet<String>, prefix: &str) {
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

impl<T> PathIntrospect for Arc<T>
where
    T: PathIntrospect,
{
    fn extend_with_fields(fields: &mut HashSet<String>, prefix: &str) {
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
        deserializer: D,
    ) -> Result<(), deserialize::Error<D::Error>>
    where
        D: Deserializer<'de>,
    {
        match self {
            Some(some) => some.deserialize_path(path, deserializer),
            None => Err(deserialize::Error::PathDoesNotExist {
                path: path.to_string(),
            }),
        }
    }
}

impl<T> PathIntrospect for Option<T>
where
    T: PathIntrospect,
{
    fn extend_with_fields(fields: &mut HashSet<String>, prefix: &str) {
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
            _ => Err(serialize::Error::PathDoesNotExist {
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
            _ => Err(deserialize::Error::PathDoesNotExist {
                path: path.to_owned(),
            }),
        }
    }
}

impl<T> PathIntrospect for Range<T>
where
    T: PathIntrospect,
{
    fn extend_with_fields(fields: &mut HashSet<String>, prefix: &str) {
        fields.insert(format!("{prefix}start"));
        fields.insert(format!("{prefix}end"));
    }
}

impl<T> PathSerialize for RangeInclusive<T>
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
            (_, Some(("start", suffix))) => self.start().serialize_path(suffix, serializer),
            (_, Some(("end", suffix))) => self.end().serialize_path(suffix, serializer),
            ("start", None) => self
                .start()
                .serialize(serializer)
                .map_err(serialize::Error::SerializationFailed),
            ("end", None) => self
                .end()
                .serialize(serializer)
                .map_err(serialize::Error::SerializationFailed),
            _ => Err(serialize::Error::PathDoesNotExist {
                path: path.to_owned(),
            }),
        }
    }
}

impl<T> PathDeserialize for RangeInclusive<T>
where
    T: PathDeserialize + Clone,
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
            (_, Some(("start", suffix))) => {
                let mut start = self.start().clone();
                start.deserialize_path(suffix, deserializer)?;
                *self = start..=self.end().clone();
                Ok(())
            }
            (_, Some(("end", suffix))) => {
                let mut end = self.end().clone();
                end.deserialize_path(suffix, deserializer)?;
                *self = self.start().clone()..=end;
                Ok(())
            }
            ("start", None) => {
                let start = T::deserialize(deserializer)
                    .map_err(deserialize::Error::DeserializationFailed)?;
                *self = start..=self.end().clone();
                Ok(())
            }
            ("end", None) => {
                let end = T::deserialize(deserializer)
                    .map_err(deserialize::Error::DeserializationFailed)?;
                *self = self.start().clone()..=end;
                Ok(())
            }
            _ => Err(deserialize::Error::PathDoesNotExist {
                path: path.to_owned(),
            }),
        }
    }
}

impl<T> PathIntrospect for RangeInclusive<T>
where
    T: PathIntrospect,
{
    fn extend_with_fields(fields: &mut HashSet<String>, prefix: &str) {
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
            _ => Err(serialize::Error::PathDoesNotExist {
                path: path.to_owned(),
            }),
        }
    }
}

impl<T, const N: usize> PathDeserialize for Matrix<T, Const<N>, U1, ArrayStorage<T, N, 1>>
where
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
                let deserialized = T::deserialize(deserializer)
                    .map_err(deserialize::Error::DeserializationFailed)?;
                self[index] = deserialized;
                Ok(())
            }
            None => Err(deserialize::Error::PathDoesNotExist {
                path: path.to_owned(),
            }),
        }
    }
}

impl<T, const N: usize> PathIntrospect for Matrix<T, Const<N>, U1, ArrayStorage<T, N, 1>> {
    fn extend_with_fields(fields: &mut HashSet<String>, prefix: &str) {
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
    fn extend_with_fields(fields: &mut HashSet<String>, prefix: &str) {
        Matrix::<T, Const<N>, U1, ArrayStorage<T, N, 1>>::extend_with_fields(fields, prefix)
    }
}

impl<T> PathSerialize for UnitQuaternion<T>
where
    T: Serialize + PathSerialize + SimdRealField + RealField,
    T::Element: SimdRealField,
{
    fn serialize_path<S>(
        &self,
        path: &str,
        serializer: S,
    ) -> Result<S::Ok, serialize::Error<S::Error>>
    where
        S: Serializer,
    {
        match path {
            "roll" => {
                let (roll, _, _) = self.euler_angles();
                roll.serialize(serializer)
                    .map_err(serialize::Error::SerializationFailed)
            }
            "pitch" => {
                let (_, pitch, _) = self.euler_angles();
                pitch
                    .serialize(serializer)
                    .map_err(serialize::Error::SerializationFailed)
            }
            "yaw" => {
                let (_, _, yaw) = self.euler_angles();
                yaw.serialize(serializer)
                    .map_err(serialize::Error::SerializationFailed)
            }
            _ => Err(serialize::Error::PathDoesNotExist {
                path: path.to_owned(),
            }),
        }
    }
}

impl<T> PathDeserialize for UnitQuaternion<T> {
    fn deserialize_path<'de, D>(
        &mut self,
        path: &str,
        _deserializer: D,
    ) -> Result<(), deserialize::Error<D::Error>>
    where
        D: Deserializer<'de>,
    {
        Err(deserialize::Error::PathDoesNotExist {
            path: path.to_string(),
        })
    }
}

impl<T> PathIntrospect for UnitQuaternion<T> {
    fn extend_with_fields(fields: &mut HashSet<String>, prefix: &str) {
        fields.insert(format!("{prefix}roll"));
        fields.insert(format!("{prefix}pitch"));
        fields.insert(format!("{prefix}yaw"));
    }
}

impl<T> PathSerialize for UnitComplex<T>
where
    T: Serialize + PathSerialize + SimdRealField + RealField + Real,
    T::Element: SimdRealField,
{
    fn serialize_path<S>(
        &self,
        path: &str,
        serializer: S,
    ) -> Result<S::Ok, serialize::Error<S::Error>>
    where
        S: Serializer,
    {
        match path {
            "rad" => self
                .angle()
                .serialize(serializer)
                .map_err(serialize::Error::SerializationFailed),
            "deg" => self
                .angle()
                .to_degrees()
                .serialize(serializer)
                .map_err(serialize::Error::SerializationFailed),
            _ => Err(serialize::Error::PathDoesNotExist {
                path: path.to_owned(),
            }),
        }
    }
}

impl<T> PathDeserialize for UnitComplex<T>
where
    for<'de> T: Deserialize<'de> + SimdRealField + Real,
    T::Element: SimdRealField,
{
    fn deserialize_path<'de, D>(
        &mut self,
        path: &str,
        deserializer: D,
    ) -> Result<(), deserialize::Error<D::Error>>
    where
        D: Deserializer<'de>,
    {
        match path {
            "rad" => {
                let angle = T::deserialize(deserializer)
                    .map_err(deserialize::Error::DeserializationFailed)?;
                *self = UnitComplex::new(angle);
                Ok(())
            }
            "deg" => {
                let angle = T::deserialize(deserializer)
                    .map_err(deserialize::Error::DeserializationFailed)?;
                *self = UnitComplex::new(angle.to_radians());
                Ok(())
            }
            _ => Err(deserialize::Error::PathDoesNotExist {
                path: path.to_owned(),
            }),
        }
    }
}

impl<T> PathIntrospect for UnitComplex<T> {
    fn extend_with_fields(fields: &mut HashSet<String>, prefix: &str) {
        fields.insert(format!("{prefix}rad"));
        fields.insert(format!("{prefix}deg"));
    }
}

impl<T> PathSerialize for Isometry2<T>
where
    T: Serialize + PathSerialize + SimdRealField + RealField + Real,
    T::Element: SimdRealField,
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
            (_, Some(("translation", suffix))) => {
                self.translation.vector.serialize_path(suffix, serializer)
            }
            (_, Some(("rotation", suffix))) => self.rotation.serialize_path(suffix, serializer),
            ("translation", None) => self
                .translation
                .vector
                .serialize(serializer)
                .map_err(serialize::Error::SerializationFailed),
            ("rotation", None) => self
                .rotation
                .serialize(serializer)
                .map_err(serialize::Error::SerializationFailed),
            _ => Err(serialize::Error::PathDoesNotExist {
                path: path.to_owned(),
            }),
        }
    }
}

impl<T> PathDeserialize for Isometry2<T>
where
    for<'de> T: Deserialize<'de> + SimdRealField + Real,
    T::Element: SimdRealField,
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
            (_, Some(("translation", suffix))) => self
                .translation
                .vector
                .deserialize_path(suffix, deserializer),
            (_, Some(("rotation", suffix))) => self.rotation.deserialize_path(suffix, deserializer),
            ("translation", None) => {
                self.translation.vector = Vector2::<T>::deserialize(deserializer)
                    .map_err(deserialize::Error::DeserializationFailed)?;
                Ok(())
            }
            ("rotation", None) => {
                self.rotation = UnitComplex::<T>::deserialize(deserializer)
                    .map_err(deserialize::Error::DeserializationFailed)?;
                Ok(())
            }
            _ => Err(deserialize::Error::PathDoesNotExist {
                path: path.to_owned(),
            }),
        }
    }
}

impl<T> PathIntrospect for Isometry2<T> {
    fn extend_with_fields(fields: &mut HashSet<String>, prefix: &str) {
        fields.insert(format!("{prefix}translation"));
        fields.insert(format!("{prefix}rotation"));
        Vector2::<T>::extend_with_fields(fields, &format!("{prefix}translation."));
        UnitComplex::<T>::extend_with_fields(fields, &format!("{prefix}rotation."));
    }
}

impl<T> PathSerialize for Isometry3<T>
where
    T: Serialize + PathSerialize + SimdRealField + RealField,
    T::Element: SimdRealField,
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
            (_, Some(("translation", suffix))) => {
                self.translation.vector.serialize_path(suffix, serializer)
            }
            (_, Some(("rotation", suffix))) => self.rotation.serialize_path(suffix, serializer),
            ("translation", None) => self
                .translation
                .vector
                .serialize(serializer)
                .map_err(serialize::Error::SerializationFailed),
            ("rotation", None) => self
                .rotation
                .serialize(serializer)
                .map_err(serialize::Error::SerializationFailed),
            _ => Err(serialize::Error::PathDoesNotExist {
                path: path.to_owned(),
            }),
        }
    }
}

impl<T> PathDeserialize for Isometry3<T>
where
    for<'de> T: Deserialize<'de> + Scalar,
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
            (_, Some(("translation", suffix))) => self
                .translation
                .vector
                .deserialize_path(suffix, deserializer),
            (_, Some(("rotation", suffix))) => self.rotation.deserialize_path(suffix, deserializer),
            ("translation", None) => {
                self.translation.vector = Vector3::<T>::deserialize(deserializer)
                    .map_err(deserialize::Error::DeserializationFailed)?;
                Ok(())
            }
            ("rotation", None) => {
                self.rotation = UnitQuaternion::<T>::deserialize(deserializer)
                    .map_err(deserialize::Error::DeserializationFailed)?;
                Ok(())
            }
            _ => Err(deserialize::Error::PathDoesNotExist {
                path: path.to_owned(),
            }),
        }
    }
}

impl<T> PathIntrospect for Isometry3<T> {
    fn extend_with_fields(fields: &mut HashSet<String>, prefix: &str) {
        fields.insert(format!("{prefix}translation"));
        fields.insert(format!("{prefix}rotation"));
        Vector3::<T>::extend_with_fields(fields, &format!("{prefix}translation."));
        UnitQuaternion::<T>::extend_with_fields(fields, &format!("{prefix}rotation."));
    }
}

impl PathSerialize for Duration {
    fn serialize_path<S>(
        &self,
        path: &str,
        serializer: S,
    ) -> Result<S::Ok, serialize::Error<S::Error>>
    where
        S: Serializer,
    {
        match path {
            "secs_f32" => self
                .as_secs_f32()
                .serialize(serializer)
                .map_err(serialize::Error::SerializationFailed),
            "millis" => self
                .as_millis()
                .serialize(serializer)
                .map_err(serialize::Error::SerializationFailed),
            _ => Err(serialize::Error::PathDoesNotExist {
                path: path.to_owned(),
            }),
        }
    }
}

impl PathDeserialize for Duration {
    fn deserialize_path<'de, D>(
        &mut self,
        path: &str,
        deserializer: D,
    ) -> Result<(), deserialize::Error<D::Error>>
    where
        D: Deserializer<'de>,
    {
        match path {
            "secs_f32" => {
                let secs_f32 = f32::deserialize(deserializer)
                    .map_err(deserialize::Error::DeserializationFailed)?;
                *self = Duration::from_secs_f32(secs_f32);
                Ok(())
            }
            "millis" => {
                let millis = u64::deserialize(deserializer)
                    .map_err(deserialize::Error::DeserializationFailed)?;
                *self = Duration::from_millis(millis);
                Ok(())
            }
            _ => Err(deserialize::Error::PathDoesNotExist {
                path: path.to_owned(),
            }),
        }
    }
}

impl PathIntrospect for Duration {
    fn extend_with_fields(fields: &mut HashSet<String>, prefix: &str) {
        fields.insert(format!("{prefix}secs_f32"));
        fields.insert(format!("{prefix}millis"));
    }
}

impl<A, B> PathSerialize for (A, B)
where
    A: PathSerialize + Serialize,
    B: PathSerialize + Serialize,
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
            (_, Some(("0", suffix))) => self.0.serialize_path(suffix, serializer),
            (_, Some(("1", suffix))) => self.1.serialize_path(suffix, serializer),
            ("0", None) => self
                .0
                .serialize(serializer)
                .map_err(serialize::Error::SerializationFailed),
            ("1", None) => self
                .1
                .serialize(serializer)
                .map_err(serialize::Error::SerializationFailed),
            _ => Err(serialize::Error::PathDoesNotExist {
                path: path.to_owned(),
            }),
        }
    }
}

impl<A, B> PathDeserialize for (A, B)
where
    A: PathDeserialize,
    B: PathDeserialize,
    for<'de> A: Deserialize<'de>,
    for<'de> B: Deserialize<'de>,
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
            (_, Some(("0", suffix))) => self.0.deserialize_path(suffix, deserializer),
            (_, Some(("1", suffix))) => self.1.deserialize_path(suffix, deserializer),
            ("0", None) => {
                self.0 = A::deserialize(deserializer)
                    .map_err(deserialize::Error::DeserializationFailed)?;
                Ok(())
            }
            ("1", None) => {
                self.1 = B::deserialize(deserializer)
                    .map_err(deserialize::Error::DeserializationFailed)?;
                Ok(())
            }
            _ => Err(deserialize::Error::PathDoesNotExist {
                path: path.to_owned(),
            }),
        }
    }
}

impl<A, B> PathIntrospect for (A, B)
where
    A: PathIntrospect,
    B: PathIntrospect,
{
    fn extend_with_fields(fields: &mut HashSet<String>, prefix: &str) {
        fields.insert(format!("{prefix}0"));
        fields.insert(format!("{prefix}1"));
    }
}
