use std::{
    collections::{BTreeSet, HashSet},
    path::PathBuf,
    time::{Duration, SystemTime},
};

use nalgebra::{Isometry2, Isometry3, SMatrix, UnitComplex, UnitQuaternion};
use serde::{Deserializer, Serializer};

use crate::{error::Error, SerializeHierarchy};

macro_rules! implement_as_not_supported {
    ($type:ty) => {
        impl SerializeHierarchy for $type {
            fn serialize_path<S>(
                &self,
                path: &str,
                _serializer: S,
            ) -> Result<S::Ok, Error<S::Error>>
            where
                S: Serializer,
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
    ($type:ty, $generic:tt) => {
        impl<$generic> SerializeHierarchy for $type {
            fn serialize_path<S>(
                &self,
                path: &str,
                _serializer: S,
            ) -> Result<S::Ok, Error<S::Error>>
            where
                S: Serializer,
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

// primary types
implement_as_not_supported!(bool);
implement_as_not_supported!(f32);
implement_as_not_supported!(i16);
implement_as_not_supported!(i32);
implement_as_not_supported!(u8);
implement_as_not_supported!(u16);
implement_as_not_supported!(u32);
implement_as_not_supported!(u64);
implement_as_not_supported!(usize);
// nalgebra
implement_as_not_supported!(SMatrix<f32, 3, 3>);
implement_as_not_supported!(Isometry2<f32>);
implement_as_not_supported!(Isometry3<f32>);
implement_as_not_supported!(UnitComplex<f32>);
implement_as_not_supported!(UnitQuaternion<f32>);
// stdlib
implement_as_not_supported!(SystemTime);
implement_as_not_supported!(Duration);
implement_as_not_supported!(String);
implement_as_not_supported!(PathBuf);
implement_as_not_supported!(Vec<T>, T);
implement_as_not_supported!(HashSet<T>, T);
