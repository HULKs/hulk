use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A fully-qualified Rust type path such as `crate_name::module::Message`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TypeName(String);

impl TypeName {
    /// Creates a validated type name.
    pub fn new(value: impl Into<String>) -> Result<Self, SchemaError> {
        let value = value.into();

        if !is_valid_rust_type_path(&value) {
            return Err(SchemaError::InvalidTypeName(value));
        }

        Ok(Self(value))
    }

    /// Returns the underlying fully-qualified type name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_valid_rust_type_path(value: &str) -> bool {
    !value.is_empty()
        && !value.contains(['/', '<', '>'])
        && !value.chars().any(char::is_whitespace)
        && value.split("::").all(is_valid_rust_identifier)
}

fn is_valid_rust_identifier(value: &str) -> bool {
    let (value, is_raw) = value
        .strip_prefix("r#")
        .map_or((value, false), |value| (value, true));
    if value == "_" {
        return false;
    }
    let mut chars = value.chars();
    matches!(chars.next(), Some(ch) if ch == '_' || ch.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        && if is_raw {
            !is_forbidden_raw_identifier(value)
        } else {
            !is_rust_keyword(value)
        }
}

fn is_forbidden_raw_identifier(value: &str) -> bool {
    matches!(value, "Self" | "self" | "super" | "crate")
}

fn is_rust_keyword(value: &str) -> bool {
    matches!(
        value,
        "as" | "async"
            | "await"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "try"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "macro_rules"
            | "union"
    )
}

impl AsRef<str> for TypeName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for TypeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Errors produced while building or validating schemas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    /// A type name does not match the native Rust path shape.
    InvalidTypeName(String),
    /// The bundle root is not present in the definition map.
    MissingRoot(String),
    /// A named field references a type that is not present in the bundle.
    MissingDefinition(TypeName),
    /// A primitive field shape uses an unknown spelling.
    InvalidPrimitiveName(String),
    /// A field default does not match the field shape.
    InvalidFieldDefault {
        /// The field name.
        field_name: String,
        /// The field shape kind.
        shape: String,
        /// The default literal kind.
        default: String,
    },
    /// A map key shape is not supported by the canonical schema model.
    UnsupportedMapKeyShape(String),
    /// A public literal value cannot be serialized into JSON.
    InvalidLiteralValue(String),
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTypeName(value) => {
                write!(
                    f,
                    "invalid type name `{value}`; expected a non-empty Rust type path"
                )
            }
            Self::MissingRoot(value) => write!(f, "missing root definition for `{value}`"),
            Self::MissingDefinition(type_name) => {
                write!(f, "missing definition for named reference `{type_name}`")
            }
            Self::InvalidPrimitiveName(name) => {
                write!(f, "invalid primitive name `{name}`")
            }
            Self::InvalidFieldDefault {
                field_name,
                shape,
                default,
            } => write!(
                f,
                "field `{field_name}` has invalid default `{default}` for shape `{shape}`"
            ),
            Self::UnsupportedMapKeyShape(shape) => {
                write!(f, "unsupported map key shape `{shape}`")
            }
            Self::InvalidLiteralValue(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for SchemaError {}

/// A schema bundle keyed by validated type names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaBundle {
    /// The schema entry point.
    pub root: TypeName,
    /// All reachable type definitions keyed by type name.
    pub definitions: BTreeMap<TypeName, TypeDef>,
}

impl SchemaBundle {
    /// Creates a builder for a schema bundle.
    pub fn builder(root: impl Into<String>) -> SchemaBundleBuilder {
        SchemaBundleBuilder::new(root)
    }

    /// Validates that the bundle root exists and that every named field is closed over the bundle.
    pub fn validate(&self) -> Result<(), SchemaError> {
        if !self.definitions.contains_key(&self.root) {
            return Err(SchemaError::MissingRoot(self.root.to_string()));
        }

        for definition in self.definitions.values() {
            definition.validate_references(&self.definitions)?;
        }

        Ok(())
    }

    /// Returns the root definition map.
    pub fn definitions(&self) -> &BTreeMap<TypeName, TypeDef> {
        &self.definitions
    }
}

/// Builder for [`SchemaBundle`].
#[derive(Debug, Clone)]
pub struct SchemaBundleBuilder {
    root: Option<TypeName>,
    definitions: BTreeMap<TypeName, TypeDef>,
    error: Option<SchemaError>,
}

impl SchemaBundleBuilder {
    fn new(root: impl Into<String>) -> Self {
        match TypeName::new(root) {
            Ok(root) => Self {
                root: Some(root),
                definitions: BTreeMap::new(),
                error: None,
            },
            Err(error) => Self {
                root: None,
                definitions: BTreeMap::new(),
                error: Some(error),
            },
        }
    }

    /// Adds or replaces a definition in the bundle.
    pub fn definition(mut self, type_name: impl Into<String>, definition: TypeDef) -> Self {
        match TypeName::new(type_name) {
            Ok(type_name) => {
                self.definitions.insert(type_name, definition);
            }
            Err(error) if self.error.is_none() => {
                self.error = Some(error);
            }
            Err(_) => {}
        }

        self
    }

    /// Builds a bundle and validates reference closure.
    pub fn build(self) -> Result<SchemaBundle, SchemaError> {
        let bundle = self.try_build_unchecked()?;
        bundle.validate()?;
        Ok(bundle)
    }

    /// Builds a bundle without validating reference closure.
    pub fn build_unchecked(self) -> SchemaBundle {
        match self.try_build_unchecked() {
            Ok(bundle) => bundle,
            Err(error) => panic!("{error}"),
        }
    }

    fn try_build_unchecked(self) -> Result<SchemaBundle, SchemaError> {
        if let Some(error) = self.error {
            return Err(error);
        }

        let Some(root) = self.root else {
            return Err(SchemaError::InvalidTypeName(String::new()));
        };

        Ok(SchemaBundle {
            root,
            definitions: self.definitions,
        })
    }
}

/// A named type definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeDef {
    /// A record-like type with named fields.
    Struct(StructDef),
    /// A tagged enum with explicit variant payload semantics.
    Enum(EnumDef),
}

