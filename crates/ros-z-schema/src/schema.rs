use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// An opaque non-empty schema type name for reusable struct and enum definitions.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct TypeName(String);

impl TypeName {
    /// Creates a named definition type name.
    pub fn new(value: impl Into<String>) -> Result<Self, SchemaError> {
        let value = value.into();

        if !is_non_empty_type_name(&value) {
            return Err(SchemaError::InvalidTypeName(value));
        }

        Ok(Self(value))
    }

    /// Returns the underlying type name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for TypeName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<'de> Deserialize<'de> for TypeName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

impl fmt::Display for TypeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Opaque non-empty endpoint root type metadata.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct RootTypeName(String);

impl RootTypeName {
    /// Creates endpoint root type metadata.
    pub fn new(value: impl Into<String>) -> Result<Self, SchemaError> {
        let value = value.into();
        if !is_non_empty_type_name(&value) {
            return Err(SchemaError::InvalidRootTypeName(value));
        }
        Ok(Self(value))
    }

    /// Returns the underlying root type metadata.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for RootTypeName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

impl fmt::Display for RootTypeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

fn is_non_empty_type_name(value: &str) -> bool {
    !value.is_empty()
}

/// Errors produced while building or validating schemas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    /// A named type name is empty.
    InvalidTypeName(String),
    /// A root type name is empty.
    InvalidRootTypeName(String),
    /// A named reference is not present in the bundle.
    MissingDefinition(TypeName),
    /// A named reference points at the wrong definition kind.
    ReferenceKindMismatch {
        /// The referenced definition name.
        name: TypeName,
        /// The expected definition kind.
        expected: &'static str,
    },
    /// A duplicate definition has a conflicting shape.
    ConflictingDefinition(TypeName),
    /// An enum definition has no variants.
    EmptyEnum(TypeName),
    /// A named definition compatibility value is not a struct or enum.
    InvalidNamedDefinition(String),
    /// A primitive field shape uses an unknown spelling.
    InvalidPrimitiveName(String),
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTypeName(value) => {
                write!(
                    f,
                    "invalid type name `{value}`; expected a non-empty string"
                )
            }
            Self::InvalidRootTypeName(value) => write!(
                f,
                "invalid root type name `{value}`; expected a non-empty string"
            ),
            Self::MissingDefinition(type_name) => {
                write!(f, "missing definition for named reference `{type_name}`")
            }
            Self::ReferenceKindMismatch { name, expected } => write!(
                f,
                "named reference `{name}` points at the wrong definition kind; expected {expected}"
            ),
            Self::ConflictingDefinition(type_name) => {
                write!(f, "conflicting definition for `{type_name}`")
            }
            Self::EmptyEnum(type_name) => {
                write!(f, "enum `{type_name}` must define at least one variant")
            }
            Self::InvalidNamedDefinition(kind) => {
                write!(
                    f,
                    "invalid named definition kind `{kind}`; expected struct or enum"
                )
            }
            Self::InvalidPrimitiveName(name) => write!(f, "invalid primitive name `{name}`"),
        }
    }
}

impl std::error::Error for SchemaError {}

/// A normalized schema bundle with inline root shape and named definitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaBundle {
    /// Graph-visible root metadata.
    pub root_name: RootTypeName,
    /// The root schema shape.
    pub root: TypeDef,
    /// All reachable named definitions keyed by type name.
    pub definitions: SchemaDefinitions,
}

impl SchemaBundle {
    /// Creates and validates a bundle without named definitions.
    pub fn new(root_name: RootTypeName, root: TypeDef) -> Result<Self, SchemaError> {
        let bundle = Self {
            root_name,
            root,
            definitions: SchemaDefinitions::new(),
        };
        bundle.validate()?;
        Ok(bundle)
    }

    /// Adds a named definition and revalidates the bundle.
    pub fn with_definition(
        mut self,
        type_name: TypeName,
        definition: NamedTypeDef,
    ) -> Result<Self, SchemaError> {
        if let Some(existing) = self.definitions.get(&type_name) {
            if existing != &definition {
                return Err(SchemaError::ConflictingDefinition(type_name));
            }
            return Ok(self);
        }
        self.definitions.insert(type_name, definition);
        self.validate()?;
        Ok(self)
    }

