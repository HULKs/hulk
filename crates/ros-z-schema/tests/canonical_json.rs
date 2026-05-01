use ros_z_schema::{
    FieldDef, FieldPrimitive, FieldShape, LiteralValue, SchemaBundle, SchemaError, SchemaHash,
    StructDef, TypeDef, TypeName, compute_hash, to_json,
};

#[test]
fn canonical_json_is_stable_and_compact() {
    let bundle = SchemaBundle::builder("ros_z_msgs::std_msgs::String")
        .definition(
            "ros_z_msgs::std_msgs::String",
            TypeDef::Struct(StructDef {
                fields: vec![FieldDef::new("data", FieldShape::String)],
            }),
        )
        .build()
        .unwrap();

    let json = to_json(&bundle).unwrap();
    assert_eq!(
        json,
        r#"{"definitions":{"ros_z_msgs::std_msgs::String":{"kind":"struct","fields":[{"name":"data","shape":{"kind":"string"}}]}},"root":"ros_z_msgs::std_msgs::String"}"#
    );
}

#[test]
fn canonical_json_encodes_map_field_shape() {
    let bundle = SchemaBundle::builder("test_pkg::Lookup")
        .definition(
            "test_pkg::Lookup",
            TypeDef::Struct(StructDef {
                fields: vec![FieldDef::new(
                    "names",
                    FieldShape::Map {
                        key: Box::new(FieldShape::String),
                        value: Box::new(FieldShape::Primitive(FieldPrimitive::U32)),
                    },
                )],
            }),
        )
        .build()
        .unwrap();

    let json = to_json(&bundle).unwrap();
    assert_eq!(
        json,
        r#"{"definitions":{"test_pkg::Lookup":{"kind":"struct","fields":[{"name":"names","shape":{"kind":"map","key":{"kind":"string"},"value":{"kind":"primitive","name":"u32"}}}]}},"root":"test_pkg::Lookup"}"#
    );
}

#[test]
fn canonical_json_encodes_named_container_fields() {
    let bundle = SchemaBundle::builder("test_pkg::Containers")
        .definition(
            "test_pkg::Containers",
            TypeDef::Struct(StructDef {
                fields: vec![
                    FieldDef::new("bounded", FieldShape::BoundedString { maximum_length: 32 }),
                    FieldDef::new(
                        "optional",
                        FieldShape::Optional {
                            element: Box::new(FieldShape::Primitive(FieldPrimitive::U32)),
                        },
                    ),
                    FieldDef::new(
                        "array",
                        FieldShape::Array {
                            element: Box::new(FieldShape::Primitive(FieldPrimitive::U8)),
                            length: 4,
                        },
                    ),
                    FieldDef::new(
                        "sequence",
                        FieldShape::Sequence {
                            element: Box::new(FieldShape::String),
                        },
                    ),
                    FieldDef::new(
                        "bounded_sequence",
                        FieldShape::BoundedSequence {
                            element: Box::new(FieldShape::String),
                            maximum_length: 8,
                        },
                    ),
                ],
            }),
        )
        .build()
        .unwrap();

    let json = to_json(&bundle).unwrap();
    assert_eq!(
        json,
        r#"{"definitions":{"test_pkg::Containers":{"kind":"struct","fields":[{"name":"bounded","shape":{"kind":"bounded_string","maximum_length":32}},{"name":"optional","shape":{"kind":"optional","element":{"kind":"primitive","name":"u32"}}},{"name":"array","shape":{"kind":"array","element":{"kind":"primitive","name":"u8"},"length":4}},{"name":"sequence","shape":{"kind":"sequence","element":{"kind":"string"}}},{"name":"bounded_sequence","shape":{"kind":"bounded_sequence","element":{"kind":"string"},"maximum_length":8}}]}},"root":"test_pkg::Containers"}"#
    );
}