impl TypeDef {
    fn validate_references(
        &self,
        definitions: &BTreeMap<TypeName, TypeDef>,
    ) -> Result<(), SchemaError> {
        match self {
            Self::Struct(definition) => definition.validate_references(definitions),
            Self::Enum(definition) => definition.validate_references(definitions),
        }
    }
}

/// A struct definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructDef {
    /// The struct fields in declaration order.
    pub fields: Vec<FieldDef>,
}

impl StructDef {
    fn validate_references(
        &self,
        definitions: &BTreeMap<TypeName, TypeDef>,
    ) -> Result<(), SchemaError> {
        for field in &self.fields {
            field.validate_references(definitions)?;
        }

        Ok(())
    }
}

/// An enum definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnumDef {
    /// The enum variants in declaration order.
    pub variants: Vec<EnumVariantDef>,
}

impl EnumDef {
    fn validate_references(
        &self,
        definitions: &BTreeMap<TypeName, TypeDef>,
    ) -> Result<(), SchemaError> {
        for variant in &self.variants {
            variant.validate_references(definitions)?;
        }

        Ok(())
    }
}

/// An enum variant definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnumVariantDef {
    /// The variant name.
    pub name: String,
    /// The variant payload shape.
    pub payload: EnumPayloadDef,
}

impl EnumVariantDef {
    /// Creates an enum variant definition.
    pub fn new(name: impl Into<String>, payload: EnumPayloadDef) -> Self {
        Self {
            name: name.into(),
            payload,
        }
    }

    fn validate_references(
        &self,
        definitions: &BTreeMap<TypeName, TypeDef>,
    ) -> Result<(), SchemaError> {
        self.payload.validate_references(definitions)
    }
}

