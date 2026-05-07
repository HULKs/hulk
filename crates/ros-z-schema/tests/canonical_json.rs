use ros_z_schema::{
    FieldDef, PrimitiveTypeDef, SchemaBundle, SchemaError, SchemaHash, SequenceLengthDef,
    StructDef, TypeDef, TypeDefinition, TypeName, compute_hash, to_json,
};

fn node_trace_bundle() -> SchemaBundle {
    let node = TypeName::new("types::behavior_tree::NodeTrace").unwrap();
    let status = TypeName::new("types::behavior_tree::Status").unwrap();
    SchemaBundle {
        root: TypeDef::Named(node.clone()),
        definitions: [
            (
                node.clone(),
                TypeDefinition::Struct(StructDef {
                    fields: vec![
                        FieldDef::new("name", TypeDef::String),
                        FieldDef::new("status", TypeDef::Named(status.clone())),
                        FieldDef::new(
                            "children",
                            TypeDef::Sequence {
                                element: Box::new(TypeDef::Named(node.clone())),
                                length: SequenceLengthDef::Dynamic,
                            },
                        ),
                    ],
                }),
            ),
            (
                status,
                TypeDefinition::Enum(ros_z_schema::EnumDef {
                    variants: vec![ros_z_schema::EnumVariantDef::new(
                        "Idle",
                        ros_z_schema::EnumPayloadDef::Unit,
                    )],
                }),
            ),
        ]
        .into(),
    }
}

fn string_bundle(root_name: &str) -> SchemaBundle {
    let type_name = TypeName::new(root_name).unwrap();
    SchemaBundle {
        root: TypeDef::Named(type_name.clone()),
        definitions: [(
            type_name,
            TypeDefinition::Struct(StructDef {
                fields: vec![FieldDef::new("data", TypeDef::String)],
            }),
        )]
        .into(),
    }
}

fn struct_bundle(type_name: &str, fields: Vec<FieldDef>) -> Result<SchemaBundle, SchemaError> {
    bundle(
        TypeDef::Named(TypeName::new(type_name).unwrap()),
        vec![(
            TypeName::new(type_name).unwrap(),
            TypeDefinition::Struct(StructDef { fields }),
        )],
    )
}

fn bundle(
    root: TypeDef,
    definitions: Vec<(TypeName, TypeDefinition)>,
) -> Result<SchemaBundle, SchemaError> {
    let bundle = SchemaBundle {
        root,
        definitions: definitions
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>()
            .into(),
    };
    bundle.validate()?;
    Ok(bundle)
}

#[test]
fn canonical_json_is_stable_compact_and_excludes_root_name() {
    let bundle = string_bundle("ros_z_msgs::std_msgs::String");

    let json = to_json(&bundle).unwrap();
    assert_eq!(
        json,
        r#"{"definitions":{"ros_z_msgs::std_msgs::String":{"kind":"struct","fields":[{"name":"data","shape":{"kind":"string"}}]}},"root":{"kind":"named","type":"ros_z_msgs::std_msgs::String"}}"#
    );
}

#[test]
fn schema_hash_includes_named_root_identity() {
    let left = string_bundle("ros_z_msgs::std_msgs::String");
    let right = string_bundle("display_alias::String");

    assert_ne!(compute_hash(&left), compute_hash(&right));
}

#[test]
fn canonical_json_encodes_map_field_shape() {
    let bundle = struct_bundle(
        "test_pkg::Lookup",
        vec![FieldDef::new(
            "names",
            TypeDef::Map {
                key: Box::new(TypeDef::String),
                value: Box::new(TypeDef::Primitive(PrimitiveTypeDef::U32)),
            },
        )],
    )
    .unwrap();

    let json = to_json(&bundle).unwrap();
    assert_eq!(
        json,
        r#"{"definitions":{"test_pkg::Lookup":{"kind":"struct","fields":[{"name":"names","shape":{"kind":"map","key":{"kind":"string"},"value":{"kind":"primitive","name":"u32"}}}]}},"root":{"kind":"named","type":"test_pkg::Lookup"}}"#
    );
}

