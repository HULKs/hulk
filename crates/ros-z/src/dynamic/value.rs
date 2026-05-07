//! Runtime representation of dynamic message values.
//!
//! This module provides the `DynamicValue` enum for representing any supported
//! value at runtime, along with conversion traits.

use super::message::DynamicStruct;
use super::schema::Schema;
use crate::dynamic::DynamicError;
use ros_z_schema::{
    EnumPayloadDef, EnumVariantDef, PrimitiveTypeDef, SequenceLengthDef, TypeDef, TypeDefinition,
    TypeName,
};
use std::collections::BTreeSet;

/// Runtime representation of any supported dynamic value.
#[derive(Clone, Debug, PartialEq)]
pub enum DynamicValue {
    // Primitives
    Bool(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Uint8(u8),
    Uint16(u16),
    Uint32(u32),
    Uint64(u64),
    Float32(f32),
    Float64(f64),
    String(String),

    /// Optimized byte array (for uint8[] / byte[])
    Bytes(Vec<u8>),

    /// Nested or root struct value.
    Struct(Box<DynamicStruct>),
    /// Optional value encoded with a `u32` presence tag.
    Optional(Option<Box<DynamicValue>>),
    /// Tagged enum encoded with a `u32` variant index.
    Enum(EnumValue),

    /// Homogeneous sequence or fixed array value.
    Sequence(Vec<DynamicValue>),
    /// Map entries in wire order.
    Map(Vec<(DynamicValue, DynamicValue)>),
}

/// Runtime representation of a serde enum value.
#[derive(Clone, Debug, PartialEq)]
pub struct EnumValue {
    pub variant_index: u32,
    pub variant_name: String,
    pub payload: EnumPayloadValue,
}

impl EnumValue {
    /// Create a new enum value.
    pub fn new(
        variant_index: u32,
        variant_name: impl Into<String>,
        payload: EnumPayloadValue,
    ) -> Self {
        Self {
            variant_index,
            variant_name: variant_name.into(),
            payload,
        }
    }
}

/// Runtime payload value for a serde enum variant.
#[derive(Clone, Debug, PartialEq)]
pub enum EnumPayloadValue {
    Unit,
    Newtype(Box<DynamicValue>),
    Tuple(Vec<DynamicValue>),
    Struct(Vec<DynamicNamedValue>),
}

/// Named field value used by struct enum variants.
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicNamedValue {
    pub name: String,
    pub value: DynamicValue,
}

/// Macro to generate accessor methods for primitive types.
macro_rules! impl_primitive_accessors {
    ($($method:ident -> $variant:ident : $ty:ty),* $(,)?) => {
        impl DynamicValue {
            $(
                #[doc = concat!("Try to extract as ", stringify!($ty), ".")]
                pub fn $method(&self) -> Option<$ty> {
                    match self {
                        DynamicValue::$variant(v) => Some(*v),
                        _ => None,
                    }
                }
            )*
        }
    };
}

impl_primitive_accessors! {
    as_bool -> Bool: bool,
    as_i8 -> Int8: i8,
    as_i16 -> Int16: i16,
    as_i32 -> Int32: i32,
    as_i64 -> Int64: i64,
    as_u8 -> Uint8: u8,
    as_u16 -> Uint16: u16,
    as_u32 -> Uint32: u32,
    as_u64 -> Uint64: u64,
    as_f32 -> Float32: f32,
    as_f64 -> Float64: f64,
}

impl DynamicValue {
    /// Try to extract as a string reference.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            DynamicValue::String(v) => Some(v),
            _ => None,
        }
    }

    /// Try to extract as a byte slice.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            DynamicValue::Bytes(v) => Some(v),
            _ => None,
        }
    }

    /// Try to extract as a struct reference.
    pub fn as_struct(&self) -> Option<&DynamicStruct> {
        match self {
            DynamicValue::Struct(v) => Some(v),
            _ => None,
        }
    }

    /// Try to extract as a mutable struct reference.
    pub fn as_struct_mut(&mut self) -> Option<&mut DynamicStruct> {
        match self {
            DynamicValue::Struct(v) => Some(v),
            _ => None,
        }
    }

    /// Try to extract as an optional reference.
    pub fn as_optional(&self) -> Option<Option<&DynamicValue>> {
        match self {
            DynamicValue::Optional(Some(value)) => Some(Some(value.as_ref())),
            DynamicValue::Optional(None) => Some(None),
            _ => None,
        }
    }

    /// Try to extract as an enum reference.
    pub fn as_enum(&self) -> Option<&EnumValue> {
        match self {
            DynamicValue::Enum(value) => Some(value),
            _ => None,
        }
    }

    /// Try to extract as a sequence reference.
    pub fn as_sequence(&self) -> Option<&[DynamicValue]> {
        match self {
            DynamicValue::Sequence(v) => Some(v),
            _ => None,
        }
    }

    /// Try to extract as a mutable sequence reference.
    pub fn as_sequence_mut(&mut self) -> Option<&mut Vec<DynamicValue>> {
        match self {
            DynamicValue::Sequence(v) => Some(v),
            _ => None,
        }
    }

    /// Try to extract as map entries.
    pub fn as_map(&self) -> Option<&[(DynamicValue, DynamicValue)]> {
        match self {
            DynamicValue::Map(v) => Some(v),
            _ => None,
        }
    }

    /// Try to extract as mutable map entries.
    pub fn as_map_mut(&mut self) -> Option<&mut Vec<(DynamicValue, DynamicValue)>> {
        match self {
            DynamicValue::Map(v) => Some(v),
            _ => None,
        }
    }

    /// Check if this value is a primitive type.
    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            DynamicValue::Bool(_)
                | DynamicValue::Int8(_)
                | DynamicValue::Int16(_)
                | DynamicValue::Int32(_)
                | DynamicValue::Int64(_)
                | DynamicValue::Uint8(_)
                | DynamicValue::Uint16(_)
                | DynamicValue::Uint32(_)
                | DynamicValue::Uint64(_)
                | DynamicValue::Float32(_)
                | DynamicValue::Float64(_)
                | DynamicValue::String(_)
        )
    }

    pub fn validate_against(&self, schema: &Schema) -> Result<(), DynamicError> {
        self.validate_against_shape(&schema.root, schema)
    }

    pub(crate) fn validate_against_shape(
        &self,
        shape: &TypeDef,
        schema: &Schema,
    ) -> Result<(), DynamicError> {
        match (self, shape) {
            (Self::Bool(_), TypeDef::Primitive(PrimitiveTypeDef::Bool))
            | (Self::Int8(_), TypeDef::Primitive(PrimitiveTypeDef::I8))
            | (Self::Uint8(_), TypeDef::Primitive(PrimitiveTypeDef::U8))
            | (Self::Int16(_), TypeDef::Primitive(PrimitiveTypeDef::I16))
            | (Self::Uint16(_), TypeDef::Primitive(PrimitiveTypeDef::U16))
            | (Self::Int32(_), TypeDef::Primitive(PrimitiveTypeDef::I32))
            | (Self::Uint32(_), TypeDef::Primitive(PrimitiveTypeDef::U32))
            | (Self::Int64(_), TypeDef::Primitive(PrimitiveTypeDef::I64))
            | (Self::Uint64(_), TypeDef::Primitive(PrimitiveTypeDef::U64))
            | (Self::Float32(_), TypeDef::Primitive(PrimitiveTypeDef::F32))
            | (Self::Float64(_), TypeDef::Primitive(PrimitiveTypeDef::F64))
            | (Self::String(_), TypeDef::String) => Ok(()),
            (Self::Bytes(bytes), TypeDef::Sequence { element, length })
                if matches!(element.as_ref(), TypeDef::Primitive(PrimitiveTypeDef::U8)) =>
            {
                validate_sequence_len(bytes.len(), length)?;
                Ok(())
            }
            (Self::Optional(value), TypeDef::Optional(element)) => {
                if let Some(value) = value {
                    value.validate_against_shape(element, schema)
                } else {
                    Ok(())
                }
            }
            (Self::Sequence(values), TypeDef::Sequence { element, length }) => {
                validate_sequence_len(values.len(), length)?;
                for value in values {
                    value.validate_against_shape(element, schema)?;
                }
                Ok(())
            }
            (Self::Struct(value), TypeDef::Named(name)) => match schema.definitions.get(name) {
                Some(TypeDefinition::Struct(definition)) => {
                    if value.type_name() != name {
                        return Err(DynamicError::SerializationError(format!(
                            "struct type mismatch: expected {name}, got {}",
                            value.type_name()
                        )));
                    }
                    value.validate_fields(&definition.fields, schema)
                }
                _ => Err(DynamicError::SerializationError(format!(
                    "named struct definition {name} not found"
                ))),
            },
            (Self::Map(entries), TypeDef::Map { key, value }) => {
                for (entry_key, entry_value) in entries {
                    entry_key.validate_against_shape(key, schema)?;
                    entry_value.validate_against_shape(value, schema)?;
                }
                Ok(())
            }
            (Self::Enum(value), TypeDef::Named(name)) => match schema.definitions.get(name) {
                Some(TypeDefinition::Enum(definition)) => {
                    let variant = definition
                        .variants
                        .get(value.variant_index as usize)
                        .ok_or_else(|| {
                            DynamicError::SerializationError(format!(
                                "enum variant index {} is out of bounds",
                                value.variant_index
                            ))
                        })?;
                    if value.variant_name != variant.name {
                        return Err(DynamicError::SerializationError(format!(
                            "enum variant name mismatch: expected {}, got {}",
                            variant.name, value.variant_name
                        )));
                    }
                    validate_enum_payload(&value.payload, &variant.payload, schema)
                }
                _ => Err(DynamicError::SerializationError(format!(
                    "named enum definition {name} not found"
                ))),
            },
            _ => Err(DynamicError::SerializationError(
                "dynamic value does not match schema".into(),
            )),
        }
    }
}

