use ros_z_schema::{
    ActionDef, EnumDef, EnumPayloadDef, EnumVariantDef, FieldDef, LiteralValue, NamedTypeDef,
    PrimitiveTypeDef, RootTypeName, SchemaBundle, SchemaError, SequenceLengthDef, ServiceDef,
    StructDef, TypeDef, TypeName,
};

#[test]
fn type_names_are_opaque_non_empty_strings() {
    let accepted = [
        "hulk::Point",
        "std_msgs/String",
        "String",
        "Option<u8,u16>",
        "HashMap<String>",
        " hulk::Message ",
        "Foo;Bar",
        "[u8;16]",
        "16",
        "_",
        "test_pkg::type::Message",
    ];

    for value in accepted {
        assert_eq!(TypeName::new(value).unwrap().as_str(), value);
        assert_eq!(RootTypeName::new(value).unwrap().as_str(), value);
    }

    assert_eq!(
        TypeName::new(""),
        Err(SchemaError::InvalidTypeName("".into()))
    );
    assert_eq!(
        RootTypeName::new(""),
        Err(SchemaError::InvalidRootTypeName("".into()))
    );
}

#[test]
fn schema_bundle_accepts_primitive_root_without_named_definition() {
    let bundle = SchemaBundle::new(
        RootTypeName::new("u8").unwrap(),
        TypeDef::Primitive(PrimitiveTypeDef::U8),
    )
    .unwrap();

    assert!(bundle.definitions().is_empty());
    assert_eq!(bundle.root_name().as_str(), "u8");
}

#[test]
fn schema_bundle_validates_named_refs() {
    let bundle = SchemaBundle {
        root_name: RootTypeName::new("hulk::Pose").unwrap(),
        root: TypeDef::StructRef(TypeName::new("hulk::Pose").unwrap()),
        definitions: [(
            TypeName::new("hulk::Pose").unwrap(),
            NamedTypeDef::Struct(StructDef {
                fields: vec![FieldDef::new(
                    "x",
                    TypeDef::Primitive(PrimitiveTypeDef::F64),
                )],
            }),
        )]
        .into(),
    };

    assert!(bundle.validate().is_ok());
}

#[test]
fn schema_bundle_new_validates_references() {
    let bundle = SchemaBundle::new(
        RootTypeName::new("hulk::Pose").unwrap(),
        TypeDef::StructRef(TypeName::new("hulk::Pose").unwrap()),
    );

    assert_eq!(
        bundle,
        Err(SchemaError::MissingDefinition(
            TypeName::new("hulk::Pose").unwrap()
        ))
    );
}

#[test]
fn schema_bundle_direct_construction_creates_named_struct_root() {
    let type_name = TypeName::new("test_msgs::Pose").unwrap();
    let definition = NamedTypeDef::Struct(StructDef {
        fields: vec![FieldDef::new(
            "x",
            TypeDef::Primitive(PrimitiveTypeDef::F64),
        )],
    });
    let bundle = SchemaBundle {
        root_name: RootTypeName::new("test_msgs::Pose").unwrap(),
        root: TypeDef::StructRef(type_name.clone()),
        definitions: [(type_name.clone(), definition.clone())].into(),
    };

    assert!(bundle.validate().is_ok());
    assert_eq!(bundle.root_name.as_str(), "test_msgs::Pose");
    assert_eq!(bundle.root, TypeDef::StructRef(type_name.clone()));
    assert_eq!(bundle.definitions().get(&type_name), Some(&definition));
}

#[test]
fn schema_bundle_with_definition_collapses_duplicate_same_definition() {
    let type_name = TypeName::new("hulk::Pose").unwrap();
    let definition = NamedTypeDef::Struct(StructDef {
        fields: vec![FieldDef::new(
            "x",
            TypeDef::Primitive(PrimitiveTypeDef::F64),
        )],
    });
    let bundle = SchemaBundle {
        root_name: RootTypeName::new("hulk::Pose").unwrap(),
        root: TypeDef::StructRef(type_name.clone()),
        definitions: [(type_name.clone(), definition.clone())].into(),
    }
    .with_definition(type_name.clone(), definition.clone())
    .unwrap();

    assert_eq!(bundle.definitions().len(), 1);
    assert_eq!(bundle.definitions().get(&type_name), Some(&definition));
}