    /// Validates reference closure.
    pub fn validate(&self) -> Result<(), SchemaError> {
        self.root.validate_references(&self.definitions)?;
        for (type_name, definition) in self.definitions.iter() {
            definition.validate_references(type_name, &self.definitions)?;
        }
        Ok(())
    }

    /// Returns the root type metadata.
    pub fn root_name(&self) -> &RootTypeName {
        &self.root_name
    }

    /// Returns the named definition map.
    pub fn definitions(&self) -> &SchemaDefinitions {
        &self.definitions
    }
}

/// Named definitions keyed by opaque schema type name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SchemaDefinitions(BTreeMap<TypeName, NamedTypeDef>);

impl SchemaDefinitions {
    fn new() -> Self {
        Self(BTreeMap::new())
    }

    fn insert(&mut self, type_name: TypeName, definition: NamedTypeDef) -> Option<NamedTypeDef> {
        self.0.insert(type_name, definition)
    }

    /// Returns a named definition by type name.
    pub fn get(&self, key: &TypeName) -> Option<&NamedTypeDef> {
        self.0.get(key)
    }

    /// Returns true if a named definition exists for the type name.
    pub fn contains_key(&self, key: &TypeName) -> bool {
        self.0.contains_key(key)
    }

    /// Returns an iterator over named definitions.
    pub fn iter(&self) -> impl Iterator<Item = (&TypeName, &NamedTypeDef)> {
        self.0.iter()
    }

    /// Returns true when there are no named definitions.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of named definitions.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns an iterator over definition values.
    pub fn values(&self) -> impl Iterator<Item = &NamedTypeDef> {
        self.0.values()
    }
}

impl Default for SchemaDefinitions {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> From<[(TypeName, NamedTypeDef); N]> for SchemaDefinitions {
    fn from(value: [(TypeName, NamedTypeDef); N]) -> Self {
        Self(BTreeMap::from(value))
    }
}

impl From<BTreeMap<TypeName, NamedTypeDef>> for SchemaDefinitions {
    fn from(value: BTreeMap<TypeName, NamedTypeDef>) -> Self {
        Self(value)
    }
}

impl<'a> IntoIterator for &'a SchemaDefinitions {
    type Item = (&'a TypeName, &'a NamedTypeDef);
    type IntoIter = std::collections::btree_map::Iter<'a, TypeName, NamedTypeDef>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// An inline schema shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeDef {
    /// A Rust-native primitive type.
    Primitive(PrimitiveTypeDef),
    /// A UTF-8 string.
    String,
    /// A reference to a named struct definition.
    StructRef(TypeName),
    /// A reference to a named enum definition.
    EnumRef(TypeName),
    /// An optional value.
    Optional(Box<TypeDef>),
    /// A dynamic or fixed sequence.
    Sequence {
        /// The element shape.
        element: Box<TypeDef>,
        /// The sequence length semantics.
        length: SequenceLengthDef,
    },
    /// A map shape.
    Map {
        /// The key shape.
        key: Box<TypeDef>,
        /// The value shape.
        value: Box<TypeDef>,
    },
}

impl TypeDef {
    fn validate_references(&self, definitions: &SchemaDefinitions) -> Result<(), SchemaError> {
        match self {
            Self::Primitive(_) | Self::String => Ok(()),
            Self::StructRef(name) => match definitions.get(name) {
                Some(NamedTypeDef::Struct(_)) => Ok(()),
                Some(NamedTypeDef::Enum(_)) => Err(SchemaError::ReferenceKindMismatch {
                    name: name.clone(),
                    expected: "struct",
                }),
                None => Err(SchemaError::MissingDefinition(name.clone())),
            },
            Self::EnumRef(name) => match definitions.get(name) {
                Some(NamedTypeDef::Enum(_)) => Ok(()),
                Some(NamedTypeDef::Struct(_)) => Err(SchemaError::ReferenceKindMismatch {
                    name: name.clone(),
                    expected: "enum",
                }),
                None => Err(SchemaError::MissingDefinition(name.clone())),
            },
            Self::Optional(element) => element.validate_references(definitions),
            Self::Sequence { element, .. } => element.validate_references(definitions),
            Self::Map { key, value } => {
                key.validate_references(definitions)?;
                value.validate_references(definitions)
            }
        }
    }
}

/// A Rust-native primitive type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PrimitiveTypeDef {
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

impl PrimitiveTypeDef {
    /// Returns the Rust-native spelling for this primitive.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::I8 => "i8",
            Self::U8 => "u8",
            Self::I16 => "i16",
            Self::U16 => "u16",
            Self::I32 => "i32",
            Self::U32 => "u32",
            Self::I64 => "i64",
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

