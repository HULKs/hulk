use ros_z_schema::{
    ActionDef, EnumDef, EnumPayloadDef, EnumVariantDef, FieldDef, PrimitiveTypeDef, SchemaBundle,
    SchemaError, SequenceLengthDef, ServiceDef, StructDef, TypeDef, TypeDefinition, TypeName,
};

#[test]
fn type_names_are_opaque_non_empty_strings() {
    // ros-z schema names are opaque protocol identities: validation only rejects
    // empty strings, leaving syntax policy to callers at ROS/native boundaries.
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
    }

    assert_eq!(
        TypeName::new(""),
        Err(SchemaError::InvalidTypeName("".into()))
    );
}

#[test]
fn schema_bundle_accepts_primitive_root_without_named_definition() {
    let bundle = SchemaBundle::new(TypeDef::Primitive(PrimitiveTypeDef::U8)).unwrap();

    assert!(bundle.definitions().is_empty());
}

#[test]
fn schema_bundle_validates_named_refs() {
    let bundle = SchemaBundle {
        root: TypeDef::Named(TypeName::new("hulk::Pose").unwrap()),
        definitions: [(
            TypeName::new("hulk::Pose").unwrap(),
            TypeDefinition::Struct(StructDef {
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
    let bundle = SchemaBundle::new(TypeDef::Named(TypeName::new("hulk::Pose").unwrap()));

    assert_eq!(
        bundle,
        Err(SchemaError::MissingDefinition(
            TypeName::new("hulk::Pose").unwrap()
        ))
    );
}

#[test]
fn type_definitions_insert_returns_existing_definition() {
    let type_name = TypeName::new("hulk::Pose").unwrap();
    let definition = TypeDefinition::Struct(StructDef {
        fields: vec![FieldDef::new(
            "x",
            TypeDef::Primitive(PrimitiveTypeDef::F64),
        )],
    });
    let mut definitions = ros_z_schema::TypeDefinitions::new();

    assert_eq!(
        definitions.insert(type_name.clone(), definition.clone()),
        None
    );
    assert_eq!(
        definitions.insert(type_name.clone(), definition.clone()),
        Some(definition.clone())
    );
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions.get(&type_name), Some(&definition));
}

#[test]
fn schema_bundle_accepts_reachable_definitions() {
    let pose = TypeName::new("hulk::Pose").unwrap();
    let point = TypeName::new("hulk::Point").unwrap();
    let bundle = SchemaBundle {
        root: TypeDef::Named(pose.clone()),
        definitions: [
            (
                pose.clone(),
                TypeDefinition::Struct(StructDef {
                    fields: vec![FieldDef::new("position", TypeDef::Named(point.clone()))],
                }),
            ),
            (
                point.clone(),
                TypeDefinition::Struct(StructDef {
                    fields: vec![FieldDef::new(
                        "x",
                        TypeDef::Primitive(PrimitiveTypeDef::F64),
                    )],
                }),
            ),
        ]
        .into(),
    };

    assert!(bundle.validate().is_ok());
}

#[test]
fn type_definitions_insert_replaces_conflicting_definition() {
    let type_name = TypeName::new("hulk::Pose").unwrap();
    let original = TypeDefinition::Struct(StructDef {
        fields: vec![FieldDef::new(
            "x",
            TypeDef::Primitive(PrimitiveTypeDef::F64),
        )],
    });
    let replacement = TypeDefinition::Struct(StructDef {
        fields: vec![FieldDef::new(
            "y",
            TypeDef::Primitive(PrimitiveTypeDef::F64),
        )],
    });
    let mut definitions = ros_z_schema::TypeDefinitions::new();

    definitions.insert(type_name.clone(), original.clone());

    assert_eq!(
        definitions.insert(type_name.clone(), replacement.clone()),
        Some(original)
    );
    assert_eq!(definitions.get(&type_name), Some(&replacement));
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
        root: TypeDef::Named(TypeName::new("geometry_msgs::Twist").unwrap()),
        definitions: [(
            TypeName::new("geometry_msgs::Twist").unwrap(),
            TypeDefinition::Struct(StructDef {
                fields: vec![FieldDef::new(
                    "linear",
                    TypeDef::Named(TypeName::new("geometry_msgs::Vector3").unwrap()),
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
fn schema_bundle_validate_rejects_duplicate_fields() {
    let state = TypeName::new("custom_msgs::State").unwrap();
    let bundle = SchemaBundle {
        root: TypeDef::Named(state.clone()),
        definitions: [(
            state.clone(),
            TypeDefinition::Struct(StructDef {
                fields: vec![
                    FieldDef::new("value", TypeDef::Primitive(PrimitiveTypeDef::U8)),
                    FieldDef::new("value", TypeDef::Primitive(PrimitiveTypeDef::U16)),
                ],
            }),
        )]
        .into(),
    };

    assert_eq!(
        bundle.validate(),
        Err(SchemaError::DuplicateField {
            type_name: state,
            field_name: "value".into(),
        })
    );
}

#[test]
fn schema_bundle_validate_rejects_empty_enum_definition() {
    let state = TypeName::new("custom_msgs::State").unwrap();
    let bundle = SchemaBundle {
        root: TypeDef::Named(state.clone()),
        definitions: [(
            state.clone(),
            TypeDefinition::Enum(EnumDef { variants: vec![] }),
        )]
        .into(),
    };

    assert_eq!(
        bundle.validate(),
        Err(SchemaError::EmptyEnum { type_name: state })
    );
}

#[test]
fn schema_bundle_validate_rejects_duplicate_enum_variants() {
    let state = TypeName::new("custom_msgs::State").unwrap();
    let bundle = SchemaBundle {
        root: TypeDef::Named(state.clone()),
        definitions: [(
            state.clone(),
            TypeDefinition::Enum(EnumDef {
                variants: vec![
                    EnumVariantDef::new("Active", EnumPayloadDef::Unit),
                    EnumVariantDef::new("Active", EnumPayloadDef::Unit),
                ],
            }),
        )]
        .into(),
    };

    assert_eq!(
        bundle.validate(),
        Err(SchemaError::DuplicateVariant {
            type_name: state,
            variant_name: "Active".into(),
        })
    );
}

#[test]
fn schema_bundle_validate_rejects_empty_struct_field_name() {
    let state = TypeName::new("custom_msgs::State").unwrap();
    let bundle = SchemaBundle {
        root: TypeDef::Named(state.clone()),
        definitions: [(
            state.clone(),
            TypeDefinition::Struct(StructDef {
                fields: vec![FieldDef::new("", TypeDef::String)],
            }),
        )]
        .into(),
    };

    assert_eq!(
        bundle.validate(),
        Err(SchemaError::EmptyFieldName { type_name: state })
    );
}

#[test]
fn schema_bundle_validate_rejects_empty_enum_variant_name() {
    let state = TypeName::new("custom_msgs::State").unwrap();
    let bundle = SchemaBundle {
        root: TypeDef::Named(state.clone()),
        definitions: [(
            state.clone(),
            TypeDefinition::Enum(EnumDef {
                variants: vec![EnumVariantDef::new("", EnumPayloadDef::Unit)],
            }),
        )]
        .into(),
    };

    assert_eq!(
        bundle.validate(),
        Err(SchemaError::EmptyVariantName { type_name: state })
    );
}

#[test]
fn field_references_validate_named_definitions() {
    let wrapper = TypeName::new("custom_msgs::Wrapper").unwrap();
    let state = TypeName::new("custom_msgs::State").unwrap();
    let bundle = SchemaBundle {
        root: TypeDef::Named(wrapper.clone()),
        definitions: [
            (
                wrapper,
                TypeDefinition::Struct(StructDef {
                    fields: vec![FieldDef::new("state", TypeDef::Named(state.clone()))],
                }),
            ),
            (state, TypeDefinition::Struct(StructDef { fields: vec![] })),
        ]
        .into(),
    };

    assert!(bundle.validate().is_ok());
}

#[test]
fn schema_bundle_validate_accepts_first_class_extended_shapes_and_enums() {
    let envelope_type = TypeName::new("custom_msgs::RobotEnvelope").unwrap();
    let state_type = TypeName::new("custom_msgs::EnvelopeState").unwrap();
    let envelope = TypeDefinition::Struct(StructDef {
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
            FieldDef::new("state", TypeDef::Named(state_type.clone())),
        ],
    });
    let state = TypeDefinition::Enum(EnumDef {
        variants: vec![EnumVariantDef::new(
            "Ready",
            EnumPayloadDef::Struct(vec![FieldDef::new(
                "priority",
                TypeDef::Primitive(PrimitiveTypeDef::U32),
            )]),
        )],
    });
    let bundle = SchemaBundle {
        root: TypeDef::Named(envelope_type.clone()),
        definitions: [(envelope_type, envelope), (state_type, state)].into(),
    };

    assert!(bundle.validate().is_ok());
}

#[test]
fn schema_accepts_string_key_map_field_shape() {
    let type_name = TypeName::new("test_pkg::Lookup").unwrap();
    let bundle = SchemaBundle {
        root: TypeDef::Named(type_name.clone()),
        definitions: [(
            type_name,
            TypeDefinition::Struct(StructDef {
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
            root: TypeDef::Named(type_name.clone()),
            definitions: [(
                type_name,
                TypeDefinition::Struct(StructDef {
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
