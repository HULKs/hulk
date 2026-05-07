use std::collections::{BTreeMap, BTreeSet};
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

fn is_non_empty_type_name(value: &str) -> bool {
    !value.is_empty()
}

/// Errors produced while building or validating schemas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    /// A named type name is empty.
    InvalidTypeName(String),
    /// A primitive field shape uses an unknown spelling.
    InvalidPrimitiveName(String),
    /// A named reference is not present in the bundle.
    MissingDefinition(TypeName),
    /// A duplicate definition has a conflicting shape.
    ConflictingDefinition(TypeName),
    /// A duplicate definition changes the definition kind.
    DefinitionKindConflict {
        /// The conflicting definition name.
        name: TypeName,
        /// The already registered definition kind.
        existing: DefinitionKind,
        /// The attempted definition kind.
        attempted: DefinitionKind,
    },
    /// A schema builder failed to construct a valid bundle.
    BuilderFailed,
    /// A definition is not reachable from the bundle root.
    UnreachableDefinition(TypeName),
    /// A struct or struct-style enum payload contains a duplicate field name.
    DuplicateField {
        /// The containing type name.
        type_name: TypeName,
        /// The duplicated field name.
        field_name: String,
    },
    /// An enum contains a duplicate variant name.
    DuplicateVariant {
        /// The containing enum type name.
        type_name: TypeName,
        /// The duplicated variant name.
        variant_name: String,
    },
    /// A struct or struct-style enum payload contains an empty field name.
    EmptyFieldName {
        /// The containing type name.
        type_name: TypeName,
    },
    /// An enum contains an empty variant name.
    EmptyVariantName {
        /// The containing enum type name.
        type_name: TypeName,
    },
    /// An enum definition has no variants.
    EmptyEnum {
        /// The empty enum type name.
        type_name: TypeName,
    },
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
            Self::InvalidPrimitiveName(name) => write!(f, "invalid primitive name `{name}`"),
            Self::MissingDefinition(type_name) => {
                write!(f, "missing definition for named reference `{type_name}`")
            }
            Self::ConflictingDefinition(type_name) => {
                write!(f, "conflicting definition for `{type_name}`")
            }
            Self::DefinitionKindConflict {
                name,
                existing,
                attempted,
            } => write!(
                f,
                "definition `{name}` kind conflict; existing {existing}, attempted {attempted}"
            ),
            Self::BuilderFailed => write!(f, "schema builder failed"),
            Self::UnreachableDefinition(type_name) => {
                write!(f, "definition `{type_name}` is unreachable from the root")
            }
            Self::DuplicateField {
                type_name,
                field_name,
            } => write!(f, "duplicate field `{field_name}` in `{type_name}`"),
            Self::DuplicateVariant {
                type_name,
                variant_name,
            } => write!(f, "duplicate variant `{variant_name}` in `{type_name}`"),
            Self::EmptyFieldName { type_name } => {
                write!(f, "field names in `{type_name}` must be non-empty")
            }
            Self::EmptyVariantName { type_name } => {
                write!(f, "variant names in `{type_name}` must be non-empty")
            }
            Self::EmptyEnum { type_name } => {
                write!(f, "enum `{type_name}` must define at least one variant")
            }
        }
    }
}

impl std::error::Error for SchemaError {}

/// The kind of a named schema definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionKind {
    /// A record-like type with named fields.
    Struct,
    /// A tagged enum with explicit variant payload semantics.
    Enum,
}

impl fmt::Display for DefinitionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Struct => f.write_str("struct"),
            Self::Enum => f.write_str("enum"),
        }
    }
}

/// A normalized schema bundle with inline root shape and named definitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaBundle {
    /// The root schema shape.
    pub root: TypeDef,
    /// All reachable named definitions keyed by type name.
    pub definitions: TypeDefinitions,
}

impl SchemaBundle {
    /// Creates and validates a bundle without named definitions.
    pub fn new(root: TypeDef) -> Result<Self, SchemaError> {
        let bundle = Self {
            root,
            definitions: TypeDefinitions::new(),
        };
        bundle.validate()?;
        Ok(bundle)
    }

    /// Validates reference closure and reachable definitions.
    pub fn validate(&self) -> Result<(), SchemaError> {
        let mut reachable = BTreeSet::new();
        self.root
            .validate_reachable(&self.definitions, &mut reachable)?;

        for type_name in self.definitions.keys() {
            if !reachable.contains(type_name) {
                return Err(SchemaError::UnreachableDefinition(type_name.clone()));
            }
        }

        Ok(())
    }

    /// Returns the named definition map.
    pub fn definitions(&self) -> &TypeDefinitions {
        &self.definitions
    }
}

/// Named definitions keyed by opaque schema type name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TypeDefinitions(BTreeMap<TypeName, TypeDefinition>);

impl TypeDefinitions {
    /// Creates an empty definition map.
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Inserts a named definition.
    pub fn insert(
        &mut self,
        type_name: TypeName,
        definition: TypeDefinition,
    ) -> Option<TypeDefinition> {
        self.0.insert(type_name, definition)
    }

    /// Returns a named definition by type name.
    pub fn get(&self, key: &TypeName) -> Option<&TypeDefinition> {
        self.0.get(key)
    }

    /// Returns true if a named definition exists for the type name.
    pub fn contains_key(&self, key: &TypeName) -> bool {
        self.0.contains_key(key)
    }

    /// Returns definition names in canonical order.
    pub fn keys(&self) -> impl Iterator<Item = &TypeName> {
        self.0.keys()
    }

