use std::{
    collections::BTreeMap,
    ops::Range,
    path::PathBuf,
    time::{Duration, SystemTime},
};

use nalgebra::{Isometry2, Isometry3, Point2, Point3, SMatrix, Vector2, Vector3};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
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

impl SerializeHierarchy for bool {
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
        anyhow::bail!("Cannot access bool with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> anyhow::Result<()> {
        anyhow::bail!("Cannot access bool with path: {}", field_path)
    }

    fn exists(_field_path: &str) -> bool {
        true
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::Primary { name: "bool" }
    }
}

impl SerializeHierarchy for f32 {
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
        anyhow::bail!("Cannot access f32 with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> anyhow::Result<()> {
        anyhow::bail!("Cannot access f32 with path: {}", field_path)
    }

    fn exists(_field_path: &str) -> bool {
        true
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::Primary { name: "f32" }
    }
}

impl SerializeHierarchy for i16 {
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
        anyhow::bail!("Cannot access i16 with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> anyhow::Result<()> {
        anyhow::bail!("Cannot access i16 with path: {}", field_path)
    }

    fn exists(_field_path: &str) -> bool {
        true
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::Primary { name: "i16" }
    }
}

impl SerializeHierarchy for i32 {
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
        anyhow::bail!("Cannot access i32 with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> anyhow::Result<()> {
        anyhow::bail!("Cannot access i32 with path: {}", field_path)
    }

    fn exists(_field_path: &str) -> bool {
        true
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::Primary { name: "i32" }
    }
}

impl SerializeHierarchy for u8 {
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
        anyhow::bail!("Cannot access u8 with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> anyhow::Result<()> {
        anyhow::bail!("Cannot access u8 with path: {}", field_path)
    }

    fn exists(_field_path: &str) -> bool {
        true
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::Primary { name: "u8" }
    }
}

impl SerializeHierarchy for usize {
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
        anyhow::bail!("Cannot access usize with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> anyhow::Result<()> {
        anyhow::bail!("Cannot access usize with path: {}", field_path)
    }

    fn exists(_field_path: &str) -> bool {
        true
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::Primary { name: "usize" }
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

impl SerializeHierarchy for Point2<f32> {
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
        anyhow::bail!("Cannot access Point2<f32> with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> anyhow::Result<()> {
        anyhow::bail!("Cannot access Point2<f32> with path: {}", field_path)
    }

    fn exists(_field_path: &str) -> bool {
        true
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::Primary {
            name: "Point2<f32>",
        }
    }
}

impl SerializeHierarchy for Point3<f32> {
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
        anyhow::bail!("Cannot access Point3<f32> with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> anyhow::Result<()> {
        anyhow::bail!("Cannot access Point3<f32> with path: {}", field_path)
    }

    fn exists(_field_path: &str) -> bool {
        true
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::Primary {
            name: "Point3<f32>",
        }
    }
}

impl SerializeHierarchy for Vector2<f32> {
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
        anyhow::bail!("Cannot access Vector2<f32> with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> anyhow::Result<()> {
        anyhow::bail!("Cannot access Vector2<f32> with path: {}", field_path)
    }

    fn exists(_field_path: &str) -> bool {
        true
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::Primary {
            name: "Vector2<f32>",
        }
    }
}

impl SerializeHierarchy for Vector3<f32> {
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
        anyhow::bail!("Cannot access Vector3<f32> with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> anyhow::Result<()> {
        anyhow::bail!("Cannot access Vector3<f32> with path: {}", field_path)
    }

    fn exists(_field_path: &str) -> bool {
        true
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::Primary {
            name: "Vector3<f32>",
        }
    }
}

impl SerializeHierarchy for SMatrix<f32, 3, 3> {
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
        anyhow::bail!("Cannot access SMatrix<f32, 3,3> with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> anyhow::Result<()> {
        anyhow::bail!("Cannot access SMatrix<f32, 3,3> with path: {}", field_path)
    }

    fn exists(_field_path: &str) -> bool {
        true
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::Primary {
            name: "SMatrix<f32, 3,3>",
        }
    }
}

impl SerializeHierarchy for Isometry2<f32> {
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
        anyhow::bail!("Cannot access Isometry2<f32> with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> anyhow::Result<()> {
        anyhow::bail!("Cannot access Isometry2<f32> with path: {}", field_path)
    }

    fn exists(_field_path: &str) -> bool {
        true
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::Primary {
            name: "Isometry2<f32>",
        }
    }
}

impl SerializeHierarchy for Isometry3<f32> {
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
        anyhow::bail!("Cannot access Isometry3<f32> with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> anyhow::Result<()> {
        anyhow::bail!("Cannot access Isometry3<f32> with path: {}", field_path)
    }

    fn exists(_field_path: &str) -> bool {
        true
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::Primary {
            name: "Isometry3<f32>",
        }
    }
}

impl SerializeHierarchy for SystemTime {
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
        anyhow::bail!("Cannot access SystemTime with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> anyhow::Result<()> {
        anyhow::bail!("Cannot access SystemTime with path: {}", field_path)
    }

    fn exists(_field_path: &str) -> bool {
        true
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::Primary { name: "SystemTime" }
    }
}

impl SerializeHierarchy for Duration {
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
        anyhow::bail!("Cannot access Duration with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> anyhow::Result<()> {
        anyhow::bail!("Cannot access Duration with path: {}", field_path)
    }

    fn exists(_field_path: &str) -> bool {
        true
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::Primary { name: "Duration" }
    }
}

impl SerializeHierarchy for String {
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
        anyhow::bail!("Cannot access String with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> anyhow::Result<()> {
        anyhow::bail!("Cannot access String with path: {}", field_path)
    }

    fn exists(_field_path: &str) -> bool {
        true
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::Primary { name: "String" }
    }
}

impl SerializeHierarchy for Range<f32> {
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
        anyhow::bail!("Cannot access Range<f32> with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> anyhow::Result<()> {
        anyhow::bail!("Cannot access Range<f32> with path: {}", field_path)
    }

    fn exists(_field_path: &str) -> bool {
        true
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::Primary { name: "Range<f32>" }
    }
}

impl SerializeHierarchy for PathBuf {
    fn serialize_hierarchy(&self, field_path: &str) -> anyhow::Result<Value> {
        anyhow::bail!("Cannot access PathBuf with path: {}", field_path)
    }

    fn deserialize_hierarchy(&mut self, field_path: &str, _data: Value) -> anyhow::Result<()> {
        anyhow::bail!("Cannot access PathBuf with path: {}", field_path)
    }

    fn exists(_field_path: &str) -> bool {
        true
    }

    fn get_hierarchy() -> HierarchyType {
        HierarchyType::Primary { name: "PathBuf" }
    }
}