    fn from_rust_name(name: &str) -> Option<Self> {
        Some(match name {
            "bool" => Self::Bool,
            "i8" => Self::I8,
            "u8" => Self::U8,
            "i16" => Self::I16,
            "u16" => Self::U16,
            "i32" => Self::I32,
            "u32" => Self::U32,
            "i64" => Self::I64,
            "u64" => Self::U64,
            "f32" => Self::F32,
            "f64" => Self::F64,
            _ => return None,
        })
    }
}

impl std::str::FromStr for PrimitiveTypeDef {
    type Err = SchemaError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::from_rust_name(value).ok_or_else(|| SchemaError::InvalidPrimitiveName(value.into()))
    }
}

impl Serialize for PrimitiveTypeDef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for PrimitiveTypeDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PrimitiveTypeDefVisitor;

        impl Visitor<'_> for PrimitiveTypeDefVisitor {
            type Value = PrimitiveTypeDef;

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

        deserializer.deserialize_str(PrimitiveTypeDefVisitor)
    }
}

/// Sequence length semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SequenceLengthDef {
    /// A dynamically sized sequence.
    Dynamic,
    /// A fixed-size sequence.
    Fixed(usize),
}

/// A named struct or enum definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NamedTypeDef {
    /// A record-like type with named fields.
    Struct(StructDef),
    /// A tagged enum with explicit variant payload semantics.
    Enum(EnumDef),
}

impl NamedTypeDef {
    fn validate_references(
        &self,
        type_name: &TypeName,
        definitions: &SchemaDefinitions,
    ) -> Result<(), SchemaError> {
        match self {
            Self::Struct(definition) => definition.validate_references(definitions),
            Self::Enum(definition) => definition.validate_references(type_name, definitions),
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
    fn validate_references(&self, definitions: &SchemaDefinitions) -> Result<(), SchemaError> {
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
        type_name: &TypeName,
        definitions: &SchemaDefinitions,
    ) -> Result<(), SchemaError> {
        if self.variants.is_empty() {
            return Err(SchemaError::EmptyEnum(type_name.clone()));
        }

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

    fn validate_references(&self, definitions: &SchemaDefinitions) -> Result<(), SchemaError> {
        self.payload.validate_references(definitions)
    }
}

/// An enum payload definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnumPayloadDef {
    /// A unit variant with no payload.
    Unit,
    /// A newtype variant carrying a single unnamed field.
    Newtype(TypeDef),
    /// A tuple variant carrying ordered unnamed fields.
    Tuple(Vec<TypeDef>),
    /// A struct variant carrying named fields.
    Struct(Vec<FieldDef>),
}

impl EnumPayloadDef {
    fn validate_references(&self, definitions: &SchemaDefinitions) -> Result<(), SchemaError> {
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
    pub shape: TypeDef,
}

impl FieldDef {
    /// Creates a field.
    pub fn new(name: impl Into<String>, shape: impl Into<TypeDef>) -> Self {
        Self {
            name: name.into(),
            shape: shape.into(),
        }
    }

    fn validate_references(&self, definitions: &SchemaDefinitions) -> Result<(), SchemaError> {
        self.shape.validate_references(definitions)
    }
}