#[test]
fn canonical_json_encodes_named_container_fields() {
    let bundle = struct_bundle(
        "test_pkg::Containers",
        vec![
            FieldDef::new(
                "optional",
                TypeDef::Optional(Box::new(TypeDef::Primitive(PrimitiveTypeDef::U32))),
            ),
            FieldDef::new(
                "fixed_sequence",
                TypeDef::Sequence {
                    element: Box::new(TypeDef::Primitive(PrimitiveTypeDef::U8)),
                    length: SequenceLengthDef::Fixed(4),
                },
            ),
            FieldDef::new(
                "sequence",
                TypeDef::Sequence {
                    element: Box::new(TypeDef::String),
                    length: SequenceLengthDef::Dynamic,
                },
            ),
        ],
    )
    .unwrap();

    let json = to_json(&bundle).unwrap();
    assert_eq!(
        json,
        r#"{"definitions":{"test_pkg::Containers":{"kind":"struct","fields":[{"name":"optional","shape":{"kind":"optional","element":{"kind":"primitive","name":"u32"}}},{"name":"fixed_sequence","shape":{"kind":"sequence","length":{"kind":"fixed","value":4},"element":{"kind":"primitive","name":"u8"}}},{"name":"sequence","shape":{"kind":"sequence","length":{"kind":"dynamic"},"element":{"kind":"string"}}}]}},"root":{"kind":"named","type":"test_pkg::Containers"}}"#
    );
}

#[test]
fn primitive_root_json_is_inline() {
    let bundle = SchemaBundle::new(TypeDef::Primitive(PrimitiveTypeDef::U8)).unwrap();

    assert_eq!(
        to_json(&bundle).unwrap(),
        r#"{"definitions":{},"root":{"kind":"primitive","name":"u8"}}"#
    );
}

#[test]
fn schema_hash_roundtrips_rzhs02_strings() {
    let hash = SchemaHash([0x12; 32]);
    let encoded = hash.to_hash_string();

    assert_eq!(SchemaHash::from_hash_string(&encoded), Ok(hash));
}

#[test]
fn schema_hash_rejects_wrong_prefix() {
    assert!(
        SchemaHash::from_hash_string(
            "OLDS01_1212121212121212121212121212121212121212121212121212121212121212"
        )
        .is_err()
    );
}

#[test]
fn schema_hash_rejects_wrong_length() {
    assert!(SchemaHash::from_hash_string("RZHS02_deadbeefdeadbeef").is_err());
}

#[test]
fn schema_hash_rejects_invalid_hex() {
    assert!(
        SchemaHash::from_hash_string(
            "RZHS02_ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ"
        )
        .is_err()
    );
}

#[test]
fn canonical_json_uses_named_references() {
    let bundle = node_trace_bundle();
    bundle.validate().unwrap();

    let json = to_json(&bundle).unwrap();

    assert_eq!(
        json,
        r#"{"definitions":{"types::behavior_tree::NodeTrace":{"kind":"struct","fields":[{"name":"name","shape":{"kind":"string"}},{"name":"status","shape":{"kind":"named","type":"types::behavior_tree::Status"}},{"name":"children","shape":{"kind":"sequence","length":{"kind":"dynamic"},"element":{"kind":"named","type":"types::behavior_tree::NodeTrace"}}}]},"types::behavior_tree::Status":{"kind":"enum","variants":[{"name":"Idle","payload":{"kind":"unit"}}]}},"root":{"kind":"named","type":"types::behavior_tree::NodeTrace"}}"#
    );
}

#[test]
fn schema_hash_strings_use_rzhs02_prefix() {
    let hash = compute_hash(&node_trace_bundle());

    assert!(hash.to_hash_string().starts_with("RZHS02_"));
    assert!(SchemaHash::from_hash_string(&hash.to_hash_string()).is_ok());
    assert!(
        SchemaHash::from_hash_string(
            "RZHS01_0000000000000000000000000000000000000000000000000000000000000000"
        )
        .is_err()
    );
}

#[test]
fn known_recursive_bundle_hash_is_stable() {
    let hash = compute_hash(&node_trace_bundle()).to_hash_string();

    assert_eq!(
        hash,
        "RZHS02_98c71138db5c8fb3e759e27cf4e37a102dfd7b0793197b8443171545393904d9"
    );
}

#[test]
fn validation_rejects_unreachable_definitions() {
    let mut bundle = node_trace_bundle();
    bundle.definitions.insert(
        TypeName::new("test::Unused").unwrap(),
        TypeDefinition::Struct(StructDef { fields: vec![] }),
    );

    assert_eq!(
        bundle.validate(),
        Err(SchemaError::UnreachableDefinition(
            TypeName::new("test::Unused").unwrap()
        ))
    );
}

#[test]
fn serde_round_trip_requires_explicit_validation() {
    let bundle = node_trace_bundle();
    let json = serde_json::to_string(&bundle).unwrap();
    let decoded: SchemaBundle = serde_json::from_str(&json).unwrap();

    decoded.validate().unwrap();
    assert_eq!(decoded, bundle);
}
