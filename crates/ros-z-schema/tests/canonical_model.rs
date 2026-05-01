use ros_z_schema::{
    ActionDef, EnumDef, EnumPayloadDef, EnumVariantDef, FieldDef, FieldPrimitive, FieldShape,
    LiteralValue, SchemaBundle, SchemaError, ServiceDef, StructDef, TypeDef, TypeName,
};

#[test]
fn native_type_names_accept_rust_paths() {
    assert!(TypeName::new("hulk::vision::MyMessage").is_ok());
    assert!(TypeName::new("ros_z_msgs::std_msgs::String").is_ok());
    assert!(TypeName::new("test_pkg::r#type::Message").is_ok());
    assert!(TypeName::new("test_pkg::r#async::Message").is_ok());
}

#[test]
fn native_type_names_reject_ros_paths_generics_and_invalid_segments() {
    assert!(TypeName::new("").is_err());
    assert!(TypeName::new("std_msgs/String").is_err());
    assert!(TypeName::new("hulk::Message<T>").is_err());
    assert!(TypeName::new("::hulk::Message").is_err());
    assert!(TypeName::new("hulk::Message::").is_err());
    assert!(TypeName::new("hulk::::Message").is_err());
    assert!(TypeName::new("hulk::vision Message").is_err());
    assert!(TypeName::new(" hulk::Message").is_err());
    assert!(TypeName::new("hulk::Message ").is_err());
    assert!(TypeName::new("_").is_err());
    assert!(TypeName::new("r#_").is_err());
    assert!(TypeName::new("test_pkg::_::Message").is_err());
    assert!(TypeName::new("test_pkg::r#_::Message").is_err());
    assert!(TypeName::new("test_pkg::type::Message").is_err());
    assert!(TypeName::new("test_pkg::async::Message").is_err());
    assert!(TypeName::new("test_pkg::r#Self::Message").is_err());
    assert!(TypeName::new("test_pkg::r#self::Message").is_err());
    assert!(TypeName::new("test_pkg::r#super::Message").is_err());
    assert!(TypeName::new("test_pkg::r#crate::Message").is_err());
}

#[test]
fn type_name_requires_native_rust_path_shape() {
    assert!(TypeName::new("std_msgs::String").is_ok());
    assert!(TypeName::new("std_msgs/String").is_err());
    assert!(TypeName::new("std_msgs::").is_err());
}

#[test]
fn field_primitive_uses_rust_native_spelling() {
    assert_eq!(FieldPrimitive::Bool.as_str(), "bool");
    assert_eq!(FieldPrimitive::I8.as_str(), "i8");
    assert_eq!(FieldPrimitive::I16.as_str(), "i16");
    assert_eq!(FieldPrimitive::I32.as_str(), "i32");
    assert_eq!(FieldPrimitive::I64.as_str(), "i64");
    assert_eq!(FieldPrimitive::U8.as_str(), "u8");
    assert_eq!(FieldPrimitive::U16.as_str(), "u16");
    assert_eq!(FieldPrimitive::U32.as_str(), "u32");
    assert_eq!(FieldPrimitive::U64.as_str(), "u64");
    assert_eq!(FieldPrimitive::F32.as_str(), "f32");
    assert_eq!(FieldPrimitive::F64.as_str(), "f64");
}

#[test]
fn field_primitive_from_str_accepts_only_rust_native_names() {
    assert_eq!("u8".parse::<FieldPrimitive>(), Ok(FieldPrimitive::U8));
    assert_eq!("f64".parse::<FieldPrimitive>(), Ok(FieldPrimitive::F64));

    assert!("uint8".parse::<FieldPrimitive>().is_err());
    assert!("byte".parse::<FieldPrimitive>().is_err());
    assert!("char".parse::<FieldPrimitive>().is_err());
    assert!("float64".parse::<FieldPrimitive>().is_err());
}

#[test]
fn field_primitive_from_ros_name_normalizes_ros_boundary_names() {
    assert_eq!(
        FieldPrimitive::from_ros_name("uint8"),
        Some(FieldPrimitive::U8)
    );
    assert_eq!(
        FieldPrimitive::from_ros_name("byte"),
        Some(FieldPrimitive::U8)
    );
    assert_eq!(
        FieldPrimitive::from_ros_name("char"),
        Some(FieldPrimitive::U8)
    );
    assert_eq!(
        FieldPrimitive::from_ros_name("float64"),
        Some(FieldPrimitive::F64)
    );
    assert_eq!(FieldPrimitive::from_ros_name("u8"), None);
}

