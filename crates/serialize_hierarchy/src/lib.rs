use std::{
    collections::BTreeMap,
    ops::Range,
    path::PathBuf,
    time::{Duration, SystemTime},
};

use nalgebra::{Isometry2, Isometry3, Point2, Point3, SMatrix, Vector2, Vector3, Vector4};
use serde::Serialize;
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
    Option {
        nested: Box<HierarchyType>,
    },
    Vec {
        nested: Box<HierarchyType>,
    },
}

pub trait SerializeHierarchy {
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value>;
    fn deserialize_hierarchy(&mut self, field_path: &str, data: Value) -> anyhow::Result<()>;
    fn exists(field_path: &str) -> bool;
    fn get_hierarchy() -> HierarchyType;
}

impl<T> SerializeHierarchy for Option<T>
where
    T: Default + SerializeHierarchy,
{
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
        match self {
            Some(some) => some.serialize_hierarchy(field_path),
            None => Ok(Value::Null),
        }
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, data: Value) -> anyhow::Result<()> {
        self.get_or_insert_with(Default::default)
            .deserialize_hierarchy(field_path, data)
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

impl<T> SerializeHierarchy for Vec<T> {
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
        anyhow::bail!("Cannot access Vec with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> anyhow::Result<()> {
        anyhow::bail!("Cannot access Vec with path: {}", field_path)
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
            fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
                anyhow::bail!(
                    "Cannot access {} with path: {}",
                    stringify!($type),
                    field_path
                )
            }

            fn deserialize_hierarchy(
                &mut self,
                field_path: &str,
                _data: Value,
            ) -> anyhow::Result<()> {
                anyhow::bail!(
                    "Cannot access {} with path: {}",
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
serialize_hierarchy_primary_impl!(u64);
serialize_hierarchy_primary_impl!(usize);
serialize_hierarchy_primary_impl!(Point2<f32>);
serialize_hierarchy_primary_impl!(Point3<f32>);
serialize_hierarchy_primary_impl!(Vector2<f32>);
serialize_hierarchy_primary_impl!(Vector3<f32>);
serialize_hierarchy_primary_impl!(Vector4<f32>);
serialize_hierarchy_primary_impl!(SMatrix<f32, 3, 3>);
serialize_hierarchy_primary_impl!(Isometry2<f32>);
serialize_hierarchy_primary_impl!(Isometry3<f32>);
serialize_hierarchy_primary_impl!(SystemTime);
serialize_hierarchy_primary_impl!(Duration);
serialize_hierarchy_primary_impl!(String);
serialize_hierarchy_primary_impl!(Range<f32>);
serialize_hierarchy_primary_impl!(Range<Duration>);
serialize_hierarchy_primary_impl!(PathBuf);
