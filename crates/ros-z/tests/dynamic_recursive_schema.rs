use std::sync::Arc;

use ros_z::dynamic::{DynamicCdrCodec, DynamicPayload, DynamicStruct, DynamicValue};
use ros_z_schema::{
    EnumDef, EnumPayloadDef, EnumVariantDef, FieldDef, PrimitiveTypeDef, SchemaBundle,
    SequenceLengthDef, StructDef, TypeDef, TypeDefinition, TypeName,
};

fn recursive_schema_with_child_shape(child_shape: TypeDef) -> Arc<SchemaBundle> {
    let node = TypeName::new("test::Node").unwrap();
    Arc::new(SchemaBundle {
        root: TypeDef::Named(node.clone()),
        definitions: [(
            node,
            TypeDefinition::Struct(StructDef {
                fields: vec![
                    FieldDef::new("name", TypeDef::String),
                    FieldDef::new("children", child_shape),
                ],
            }),
        )]
        .into(),
    })
}

fn recursive_sequence_schema() -> Arc<SchemaBundle> {
    let node = TypeName::new("test::Node").unwrap();
    recursive_schema_with_child_shape(TypeDef::Sequence {
        element: Box::new(TypeDef::Named(node)),
        length: SequenceLengthDef::Dynamic,
    })
}

fn recursive_required_schema() -> Arc<SchemaBundle> {
    let node = TypeName::new("test::Node").unwrap();
    recursive_schema_with_child_shape(TypeDef::Named(node))
}

fn recursive_boundary_schema(child_shape: TypeDef) -> Arc<SchemaBundle> {
    recursive_schema_with_child_shape(child_shape)
}

fn nested_schema(leaf_shape: TypeDef) -> Arc<SchemaBundle> {
    let root = TypeName::new("test::Root").unwrap();
    let child = TypeName::new("test::Child").unwrap();
    let leaf = TypeName::new("test::Leaf").unwrap();
    Arc::new(SchemaBundle {
        root: TypeDef::Named(root.clone()),
        definitions: [
            (
                root,
                TypeDefinition::Struct(StructDef {
                    fields: vec![FieldDef::new("child", TypeDef::Named(child.clone()))],
                }),
            ),
            (
                child,
                TypeDefinition::Struct(StructDef {
                    fields: vec![FieldDef::new("leaf", TypeDef::Named(leaf.clone()))],
                }),
            ),
            (
                leaf,
                TypeDefinition::Struct(StructDef {
                    fields: vec![FieldDef::new("value", leaf_shape)],
                }),
            ),
        ]
        .into(),
    })
}

#[test]
fn constructs_finite_recursive_value_with_shared_schema_bundle() {
    let schema = recursive_sequence_schema();
    schema.validate().unwrap();
    let node = TypeName::new("test::Node").unwrap();

    let child = DynamicStruct::new(
        Arc::clone(&schema),
        node.clone(),
        vec![
            DynamicValue::String("child".into()),
            DynamicValue::Sequence(vec![]),
        ],
    )
    .unwrap();
    let root = DynamicStruct::new(
        Arc::clone(&schema),
        node,
        vec![
            DynamicValue::String("root".into()),
            DynamicValue::Sequence(vec![DynamicValue::Struct(Box::new(child))]),
        ],
    )
    .unwrap();

    DynamicPayload::new(schema, DynamicValue::Struct(Box::new(root))).unwrap();
}

#[test]
fn required_recursive_default_returns_error() {
    let schema = recursive_required_schema();

    let result = DynamicPayload::default_for_schema(schema);

    assert!(result.is_err());
}

#[test]
fn required_recursive_enum_default_returns_error() {
    let state = TypeName::new("test::State").unwrap();
    let schema = Arc::new(SchemaBundle {
        root: TypeDef::Named(state.clone()),
        definitions: [(
            state.clone(),
            TypeDefinition::Enum(EnumDef {
                variants: vec![EnumVariantDef::new(
                    "Again",
                    EnumPayloadDef::Newtype(TypeDef::Named(state)),
                )],
            }),
        )]
        .into(),
    });

    let result = DynamicPayload::default_for_schema(schema);

    assert!(result.is_err());
}

#[test]
fn recursive_default_boundaries_are_finite() {
    let node = TypeName::new("test::Node").unwrap();
    let cases = [
        TypeDef::Optional(Box::new(TypeDef::Named(node.clone()))),
        TypeDef::Map {
            key: Box::new(TypeDef::Primitive(PrimitiveTypeDef::U32)),
            value: Box::new(TypeDef::Named(node.clone())),
        },
        TypeDef::Sequence {
            element: Box::new(TypeDef::Named(node.clone())),
            length: SequenceLengthDef::Dynamic,
        },
        TypeDef::Sequence {
            element: Box::new(TypeDef::Named(node)),
            length: SequenceLengthDef::Fixed(0),
        },
    ];

    for child_shape in cases {
        let schema = recursive_boundary_schema(child_shape);

        DynamicPayload::default_for_schema(schema).unwrap();
    }
}

