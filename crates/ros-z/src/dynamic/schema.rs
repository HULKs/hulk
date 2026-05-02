//! Schema types for dynamic CDR-backed messages.
//!
//! This module provides runtime representations of message types,
//! including field types, field schemas, and complete message schemas.

use std::sync::Arc;

use super::value::DynamicValue;
use ros_z_schema::TypeName;

/// Runtime schema tree for dynamic root and field shapes.
pub type Schema = Arc<TypeShape>;

/// Recursive runtime representation of Rust-native schema shapes.
#[derive(Clone, Debug, PartialEq)]
pub enum TypeShape {
    Struct {
        name: TypeName,
        fields: Vec<FieldSchema>,
    },
    Enum {
        name: TypeName,
        variants: Vec<RuntimeDynamicEnumVariant>,
    },
    Primitive(PrimitiveType),
    String,
    Optional(Schema),
    Sequence {
        element: Schema,
        length: SequenceLength,
    },
    Map {
        key: Schema,
        value: Schema,
    },
}

/// Rust-native primitive runtime schema types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PrimitiveType {
    Bool,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
}

/// Dynamic or fixed sequence length semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SequenceLength {
    Dynamic,
    Fixed(usize),
}

/// Schema for a field in the runtime schema tree.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldSchema {
    pub name: String,
    pub schema: Schema,
    pub default_value: Option<DynamicValue>,
}

impl FieldSchema {
    pub fn new(name: impl Into<String>, schema: Schema) -> Self {
        Self {
            name: name.into(),
            schema,
            default_value: None,
        }
    }

    pub fn with_default(mut self, value: DynamicValue) -> Self {
        self.default_value = Some(value);
        self
    }
}

/// Schema for a runtime enum variant.
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeDynamicEnumVariant {
    pub name: String,
    pub payload: RuntimeDynamicEnumPayload,
}

impl RuntimeDynamicEnumVariant {
    pub fn new(name: impl Into<String>, payload: RuntimeDynamicEnumPayload) -> Self {
        Self {
            name: name.into(),
            payload,
        }
    }
}

/// Runtime enum variant payload schemas using the recursive schema tree.
#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeDynamicEnumPayload {
    Unit,
    Newtype(Schema),
    Tuple(Vec<Schema>),
    Struct(Vec<FieldSchema>),
}