#[test]
fn schema_bundle_with_definition_accepts_new_reachable_definition() {
    let pose = TypeName::new("hulk::Pose").unwrap();
    let point = TypeName::new("hulk::Point").unwrap();
    let bundle = SchemaBundle {
        root_name: RootTypeName::new("hulk::Pose").unwrap(),
        root: TypeDef::StructRef(pose.clone()),
        definitions: [(
            pose.clone(),
            NamedTypeDef::Struct(StructDef {
                fields: vec![FieldDef::new("position", TypeDef::StructRef(point.clone()))],
            }),
        )]
        .into(),
    }
    .with_definition(
        point.clone(),
        NamedTypeDef::Struct(StructDef {
            fields: vec![FieldDef::new(
                "x",
                TypeDef::Primitive(PrimitiveTypeDef::F64),
            )],
        }),
    )
    .unwrap();

    assert!(bundle.definitions().contains_key(&pose));
    assert!(bundle.definitions().contains_key(&point));
}

#[test]
fn schema_bundle_with_definition_rejects_conflicting_definitions() {
    let type_name = TypeName::new("hulk::Pose").unwrap();
    let bundle = SchemaBundle {
        root_name: RootTypeName::new("hulk::Pose").unwrap(),
        root: TypeDef::StructRef(type_name.clone()),
        definitions: [(
            type_name.clone(),
            NamedTypeDef::Struct(StructDef {
                fields: vec![FieldDef::new(
                    "x",
                    TypeDef::Primitive(PrimitiveTypeDef::F64),
                )],
            }),
        )]
        .into(),
    }
    .with_definition(
        type_name.clone(),
        NamedTypeDef::Struct(StructDef {
            fields: vec![FieldDef::new(
                "y",
                TypeDef::Primitive(PrimitiveTypeDef::F64),
            )],
        }),
    );

    assert_eq!(bundle, Err(SchemaError::ConflictingDefinition(type_name)));
}

#[test]
fn opaque_type_names_accept_non_empty_strings() {
    assert!(TypeName::new("hulk::vision::MyMessage").is_ok());
    assert!(TypeName::new("ros_z_msgs::std_msgs::String").is_ok());
    assert!(TypeName::new("test_pkg::r#type::Message").is_ok());
    assert!(TypeName::new("test_pkg::r#async::Message").is_ok());
    assert!(TypeName::new("hulk::Message<T>").is_ok());
    assert!(TypeName::new("hulk::Message<alloc::string::String>").is_ok());
    assert!(TypeName::new("hulk::Envelope<Option<Vec<hulk::Point>>>").is_ok());
    assert!(TypeName::new("hulk::Envelope<HashMap<String,Vec<u32>>>").is_ok());
    assert!(TypeName::new("hulk::Envelope<[u8;16]>").is_ok());
}

#[test]
fn opaque_type_names_reject_empty_strings() {
    assert!(TypeName::new("").is_err());
}

#[test]
fn opaque_type_names_deserialize_non_empty_strings() {
    let type_name: TypeName =
        serde_json::from_str("\"hulk::Envelope<Option<Vec<hulk::Point>>>\"").unwrap();

    assert_eq!(
        type_name.as_str(),
        "hulk::Envelope<Option<Vec<hulk::Point>>>"
    );
}

#[test]
fn opaque_type_names_reject_empty_deserialization() {
    assert!(serde_json::from_str::<TypeName>("\"\"").is_err());
}

