//! Schema registry for dynamic root schema trees.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use crate::entity::SchemaHash;

#[cfg(test)]
use super::error::DynamicError;
use super::schema::Schema;

/// Global registry of dynamic root schemas.
pub struct SchemaRegistry {
    schemas: HashMap<String, Vec<RegisteredSchema>>,
}

struct RegisteredSchema {
    schema_hash: Option<SchemaHash>,
    schema: Schema,
}

impl SchemaRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    /// Get the global registry.
    pub fn global() -> &'static RwLock<SchemaRegistry> {
        static REGISTRY: OnceLock<RwLock<SchemaRegistry>> = OnceLock::new();
        REGISTRY.get_or_init(|| RwLock::new(SchemaRegistry::new()))
    }

    /// Get the latest root schema registered for a type name.
    pub fn get_root(&self, type_name: &str) -> Option<Schema> {
        self.schemas
            .get(type_name)
            .and_then(|schemas| schemas.last().map(|registered| registered.schema.clone()))
    }

    /// Get root schema by type name and schema hash.
    pub fn get_root_with_hash(&self, type_name: &str, schema_hash: &SchemaHash) -> Option<Schema> {
        self.schemas.get(type_name).and_then(|schemas| {
            schemas
                .iter()
                .find(|registered| registered.schema_hash.as_ref() == Some(schema_hash))
                .map(|registered| registered.schema.clone())
        })
    }

    /// Register a root schema and return the Arc for sharing.
    pub fn register_root_schema(&mut self, root_name: &str, schema: Schema) -> Schema {
        let schema_hash = registry_root_schema_hash(root_name, &schema);
        let schemas = self.schemas.entry(root_name.to_string()).or_default();
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
        self.schemas.keys().map(String::as_str)
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

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn registry_root_schema_hash(root_name: &str, schema: &Schema) -> Option<SchemaHash> {
    crate::dynamic::schema_bridge::schema_hash_with_root_name(root_name, schema).ok()
}

/// Get a root schema from the global registry by type name and schema hash.
pub fn get_root_schema_with_hash(type_name: &str, schema_hash: &SchemaHash) -> Option<Schema> {
    get_root_schema_with_hash_in(SchemaRegistry::global(), type_name, schema_hash)
}

/// Register a root schema in the global registry.
pub fn register_root_schema(root_name: &str, schema: Schema) -> Schema {
    register_root_schema_in(SchemaRegistry::global(), root_name, schema)
}

/// Check if a schema is registered.
pub fn has_schema(type_name: &str) -> bool {
    has_schema_in(SchemaRegistry::global(), type_name)
}

fn get_root_schema_with_hash_in(
    registry: &RwLock<SchemaRegistry>,
    type_name: &str,
    schema_hash: &SchemaHash,
) -> Option<Schema> {
    registry
        .read()
        .unwrap_or_else(|poisoned| {
            tracing::error!("schema registry read lock poisoned; recovering inner state");
            poisoned.into_inner()
        })
        .get_root_with_hash(type_name, schema_hash)
}

fn register_root_schema_in(
    registry: &RwLock<SchemaRegistry>,
    root_name: &str,
    schema: Schema,
) -> Schema {
    registry
        .write()
        .unwrap_or_else(|poisoned| {
            tracing::error!("schema registry write lock poisoned; recovering inner state");
            poisoned.into_inner()
        })
        .register_root_schema(root_name, schema)
}

#[cfg(test)]
fn try_register_root_schema_in(
    registry: &RwLock<SchemaRegistry>,
    root_name: &str,
    schema: Schema,
) -> Result<Schema, DynamicError> {
    registry
        .write()
        .map(|mut registry| registry.register_root_schema(root_name, schema))
        .map_err(|_| DynamicError::RegistryLockPoisoned)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamic::{TypeShape, schema::PrimitiveType};
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::Arc;

    fn test_schema() -> Schema {
        Arc::new(TypeShape::Primitive(PrimitiveType::Bool))
    }

    #[test]
    fn try_register_schema_reports_poisoned_registry() {
        let registry = RwLock::new(SchemaRegistry::default());
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = registry.write().unwrap();
            panic!("poison local registry");
        }));

        let error = try_register_root_schema_in(&registry, "test_msgs::AfterPoison", test_schema())
            .expect_err("poison should be returned");
        assert!(error.to_string().contains("poison"));
    }

    #[test]
    fn register_schema_in_recovers_from_poisoned_registry() {
        let registry = RwLock::new(SchemaRegistry::default());
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = registry.write().unwrap();
            panic!("poison local registry");
        }));

        register_root_schema_in(&registry, "test_msgs::RecoveredRegister", test_schema());

        assert!(
            try_register_root_schema_in(&registry, "test_msgs::RecoveredRegister", test_schema())
                .is_err()
        );
        assert!(has_schema_in(&registry, "test_msgs::RecoveredRegister"));
    }

    #[test]
    fn read_helpers_recover_from_poisoned_registry() {
        let registry = RwLock::new(SchemaRegistry::default());
        let schema = test_schema();
        registry
            .write()
            .unwrap()
            .register_root_schema("test_msgs::RecoveredRead", Arc::clone(&schema));
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = registry.write().unwrap();
            panic!("poison local registry");
        }));

        assert!(has_schema_in(&registry, "test_msgs::RecoveredRead"));
    }
}