/// An enum payload definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnumPayloadDef {
    /// A unit variant with no payload.
    Unit,
    /// A newtype variant carrying a single unnamed field.
    Newtype(FieldShape),
    /// A tuple variant carrying ordered unnamed fields.
    Tuple(Vec<FieldShape>),
    /// A struct variant carrying named fields.
    Struct(Vec<FieldDef>),
}

impl EnumPayloadDef {
    fn validate_references(
        &self,
        definitions: &BTreeMap<TypeName, TypeDef>,
    ) -> Result<(), SchemaError> {
        match self {
            Self::Unit => Ok(()),
            Self::Newtype(shape) => shape.validate_references(definitions),
            Self::Tuple(shapes) => {
                for shape in shapes {
                    shape.validate_references(definitions)?;
                }
                Ok(())
            }
            Self::Struct(fields) => {
                for field in fields {
                    field.validate_references(definitions)?;
                }
                Ok(())
            }
        }
    }
}

/// A field definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDef {
    /// The field name.
    pub name: String,
    /// The field shape.
    pub shape: FieldShape,
    /// An optional default value.
    pub default: Option<LiteralValue>,
}

impl FieldDef {
    /// Creates a field without a default value.
    pub fn new(name: impl Into<String>, shape: FieldShape) -> Self {
        Self {
            name: name.into(),
            shape,
            default: None,
        }
    }

    /// Attaches a default value to the field.
    pub fn with_default(mut self, default: LiteralValue) -> Self {
        self.default = Some(default);
        self
    }

    fn validate_references(
        &self,
        definitions: &BTreeMap<TypeName, TypeDef>,
    ) -> Result<(), SchemaError> {
        self.shape.validate_references(definitions)?;
        self.validate_default()
    }

    fn validate_default(&self) -> Result<(), SchemaError> {
        let Some(default) = &self.default else {
            return Ok(());
        };

        if default_matches_shape(default, &self.shape)
            && default_matches_shape_constraints(default, &self.shape)
        {
            return Ok(());
        }

        Err(SchemaError::InvalidFieldDefault {
            field_name: self.name.clone(),
            shape: self.shape.kind_name().to_string(),
            default: default.kind_name().to_string(),
        })
    }
}

/// A Rust-native primitive field type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FieldPrimitive {
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
}

impl FieldPrimitive {
    /// Returns the Rust-native spelling for this primitive.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::F32 => "f32",
            Self::F64 => "f64",
        }
    }

    /// Converts a ROS `.msg` primitive name at the import boundary.
    pub fn from_ros_name(name: &str) -> Option<Self> {
        Some(match name {
            "bool" => Self::Bool,
            "int8" => Self::I8,
            "int16" => Self::I16,
            "int32" => Self::I32,
            "int64" => Self::I64,
            "byte" | "char" | "uint8" => Self::U8,
            "uint16" => Self::U16,
            "uint32" => Self::U32,
            "uint64" => Self::U64,
            "float32" => Self::F32,
            "float64" => Self::F64,
            _ => return None,
        })
    }
}

impl std::str::FromStr for FieldPrimitive {
    type Err = SchemaError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "bool" => Ok(Self::Bool),
            "i8" => Ok(Self::I8),
            "i16" => Ok(Self::I16),
            "i32" => Ok(Self::I32),
            "i64" => Ok(Self::I64),
            "u8" => Ok(Self::U8),
            "u16" => Ok(Self::U16),
            "u32" => Ok(Self::U32),
            "u64" => Ok(Self::U64),
            "f32" => Ok(Self::F32),
            "f64" => Ok(Self::F64),
            other => Err(SchemaError::InvalidPrimitiveName(other.to_string())),
        }
    }
}

impl Serialize for FieldPrimitive {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for FieldPrimitive {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FieldPrimitiveVisitor;

        impl Visitor<'_> for FieldPrimitiveVisitor {
            type Value = FieldPrimitive;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a Rust-native primitive name")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                value.parse().map_err(E::custom)
            }
        }

        deserializer.deserialize_str(FieldPrimitiveVisitor)
    }
}