#[test]
fn primitive_type_def_uses_rust_native_spelling() {
    assert_eq!(PrimitiveTypeDef::Bool.as_str(), "bool");
    assert_eq!(PrimitiveTypeDef::I8.as_str(), "i8");
    assert_eq!(PrimitiveTypeDef::I16.as_str(), "i16");
    assert_eq!(PrimitiveTypeDef::I32.as_str(), "i32");
    assert_eq!(PrimitiveTypeDef::I64.as_str(), "i64");
    assert_eq!(PrimitiveTypeDef::U8.as_str(), "u8");
    assert_eq!(PrimitiveTypeDef::U16.as_str(), "u16");
    assert_eq!(PrimitiveTypeDef::U32.as_str(), "u32");
    assert_eq!(PrimitiveTypeDef::U64.as_str(), "u64");
    assert_eq!(PrimitiveTypeDef::F32.as_str(), "f32");
    assert_eq!(PrimitiveTypeDef::F64.as_str(), "f64");
}

#[test]
fn primitive_type_def_from_str_accepts_only_rust_native_names() {
    assert_eq!("u8".parse::<PrimitiveTypeDef>(), Ok(PrimitiveTypeDef::U8));
    assert_eq!("f64".parse::<PrimitiveTypeDef>(), Ok(PrimitiveTypeDef::F64));

    assert!("uint8".parse::<PrimitiveTypeDef>().is_err());
    assert!("byte".parse::<PrimitiveTypeDef>().is_err());
    assert!("char".parse::<PrimitiveTypeDef>().is_err());
    assert!("float64".parse::<PrimitiveTypeDef>().is_err());
}

#[test]
fn primitive_type_def_from_ros_name_normalizes_ros_boundary_names() {
    assert_eq!(
        PrimitiveTypeDef::from_ros_name("uint8"),
        Some(PrimitiveTypeDef::U8)
    );
    assert_eq!(
        PrimitiveTypeDef::from_ros_name("byte"),
        Some(PrimitiveTypeDef::U8)
    );
    assert_eq!(
        PrimitiveTypeDef::from_ros_name("char"),
        Some(PrimitiveTypeDef::U8)
    );
    assert_eq!(
        PrimitiveTypeDef::from_ros_name("float64"),
        Some(PrimitiveTypeDef::F64)
    );
    assert_eq!(PrimitiveTypeDef::from_ros_name("u8"), None);
}

#[test]
fn primitive_type_def_serde_uses_rust_native_names_without_ros_aliases() {
    let encoded = serde_json::to_string(&PrimitiveTypeDef::U8).unwrap();
    assert_eq!(encoded, "\"u8\"");

    assert_eq!(
        serde_json::from_str::<PrimitiveTypeDef>("\"u8\"").unwrap(),
        PrimitiveTypeDef::U8
    );
    for ros_name in ["uint8", "byte", "char", "float64"] {
        let encoded = serde_json::to_string(ros_name).unwrap();
        assert!(serde_json::from_str::<PrimitiveTypeDef>(&encoded).is_err());
    }
}

#[test]
fn schema_bundle_validate_rejects_missing_named_reference() {
    let bundle = SchemaBundle {
        root_name: RootTypeName::new("geometry_msgs::Twist").unwrap(),
        root: TypeDef::StructRef(TypeName::new("geometry_msgs::Twist").unwrap()),
        definitions: [(
            TypeName::new("geometry_msgs::Twist").unwrap(),
            NamedTypeDef::Struct(StructDef {
                fields: vec![FieldDef::new(
                    "linear",
                    TypeDef::StructRef(TypeName::new("geometry_msgs::Vector3").unwrap()),
                )],
            }),
        )]
        .into(),
    };

    assert_eq!(
        bundle.validate(),
        Err(SchemaError::MissingDefinition(
            TypeName::new("geometry_msgs::Vector3").unwrap()
        ))
    );
}

#[test]
fn schema_bundle_validate_rejects_reference_kind_mismatch() {
    let bundle = SchemaBundle {
        root_name: RootTypeName::new("custom_msgs::State").unwrap(),
        root: TypeDef::StructRef(TypeName::new("custom_msgs::State").unwrap()),
        definitions: [(
            TypeName::new("custom_msgs::State").unwrap(),
            NamedTypeDef::Enum(EnumDef { variants: vec![] }),
        )]
        .into(),
    };

    assert!(matches!(
        bundle.validate(),
        Err(SchemaError::ReferenceKindMismatch {
            expected: "struct",
            ..
        })
    ));
}

