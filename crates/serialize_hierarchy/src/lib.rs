use std::{
    collections::{BTreeMap, HashSet},
    ops::{Deref, Range},
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime},
};

use color_eyre::{eyre::bail, Result};
use nalgebra::{
    Isometry2, Isometry3, Point2, Point3, SMatrix, UnitComplex, Vector2, Vector3, Vector4,
};
use serde::Serialize;
pub use serde_json;
use serde_json::Value;
pub use serialize_hierarchy_derive::SerializeHierarchy;

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
pub enum HierarchyType {
    Primary {
        name: &'static str,
    },
    Struct {
        fields: BTreeMap<String, HierarchyType>,
    },
    GenericStruct,
    GenericEnum,
    Option {
        nested: Box<HierarchyType>,
    },
    Vec {
        nested: Box<HierarchyType>,
    },
}

pub trait SerializeHierarchy {
    fn serialize_hierarchy(&self, field_path: &str) -> Result<Value>;
    fn deserialize_hierarchy(&mut self, field_path: &str, data: Value) -> Result<()>;
    fn exists(field_path: &str) -> bool;
    fn get_hierarchy() -> HierarchyType;
}

impl<T> SerializeHierarchy for Arc<T>
where
    T: SerializeHierarchy,
{
    fn serialize_hierarchy(&self, field_path: &str) -> Result<Value> {
        self.deref().serialize_hierarchy(field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> Result<()> {
        bail!("Cannot deserialize into Arc with path: `{field_path}`")
    }

    fn exists(field_path: &str) -> bool {
        T::exists(field_path)
    }

    fn get_hierarchy() -> HierarchyType {
        T::get_hierarchy()
    }
}

impl<T> SerializeHierarchy for Option<T>
where
    T: SerializeHierarchy,
{
    fn serialize_hierarchy(&self, field_path: &str) -> Result<Value> {
        match self {
            Some(some) => some.serialize_hierarchy(field_path),
            None => Ok(Value::Null),
        }
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> Result<()> {
        bail!("Cannot deserialize into Option with path: `{field_path}`")
    }

    fn exists(field_path: &str) -> bool {
        T::exists(field_path)
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::Option {
            nested: Box::new(T::get_hierarchy()),
        }
    }
}

impl<T> SerializeHierarchy for HashSet<T> {
    fn serialize_hierarchy(&self, field_path: &str) -> Result<Value> {
        bail!("cannot access HashSet with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> Result<()> {
        bail!("cannot access HashSet with path: {}", field_path)
    }

    fn exists(_field_path: &str) -> bool {
        true
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::GenericStruct
    }
}

impl<T> SerializeHierarchy for Vec<T> {
    fn serialize_hierarchy(&self, field_path: &str) -> Result<Value> {
        bail!("cannot access Vec with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> Result<()> {
        bail!("cannot access Vec with path: {}", field_path)
    }

    fn exists(_field_path: &str) -> bool {
        true
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::GenericStruct
    }
}

macro_rules! serialize_hierarchy_primary_impl {
    ($type:ty) => {
        impl SerializeHierarchy for $type {
            fn serialize_hierarchy(&self, field_path: &str) -> Result<Value> {
                bail!(
                    "cannot access {} with path: {}",
                    stringify!($type),
                    field_path
                )
            }

            fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> Result<()> {
                bail!(
                    "cannot access {} with path: {}",
                    stringify!($type),
                    field_path
                )
            }

            fn exists(_field_path: &str) -> bool {
                true
            }

            fn get_hierarchy() -> HierarchyType {
                HierarchyType::Primary {
                    name: stringify!($type),
                }
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
