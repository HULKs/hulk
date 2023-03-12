use std::{
    collections::{BTreeSet, HashSet},
    path::PathBuf,
    time::{Duration, SystemTime},
};

use nalgebra::{Isometry2, Isometry3, SMatrix, UnitComplex};
use serde::{Deserializer, Serializer};

use crate::{error::Error, SerializeHierarchy};

macro_rules! not_supported {
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
not_supported!(bool);
not_supported!(f32);
not_supported!(i16);
not_supported!(i32);
not_supported!(u8);
not_supported!(u16);
not_supported!(u32);
not_supported!(u64);
not_supported!(usize);
// nalgebra
not_supported!(SMatrix<f32, 3, 3>);
not_supported!(Isometry2<f32>);
not_supported!(Isometry3<f32>);
not_supported!(UnitComplex<f32>);
// stdlib
not_supported!(SystemTime);
not_supported!(Duration);
not_supported!(String);
not_supported!(PathBuf);
not_supported!(Vec<T>, T);
not_supported!(HashSet<T>, T);
