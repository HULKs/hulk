//! Tests for schema types (FieldType, FieldSchema, MessageSchema).

use std::sync::Arc;

use crate::dynamic::schema::{
    EnumPayloadSchema, EnumSchema, EnumVariantSchema, FieldType, MessageSchema,
};
use crate::dynamic::schema_bridge;
use crate::dynamic::{DynamicValue, FieldSchema};
use ros_z_schema::{
    EnumDef, EnumPayloadDef, EnumVariantDef, FieldDef, FieldPrimitive, FieldShape, LiteralValue,
    SchemaBundle, StructDef, TypeDef, TypeName,
};

#[test]
fn test_field_type_fixed_size() {
    assert_eq!(FieldType::Bool.fixed_size(), Some(1));
    assert_eq!(FieldType::Int8.fixed_size(), Some(1));
    assert_eq!(FieldType::Int16.fixed_size(), Some(2));
    assert_eq!(FieldType::Int32.fixed_size(), Some(4));
    assert_eq!(FieldType::Int64.fixed_size(), Some(8));
    assert_eq!(FieldType::Uint8.fixed_size(), Some(1));
    assert_eq!(FieldType::Uint16.fixed_size(), Some(2));
    assert_eq!(FieldType::Uint32.fixed_size(), Some(4));
    assert_eq!(FieldType::Uint64.fixed_size(), Some(8));
    assert_eq!(FieldType::Float32.fixed_size(), Some(4));
    assert_eq!(FieldType::Float64.fixed_size(), Some(8));
    assert_eq!(FieldType::String.fixed_size(), None);

    let arr = FieldType::Array(Box::new(FieldType::Float64), 3);
    assert_eq!(arr.fixed_size(), Some(24));

    let seq = FieldType::Sequence(Box::new(FieldType::Int32));
    assert_eq!(seq.fixed_size(), None);
}

#[test]
fn test_field_type_alignment() {
    assert_eq!(FieldType::Bool.alignment(), 1);
    assert_eq!(FieldType::Int8.alignment(), 1);
    assert_eq!(FieldType::Uint8.alignment(), 1);
    assert_eq!(FieldType::Int16.alignment(), 2);
    assert_eq!(FieldType::Uint16.alignment(), 2);
    assert_eq!(FieldType::Int32.alignment(), 4);
    assert_eq!(FieldType::Uint32.alignment(), 4);
    assert_eq!(FieldType::Float32.alignment(), 4);
    assert_eq!(FieldType::Int64.alignment(), 8);
    assert_eq!(FieldType::Uint64.alignment(), 8);
    assert_eq!(FieldType::Float64.alignment(), 8);
    assert_eq!(FieldType::String.alignment(), 4);
}

#[test]
fn test_field_type_is_primitive() {
    assert!(FieldType::Bool.is_primitive());
    assert!(FieldType::Int32.is_primitive());
    assert!(FieldType::Float64.is_primitive());
    assert!(FieldType::String.is_primitive());
    assert!(FieldType::BoundedString(100).is_primitive());

    assert!(!FieldType::Sequence(Box::new(FieldType::Int32)).is_primitive());
    assert!(!FieldType::Array(Box::new(FieldType::Int32), 10).is_primitive());
}

#[test]
fn test_field_type_is_numeric() {
    assert!(FieldType::Int8.is_numeric());
    assert!(FieldType::Int32.is_numeric());
    assert!(FieldType::Float64.is_numeric());

    assert!(!FieldType::Bool.is_numeric());
    assert!(!FieldType::String.is_numeric());
}

#[test]
fn test_message_schema_builder() {
    let schema = MessageSchema::builder("geometry_msgs::Point")
        .field("x", FieldType::Float64)
        .field("y", FieldType::Float64)
        .field("z", FieldType::Float64)
        .build()
        .unwrap();

    assert_eq!(schema.type_name_str(), "geometry_msgs::Point");
    assert_eq!(schema.fields().len(), 3);
    assert_eq!(schema.field("x").unwrap().name, "x");
    assert_eq!(schema.field_count(), 3);
}

#[test]
fn test_invalid_type_name() {
    // Empty path segment
    let result = MessageSchema::builder("invalid::").build();
    assert!(result.is_err());

    // Wrong separator
    let result = MessageSchema::builder("pkg/Name").build();
    assert!(result.is_err());
}