#[test]
fn cdr_round_trips_finite_recursive_value_preserving_nominal_type_names() {
    let schema = recursive_sequence_schema();
    let node = TypeName::new("test::Node").unwrap();
    let child = DynamicStruct::new(
        Arc::clone(&schema),
        node.clone(),
        vec![
            DynamicValue::String("child".into()),
            DynamicValue::Sequence(vec![]),
        ],
    )
    .unwrap();
    let root = DynamicStruct::new(
        Arc::clone(&schema),
        node.clone(),
        vec![
            DynamicValue::String("root".into()),
            DynamicValue::Sequence(vec![DynamicValue::Struct(Box::new(child))]),
        ],
    )
    .unwrap();
    let payload =
        DynamicPayload::new(Arc::clone(&schema), DynamicValue::Struct(Box::new(root))).unwrap();

    let bytes = DynamicCdrCodec::try_serialize_payload(&payload).unwrap();
    let decoded = DynamicCdrCodec::decode(&bytes, &schema).unwrap();

    let DynamicValue::Struct(decoded_root) = decoded.value else {
        panic!("expected root struct");
    };
    assert_eq!(decoded_root.type_name(), &node);
    let children = decoded_root.get_dynamic("children").unwrap();
    let DynamicValue::Sequence(children) = children else {
        panic!("expected children sequence");
    };
    let DynamicValue::Struct(decoded_child) = &children[0] else {
        panic!("expected child struct");
    };
    assert_eq!(decoded_child.type_name(), &node);
}

#[test]
fn nested_struct_validation_uses_traversal_schema_bundle() {
    let advertised_schema = nested_schema(TypeDef::String);
    let conflicting_schema = nested_schema(TypeDef::Primitive(PrimitiveTypeDef::U32));
    let child = TypeName::new("test::Child").unwrap();
    let leaf = TypeName::new("test::Leaf").unwrap();
    let root = TypeName::new("test::Root").unwrap();
    let leaf_value = DynamicStruct::new(
        Arc::clone(&conflicting_schema),
        leaf,
        vec![DynamicValue::Uint32(7)],
    )
    .unwrap();
    let child_value = DynamicStruct::new(
        conflicting_schema,
        child,
        vec![DynamicValue::Struct(Box::new(leaf_value))],
    )
    .unwrap();

    let result = DynamicStruct::new(
        advertised_schema,
        root,
        vec![DynamicValue::Struct(Box::new(child_value))],
    );

    assert!(result.is_err());
}

#[test]
fn deserialization_rejects_invalid_schema_before_traversal() {
    let root = TypeName::new("test::Invalid").unwrap();
    let schema = Arc::new(SchemaBundle {
        root: TypeDef::Named(root.clone()),
        definitions: [(
            root,
            TypeDefinition::Struct(StructDef {
                fields: vec![
                    FieldDef::new("value", TypeDef::Primitive(PrimitiveTypeDef::U32)),
                    FieldDef::new("value", TypeDef::Primitive(PrimitiveTypeDef::U32)),
                ],
            }),
        )]
        .into(),
    });

    let result = DynamicCdrCodec::decode(&[0x00, 0x01, 0x00, 0x00, 0, 0, 0, 0], &schema);

    assert!(result.unwrap_err().to_string().contains("duplicate field"));
}

#[test]
fn dynamic_structs_and_payloads_with_distinct_equal_schema_arcs_compare_equal() {
    let left_schema = recursive_sequence_schema();
    let right_schema = recursive_sequence_schema();
    assert!(!Arc::ptr_eq(&left_schema, &right_schema));
    let node = TypeName::new("test::Node").unwrap();
    let left = DynamicStruct::new(
        Arc::clone(&left_schema),
        node.clone(),
        vec![
            DynamicValue::String("root".into()),
            DynamicValue::Sequence(vec![]),
        ],
    )
    .unwrap();
    let right = DynamicStruct::new(
        Arc::clone(&right_schema),
        node,
        vec![
            DynamicValue::String("root".into()),
            DynamicValue::Sequence(vec![]),
        ],
    )
    .unwrap();

    assert_eq!(left, right);

    let left_payload =
        DynamicPayload::new(left_schema, DynamicValue::Struct(Box::new(left))).unwrap();
    let right_payload =
        DynamicPayload::new(right_schema, DynamicValue::Struct(Box::new(right))).unwrap();

    assert_eq!(left_payload, right_payload);
}
