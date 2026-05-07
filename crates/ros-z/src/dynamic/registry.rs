//! Schema registry for dynamic schema bundles.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use super::error::DynamicError;
use super::schema::{Schema, TypeDef};
use crate::entity::SchemaHash;

/// Global registry of dynamic root schemas.
pub struct SchemaRegistry {
    schemas: HashMap<String, SchemaVersions>,
}

struct SchemaVersions {
    latest_hash: SchemaHash,
    by_hash: HashMap<SchemaHash, Schema>,
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
            .and_then(|schemas| schemas.by_hash.get(&schemas.latest_hash).cloned())
    }

    /// Get root schema by type name and schema hash.
    pub fn get_root_with_hash(&self, type_name: &str, schema_hash: &SchemaHash) -> Option<Schema> {
        self.schemas
            .get(type_name)
            .and_then(|schemas| schemas.by_hash.get(schema_hash).cloned())
    }

    /// Register a root schema and return the Arc for sharing.
    pub fn register_root_schema(
        &mut self,
        root_name: &str,
        schema: Schema,
    ) -> Result<Schema, DynamicError> {
        let schema_hash = registry_root_schema_hash(root_name, &schema)?;
        match self.schemas.get_mut(root_name) {
            Some(schemas) => {
                schemas.latest_hash = schema_hash;
                schemas.by_hash.insert(schema_hash, schema.clone());
            }
            None => {
                self.schemas.insert(
                    root_name.to_string(),
                    SchemaVersions {
                        latest_hash: schema_hash,
                        by_hash: [(schema_hash, schema.clone())].into(),
                    },
                );
            }
        }
        Ok(schema)
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
        self.schemas
            .values()
            .map(|schemas| schemas.by_hash.len())
            .sum()
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

fn registry_root_schema_hash(root_name: &str, schema: &Schema) -> Result<SchemaHash, DynamicError> {
    schema
        .validate()
        .map_err(|error| DynamicError::SerializationError(error.to_string()))?;
    let TypeDef::Named(actual_root_name) = &schema.root else {
        return Err(DynamicError::SerializationError(format!(
            "schema root for '{root_name}' is not a named type"
        )));
    };
    if actual_root_name.as_str() != root_name {
        return Err(DynamicError::SerializationError(format!(
            "schema root '{}' does not match registered root name '{root_name}'",
            actual_root_name.as_str()
        )));
    }
    Ok(ros_z_schema::compute_hash(schema.as_ref()))
}

/// Get a root schema from the global registry by type name and schema hash.
pub fn get_root_schema_with_hash(type_name: &str, schema_hash: &SchemaHash) -> Option<Schema> {
    get_root_schema_with_hash_in(SchemaRegistry::global(), type_name, schema_hash)
}

/// Register a root schema in the global registry.
pub fn register_root_schema(root_name: &str, schema: Schema) -> Result<Schema, DynamicError> {
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
) -> Result<Schema, DynamicError> {
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
        .map_err(|_| DynamicError::RegistryLockPoisoned)?
        .register_root_schema(root_name, schema)
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
    use crate::dynamic::schema::Schema;
    use ros_z_schema::{SchemaBundle, StructDef, TypeDef, TypeDefinition, TypeName};
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::Arc;

    fn test_schema(type_name: &str) -> Schema {
        empty_struct_schema(type_name)
    }

    fn empty_struct_schema(type_name: &str) -> Schema {
        let type_name = TypeName::new(type_name).unwrap();
        Arc::new(SchemaBundle {
            root: TypeDef::Named(type_name.clone()),
            definitions: [(
                type_name,
                TypeDefinition::Struct(StructDef { fields: Vec::new() }),
            )]
            .into(),
        })
    }

    #[test]
    fn try_register_schema_reports_poisoned_registry() {
        let registry = RwLock::new(SchemaRegistry::default());
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = registry.write().unwrap();
            panic!("poison local registry");
        }));

        let error = try_register_root_schema_in(
            &registry,
            "test_msgs::AfterPoison",
            test_schema("test_msgs::AfterPoison"),
        )
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

        register_root_schema_in(
            &registry,
            "test_msgs::RecoveredRegister",
            test_schema("test_msgs::RecoveredRegister"),
        )
        .unwrap();

        assert!(
            try_register_root_schema_in(
                &registry,
                "test_msgs::RecoveredRegister",
                test_schema("test_msgs::RecoveredRegister"),
            )
            .is_err()
        );
        assert!(has_schema_in(&registry, "test_msgs::RecoveredRegister"));
    }

    #[test]
    fn read_helpers_recover_from_poisoned_registry() {
        let registry = RwLock::new(SchemaRegistry::default());
        let schema = test_schema("test_msgs::RecoveredRead");
        registry
            .write()
            .unwrap()
            .register_root_schema("test_msgs::RecoveredRead", Arc::clone(&schema))
            .unwrap();
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = registry.write().unwrap();
            panic!("poison local registry");
        }));

        assert!(has_schema_in(&registry, "test_msgs::RecoveredRead"));
    }

    #[test]
    fn register_schema_rejects_invalid_bundle_before_storage() {
        let root = ros_z_schema::TypeName::new("test_msgs::Invalid").unwrap();
        let schema = Arc::new(SchemaBundle {
            root: TypeDef::Named(root.clone()),
            definitions: [(
                root,
                ros_z_schema::TypeDefinition::Struct(ros_z_schema::StructDef {
                    fields: vec![
                        ros_z_schema::FieldDef::new("value", TypeDef::String),
                        ros_z_schema::FieldDef::new("value", TypeDef::String),
                    ],
                }),
            )]
            .into(),
        });
        let mut registry = SchemaRegistry::default();

        let error = registry
            .register_root_schema("test_msgs::Invalid", schema)
            .expect_err("invalid schema should be rejected");

        assert!(error.to_string().contains("duplicate field"));
        assert!(!registry.contains("test_msgs::Invalid"));
    }

    #[test]
    fn register_schema_rejects_root_name_that_does_not_match_bundle_root() {
        let schema = empty_struct_schema("test_msgs::Actual");
        let schema_hash = ros_z_schema::compute_hash(schema.as_ref());
        let mut registry = SchemaRegistry::default();

        let error = registry
            .register_root_schema("test_msgs::Requested", schema)
            .expect_err("mismatched root name should be rejected");

        assert!(error.to_string().contains("root"));
        assert!(error.to_string().contains("test_msgs::Requested"));
        assert!(error.to_string().contains("test_msgs::Actual"));
        assert!(
            registry
                .get_root_with_hash("test_msgs::Requested", &schema_hash)
                .is_none()
        );
    }
}
