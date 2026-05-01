//! Schema registry for dynamic message types.
//!
//! Provides a global cache of message schemas with lazy initialization
//! and pre-registration of bundled schemas.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use crate::entity::SchemaHash;

use super::error::DynamicError;
use super::schema::MessageSchema;
#[cfg(feature = "dynamic-schema-loader")]
use super::schema::{FieldSchema, FieldType};
use super::schema_bridge::message_schema_to_bundle;

/// Global registry of message schemas.
///
/// Provides fast O(1) lookup by type name and ensures schema sharing
/// via `Arc<MessageSchema>`. Can be pre-populated with bundled schemas.
pub struct SchemaRegistry {
    schemas: HashMap<String, Vec<RegisteredSchema>>,
}

struct RegisteredSchema {
    schema_hash: Option<SchemaHash>,
    schema: Arc<MessageSchema>,
}

impl SchemaRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    /// Get the global registry (lazy initialized).
    pub fn global() -> &'static RwLock<SchemaRegistry> {
        static REGISTRY: OnceLock<RwLock<SchemaRegistry>> = OnceLock::new();
        REGISTRY.get_or_init(|| RwLock::new(SchemaRegistry::new()))
    }

    /// Get the latest schema registered for a native type name (e.g., "geometry_msgs::Twist").
    pub fn get(&self, type_name: &str) -> Option<Arc<MessageSchema>> {
        self.schemas
            .get(type_name)
            .and_then(|schemas| schemas.last().map(|registered| registered.schema.clone()))
    }

    /// Get schema by native type name and schema hash.
    pub fn get_with_hash(
        &self,
        type_name: &str,
        schema_hash: &SchemaHash,
    ) -> Option<Arc<MessageSchema>> {
        self.schemas.get(type_name).and_then(|schemas| {
            schemas
                .iter()
                .find(|registered| registered.schema_hash.as_ref() == Some(schema_hash))
                .map(|registered| registered.schema.clone())
        })
    }

    /// Register a schema and return the Arc for sharing.
    pub fn register(&mut self, schema: Arc<MessageSchema>) -> Arc<MessageSchema> {
        let type_name = schema.type_name_str().to_string();
        let schema_hash = registry_schema_hash(&schema);
        let schemas = self.schemas.entry(type_name).or_default();
        if let Some(existing) = schemas
            .iter_mut()
            .find(|existing| existing.schema_hash == schema_hash)
        {
            existing.schema = schema.clone();
        } else {
            schemas.push(RegisteredSchema {
                schema_hash,
                schema: schema.clone(),
            });
        }
        schema
    }

    /// Check if a type is registered.
    pub fn contains(&self, type_name: &str) -> bool {
        self.schemas.contains_key(type_name)
    }

    /// List all registered type names.
    pub fn type_names(&self) -> impl Iterator<Item = &str> {
        self.schemas.keys().map(|s| s.as_str())
    }

    /// Number of registered schemas.
    pub fn len(&self) -> usize {
        self.schemas.values().map(Vec::len).sum()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.schemas.is_empty()
    }

    /// Clear all registered schemas.
    pub fn clear(&mut self) {
        self.schemas.clear();
    }
}