#[test]
fn schema_bundle_validate_rejects_empty_enum_definition() {
    let state = TypeName::new("custom_msgs::State").unwrap();
    let bundle = SchemaBundle {
        root_name: RootTypeName::new("custom_msgs::State").unwrap(),
        root: TypeDef::EnumRef(state.clone()),
        definitions: [(
            state.clone(),
            NamedTypeDef::Enum(EnumDef { variants: vec![] }),
        )]
        .into(),
    };

    assert_eq!(bundle.validate(), Err(SchemaError::EmptyEnum(state)));
}

#[test]
fn field_references_validate_struct_and_enum_kinds_distinctly() {
    let wrapper = TypeName::new("custom_msgs::Wrapper").unwrap();
    let state = TypeName::new("custom_msgs::State").unwrap();
    let bundle = SchemaBundle {
        root_name: RootTypeName::new("custom_msgs::Wrapper").unwrap(),
        root: TypeDef::StructRef(wrapper.clone()),
        definitions: [
            (
                wrapper,
                NamedTypeDef::Struct(StructDef {
                    fields: vec![FieldDef::new("state", TypeDef::EnumRef(state.clone()))],
                }),
            ),
            (state, NamedTypeDef::Struct(StructDef { fields: vec![] })),
        ]
        .into(),
    };

    assert!(matches!(
        bundle.validate(),
        Err(SchemaError::ReferenceKindMismatch {
            expected: "enum",
            ..
        })
    ));
}

#[test]
fn schema_bundle_validate_accepts_first_class_extended_shapes_and_enums() {
    let envelope_type = TypeName::new("custom_msgs::RobotEnvelope").unwrap();
    let state_type = TypeName::new("custom_msgs::EnvelopeState").unwrap();
    let envelope = NamedTypeDef::Struct(StructDef {
        fields: vec![
            FieldDef::new(
                "mission_id",
                TypeDef::Optional(Box::new(TypeDef::Primitive(PrimitiveTypeDef::U32))),
            ),
            FieldDef::new(
                "checkpoints",
                TypeDef::Sequence {
                    element: Box::new(TypeDef::String),
                    length: SequenceLengthDef::Dynamic,
                },
            ),
            FieldDef::new("state", TypeDef::EnumRef(state_type.clone())),
        ],
    });
    let state = NamedTypeDef::Enum(EnumDef {
        variants: vec![EnumVariantDef::new(
            "Ready",
            EnumPayloadDef::Struct(vec![FieldDef::new(
                "priority",
                TypeDef::Primitive(PrimitiveTypeDef::U32),
            )]),
        )],
    });
    let bundle = SchemaBundle {
        root_name: RootTypeName::new("custom_msgs::RobotEnvelope").unwrap(),
        root: TypeDef::StructRef(envelope_type.clone()),
        definitions: [(envelope_type, envelope), (state_type, state)].into(),
    };

    assert!(bundle.validate().is_ok());
}

#[test]
fn schema_supports_map_field_shape() {
    let type_name = TypeName::new("test_pkg::Lookup").unwrap();
    let bundle = SchemaBundle {
        root_name: RootTypeName::new("test_pkg::Lookup").unwrap(),
        root: TypeDef::StructRef(type_name.clone()),
        definitions: [(
            type_name,
            NamedTypeDef::Struct(StructDef {
                fields: vec![FieldDef::new(
                    "names",
                    TypeDef::Map {
                        key: Box::new(TypeDef::String),
                        value: Box::new(TypeDef::Primitive(PrimitiveTypeDef::U32)),
                    },
                )],
            }),
        )]
        .into(),
    };

    assert!(bundle.validate().is_ok());
}