    /// Returns an iterator over named definitions.
    pub fn iter(&self) -> impl Iterator<Item = (&TypeName, &TypeDefinition)> {
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
    pub fn values(&self) -> impl Iterator<Item = &TypeDefinition> {
        self.0.values()
    }
}

impl Default for TypeDefinitions {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> From<[(TypeName, TypeDefinition); N]> for TypeDefinitions {
    fn from(value: [(TypeName, TypeDefinition); N]) -> Self {
        Self(BTreeMap::from(value))
    }
}

impl From<BTreeMap<TypeName, TypeDefinition>> for TypeDefinitions {
    fn from(value: BTreeMap<TypeName, TypeDefinition>) -> Self {
        Self(value)
    }
}

impl<'a> IntoIterator for &'a TypeDefinitions {
    type Item = (&'a TypeName, &'a TypeDefinition);
    type IntoIter = std::collections::btree_map::Iter<'a, TypeName, TypeDefinition>;

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
    /// A reference to a named definition.
    Named(TypeName),
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
    fn validate_reachable(
        &self,
        definitions: &TypeDefinitions,
        reachable: &mut BTreeSet<TypeName>,
    ) -> Result<(), SchemaError> {
        match self {
            Self::Primitive(_) | Self::String => Ok(()),
            Self::Named(name) => {
                let Some(definition) = definitions.get(name) else {
                    return Err(SchemaError::MissingDefinition(name.clone()));
                };

                if !reachable.insert(name.clone()) {
                    return Ok(());
                }

                definition.validate_reachable(name, definitions, reachable)
            }
            Self::Optional(element) => element.validate_reachable(definitions, reachable),
            Self::Sequence { element, .. } => element.validate_reachable(definitions, reachable),
            Self::Map { key, value } => {
                key.validate_reachable(definitions, reachable)?;
                value.validate_reachable(definitions, reachable)
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
pub enum TypeDefinition {
    /// A record-like type with named fields.
    Struct(StructDef),
    /// A tagged enum with explicit variant payload semantics.
    Enum(EnumDef),
}

impl TypeDefinition {
    /// Returns the definition kind.
    pub fn kind(&self) -> DefinitionKind {
        match self {
            Self::Struct(_) => DefinitionKind::Struct,
            Self::Enum(_) => DefinitionKind::Enum,
        }
    }

    fn validate_reachable(
        &self,
        type_name: &TypeName,
        definitions: &TypeDefinitions,
        reachable: &mut BTreeSet<TypeName>,
    ) -> Result<(), SchemaError> {
        match self {
            Self::Struct(definition) => {
                definition.validate_reachable(type_name, definitions, reachable)
            }
            Self::Enum(definition) => {
                definition.validate_reachable(type_name, definitions, reachable)
            }
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
    fn validate_reachable(
        &self,
        type_name: &TypeName,
        definitions: &TypeDefinitions,
        reachable: &mut BTreeSet<TypeName>,
    ) -> Result<(), SchemaError> {
        validate_fields(type_name, &self.fields)?;
        for field in &self.fields {
            field.validate_reachable(definitions, reachable)?;
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
    fn validate_reachable(
        &self,
        type_name: &TypeName,
        definitions: &TypeDefinitions,
        reachable: &mut BTreeSet<TypeName>,
    ) -> Result<(), SchemaError> {
        if self.variants.is_empty() {
            return Err(SchemaError::EmptyEnum {
                type_name: type_name.clone(),
            });
        }

        let mut names = BTreeSet::new();
        for variant in &self.variants {
            if variant.name.is_empty() {
                return Err(SchemaError::EmptyVariantName {
                    type_name: type_name.clone(),
                });
            }
            if !names.insert(variant.name.clone()) {
                return Err(SchemaError::DuplicateVariant {
                    type_name: type_name.clone(),
                    variant_name: variant.name.clone(),
                });
            }
            variant.validate_reachable(type_name, definitions, reachable)?;
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

    fn validate_reachable(
        &self,
        type_name: &TypeName,
        definitions: &TypeDefinitions,
        reachable: &mut BTreeSet<TypeName>,
    ) -> Result<(), SchemaError> {
        self.payload
            .validate_reachable(type_name, definitions, reachable)
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
    fn validate_reachable(
        &self,
        type_name: &TypeName,
        definitions: &TypeDefinitions,
        reachable: &mut BTreeSet<TypeName>,
    ) -> Result<(), SchemaError> {
        match self {
            Self::Unit => Ok(()),
            Self::Newtype(shape) => shape.validate_reachable(definitions, reachable),
            Self::Tuple(shapes) => {
                for shape in shapes {
                    shape.validate_reachable(definitions, reachable)?;
                }
                Ok(())
            }
            Self::Struct(fields) => {
                validate_fields(type_name, fields)?;
                for field in fields {
                    field.validate_reachable(definitions, reachable)?;
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

    fn validate_reachable(
        &self,
        definitions: &TypeDefinitions,
        reachable: &mut BTreeSet<TypeName>,
    ) -> Result<(), SchemaError> {
        self.shape.validate_reachable(definitions, reachable)
    }
}

fn validate_fields(type_name: &TypeName, fields: &[FieldDef]) -> Result<(), SchemaError> {
    let mut names = BTreeSet::new();
    for field in fields {
        if field.name.is_empty() {
            return Err(SchemaError::EmptyFieldName {
                type_name: type_name.clone(),
            });
        }
        if !names.insert(field.name.clone()) {
            return Err(SchemaError::DuplicateField {
                type_name: type_name.clone(),
                field_name: field.name.clone(),
            });
        }
    }
    Ok(())
}
