//! ROS Schema Model
//!
//! This crate provides the schema model used by ros-z for:
//! - schema hashing via [`SchemaHash`] and `RZHS02_<hex>` strings
//! - cross-crate schema exchange through [`SchemaBundle`]
//! - dynamic runtime schema conversion between `ros-z` and schema bundles
//! - stable JSON serialization and hashing for ros-z-native schema identity
//!
//! [`SchemaBundle`] and its first-class field/type semantics are the authoritative
//! representation for ros-z schema identity and hashing.
mod composite;
mod hash;
mod json;
mod schema;

pub use composite::{ActionDef, ActionSemanticIdentity, ServiceDef};
pub use hash::SchemaHash;
pub use hash::compute_hash;
pub use json::{JsonEncode, to_json};
pub use schema::{
    DefinitionKind, EnumDef, EnumPayloadDef, EnumVariantDef, FieldDef, PrimitiveTypeDef,
    SchemaBundle, SchemaError, SequenceLengthDef, StructDef, TypeDef, TypeDefinition,
    TypeDefinitions, TypeName,
};