fn validate_sequence_len(actual: usize, length: &SequenceLengthDef) -> Result<(), DynamicError> {
    if let SequenceLengthDef::Fixed(expected) = length
        && actual != *expected
    {
        return Err(DynamicError::SerializationError(format!(
            "fixed sequence expected {expected} values, got {actual}"
        )));
    }
    Ok(())
}

fn validate_enum_payload(
    value: &EnumPayloadValue,
    schema: &EnumPayloadDef,
    bundle: &Schema,
) -> Result<(), DynamicError> {
    match (value, schema) {
        (EnumPayloadValue::Unit, EnumPayloadDef::Unit) => Ok(()),
        (EnumPayloadValue::Newtype(value), EnumPayloadDef::Newtype(schema)) => {
            value.validate_against_shape(schema, bundle)
        }
        (EnumPayloadValue::Tuple(values), EnumPayloadDef::Tuple(schemas)) => {
            if values.len() != schemas.len() {
                return Err(DynamicError::SerializationError(format!(
                    "enum tuple payload length mismatch: expected {}, got {}",
                    schemas.len(),
                    values.len()
                )));
            }
            for (value, schema) in values.iter().zip(schemas.iter()) {
                value.validate_against_shape(schema, bundle)?;
            }
            Ok(())
        }
        (EnumPayloadValue::Struct(values), EnumPayloadDef::Struct(fields)) => {
            if values.len() != fields.len() {
                return Err(DynamicError::SerializationError(format!(
                    "enum struct payload length mismatch: expected {}, got {}",
                    fields.len(),
                    values.len()
                )));
            }
            for (value, field) in values.iter().zip(fields.iter()) {
                if value.name != field.name {
                    return Err(DynamicError::SerializationError(format!(
                        "enum struct payload field mismatch: expected {}, got {}",
                        field.name, value.name
                    )));
                }
                value.value.validate_against_shape(&field.shape, bundle)?;
            }
            Ok(())
        }
        _ => Err(DynamicError::SerializationError(
            "enum payload mismatch".into(),
        )),
    }
}

