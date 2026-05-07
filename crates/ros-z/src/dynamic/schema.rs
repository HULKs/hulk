//! Canonical schema types used by dynamic CDR-backed messages.

use std::sync::Arc;

pub use ros_z_schema::{
    EnumDef, EnumPayloadDef, EnumVariantDef, FieldDef, PrimitiveTypeDef, SchemaBundle, SchemaError,
    SequenceLengthDef, StructDef, TypeDef, TypeDefinition, TypeDefinitions, TypeName,
};

/// Shared canonical schema bundle for dynamic root and field shapes.
pub type Schema = Arc<SchemaBundle>;
