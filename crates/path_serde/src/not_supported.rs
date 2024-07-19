use std::{
    collections::{HashMap, HashSet, VecDeque},
    net::SocketAddr,
    path::PathBuf,
    time::{Duration, SystemTime},
};

use crate::{deserialize, serialize, PathDeserialize, PathIntrospect, PathSerialize};
use nalgebra::{DMatrix, Rotation3, SMatrix};
use serde::{Deserializer, Serializer};
use serde_json::Value;

macro_rules! implement_as_not_supported {
    ($type:ty) => {
        impl PathSerialize for $type {
            fn serialize_path<S>(
                &self,
                path: &str,
                _serializer: S,
            ) -> Result<S::Ok, serialize::Error<S::Error>>
            where
                S: Serializer,
            {
                Err(serialize::Error::PathDoesNotExist {
                    path: path.to_string(),
                })
            }
        }

        impl PathDeserialize for $type {
            fn deserialize_path<'de, D>(
                &mut self,
                path: &str,
                _data: D,
            ) -> Result<(), deserialize::Error<D::Error>>
            where
                D: Deserializer<'de>,
            {
                Err(deserialize::Error::PathDoesNotExist {
                    path: path.to_string(),
                })
            }
        }

        impl PathIntrospect for $type {
            fn extend_with_fields(_fields: &mut HashSet<String>, _prefix: &str) {}
        }
    };
    ($type:ty, $($generic:tt),*) => {
        impl<$($generic),*> PathSerialize for $type {
            fn serialize_path<S>(
                &self,
                path: &str,
                _serializer: S,
            ) -> Result<S::Ok, serialize::Error<S::Error>>
            where
                S: Serializer,
            {
                Err(serialize::Error::PathDoesNotExist {
                    path: path.to_string(),
                })
            }
        }

        impl<$($generic),*> PathDeserialize for $type {
            fn deserialize_path<'de, D>(
                &mut self,
                path: &str,
                _data: D,
            ) -> Result<(), deserialize::Error<D::Error>>
            where
                D: Deserializer<'de>,
            {
                Err(deserialize::Error::PathDoesNotExist {
                    path: path.to_string(),
                })
            }
        }

        impl<$($generic),*> PathIntrospect for $type {
            fn extend_with_fields(_fields: &mut HashSet<String>, _prefix: &str) {}
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
implement_as_not_supported!(DMatrix<f32>);
implement_as_not_supported!(Rotation3<f32>);
implement_as_not_supported!(SMatrix<f32, 3, 3>);
implement_as_not_supported!(SMatrix<f32, 3, 4>);
// stdlib
implement_as_not_supported!(Duration);
implement_as_not_supported!(HashMap<K, V>, K, V);
implement_as_not_supported!(HashSet<T>, T);
implement_as_not_supported!(PathBuf);
implement_as_not_supported!(SocketAddr);
implement_as_not_supported!(String);
implement_as_not_supported!(SystemTime);
implement_as_not_supported!(Vec<T>, T);
implement_as_not_supported!(VecDeque<T>, T);
implement_as_not_supported!(Value);