/// Trait for types that can be converted to DynamicValue.
pub trait IntoDynamic {
    fn into_dynamic(self) -> DynamicValue;
}

/// Trait for types that can be extracted from DynamicValue.
pub trait FromDynamic: Sized {
    fn from_dynamic(value: &DynamicValue) -> Option<Self>;
}

/// Macro to implement IntoDynamic and FromDynamic for primitive types.
macro_rules! impl_dynamic_conversions {
    ($($ty:ty => $variant:ident, $accessor:ident);* $(;)?) => {
        $(
            impl IntoDynamic for $ty {
                fn into_dynamic(self) -> DynamicValue {
                    DynamicValue::$variant(self)
                }
            }

            impl FromDynamic for $ty {
                fn from_dynamic(v: &DynamicValue) -> Option<Self> {
                    v.$accessor()
                }
            }
        )*
    };
}

impl_dynamic_conversions! {
    bool => Bool, as_bool;
    i8 => Int8, as_i8;
    i16 => Int16, as_i16;
    i32 => Int32, as_i32;
    i64 => Int64, as_i64;
    u8 => Uint8, as_u8;
    u16 => Uint16, as_u16;
    u32 => Uint32, as_u32;
    u64 => Uint64, as_u64;
    f32 => Float32, as_f32;
    f64 => Float64, as_f64;
}