#[test]
fn schema_accepts_bool_and_integer_map_key_primitives() {
    for key_primitive in [
        PrimitiveTypeDef::Bool,
        PrimitiveTypeDef::I8,
        PrimitiveTypeDef::I16,
        PrimitiveTypeDef::I32,
        PrimitiveTypeDef::I64,
        PrimitiveTypeDef::U8,
        PrimitiveTypeDef::U16,
        PrimitiveTypeDef::U32,
        PrimitiveTypeDef::U64,
    ] {
        let type_name = TypeName::new("test_pkg::Lookup").unwrap();
        let bundle = SchemaBundle {
            root_name: RootTypeName::new("test_pkg::Lookup").unwrap(),
            root: TypeDef::StructRef(type_name.clone()),
            definitions: [(
                type_name,
                NamedTypeDef::Struct(StructDef {
                    fields: vec![FieldDef::new(
                        "names",
                        TypeDef::Map {
                            key: Box::new(TypeDef::Primitive(key_primitive)),
                            value: Box::new(TypeDef::String),
                        },
                    )],
                }),
            )]
            .into(),
        };

        assert!(
            bundle.validate().is_ok(),
            "{key_primitive:?} should be a map key"
        );
    }
}

#[test]
fn schema_rejects_unsupported_map_key_shape() {
    let type_name = TypeName::new("test_pkg::Lookup").unwrap();
    let bundle = SchemaBundle {
        root_name: RootTypeName::new("test_pkg::Lookup").unwrap(),
        root: TypeDef::StructRef(type_name.clone()),
        definitions: [(
            type_name,
            NamedTypeDef::Struct(StructDef {
                fields: vec![FieldDef::new(
                    "names",
                    TypeDef::Map {
                        key: Box::new(TypeDef::Primitive(PrimitiveTypeDef::F32)),
                        value: Box::new(TypeDef::String),
                    },
                )],
            }),
        )]
        .into(),
    };

    assert_eq!(
        bundle.validate(),
        Err(SchemaError::UnsupportedMapKeyShape("primitive".to_string()))
    );
}

#[test]
fn schema_rejects_map_field_defaults() {
    let type_name = TypeName::new("test_pkg::Lookup").unwrap();
    let bundle = SchemaBundle {
        root_name: RootTypeName::new("test_pkg::Lookup").unwrap(),
        root: TypeDef::StructRef(type_name.clone()),
        definitions: [(
            type_name,
            NamedTypeDef::Struct(StructDef {
                fields: vec![
                    FieldDef::new(
                        "names",
                        TypeDef::Map {
                            key: Box::new(TypeDef::String),
                            value: Box::new(TypeDef::Primitive(PrimitiveTypeDef::U32)),
                        },
                    )
                    .with_default(LiteralValue::String("not-a-map-default".to_string())),
                ],
            }),
        )]
        .into(),
    };

    assert!(matches!(
        bundle.validate(),
        Err(SchemaError::InvalidFieldDefault { field_name, .. }) if field_name == "names"
    ));
}

#[test]
fn schema_bundle_validate_accepts_float_defaults_for_float_primitives() {
    let type_name = TypeName::new("test_pkg::Foo").unwrap();
    let bundle = SchemaBundle {
        root_name: RootTypeName::new("test_pkg::Foo").unwrap(),
        root: TypeDef::StructRef(type_name.clone()),
        definitions: [(
            type_name,
            NamedTypeDef::Struct(StructDef {
                fields: vec![
                    FieldDef::new("gain", TypeDef::Primitive(PrimitiveTypeDef::F64))
                        .with_default(LiteralValue::Float64(1.25)),
                ],
            }),
        )]
        .into(),
    };

    assert!(bundle.validate().is_ok());
}

#[test]
fn service_def_identity_includes_nominal_name_and_components_without_event() {
    let service = ServiceDef::new(
        "example_interfaces::AddTwoInts",
        "example_interfaces::AddTwoIntsRequest",
        "example_interfaces::AddTwoIntsResponse",
    )
    .unwrap();

    let json = ros_z_schema::to_json(&service).unwrap();

    assert!(json.contains("\"type_name\":\"example_interfaces::AddTwoInts\""));
    assert!(json.contains("\"request\":\"example_interfaces::AddTwoIntsRequest\""));
    assert!(json.contains("\"response\":\"example_interfaces::AddTwoIntsResponse\""));
    assert!(!json.contains("event"));
}