#[test]
fn field_primitive_serde_uses_rust_native_names_without_ros_aliases() {
    let encoded = serde_json::to_string(&FieldPrimitive::U8).unwrap();
    assert_eq!(encoded, r#""u8""#);

    assert_eq!(
        serde_json::from_str::<FieldPrimitive>(r#""u8""#).unwrap(),
        FieldPrimitive::U8
    );
    for ros_name in ["uint8", "byte", "char", "float64"] {
        let encoded = serde_json::to_string(ros_name).unwrap();
        assert!(serde_json::from_str::<FieldPrimitive>(&encoded).is_err());
    }
}

#[test]
fn schema_bundle_validate_rejects_missing_root_definition() {
    let bundle = SchemaBundle::builder("geometry_msgs::Twist")
        .definition(
            "geometry_msgs::Vector3",
            TypeDef::Struct(StructDef { fields: vec![] }),
        )
        .build_unchecked();

    assert_eq!(
        bundle.validate(),
        Err(SchemaError::MissingRoot("geometry_msgs::Twist".to_string()))
    );
}

#[test]
fn schema_bundle_validate_rejects_missing_named_reference() {
    let twist = TypeDef::Struct(StructDef {
        fields: vec![FieldDef::new(
            "linear",
            FieldShape::Named(TypeName::new("geometry_msgs::Vector3").unwrap()),
        )],
    });

    let bundle = SchemaBundle::builder("geometry_msgs::Twist")
        .definition("geometry_msgs::Twist", twist)
        .build_unchecked();

    assert!(bundle.validate().is_err());
}

#[test]
fn schema_bundle_validate_accepts_first_class_extended_shapes_and_enums() {
    let envelope = TypeDef::Struct(StructDef {
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
    });
    let state = TypeDef::Enum(EnumDef {
        variants: vec![EnumVariantDef::new(
            "Ready",
            EnumPayloadDef::Struct(vec![FieldDef::new(
                "priority",
                FieldShape::Primitive(FieldPrimitive::U32),
            )]),
        )],
    });

    let bundle = SchemaBundle::builder("custom_msgs::RobotEnvelope")
        .definition("custom_msgs::RobotEnvelope", envelope)
        .definition("custom_msgs::EnvelopeState", state)
        .build();

    assert!(bundle.is_ok());
}

#[test]
fn schema_supports_map_field_shape() {
    let bundle = ros_z_schema::SchemaBundle::builder("test_pkg::Lookup")
        .definition(
            "test_pkg::Lookup",
            ros_z_schema::TypeDef::Struct(ros_z_schema::StructDef {
                fields: vec![ros_z_schema::FieldDef::new(
                    "names",
                    ros_z_schema::FieldShape::Map {
                        key: Box::new(ros_z_schema::FieldShape::String),
                        value: Box::new(ros_z_schema::FieldShape::Primitive(FieldPrimitive::U32)),
                    },
                )],
            }),
        )
        .build();

    assert!(bundle.is_ok());
}

#[test]
fn schema_accepts_bool_and_integer_map_key_primitives() {
    for key_primitive in [
        FieldPrimitive::Bool,
        FieldPrimitive::I8,
        FieldPrimitive::I16,
        FieldPrimitive::I32,
        FieldPrimitive::I64,
        FieldPrimitive::U8,
        FieldPrimitive::U16,
        FieldPrimitive::U32,
        FieldPrimitive::U64,
    ] {
        let bundle = SchemaBundle::builder("test_pkg::Lookup")
            .definition(
                "test_pkg::Lookup",
                TypeDef::Struct(StructDef {
                    fields: vec![FieldDef::new(
                        "names",
                        FieldShape::Map {
                            key: Box::new(FieldShape::Primitive(key_primitive)),
                            value: Box::new(FieldShape::String),
                        },
                    )],
                }),
            )
            .build();

        assert!(bundle.is_ok(), "{key_primitive:?} should be a map key");
    }
}

#[test]
fn schema_rejects_unsupported_map_key_shape() {
    let bundle = SchemaBundle::builder("test_pkg::Lookup")
        .definition(
            "test_pkg::Lookup",
            TypeDef::Struct(StructDef {
                fields: vec![FieldDef::new(
                    "names",
                    FieldShape::Map {
                        key: Box::new(FieldShape::Primitive(FieldPrimitive::F32)),
                        value: Box::new(FieldShape::String),
                    },
                )],
            }),
        )
        .build();

    assert_eq!(
        bundle,
        Err(SchemaError::UnsupportedMapKeyShape("primitive".to_string()))
    );
}

#[test]
fn schema_rejects_map_field_defaults() {
    let bundle = SchemaBundle::builder("test_pkg::Lookup")
        .definition(
            "test_pkg::Lookup",
            TypeDef::Struct(StructDef {
                fields: vec![
                    FieldDef::new(
                        "names",
                        FieldShape::Map {
                            key: Box::new(FieldShape::String),
                            value: Box::new(FieldShape::Primitive(FieldPrimitive::U32)),
                        },
                    )
                    .with_default(LiteralValue::String("not-a-map-default".to_string())),
                ],
            }),
        )
        .build();

    assert!(matches!(
        bundle,
        Err(SchemaError::InvalidFieldDefault { field_name, .. }) if field_name == "names"
    ));
}

#[test]
fn schema_bundle_validate_accepts_float_defaults_for_float_primitives() {
    let bundle = SchemaBundle::builder("test_pkg::Foo")
        .definition(
            "test_pkg::Foo",
            TypeDef::Struct(StructDef {
                fields: vec![
                    FieldDef::new("gain", FieldShape::Primitive(FieldPrimitive::F64))
                        .with_default(LiteralValue::Float64(1.25)),
                ],
            }),
        )
        .build();

    assert!(bundle.is_ok());
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