impl IntoDynamic for String {
    fn into_dynamic(self) -> DynamicValue {
        DynamicValue::String(self)
    }
}

impl IntoDynamic for &str {
    fn into_dynamic(self) -> DynamicValue {
        DynamicValue::String(self.to_string())
    }
}

impl FromDynamic for String {
    fn from_dynamic(v: &DynamicValue) -> Option<Self> {
        v.as_str().map(|s| s.to_string())
    }
}

// Note: Vec<u8> uses DynamicValue::Bytes via the special-cased serialization,
// but for generic Vec<T> we use DynamicValue::Sequence. The Bytes variant is for
// optimized byte array handling in serialization.

impl IntoDynamic for DynamicStruct {
    fn into_dynamic(self) -> DynamicValue {
        DynamicValue::Struct(Box::new(self))
    }
}

impl<T: IntoDynamic> IntoDynamic for Vec<T> {
    fn into_dynamic(self) -> DynamicValue {
        DynamicValue::Sequence(self.into_iter().map(|v| v.into_dynamic()).collect())
    }
}

impl<T: IntoDynamic> IntoDynamic for Option<T> {
    fn into_dynamic(self) -> DynamicValue {
        DynamicValue::Optional(self.map(|value| Box::new(value.into_dynamic())))
    }
}

impl<T: FromDynamic> FromDynamic for Option<T> {
    fn from_dynamic(value: &DynamicValue) -> Option<Self> {
        match value {
            DynamicValue::Optional(None) => Some(None),
            DynamicValue::Optional(Some(inner)) => T::from_dynamic(inner.as_ref()).map(Some),
            _ => None,
        }
    }
}

impl IntoDynamic for EnumValue {
    fn into_dynamic(self) -> DynamicValue {
        DynamicValue::Enum(self)
    }
}

/// Create the default value for a given schema shape.
pub fn default_for_schema(schema: &Schema) -> Result<DynamicValue, DynamicError> {
    let mut active = BTreeSet::new();
    default_for_shape_with_active(&schema.root, schema, &mut active)
}

pub(crate) fn default_for_shape(
    shape: &TypeDef,
    schema: &Schema,
) -> Result<DynamicValue, DynamicError> {
    let mut active = BTreeSet::new();
    default_for_shape_with_active(shape, schema, &mut active)
}

