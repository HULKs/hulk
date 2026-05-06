use ros_z_schema::{
    FieldDef, NamedTypeDef, PrimitiveTypeDef, RootTypeName, SchemaBundle, SchemaError, SchemaHash,
    SequenceLengthDef, StructDef, TypeDef, TypeName, compute_hash, to_json,
};

fn string_bundle(root_name: &str) -> SchemaBundle {
    SchemaBundle {
        root_name: RootTypeName::new(root_name).unwrap(),
        root: TypeDef::StructRef(TypeName::new("ros_z_msgs::std_msgs::String").unwrap()),
        definitions: [(
            TypeName::new("ros_z_msgs::std_msgs::String").unwrap(),
            NamedTypeDef::Struct(StructDef {
                fields: vec![FieldDef::new("data", TypeDef::String)],
            }),
        )]
        .into(),
    }
}

fn struct_bundle(type_name: &str, fields: Vec<FieldDef>) -> Result<SchemaBundle, SchemaError> {
    bundle(
        type_name,
        TypeDef::StructRef(TypeName::new(type_name).unwrap()),
        vec![(
            TypeName::new(type_name).unwrap(),
            NamedTypeDef::Struct(StructDef { fields }),
        )],
    )
}

fn bundle(
    root_name: &str,
    root: TypeDef,
    definitions: Vec<(TypeName, NamedTypeDef)>,
) -> Result<SchemaBundle, SchemaError> {
    let bundle = SchemaBundle {
        root_name: RootTypeName::new(root_name).unwrap(),
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
        r#"{"definitions":{"ros_z_msgs::std_msgs::String":{"kind":"struct","fields":[{"name":"data","shape":{"kind":"string"}}]}},"root":{"kind":"struct_ref","type":"ros_z_msgs::std_msgs::String"}}"#
    );
    assert!(!json.contains("root_name"));
}

#[test]
fn root_name_does_not_affect_schema_hash() {
    let left = string_bundle("ros_z_msgs::std_msgs::String");
    let right = string_bundle("display_alias::String");

    assert_eq!(compute_hash(&left), compute_hash(&right));
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
        r#"{"definitions":{"test_pkg::Lookup":{"kind":"struct","fields":[{"name":"names","shape":{"kind":"map","key":{"kind":"string"},"value":{"kind":"primitive","name":"u32"}}}]}},"root":{"kind":"struct_ref","type":"test_pkg::Lookup"}}"#
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
        r#"{"definitions":{"test_pkg::Containers":{"kind":"struct","fields":[{"name":"optional","shape":{"kind":"optional","element":{"kind":"primitive","name":"u32"}}},{"name":"fixed_sequence","shape":{"kind":"sequence","length":{"kind":"fixed","value":4},"element":{"kind":"primitive","name":"u8"}}},{"name":"sequence","shape":{"kind":"sequence","length":{"kind":"dynamic"},"element":{"kind":"string"}}}]}},"root":{"kind":"struct_ref","type":"test_pkg::Containers"}}"#
    );
}

#[test]
fn primitive_root_json_is_inline() {
    let bundle = SchemaBundle::new(
        RootTypeName::new("u8").unwrap(),
        TypeDef::Primitive(PrimitiveTypeDef::U8),
    )
    .unwrap();

    assert_eq!(
        to_json(&bundle).unwrap(),
        r#"{"definitions":{},"root":{"kind":"primitive","name":"u8"}}"#
    );
}

#[test]
fn hash_strings_use_rzhs01_prefix() {
    let hash = compute_hash(&string_bundle("ros_z_msgs::std_msgs::String"));

    assert!(hash.to_hash_string().starts_with("RZHS01_"));
}

#[test]
fn schema_hash_roundtrips_rzhs01_strings() {
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
    assert!(SchemaHash::from_hash_string("RZHS01_deadbeefdeadbeef").is_err());
}

#[test]
fn schema_hash_rejects_invalid_hex() {
    assert!(
        SchemaHash::from_hash_string(
            "RZHS01_ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ"
        )
        .is_err()
    );
}