#[test]
fn service_hash_changes_when_nominal_or_component_identity_changes() {
    let service = ServiceDef::new(
        "example_interfaces::AddTwoInts",
        "example_interfaces::AddTwoIntsRequest",
        "example_interfaces::AddTwoIntsResponse",
    )
    .unwrap();
    let renamed = ServiceDef::new(
        "custom_interfaces::AddTwoInts",
        "example_interfaces::AddTwoIntsRequest",
        "example_interfaces::AddTwoIntsResponse",
    )
    .unwrap();
    let different_request = ServiceDef::new(
        "example_interfaces::AddTwoInts",
        "custom_interfaces::AddTwoIntsRequest",
        "example_interfaces::AddTwoIntsResponse",
    )
    .unwrap();
    let different_response = ServiceDef::new(
        "example_interfaces::AddTwoInts",
        "example_interfaces::AddTwoIntsRequest",
        "custom_interfaces::AddTwoIntsResponse",
    )
    .unwrap();

    let hash = ros_z_schema::compute_hash(&service);

    assert_ne!(hash, ros_z_schema::compute_hash(&renamed));
    assert_ne!(hash, ros_z_schema::compute_hash(&different_request));
    assert_ne!(hash, ros_z_schema::compute_hash(&different_response));
}

#[test]
fn action_def_identity_includes_nominal_name_and_required_components() {
    let action = ActionDef::new(
        "example_interfaces::Fibonacci",
        "example_interfaces::FibonacciGoal",
        "example_interfaces::FibonacciResult",
        "example_interfaces::FibonacciFeedback",
    )
    .unwrap();

    let same_identity_different_type_name = ActionDef::new(
        "custom_interfaces::OtherFibonacci",
        "example_interfaces::FibonacciGoal",
        "example_interfaces::FibonacciResult",
        "example_interfaces::FibonacciFeedback",
    )
    .unwrap();

    let json = ros_z_schema::to_json(&action).unwrap();

    assert_eq!(action.goal().as_str(), "example_interfaces::FibonacciGoal");
    assert_ne!(action, same_identity_different_type_name);
    assert!(json.contains("\"type_name\":\"example_interfaces::Fibonacci\""));
    assert!(json.contains("\"goal\":\"example_interfaces::FibonacciGoal\""));
    assert!(json.contains("\"result\":\"example_interfaces::FibonacciResult\""));
    assert!(json.contains("\"feedback\":\"example_interfaces::FibonacciFeedback\""));
}

#[test]
fn action_hash_changes_when_nominal_or_component_identity_changes() {
    let action = ActionDef::new(
        "example_interfaces::Fibonacci",
        "example_interfaces::FibonacciGoal",
        "example_interfaces::FibonacciResult",
        "example_interfaces::FibonacciFeedback",
    )
    .unwrap();
    let renamed = ActionDef::new(
        "custom_interfaces::Fibonacci",
        "example_interfaces::FibonacciGoal",
        "example_interfaces::FibonacciResult",
        "example_interfaces::FibonacciFeedback",
    )
    .unwrap();
    let different_goal = ActionDef::new(
        "example_interfaces::Fibonacci",
        "custom_interfaces::FibonacciGoal",
        "example_interfaces::FibonacciResult",
        "example_interfaces::FibonacciFeedback",
    )
    .unwrap();
    let different_result = ActionDef::new(
        "example_interfaces::Fibonacci",
        "example_interfaces::FibonacciGoal",
        "custom_interfaces::FibonacciResult",
        "example_interfaces::FibonacciFeedback",
    )
    .unwrap();
    let different_feedback = ActionDef::new(
        "example_interfaces::Fibonacci",
        "example_interfaces::FibonacciGoal",
        "example_interfaces::FibonacciResult",
        "custom_interfaces::FibonacciFeedback",
    )
    .unwrap();

    let hash = ros_z_schema::compute_hash(&action);

    assert_ne!(hash, ros_z_schema::compute_hash(&renamed));
    assert_ne!(hash, ros_z_schema::compute_hash(&different_goal));
    assert_ne!(hash, ros_z_schema::compute_hash(&different_result));
    assert_ne!(hash, ros_z_schema::compute_hash(&different_feedback));
}