pub(crate) fn default_for_shape_with_active(
    shape: &TypeDef,
    schema: &Schema,
    active: &mut BTreeSet<TypeName>,
) -> Result<DynamicValue, DynamicError> {
    match shape {
        TypeDef::Primitive(PrimitiveTypeDef::Bool) => Ok(DynamicValue::Bool(false)),
        TypeDef::Primitive(PrimitiveTypeDef::I8) => Ok(DynamicValue::Int8(0)),
        TypeDef::Primitive(PrimitiveTypeDef::U8) => Ok(DynamicValue::Uint8(0)),
        TypeDef::Primitive(PrimitiveTypeDef::I16) => Ok(DynamicValue::Int16(0)),
        TypeDef::Primitive(PrimitiveTypeDef::U16) => Ok(DynamicValue::Uint16(0)),
        TypeDef::Primitive(PrimitiveTypeDef::I32) => Ok(DynamicValue::Int32(0)),
        TypeDef::Primitive(PrimitiveTypeDef::U32) => Ok(DynamicValue::Uint32(0)),
        TypeDef::Primitive(PrimitiveTypeDef::I64) => Ok(DynamicValue::Int64(0)),
        TypeDef::Primitive(PrimitiveTypeDef::U64) => Ok(DynamicValue::Uint64(0)),
        TypeDef::Primitive(PrimitiveTypeDef::F32) => Ok(DynamicValue::Float32(0.0)),
        TypeDef::Primitive(PrimitiveTypeDef::F64) => Ok(DynamicValue::Float64(0.0)),
        TypeDef::String => Ok(DynamicValue::String(String::new())),
        TypeDef::Named(name) => match schema.definitions.get(name) {
            Some(TypeDefinition::Struct(_)) => {
                if !active.insert(name.clone()) {
                    return Err(DynamicError::InvalidDefaultValue {
                        field: name.as_str().to_string(),
                        reason: "required recursive field has no finite default".to_string(),
                    });
                }
                let result = DynamicStruct::try_new_with_active(schema, name, active)
                    .map(|value| DynamicValue::Struct(Box::new(value)));
                active.remove(name);
                result
            }
            Some(TypeDefinition::Enum(definition)) => {
                if !active.insert(name.clone()) {
                    return Err(DynamicError::InvalidDefaultValue {
                        field: name.as_str().to_string(),
                        reason: "required recursive enum variant has no finite default".to_string(),
                    });
                }
                let result = default_enum_value_with_bundle(
                    name.as_str(),
                    &definition.variants,
                    schema,
                    active,
                )
                .map(DynamicValue::Enum);
                active.remove(name);
                result
            }
            None => Err(DynamicError::SerializationError(format!(
                "named definition {name} not found"
            ))),
        },
        TypeDef::Optional(_) => Ok(DynamicValue::Optional(None)),
        TypeDef::Sequence { element, length } => match length {
            SequenceLengthDef::Dynamic => Ok(DynamicValue::Sequence(Vec::new())),
            SequenceLengthDef::Fixed(0) => Ok(DynamicValue::Sequence(Vec::new())),
            SequenceLengthDef::Fixed(len) => Ok(DynamicValue::Sequence(
                (0..*len)
                    .map(|_| default_for_shape_with_active(element, schema, active))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
        },
        TypeDef::Map { .. } => Ok(DynamicValue::Map(Vec::new())),
    }
}

pub fn try_default_for_schema(schema: &Schema) -> Result<DynamicValue, DynamicError> {
    default_for_schema(schema)
}

fn default_enum_payload_with_bundle(
    payload: &EnumPayloadDef,
    schema: &Schema,
    active: &mut BTreeSet<TypeName>,
) -> Result<EnumPayloadValue, DynamicError> {
    match payload {
        EnumPayloadDef::Unit => Ok(EnumPayloadValue::Unit),
        EnumPayloadDef::Newtype(shape) => Ok(EnumPayloadValue::Newtype(Box::new(
            default_for_shape_with_active(shape, schema, active)?,
        ))),
        EnumPayloadDef::Tuple(shapes) => Ok(EnumPayloadValue::Tuple(
            shapes
                .iter()
                .map(|shape| default_for_shape_with_active(shape, schema, active))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        EnumPayloadDef::Struct(fields) => Ok(EnumPayloadValue::Struct(
            fields
                .iter()
                .map(|field| {
                    Ok(DynamicNamedValue {
                        name: field.name.clone(),
                        value: default_for_shape_with_active(&field.shape, schema, active)?,
                    })
                })
                .collect::<Result<Vec<_>, DynamicError>>()?,
        )),
    }
}

fn default_enum_value_with_bundle(
    enum_name: &str,
    variants: &[EnumVariantDef],
    schema: &Schema,
    active: &mut BTreeSet<TypeName>,
) -> Result<EnumValue, DynamicError> {
    let Some(variant) = variants.first() else {
        return Err(DynamicError::InvalidDefaultValue {
            field: enum_name.to_string(),
            reason: "empty enum schema has no default variant".to_string(),
        });
    };

    Ok(EnumValue {
        variant_index: 0,
        variant_name: variant.name.clone(),
        payload: default_enum_payload_with_bundle(&variant.payload, schema, active)?,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ros_z_schema::{
        EnumDef, FieldDef, SchemaBundle, SequenceLengthDef, StructDef, TypeDefinition, TypeName,
    };

    use super::*;

    #[test]
    fn try_default_for_schema_rejects_empty_runtime_enum_without_panicking() {
        let type_name = TypeName::new("test_msgs::Empty").unwrap();
        let schema = Arc::new(SchemaBundle {
            root: TypeDef::Named(type_name.clone()),
            definitions: [(
                type_name,
                TypeDefinition::Enum(EnumDef { variants: vec![] }),
            )]
            .into(),
        });

        let error = try_default_for_schema(&schema).unwrap_err();

        assert!(error.to_string().contains("empty enum"));
        assert!(error.to_string().contains("test_msgs::Empty"));
    }

    #[test]
    fn required_recursive_struct_default_returns_error() {
        let type_name = TypeName::new("test_msgs::Node").unwrap();
        let schema = Arc::new(SchemaBundle {
            root: TypeDef::Named(type_name.clone()),
            definitions: [(
                type_name.clone(),
                TypeDefinition::Struct(StructDef {
                    fields: vec![FieldDef::new("child", TypeDef::Named(type_name))],
                }),
            )]
            .into(),
        });

        let error = try_default_for_schema(&schema).unwrap_err();

        assert!(matches!(error, DynamicError::InvalidDefaultValue { .. }));
        assert!(error.to_string().contains("required recursive field"));
    }

    #[test]
    fn finite_recursive_boundaries_default_successfully() {
        let type_name = TypeName::new("test_msgs::Node").unwrap();
        let schema = Arc::new(SchemaBundle {
            root: TypeDef::Named(type_name.clone()),
            definitions: [(
                type_name.clone(),
                TypeDefinition::Struct(StructDef {
                    fields: vec![
                        FieldDef::new(
                            "optional_child",
                            TypeDef::Optional(Box::new(TypeDef::Named(type_name.clone()))),
                        ),
                        FieldDef::new(
                            "children",
                            TypeDef::Sequence {
                                element: Box::new(TypeDef::Named(type_name.clone())),
                                length: SequenceLengthDef::Dynamic,
                            },
                        ),
                        FieldDef::new(
                            "empty_children",
                            TypeDef::Sequence {
                                element: Box::new(TypeDef::Named(type_name.clone())),
                                length: SequenceLengthDef::Fixed(0),
                            },
                        ),
                        FieldDef::new(
                            "child_by_name",
                            TypeDef::Map {
                                key: Box::new(TypeDef::String),
                                value: Box::new(TypeDef::Named(type_name)),
                            },
                        ),
                    ],
                }),
            )]
            .into(),
        });

        let value = try_default_for_schema(&schema).unwrap();

        let DynamicValue::Struct(message) = value else {
            panic!("expected struct default");
        };
        assert_eq!(
            message.get_dynamic("optional_child").unwrap(),
            DynamicValue::Optional(None)
        );
        assert_eq!(
            message.get_dynamic("children").unwrap(),
            DynamicValue::Sequence(Vec::new())
        );
        assert_eq!(
            message.get_dynamic("empty_children").unwrap(),
            DynamicValue::Sequence(Vec::new())
        );
        assert_eq!(
            message.get_dynamic("child_by_name").unwrap(),
            DynamicValue::Map(Vec::new())
        );
    }
}
