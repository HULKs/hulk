//! Tests for the schema registry.

use std::sync::Arc;

use crate::dynamic::registry::{
    SchemaRegistry, get_schema, get_schema_with_hash, has_schema, register_schema,
};
use crate::dynamic::schema::{FieldType, MessageSchema};
use crate::dynamic::schema_bridge::message_schema_to_bundle;
use crate::entity::SchemaHash;

fn create_test_schema(name: &str) -> Arc<MessageSchema> {
    MessageSchema::builder(name)
        .field("x", FieldType::Float64)
        .build()
        .unwrap()
}

#[test]
fn test_registry_basic_operations() {
    let mut registry = SchemaRegistry::new();
    assert!(registry.is_empty());

    let schema = create_test_schema("test_msgs::Point");
    registry.register(schema.clone());

    assert!(!registry.is_empty());
    assert_eq!(registry.len(), 1);
    assert!(registry.contains("test_msgs::Point"));

    let retrieved = registry.get("test_msgs::Point");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().type_name_str(), "test_msgs::Point");
}

#[test]
fn test_registry_not_found() {
    let registry = SchemaRegistry::new();
    assert!(registry.get("nonexistent::Type").is_none());
    assert!(!registry.contains("nonexistent::Type"));
}

#[test]
fn test_registry_multiple_schemas() {
    let mut registry = SchemaRegistry::new();

    registry.register(create_test_schema("pkg1::A"));
    registry.register(create_test_schema("pkg2::B"));
    registry.register(create_test_schema("pkg3::C"));

    assert_eq!(registry.len(), 3);
    assert!(registry.contains("pkg1::A"));
    assert!(registry.contains("pkg2::B"));
    assert!(registry.contains("pkg3::C"));
}

#[test]
fn test_registry_type_names_iteration() {
    let mut registry = SchemaRegistry::new();

    registry.register(create_test_schema("pkg1::A"));
    registry.register(create_test_schema("pkg2::B"));

    let names: Vec<&str> = registry.type_names().collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"pkg1::A"));
    assert!(names.contains(&"pkg2::B"));
}

#[test]
fn test_registry_clear() {
    let mut registry = SchemaRegistry::new();

    registry.register(create_test_schema("pkg1::A"));
    registry.register(create_test_schema("pkg2::B"));
    assert_eq!(registry.len(), 2);

    registry.clear();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
}

#[test]
fn test_registry_replace_schema() {
    let mut registry = SchemaRegistry::new();

    let schema1 = MessageSchema::builder("test_msgs::Point")
        .field("x", FieldType::Float64)
        .build()
        .unwrap();

    let schema2 = MessageSchema::builder("test_msgs::Point")
        .field("x", FieldType::Float64)
        .field("y", FieldType::Float64)
        .build()
        .unwrap();

    registry.register(schema1);
    assert_eq!(registry.get("test_msgs::Point").unwrap().fields().len(), 1);

    registry.register(schema2);
    assert_eq!(registry.get("test_msgs::Point").unwrap().fields().len(), 2);
}

#[test]
fn registry_keeps_same_type_name_with_different_schema_hashes() {
    let mut registry = SchemaRegistry::new();

    let hash1 = SchemaHash([0x11; 32]);
    let hash2 = SchemaHash([0x22; 32]);
    let schema1 = MessageSchema::builder("test_msgs::Point")
        .field("x", FieldType::Float64)
        .schema_hash(hash1)
        .build()
        .unwrap();
    let schema2 = MessageSchema::builder("test_msgs::Point")
        .field("x", FieldType::Float64)
        .field("y", FieldType::Float64)
        .schema_hash(hash2)
        .build()
        .unwrap();

    registry.register(schema1);
    registry.register(schema2);

    assert_eq!(registry.len(), 2);
    assert_eq!(
        registry
            .get_with_hash("test_msgs::Point", &hash1)
            .unwrap()
            .fields()
            .len(),
        1
    );
    assert_eq!(
        registry
            .get_with_hash("test_msgs::Point", &hash2)
            .unwrap()
            .fields()
            .len(),
        2
    );
}

#[test]
fn registry_uses_computed_schema_hash_when_explicit_hash_is_absent() {
    let mut registry = SchemaRegistry::new();

    let schema1 = MessageSchema::builder("test_msgs::Point")
        .field("x", FieldType::Float64)
        .build()
        .unwrap();
    let schema2 = MessageSchema::builder("test_msgs::Point")
        .field("x", FieldType::Float64)
        .field("y", FieldType::Float64)
        .build()
        .unwrap();
    let hash1 = ros_z_schema::compute_hash(&message_schema_to_bundle(&schema1).unwrap());
    let hash2 = ros_z_schema::compute_hash(&message_schema_to_bundle(&schema2).unwrap());

    registry.register(schema1);
    registry.register(schema2);

    assert_eq!(registry.len(), 2);
    assert_eq!(
        registry
            .get_with_hash("test_msgs::Point", &hash1)
            .unwrap()
            .fields()
            .len(),
        1
    );
    assert_eq!(
        registry
            .get_with_hash("test_msgs::Point", &hash2)
            .unwrap()
            .fields()
            .len(),
        2
    );
}

#[test]
fn test_global_registry_functions() {
    // Use a unique type name to avoid conflicts with other tests
    let type_name = "test_global_unique::TestMessage123";

    // Register a schema
    let schema = create_test_schema(type_name);
    register_schema(schema);

    // It should be retrievable
    assert!(has_schema(type_name));
    let retrieved = get_schema(type_name);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().type_name_str(), type_name);
}

#[test]
fn global_registry_gets_schema_by_type_name_and_schema_hash() {
    let type_name = "test_global_unique::HashVersionedMessage123";
    let hash1 = SchemaHash([0x33; 32]);
    let hash2 = SchemaHash([0x44; 32]);

    let schema1 = MessageSchema::builder(type_name)
        .field("x", FieldType::Float64)
        .schema_hash(hash1)
        .build()
        .unwrap();
    let schema2 = MessageSchema::builder(type_name)
        .field("x", FieldType::Float64)
        .field("y", FieldType::Float64)
        .schema_hash(hash2)
        .build()
        .unwrap();

    register_schema(schema1);
    register_schema(schema2);

    assert_eq!(
        get_schema_with_hash(type_name, &hash1)
            .unwrap()
            .fields()
            .len(),
        1
    );
    assert_eq!(
        get_schema_with_hash(type_name, &hash2)
            .unwrap()
            .fields()
            .len(),
        2
    );
}

#[test]
fn test_global_registry_returns_none_for_unknown() {
    assert!(!has_schema("completely_unknown::Type"));
    assert!(get_schema("completely_unknown::Type").is_none());
}

#[test]
fn test_schema_sharing() {
    let mut registry = SchemaRegistry::new();

    let schema = create_test_schema("shared::Schema");
    let returned = registry.register(schema.clone());

    // The returned Arc should be the same as the input
    assert!(Arc::ptr_eq(&schema, &returned));

    // Getting from registry should return an equivalent Arc
    let retrieved = registry.get("shared::Schema").unwrap();
    assert!(Arc::ptr_eq(&schema, &retrieved));
}