/// The shape of a field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldShape {
    /// A Rust-native primitive field type.
    Primitive(FieldPrimitive),
    /// A UTF-8 string field.
    String,
    /// A bounded UTF-8 string field.
    BoundedString { maximum_length: usize },
    /// A reference to another named type in the bundle.
    Named(TypeName),
    /// An optional field with an inner field shape.
    Optional { element: Box<FieldShape> },
    /// A fixed-size array field.
    Array {
        element: Box<FieldShape>,
        length: usize,
    },
    /// An unbounded sequence field.
    Sequence { element: Box<FieldShape> },
    /// A bounded sequence field.
    BoundedSequence {
        element: Box<FieldShape>,
        maximum_length: usize,
    },
    /// A map field with key and value shapes.
    Map {
        key: Box<FieldShape>,
        value: Box<FieldShape>,
    },
}

impl FieldShape {
    fn validate_references(
        &self,
        definitions: &BTreeMap<TypeName, TypeDef>,
    ) -> Result<(), SchemaError> {
        match self {
            Self::Primitive(_) | Self::String | Self::BoundedString { .. } => Ok(()),
            Self::Named(type_name) if !definitions.contains_key(type_name) => {
                Err(SchemaError::MissingDefinition(type_name.clone()))
            }
            Self::Named(_) => Ok(()),
            Self::Optional { element } | Self::Sequence { element } => {
                element.validate_references(definitions)
            }
            Self::Array { element, .. } | Self::BoundedSequence { element, .. } => {
                element.validate_references(definitions)
            }
            Self::Map { key, value } => {
                key.validate_references(definitions)?;
                validate_map_key_shape(key)?;
                value.validate_references(definitions)
            }
        }
    }

    fn kind_name(&self) -> &'static str {
        match self {
            Self::Primitive(_) => "primitive",
            Self::String => "string",
            Self::BoundedString { .. } => "bounded_string",
            Self::Named(_) => "named",
            Self::Optional { .. } => "optional",
            Self::Array { .. } => "array",
            Self::Sequence { .. } => "sequence",
            Self::BoundedSequence { .. } => "bounded_sequence",
            Self::Map { .. } => "map",
        }
    }
}

fn validate_map_key_shape(shape: &FieldShape) -> Result<(), SchemaError> {
    match shape {
        FieldShape::String => Ok(()),
        FieldShape::Primitive(
            FieldPrimitive::Bool
            | FieldPrimitive::I8
            | FieldPrimitive::I16
            | FieldPrimitive::I32
            | FieldPrimitive::I64
            | FieldPrimitive::U8
            | FieldPrimitive::U16
            | FieldPrimitive::U32
            | FieldPrimitive::U64,
        ) => Ok(()),
        _ => Err(SchemaError::UnsupportedMapKeyShape(
            shape.kind_name().to_string(),
        )),
    }
}

/// Canonical literal values for field defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LiteralValue {
    /// A boolean value.
    Bool(bool),
    /// A signed integer value.
    Int(i64),
    /// An unsigned integer value.
    UInt(u64),
    /// A 32-bit floating point value.
    Float32(f32),
    /// A 64-bit floating point value.
    Float64(f64),
    /// A UTF-8 string value.
    String(String),
    /// A boolean array value.
    BoolArray(Vec<bool>),
    /// A signed integer array value.
    IntArray(Vec<i64>),
    /// An unsigned integer array value.
    UIntArray(Vec<u64>),
    /// A 32-bit floating point array value.
    Float32Array(Vec<f32>),
    /// A 64-bit floating point array value.
    Float64Array(Vec<f64>),
    /// A UTF-8 string array value.
    StringArray(Vec<String>),
}