fn registry_schema_hash(schema: &MessageSchema) -> Option<SchemaHash> {
    schema.schema_hash().or_else(|| {
        message_schema_to_bundle(schema)
            .ok()
            .map(|bundle| ros_z_schema::compute_hash(&bundle))
    })
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Convenience functions for working with the global registry

/// Get a schema from the global registry (read-only, fast path).
pub fn get_schema(type_name: &str) -> Option<Arc<MessageSchema>> {
    get_schema_in(SchemaRegistry::global(), type_name)
}

/// Get a schema from the global registry by native type name and schema hash.
pub fn get_schema_with_hash(
    type_name: &str,
    schema_hash: &SchemaHash,
) -> Option<Arc<MessageSchema>> {
    get_schema_with_hash_in(SchemaRegistry::global(), type_name, schema_hash)
}

/// Register a schema in the global registry.
pub fn register_schema(schema: Arc<MessageSchema>) -> Arc<MessageSchema> {
    register_schema_in(SchemaRegistry::global(), schema)
}

/// Register a schema in the global registry, returning lock errors to the caller.
pub fn try_register_schema(schema: Arc<MessageSchema>) -> Result<Arc<MessageSchema>, DynamicError> {
    try_register_schema_in(SchemaRegistry::global(), schema)
}

fn try_register_schema_in(
    registry: &RwLock<SchemaRegistry>,
    schema: Arc<MessageSchema>,
) -> Result<Arc<MessageSchema>, DynamicError> {
    registry
        .write()
        .map(|mut registry| registry.register(schema))
        .map_err(|_| DynamicError::RegistryLockPoisoned)
}

/// Check if a schema is registered.
pub fn has_schema(type_name: &str) -> bool {
    has_schema_in(SchemaRegistry::global(), type_name)
}

fn get_schema_in(registry: &RwLock<SchemaRegistry>, type_name: &str) -> Option<Arc<MessageSchema>> {
    registry
        .read()
        .unwrap_or_else(|poisoned| {
            tracing::error!("schema registry read lock poisoned; recovering inner state");
            poisoned.into_inner()
        })
        .get(type_name)
}

fn get_schema_with_hash_in(
    registry: &RwLock<SchemaRegistry>,
    type_name: &str,
    schema_hash: &SchemaHash,
) -> Option<Arc<MessageSchema>> {
    registry
        .read()
        .unwrap_or_else(|poisoned| {
            tracing::error!("schema registry read lock poisoned; recovering inner state");
            poisoned.into_inner()
        })
        .get_with_hash(type_name, schema_hash)
}

fn register_schema_in(
    registry: &RwLock<SchemaRegistry>,
    schema: Arc<MessageSchema>,
) -> Arc<MessageSchema> {
    registry
        .write()
        .unwrap_or_else(|poisoned| {
            tracing::error!("schema registry write lock poisoned; recovering inner state");
            poisoned.into_inner()
        })
        .register(schema)
}

fn has_schema_in(registry: &RwLock<SchemaRegistry>, type_name: &str) -> bool {
    registry
        .read()
        .unwrap_or_else(|poisoned| {
            tracing::error!("schema registry read lock poisoned; recovering inner state");
            poisoned.into_inner()
        })
        .contains(type_name)
}

/// Convert a ros-z-codegen ParsedMessage to a dynamic MessageSchema.
///
/// This function handles the conversion of field types from the codegen
/// representation to the dynamic schema representation.
#[cfg(feature = "dynamic-schema-loader")]
pub fn parsed_message_to_schema(
    message: &ros_z_codegen::types::ParsedMessage,
    resolver: &impl Fn(&str, &str) -> Option<Arc<MessageSchema>>,
) -> Result<Arc<MessageSchema>, DynamicError> {
    let fields: Result<Vec<FieldSchema>, DynamicError> = message
        .fields
        .iter()
        .map(|f| {
            let field_type = convert_field_type(f, resolver)?;
            Ok(FieldSchema::new(&f.name, field_type))
        })
        .collect();

    let mut builder = MessageSchema::builder(&format!("{}::{}", message.package, message.name));
    for field in fields? {
        builder = if let Some(default) = field.default_value {
            builder.field_with_default(&field.name, field.field_type, default)
        } else {
            builder.field(&field.name, field.field_type)
        };
    }
    builder.build()
}

#[cfg(feature = "dynamic-schema-loader")]
fn convert_field_type(
    field: &ros_z_codegen::types::Field,
    resolver: &impl Fn(&str, &str) -> Option<Arc<MessageSchema>>,
) -> Result<FieldType, DynamicError> {
    use ros_z_codegen::types::ArrayType;

    let base_type = convert_base_type(
        &field.field_type.base_type,
        &field.field_type.package,
        resolver,
    )?;

    match &field.field_type.array {
        ArrayType::Single => Ok(base_type),
        ArrayType::Fixed(n) => Ok(FieldType::Array(Box::new(base_type), *n)),
        ArrayType::Bounded(n) => Ok(FieldType::BoundedSequence(Box::new(base_type), *n)),
        ArrayType::Unbounded => Ok(FieldType::Sequence(Box::new(base_type))),
    }
}

#[cfg(feature = "dynamic-schema-loader")]
fn convert_base_type(
    base_type: &str,
    package: &Option<String>,
    resolver: &impl Fn(&str, &str) -> Option<Arc<MessageSchema>>,
) -> Result<FieldType, DynamicError> {
    // Check if it's a primitive type
    match base_type {
        "bool" => return Ok(FieldType::Bool),
        "int8" | "byte" => return Ok(FieldType::Int8),
        "int16" => return Ok(FieldType::Int16),
        "int32" => return Ok(FieldType::Int32),
        "int64" => return Ok(FieldType::Int64),
        "uint8" | "char" => return Ok(FieldType::Uint8),
        "uint16" => return Ok(FieldType::Uint16),
        "uint32" => return Ok(FieldType::Uint32),
        "uint64" => return Ok(FieldType::Uint64),
        "float32" => return Ok(FieldType::Float32),
        "float64" => return Ok(FieldType::Float64),
        "string" => return Ok(FieldType::String),
        _ => {}
    }

    // Check for bounded string
    if let Some(rest) = base_type.strip_prefix("string<=")
        && let Ok(max_len) = rest.parse::<usize>()
    {
        return Ok(FieldType::BoundedString(max_len));
    }

    // It's a message type - resolve it
    let pkg = package
        .as_ref()
        .ok_or_else(|| DynamicError::InvalidTypeName(base_type.to_string()))?;
    let schema = resolver(pkg, base_type)
        .ok_or_else(|| DynamicError::SchemaNotFound(format!("{}::{}", pkg, base_type)))?;

    Ok(FieldType::Message(schema))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    #[test]
    fn try_register_schema_reports_poisoned_registry() {
        let registry = RwLock::new(SchemaRegistry::default());
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = registry.write().unwrap();
            panic!("poison local registry");
        }));

        let schema = MessageSchema::builder("test_msgs::AfterPoison")
            .build()
            .unwrap();
        let error =
            try_register_schema_in(&registry, schema).expect_err("poison should be returned");
        assert!(error.to_string().contains("poison"));
    }

    #[test]
    fn register_schema_in_recovers_from_poisoned_registry() {
        let registry = RwLock::new(SchemaRegistry::default());
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = registry.write().unwrap();
            panic!("poison local registry");
        }));

        let schema = MessageSchema::builder("test_msgs::RecoveredRegister")
            .build()
            .unwrap();
        register_schema_in(&registry, Arc::clone(&schema));

        assert!(try_register_schema_in(&registry, schema).is_err());
        assert!(has_schema_in(&registry, "test_msgs::RecoveredRegister"));
    }

    #[test]
    fn read_helpers_recover_from_poisoned_registry() {
        let registry = RwLock::new(SchemaRegistry::default());
        let schema = MessageSchema::builder("test_msgs::RecoveredRead")
            .build()
            .unwrap();
        registry.write().unwrap().register(Arc::clone(&schema));
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = registry.write().unwrap();
            panic!("poison local registry");
        }));

        assert!(has_schema_in(&registry, "test_msgs::RecoveredRead"));
        assert!(Arc::ptr_eq(
            &schema,
            &get_schema_in(&registry, "test_msgs::RecoveredRead").unwrap()
        ));
    }
}