#[test]
fn field_default_changes_schema_hash() {
    let without_default = SchemaBundle::builder("test_pkg::Foo")
        .definition(
            "test_pkg::Foo",
            TypeDef::Struct(StructDef {
                fields: vec![FieldDef::new(
                    "count",
                    FieldShape::Primitive(FieldPrimitive::U32),
                )],
            }),
        )
        .build()
        .unwrap();

    let with_default = SchemaBundle::builder("test_pkg::Foo")
        .definition(
            "test_pkg::Foo",
            TypeDef::Struct(StructDef {
                fields: vec![
                    FieldDef::new("count", FieldShape::Primitive(FieldPrimitive::U32))
                        .with_default(LiteralValue::UInt(7)),
                ],
            }),
        )
        .build()
        .unwrap();

    assert_ne!(compute_hash(&without_default), compute_hash(&with_default));
}

#[test]
fn float_field_defaults_change_schema_hash() {
    let without_default = SchemaBundle::builder("test_pkg::Foo")
        .definition(
            "test_pkg::Foo",
            TypeDef::Struct(StructDef {
                fields: vec![FieldDef::new(
                    "gain",
                    FieldShape::Primitive(FieldPrimitive::F64),
                )],
            }),
        )
        .build()
        .unwrap();

    let with_default = SchemaBundle::builder("test_pkg::Foo")
        .definition(
            "test_pkg::Foo",
            TypeDef::Struct(StructDef {
                fields: vec![
                    FieldDef::new("gain", FieldShape::Primitive(FieldPrimitive::F64))
                        .with_default(LiteralValue::Float64(1.25)),
                ],
            }),
        )
        .build()
        .unwrap();

    assert_ne!(compute_hash(&without_default), compute_hash(&with_default));
}

#[test]
fn hash_strings_use_rzhs01_prefix() {
    let hash = compute_hash(
        &SchemaBundle::builder("ros_z_msgs::std_msgs::String")
            .definition(
                "ros_z_msgs::std_msgs::String",
                TypeDef::Struct(StructDef {
                    fields: vec![FieldDef::new("data", FieldShape::String)],
                }),
            )
            .build()
            .unwrap(),
    );

    assert!(hash.to_hash_string().starts_with("RZHS01_"));
}

#[test]
fn canonical_json_rejects_non_finite_float_literal() {
    let err = to_json(&LiteralValue::Float64(f64::NAN)).unwrap_err();

    assert_eq!(
        err,
        SchemaError::InvalidLiteralValue("non-finite float64 literal".into())
    );
}

#[test]
fn canonical_json_rejects_non_finite_float_array_literal() {
    let err = to_json(&LiteralValue::Float32Array(vec![f32::INFINITY])).unwrap_err();

    assert_eq!(
        err,
        SchemaError::InvalidLiteralValue("non-finite float32[] literal".into())
    );
}

#[test]
fn string_fields_reject_non_string_defaults() {
    let bundle = SchemaBundle::builder("ros_z_msgs::std_msgs::String")
        .definition(
            "ros_z_msgs::std_msgs::String",
            TypeDef::Struct(StructDef {
                fields: vec![
                    FieldDef::new("data", FieldShape::String).with_default(LiteralValue::UInt(7)),
                ],
            }),
        )
        .build();

    assert_eq!(
        bundle,
        Err(SchemaError::InvalidFieldDefault {
            field_name: "data".into(),
            shape: "string".into(),
            default: "uint".into(),
        })
    );
}

#[test]
fn named_fields_reject_defaults() {
    let bundle = SchemaBundle::builder("test_pkg::Foo")
        .definition(
            "test_pkg::Foo",
            TypeDef::Struct(StructDef {
                fields: vec![
                    FieldDef::new(
                        "child",
                        FieldShape::Named(TypeName::new("test_pkg::Bar").unwrap()),
                    )
                    .with_default(LiteralValue::String("x".into())),
                ],
            }),
        )
        .definition(
            "test_pkg::Bar",
            TypeDef::Struct(StructDef { fields: vec![] }),
        )
        .build();

    assert_eq!(
        bundle,
        Err(SchemaError::InvalidFieldDefault {
            field_name: "child".into(),
            shape: "named".into(),
            default: "string".into(),
        })
    );
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