impl PartialEq for LiteralValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::Int(left), Self::Int(right)) => left == right,
            (Self::UInt(left), Self::UInt(right)) => left == right,
            (Self::Float32(left), Self::Float32(right)) => left.to_bits() == right.to_bits(),
            (Self::Float64(left), Self::Float64(right)) => left.to_bits() == right.to_bits(),
            (Self::String(left), Self::String(right)) => left == right,
            (Self::BoolArray(left), Self::BoolArray(right)) => left == right,
            (Self::IntArray(left), Self::IntArray(right)) => left == right,
            (Self::UIntArray(left), Self::UIntArray(right)) => left == right,
            (Self::Float32Array(left), Self::Float32Array(right)) => left
                .iter()
                .map(|value| value.to_bits())
                .eq(right.iter().map(|value| value.to_bits())),
            (Self::Float64Array(left), Self::Float64Array(right)) => left
                .iter()
                .map(|value| value.to_bits())
                .eq(right.iter().map(|value| value.to_bits())),
            (Self::StringArray(left), Self::StringArray(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for LiteralValue {}

impl LiteralValue {
    fn kind_name(&self) -> &'static str {
        match self {
            Self::Bool(_) => "bool",
            Self::Int(_) => "int",
            Self::UInt(_) => "uint",
            Self::Float32(_) => "float32",
            Self::Float64(_) => "float64",
            Self::String(_) => "string",
            Self::BoolArray(_) => "bool[]",
            Self::IntArray(_) => "int[]",
            Self::UIntArray(_) => "uint[]",
            Self::Float32Array(_) => "float32[]",
            Self::Float64Array(_) => "float64[]",
            Self::StringArray(_) => "string[]",
        }
    }
}

fn default_matches_shape(default: &LiteralValue, shape: &FieldShape) -> bool {
    match shape {
        FieldShape::String | FieldShape::BoundedString { .. } => {
            matches!(default, LiteralValue::String(_))
        }
        FieldShape::Named(_) | FieldShape::Optional { .. } | FieldShape::Map { .. } => false,
        FieldShape::Array { element, .. }
        | FieldShape::Sequence { element }
        | FieldShape::BoundedSequence { element, .. } => {
            array_default_matches_shape(default, element)
        }
        FieldShape::Primitive(primitive) => primitive_accepts_default(*primitive, default),
    }
}

fn default_matches_shape_constraints(default: &LiteralValue, shape: &FieldShape) -> bool {
    match shape {
        FieldShape::String => matches!(default, LiteralValue::String(_)),
        FieldShape::BoundedString { maximum_length } => {
            matches!(default, LiteralValue::String(value) if value.len() <= *maximum_length)
        }
        FieldShape::Primitive(primitive) => {
            primitive_default_matches_constraints(*primitive, default)
        }
        FieldShape::Named(_) | FieldShape::Optional { .. } | FieldShape::Map { .. } => false,
        FieldShape::Array { element, length } => {
            array_default_len(default) == Some(*length)
                && array_default_matches_constraints(default, element)
        }
        FieldShape::Sequence { element } => array_default_matches_constraints(default, element),
        FieldShape::BoundedSequence {
            element,
            maximum_length,
        } => {
            array_default_len(default).is_some_and(|len| len <= *maximum_length)
                && array_default_matches_constraints(default, element)
        }
    }
}

fn array_default_matches_shape(default: &LiteralValue, inner: &FieldShape) -> bool {
    match default {
        LiteralValue::BoolArray(_) => {
            matches!(inner.as_ref_primitive(), Some(FieldPrimitive::Bool))
        }
        LiteralValue::IntArray(_) => matches!(
            inner.as_ref_primitive(),
            Some(
                FieldPrimitive::I8
                    | FieldPrimitive::I16
                    | FieldPrimitive::I32
                    | FieldPrimitive::I64
            )
        ),
        LiteralValue::UIntArray(_) => matches!(
            inner.as_ref_primitive(),
            Some(
                FieldPrimitive::U8
                    | FieldPrimitive::U16
                    | FieldPrimitive::U32
                    | FieldPrimitive::U64
            )
        ),
        LiteralValue::Float32Array(values) => {
            values.iter().all(|value| value.is_finite())
                && matches!(inner.as_ref_primitive(), Some(FieldPrimitive::F32))
        }
        LiteralValue::Float64Array(values) => {
            values.iter().all(|value| value.is_finite())
                && matches!(inner.as_ref_primitive(), Some(FieldPrimitive::F64))
        }
        LiteralValue::StringArray(_) => {
            matches!(inner, FieldShape::String | FieldShape::BoundedString { .. })
        }
        _ => false,
    }
}

fn array_default_matches_constraints(default: &LiteralValue, inner: &FieldShape) -> bool {
    match (default, inner) {
        (LiteralValue::StringArray(values), FieldShape::BoundedString { maximum_length }) => {
            values.iter().all(|value| value.len() <= *maximum_length)
        }
        (LiteralValue::StringArray(_), FieldShape::String) => true,
        (LiteralValue::BoolArray(values), FieldShape::Primitive(primitive)) => {
            values.iter().all(|value| {
                primitive_default_matches_constraints(*primitive, &LiteralValue::Bool(*value))
            })
        }
        (LiteralValue::IntArray(values), FieldShape::Primitive(primitive)) => {
            values.iter().all(|value| {
                primitive_default_matches_constraints(*primitive, &LiteralValue::Int(*value))
            })
        }
        (LiteralValue::UIntArray(values), FieldShape::Primitive(primitive)) => {
            values.iter().all(|value| {
                primitive_default_matches_constraints(*primitive, &LiteralValue::UInt(*value))
            })
        }
        (LiteralValue::Float32Array(values), FieldShape::Primitive(primitive)) => {
            values.iter().all(|value| {
                primitive_default_matches_constraints(*primitive, &LiteralValue::Float32(*value))
            })
        }
        (LiteralValue::Float64Array(values), FieldShape::Primitive(primitive)) => {
            values.iter().all(|value| {
                primitive_default_matches_constraints(*primitive, &LiteralValue::Float64(*value))
            })
        }
        _ => false,
    }
}

fn array_default_len(default: &LiteralValue) -> Option<usize> {
    match default {
        LiteralValue::BoolArray(values) => Some(values.len()),
        LiteralValue::IntArray(values) => Some(values.len()),
        LiteralValue::UIntArray(values) => Some(values.len()),
        LiteralValue::Float32Array(values) => Some(values.len()),
        LiteralValue::Float64Array(values) => Some(values.len()),
        LiteralValue::StringArray(values) => Some(values.len()),
        _ => None,
    }
}

trait FieldShapeExt {
    fn as_ref_primitive(&self) -> Option<FieldPrimitive>;
}

impl FieldShapeExt for FieldShape {
    fn as_ref_primitive(&self) -> Option<FieldPrimitive> {
        match self {
            FieldShape::Primitive(primitive) => Some(*primitive),
            _ => None,
        }
    }
}

fn primitive_accepts_default(primitive: FieldPrimitive, default: &LiteralValue) -> bool {
    match primitive {
        FieldPrimitive::Bool => matches!(default, LiteralValue::Bool(_)),
        FieldPrimitive::I8 | FieldPrimitive::I16 | FieldPrimitive::I32 | FieldPrimitive::I64 => {
            matches!(default, LiteralValue::Int(_))
        }
        FieldPrimitive::U8 | FieldPrimitive::U16 | FieldPrimitive::U32 | FieldPrimitive::U64 => {
            matches!(default, LiteralValue::UInt(_))
        }
        FieldPrimitive::F32 => matches!(default, LiteralValue::Float32(value) if value.is_finite()),
        FieldPrimitive::F64 => matches!(default, LiteralValue::Float64(value) if value.is_finite()),
    }
}

fn primitive_default_matches_constraints(
    primitive: FieldPrimitive,
    default: &LiteralValue,
) -> bool {
    match (primitive, default) {
        (FieldPrimitive::Bool, LiteralValue::Bool(_)) => true,
        (FieldPrimitive::I8, LiteralValue::Int(value)) => i8::try_from(*value).is_ok(),
        (FieldPrimitive::I16, LiteralValue::Int(value)) => i16::try_from(*value).is_ok(),
        (FieldPrimitive::I32, LiteralValue::Int(value)) => i32::try_from(*value).is_ok(),
        (FieldPrimitive::I64, LiteralValue::Int(_)) => true,
        (FieldPrimitive::U8, LiteralValue::UInt(value)) => u8::try_from(*value).is_ok(),
        (FieldPrimitive::U16, LiteralValue::UInt(value)) => u16::try_from(*value).is_ok(),
        (FieldPrimitive::U32, LiteralValue::UInt(value)) => u32::try_from(*value).is_ok(),
        (FieldPrimitive::U64, LiteralValue::UInt(_)) => true,
        (FieldPrimitive::F32, LiteralValue::Float32(value)) => value.is_finite(),
        (FieldPrimitive::F64, LiteralValue::Float64(value)) => value.is_finite(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_bounded_string_default_longer_than_bound() {
        let bundle = SchemaBundle::builder("test_msgs::Label")
            .definition(
                "test_msgs::Label",
                TypeDef::Struct(StructDef {
                    fields: vec![
                        FieldDef::new("value", FieldShape::BoundedString { maximum_length: 3 })
                            .with_default(LiteralValue::String("toolong".to_string())),
                    ],
                }),
            )
            .build();

        let err = bundle.unwrap_err();
        assert!(matches!(
            err,
            SchemaError::InvalidFieldDefault { field_name, .. } if field_name == "value"
        ));
    }

    #[test]
    fn reject_fixed_array_default_with_wrong_length() {
        let bundle = SchemaBundle::builder("test_msgs::Bytes")
            .definition(
                "test_msgs::Bytes",
                TypeDef::Struct(StructDef {
                    fields: vec![
                        FieldDef::new(
                            "value",
                            FieldShape::Array {
                                element: Box::new(FieldShape::Primitive(FieldPrimitive::U8)),
                                length: 4,
                            },
                        )
                        .with_default(LiteralValue::UIntArray(vec![1, 2, 3])),
                    ],
                }),
            )
            .build();

        let err = bundle.unwrap_err();
        assert!(matches!(
            err,
            SchemaError::InvalidFieldDefault { field_name, .. } if field_name == "value"
        ));
    }

    #[test]
    fn reject_bounded_sequence_default_longer_than_bound() {
        let bundle = SchemaBundle::builder("test_msgs::Numbers")
            .definition(
                "test_msgs::Numbers",
                TypeDef::Struct(StructDef {
                    fields: vec![
                        FieldDef::new(
                            "value",
                            FieldShape::BoundedSequence {
                                element: Box::new(FieldShape::Primitive(FieldPrimitive::I32)),
                                maximum_length: 2,
                            },
                        )
                        .with_default(LiteralValue::IntArray(vec![1, 2, 3])),
                    ],
                }),
            )
            .build();

        let err = bundle.unwrap_err();
        assert!(matches!(
            err,
            SchemaError::InvalidFieldDefault { field_name, .. } if field_name == "value"
        ));
    }

    #[test]
    fn reject_non_finite_float_array_defaults() {
        let bundle = SchemaBundle::builder("test_msgs::Floats")
            .definition(
                "test_msgs::Floats",
                TypeDef::Struct(StructDef {
                    fields: vec![
                        FieldDef::new(
                            "values",
                            FieldShape::BoundedSequence {
                                element: Box::new(FieldShape::Primitive(FieldPrimitive::F64)),
                                maximum_length: 2,
                            },
                        )
                        .with_default(LiteralValue::Float64Array(vec![1.0, f64::NAN])),
                    ],
                }),
            )
            .build();

        let err = bundle.unwrap_err();
        assert!(matches!(
            err,
            SchemaError::InvalidFieldDefault { field_name, .. } if field_name == "values"
        ));
    }

    #[test]
    fn reject_oversized_bounded_defaults() {
        let string_err = SchemaBundle::builder("test_msgs::Strings")
            .definition(
                "test_msgs::Strings",
                TypeDef::Struct(StructDef {
                    fields: vec![
                        FieldDef::new("name", FieldShape::BoundedString { maximum_length: 3 })
                            .with_default(LiteralValue::String("toolong".into())),
                    ],
                }),
            )
            .build()
            .unwrap_err();
        assert!(matches!(
            string_err,
            SchemaError::InvalidFieldDefault { .. }
        ));

        let array_err = SchemaBundle::builder("test_msgs::Arrays")
            .definition(
                "test_msgs::Arrays",
                TypeDef::Struct(StructDef {
                    fields: vec![
                        FieldDef::new(
                            "values",
                            FieldShape::Array {
                                element: Box::new(FieldShape::Primitive(FieldPrimitive::I32)),
                                length: 2,
                            },
                        )
                        .with_default(LiteralValue::IntArray(vec![1, 2, 3])),
                    ],
                }),
            )
            .build()
            .unwrap_err();
        assert!(matches!(array_err, SchemaError::InvalidFieldDefault { .. }));

        let seq_err = SchemaBundle::builder("test_msgs::Seq")
            .definition(
                "test_msgs::Seq",
                TypeDef::Struct(StructDef {
                    fields: vec![
                        FieldDef::new(
                            "values",
                            FieldShape::BoundedSequence {
                                element: Box::new(FieldShape::Primitive(FieldPrimitive::I32)),
                                maximum_length: 2,
                            },
                        )
                        .with_default(LiteralValue::IntArray(vec![1, 2, 3])),
                    ],
                }),
            )
            .build()
            .unwrap_err();
        assert!(matches!(seq_err, SchemaError::InvalidFieldDefault { .. }));
    }

    #[test]
    fn reject_out_of_range_unsigned_primitive_default() {
        let err = SchemaBundle::builder("test_msgs::Number")
            .definition(
                "test_msgs::Number",
                TypeDef::Struct(StructDef {
                    fields: vec![
                        FieldDef::new("value", FieldShape::Primitive(FieldPrimitive::U8))
                            .with_default(LiteralValue::UInt(300)),
                    ],
                }),
            )
            .build()
            .unwrap_err();

        assert!(matches!(err, SchemaError::InvalidFieldDefault { .. }));
    }

    #[test]
    fn reject_out_of_range_signed_primitive_default() {
        let err = SchemaBundle::builder("test_msgs::Number")
            .definition(
                "test_msgs::Number",
                TypeDef::Struct(StructDef {
                    fields: vec![
                        FieldDef::new("value", FieldShape::Primitive(FieldPrimitive::I8))
                            .with_default(LiteralValue::Int(200)),
                    ],
                }),
            )
            .build()
            .unwrap_err();

        assert!(matches!(err, SchemaError::InvalidFieldDefault { .. }));
    }

    #[test]
    fn reject_out_of_range_unsigned_array_default() {
        let err = SchemaBundle::builder("test_msgs::Numbers")
            .definition(
                "test_msgs::Numbers",
                TypeDef::Struct(StructDef {
                    fields: vec![
                        FieldDef::new(
                            "values",
                            FieldShape::Array {
                                element: Box::new(FieldShape::Primitive(FieldPrimitive::U8)),
                                length: 1,
                            },
                        )
                        .with_default(LiteralValue::UIntArray(vec![300])),
                    ],
                }),
            )
            .build()
            .unwrap_err();

        assert!(matches!(err, SchemaError::InvalidFieldDefault { .. }));
    }

    #[test]
    fn reject_out_of_range_signed_array_default() {
        let err = SchemaBundle::builder("test_msgs::Numbers")
            .definition(
                "test_msgs::Numbers",
                TypeDef::Struct(StructDef {
                    fields: vec![
                        FieldDef::new(
                            "values",
                            FieldShape::Array {
                                element: Box::new(FieldShape::Primitive(FieldPrimitive::I8)),
                                length: 1,
                            },
                        )
                        .with_default(LiteralValue::IntArray(vec![200])),
                    ],
                }),
            )
            .build()
            .unwrap_err();

        assert!(matches!(err, SchemaError::InvalidFieldDefault { .. }));
    }
}