#[test]
fn test_field_path_indices() {
    let point = MessageSchema::builder("geometry_msgs::Point")
        .field("x", FieldType::Float64)
        .field("y", FieldType::Float64)
        .field("z", FieldType::Float64)
        .build()
        .unwrap();

    let vector3 = MessageSchema::builder("geometry_msgs::Vector3")
        .field("x", FieldType::Float64)
        .field("y", FieldType::Float64)
        .field("z", FieldType::Float64)
        .build()
        .unwrap();

    let twist = MessageSchema::builder("geometry_msgs::Twist")
        .field("linear", FieldType::Message(vector3.clone()))
        .field("angular", FieldType::Message(vector3))
        .build()
        .unwrap();

    // Simple path
    let indices = point.field_path_indices("x").unwrap();
    assert_eq!(indices, vec![0]);

    let indices = point.field_path_indices("z").unwrap();
    assert_eq!(indices, vec![2]);

    // Nested path
    let indices = twist.field_path_indices("linear.x").unwrap();
    assert_eq!(indices, vec![0, 0]);

    let indices = twist.field_path_indices("angular.z").unwrap();
    assert_eq!(indices, vec![1, 2]);

    // Field not found
    let result = point.field_path_indices("nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_message_schema_field_names() {
    let schema = MessageSchema::builder("test_msgs::Test")
        .field("a", FieldType::Int32)
        .field("b", FieldType::String)
        .field("c", FieldType::Float64)
        .build()
        .unwrap();

    let names: Vec<&str> = schema.field_names().collect();
    assert_eq!(names, vec!["a", "b", "c"]);
}

#[test]
fn test_message_schema_hash_override() {
    let schema = MessageSchema::builder("test_msgs::Test")
        .field("x", FieldType::Int32)
        .schema_hash(crate::entity::SchemaHash([0xab; 32]))
        .build()
        .unwrap();

    assert_eq!(
        schema.schema_hash(),
        Some(crate::entity::SchemaHash([0xab; 32]))
    );
}

#[test]
fn test_schema_equality() {
    let schema1 = MessageSchema::builder("test_msgs::Test")
        .field("x", FieldType::Int32)
        .build()
        .unwrap();

    let schema2 = MessageSchema::builder("test_msgs::Test")
        .field("x", FieldType::Int32)
        .build()
        .unwrap();

    let schema3 = MessageSchema::builder("test_msgs::Other")
        .field("x", FieldType::Int32)
        .build()
        .unwrap();

    assert_eq!(*schema1, *schema2);
    assert_ne!(*schema1, *schema3);
}

#[test]
fn test_schema_equality_reflects_field_structure() {
    let schema1 = MessageSchema::builder("test_msgs::Test")
        .field("x", FieldType::Int32)
        .build()
        .unwrap();
    let schema2 = MessageSchema::builder("test_msgs::Test")
        .field("x", FieldType::String)
        .build()
        .unwrap();

    assert_ne!(*schema1, *schema2);
}

#[test]
fn message_schema_to_bundle_roundtrip_preserves_extended_shapes() {
    let runtime = MessageSchema::builder("custom_msgs::RobotEnvelope")
        .field(
            "mission_id",
            FieldType::Optional(Box::new(FieldType::Uint32)),
        )
        .build()
        .unwrap();

    let bundle = schema_bridge::message_schema_to_bundle(&runtime).unwrap();
    let rebuilt = schema_bridge::bundle_to_message_schema(&bundle).unwrap();

    assert_eq!(rebuilt.type_name_str(), runtime.type_name_str());
    assert!(matches!(
        rebuilt.fields()[0].field_type,
        FieldType::Optional(_)
    ));

    let root = bundle.definitions.get(&bundle.root).unwrap();
    let TypeDef::Struct(root) = root else {
        panic!("expected struct root");
    };
    assert!(matches!(root.fields[0].shape, FieldShape::Optional { .. }));
}

#[test]
fn message_schema_to_bundle_roundtrip_preserves_enum_struct_field_defaults() {
    let runtime = MessageSchema::builder("custom_msgs::RobotEnvelope")
        .field(
            "state",
            FieldType::Enum(Arc::new(EnumSchema::new(
                "custom_msgs::EnvelopeState",
                vec![EnumVariantSchema::new(
                    "Ready",
                    EnumPayloadSchema::Struct(vec![
                        FieldSchema::new("priority", FieldType::Uint32)
                            .with_default(DynamicValue::Uint32(7)),
                    ]),
                )],
            ))),
        )
        .build()
        .unwrap();

    let bundle = schema_bridge::message_schema_to_bundle(&runtime).unwrap();
    let rebuilt = schema_bridge::bundle_to_message_schema(&bundle).unwrap();

    let FieldType::Enum(schema) = &rebuilt.fields()[0].field_type else {
        panic!("expected enum field");
    };
    let EnumPayloadSchema::Struct(fields) = &schema.variants[0].payload else {
        panic!("expected struct enum payload");
    };

    assert_eq!(fields[0].default_value, Some(DynamicValue::Uint32(7)));
}

#[test]
fn message_schema_to_bundle_roundtrip_preserves_float_defaults() {
    let runtime = MessageSchema::builder("custom_msgs::FloatDefaults")
        .field_with_default("gain", FieldType::Float64, DynamicValue::Float64(1.25))
        .build()
        .unwrap();

    let bundle = schema_bridge::message_schema_to_bundle(&runtime).unwrap();
    let rebuilt = schema_bridge::bundle_to_message_schema(&bundle).unwrap();

    assert_eq!(
        rebuilt.fields()[0].default_value,
        Some(DynamicValue::Float64(1.25))
    );
}

#[test]
fn message_schema_to_bundle_uses_rust_native_primitive_enum() {
    let runtime = MessageSchema::builder("custom_msgs::NativePrimitives")
        .field("bool_value", FieldType::Bool)
        .field("int8_value", FieldType::Int8)
        .field("int16_value", FieldType::Int16)
        .field("int32_value", FieldType::Int32)
        .field("int64_value", FieldType::Int64)
        .field("uint8_value", FieldType::Uint8)
        .field("uint16_value", FieldType::Uint16)
        .field("uint32_value", FieldType::Uint32)
        .field("uint64_value", FieldType::Uint64)
        .field("float32_value", FieldType::Float32)
        .field("float64_value", FieldType::Float64)
        .build()
        .unwrap();

    let bundle = schema_bridge::message_schema_to_bundle(&runtime).unwrap();
    let root = bundle.definitions.get(&bundle.root).unwrap();
    let TypeDef::Struct(root) = root else {
        panic!("expected struct root");
    };

    let shapes: Vec<_> = root.fields.iter().map(|field| &field.shape).collect();
    assert_eq!(
        shapes,
        vec![
            &FieldShape::Primitive(FieldPrimitive::Bool),
            &FieldShape::Primitive(FieldPrimitive::I8),
            &FieldShape::Primitive(FieldPrimitive::I16),
            &FieldShape::Primitive(FieldPrimitive::I32),
            &FieldShape::Primitive(FieldPrimitive::I64),
            &FieldShape::Primitive(FieldPrimitive::U8),
            &FieldShape::Primitive(FieldPrimitive::U16),
            &FieldShape::Primitive(FieldPrimitive::U32),
            &FieldShape::Primitive(FieldPrimitive::U64),
            &FieldShape::Primitive(FieldPrimitive::F32),
            &FieldShape::Primitive(FieldPrimitive::F64),
        ]
    );
}

#[test]
fn bundle_to_message_schema_converts_rust_native_primitive_enum() {
    let bundle = SchemaBundle::builder("custom_msgs::NativePrimitives")
        .definition(
            "custom_msgs::NativePrimitives",
            TypeDef::Struct(StructDef {
                fields: vec![
                    FieldDef::new("bool_value", FieldShape::Primitive(FieldPrimitive::Bool)),
                    FieldDef::new("int8_value", FieldShape::Primitive(FieldPrimitive::I8)),
                    FieldDef::new("int16_value", FieldShape::Primitive(FieldPrimitive::I16)),
                    FieldDef::new("int32_value", FieldShape::Primitive(FieldPrimitive::I32)),
                    FieldDef::new("int64_value", FieldShape::Primitive(FieldPrimitive::I64)),
                    FieldDef::new("uint8_value", FieldShape::Primitive(FieldPrimitive::U8)),
                    FieldDef::new("uint16_value", FieldShape::Primitive(FieldPrimitive::U16)),
                    FieldDef::new("uint32_value", FieldShape::Primitive(FieldPrimitive::U32)),
                    FieldDef::new("uint64_value", FieldShape::Primitive(FieldPrimitive::U64)),
                    FieldDef::new("float32_value", FieldShape::Primitive(FieldPrimitive::F32)),
                    FieldDef::new("float64_value", FieldShape::Primitive(FieldPrimitive::F64)),
                ],
            }),
        )
        .build()
        .unwrap();

    let runtime = schema_bridge::bundle_to_message_schema(&bundle).unwrap();

    let field_types: Vec<_> = runtime
        .fields()
        .iter()
        .map(|field| &field.field_type)
        .collect();
    assert_eq!(
        field_types,
        vec![
            &FieldType::Bool,
            &FieldType::Int8,
            &FieldType::Int16,
            &FieldType::Int32,
            &FieldType::Int64,
            &FieldType::Uint8,
            &FieldType::Uint16,
            &FieldType::Uint32,
            &FieldType::Uint64,
            &FieldType::Float32,
            &FieldType::Float64,
        ]
    );
}

#[test]
fn bundle_to_message_schema_accepts_canonical_enum_and_container_shapes() {
    let bundle = SchemaBundle::builder("custom_msgs::RobotEnvelope")
        .definition(
            "custom_msgs::RobotEnvelope",
            TypeDef::Struct(StructDef {
                fields: vec![
                    FieldDef::new(
                        "mission_id",
                        FieldShape::Optional {
                            element: Box::new(FieldShape::Primitive(FieldPrimitive::U32)),
                        },
                    ),
                    FieldDef::new(
                        "checkpoints",
                        FieldShape::BoundedSequence {
                            element: Box::new(FieldShape::String),
                            maximum_length: 8,
                        },
                    ),
                    FieldDef::new(
                        "state",
                        FieldShape::Named(TypeName::new("custom_msgs::EnvelopeState").unwrap()),
                    ),
                ],
            }),
        )
        .definition(
            "custom_msgs::EnvelopeState",
            TypeDef::Enum(EnumDef {
                variants: vec![EnumVariantDef::new(
                    "Ready",
                    EnumPayloadDef::Struct(vec![
                        FieldDef::new("priority", FieldShape::Primitive(FieldPrimitive::U32))
                            .with_default(LiteralValue::UInt(7)),
                    ]),
                )],
            }),
        )
        .build()
        .unwrap();

    let runtime = schema_bridge::bundle_to_message_schema(&bundle).unwrap();

    assert!(matches!(
        runtime.fields()[0].field_type,
        FieldType::Optional(_)
    ));
    assert!(matches!(
        runtime.fields()[1].field_type,
        FieldType::BoundedSequence(_, 8)
    ));

    let FieldType::Enum(enum_schema) = &runtime.fields()[2].field_type else {
        panic!("expected enum field");
    };
    let EnumPayloadSchema::Struct(fields) = &enum_schema.variants[0].payload else {
        panic!("expected struct enum payload");
    };

    assert_eq!(fields[0].default_value, Some(DynamicValue::Uint32(7)));
}

#[test]
fn top_level_enum_wrapper_bundle_uses_distinct_enum_definition_name() {
    let runtime = MessageSchema::builder("custom_msgs::RobotState")
        .field(
            "value",
            FieldType::Enum(Arc::new(EnumSchema::new(
                "custom_msgs::RobotState",
                vec![EnumVariantSchema::new(
                    "Error",
                    EnumPayloadSchema::Newtype(Box::new(FieldType::String)),
                )],
            ))),
        )
        .build()
        .unwrap();

    let bundle = schema_bridge::message_schema_to_bundle(&runtime).unwrap();

    let root = bundle.definitions.get(&bundle.root).unwrap();
    let TypeDef::Struct(root) = root else {
        panic!("expected struct root");
    };
    let FieldShape::Named(enum_type_name) = &root.fields[0].shape else {
        panic!("expected named enum field");
    };

    assert_ne!(enum_type_name.as_str(), bundle.root.as_str());
    assert!(bundle.definitions.contains_key(enum_type_name));
}
