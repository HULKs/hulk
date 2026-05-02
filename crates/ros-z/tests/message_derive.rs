use std::time::Duration;

use ros_z::{
    Message,
    context::ContextBuilder,
    dynamic::{DynamicValue, PrimitiveType, RuntimeFieldSchema, TypeShape, schema_to_bundle},
};
use serde::{Deserialize, Serialize};
use zenoh::{Wait, config::WhatAmI};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
struct DerivedMessage {
    count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
struct EmptyMarker;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
#[message(name = "test_pkg::ExplicitNativeMessage")]
struct ExplicitNativeMessage {
    count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
#[message(name = "test_pkg/msg/SlashStyleMessage")]
struct ExplicitSlashStyleMessage {
    count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
struct Position2D {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
struct RobotTelemetry {
    label: String,
    pose: Position2D,
    temperatures: Vec<f32>,
    flags: [bool; 2],
    payload: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
struct GenericTelemetry<T> {
    data: Vec<T>,
    foo: T,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
struct NestedGenericTelemetry<T> {
    inner: GenericTelemetry<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
#[message(name = "test_pkg::ExplicitGenericTelemetry")]
struct ExplicitGenericTelemetry<T> {
    data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
enum DriveMode {
    Idle,
    Manual { speed_limit: u32 },
}

fn schema_type_name(schema: &ros_z::dynamic::Schema) -> &str {
    match schema.as_ref() {
        TypeShape::Struct { name, .. } | TypeShape::Enum { name, .. } => name.as_str(),
        other => panic!("expected named schema, got {other:?}"),
    }
}

fn shape_type_name(schema: &TypeShape) -> &str {
    match schema {
        TypeShape::Struct { name, .. } | TypeShape::Enum { name, .. } => name.as_str(),
        other => panic!("expected named schema, got {other:?}"),
    }
}

fn shape_fields(schema: &TypeShape) -> &[RuntimeFieldSchema] {
    let TypeShape::Struct { fields, .. } = schema else {
        panic!("expected struct schema, got {schema:?}");
    };
    fields
}

fn payload_field(payload: &ros_z::dynamic::DynamicPayload, name: &str) -> DynamicValue {
    let DynamicValue::Struct(message) = &payload.value else {
        panic!("expected struct payload, got {payload:?}");
    };
    message.get_dynamic(name).expect("payload field")
}

fn nested_payload_field(
    payload: &ros_z::dynamic::DynamicPayload,
    parent: &str,
    child: &str,
) -> DynamicValue {
    let DynamicValue::Struct(message) = payload_field(payload, parent) else {
        panic!("expected nested struct payload");
    };
    message.get_dynamic(child).expect("nested payload field")
}

fn struct_fields(schema: &ros_z::dynamic::Schema) -> &[RuntimeFieldSchema] {
    let TypeShape::Struct { fields, .. } = schema.as_ref() else {
        panic!("expected struct schema, got {schema:?}");
    };
    fields
}

fn field<'a>(schema: &'a ros_z::dynamic::Schema, name: &str) -> &'a RuntimeFieldSchema {
    struct_fields(schema)
        .iter()
        .find(|field| field.name == name)
        .unwrap_or_else(|| panic!("{name} field"))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
struct OptionalTelemetry {
    mode: Option<DriveMode>,
}

mod shadow_types {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
    pub struct String {
        pub value: std::string::String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
    pub struct Vec {
        pub values: std::vec::Vec<u8>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
    pub struct Option<T> {
        pub value: T,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
struct ShadowedNameEnvelope {
    shadow_string: shadow_types::String,
    shadow_vec: shadow_types::Vec,
    shadow_option: shadow_types::Option<u32>,
}

struct TestRouter {
    endpoint: String,
    _session: zenoh::Session,
}

impl TestRouter {
    fn new() -> Self {
        let port = {
            let listener =
                std::net::TcpListener::bind("127.0.0.1:0").expect("failed to bind port 0");
            listener.local_addr().unwrap().port()
        };

        let endpoint = format!("tcp/127.0.0.1:{port}");
        let mut config = zenoh::Config::default();
        config.set_mode(Some(WhatAmI::Router)).unwrap();
        config
            .insert_json5("listen/endpoints", &format!("[\"{endpoint}\"]"))
            .unwrap();
        config
            .insert_json5("scouting/multicast/enabled", "false")
            .unwrap();

        let session = zenoh::open(config)
            .wait()
            .expect("failed to open test router");
        std::thread::sleep(Duration::from_millis(300));

        Self {
            endpoint,
            _session: session,
        }
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

async fn create_context_with_router(router: &TestRouter) -> ros_z::Result<ros_z::context::Context> {
    ContextBuilder::default()
        .disable_multicast_scouting()
        .with_connect_endpoints([router.endpoint()])
        .build()
        .await
}

#[test]
fn derive_message_uses_module_path_type_name() {
    assert!(DerivedMessage::type_name().ends_with("::DerivedMessage"));
    let schema = DerivedMessage::schema();
    assert_eq!(schema_type_name(&schema), DerivedMessage::type_name());
}

#[test]
fn derive_message_accepts_explicit_native_type_name() {
    assert_eq!(
        ExplicitNativeMessage::type_name(),
        "test_pkg::ExplicitNativeMessage"
    );
    let schema = ExplicitNativeMessage::schema();
    assert_eq!(schema_type_name(&schema), "test_pkg::ExplicitNativeMessage");
}

#[test]
fn derive_message_accepts_explicit_opaque_type_name() {
    assert_eq!(
        ExplicitSlashStyleMessage::type_name(),
        "test_pkg/msg/SlashStyleMessage"
    );
    let schema = ExplicitSlashStyleMessage::schema();
    assert_eq!(schema_type_name(&schema), "test_pkg/msg/SlashStyleMessage");
}

#[test]
fn derive_supports_unit_struct_as_empty_struct_schema() {
    assert!(EmptyMarker::type_name().ends_with("::EmptyMarker"));

    let schema = EmptyMarker::schema();
    assert_eq!(schema_type_name(&schema), EmptyMarker::type_name());

    let TypeShape::Struct { fields, .. } = schema.as_ref() else {
        panic!("expected struct schema for unit struct, got {schema:?}");
    };
    assert!(fields.is_empty());
}

#[test]
fn derive_generates_type_info_and_schema() {
    let schema = RobotTelemetry::schema();

    assert_eq!(
        RobotTelemetry::type_name(),
        "message_derive::RobotTelemetry"
    );
    assert_eq!(schema_type_name(&schema), "message_derive::RobotTelemetry");
    assert_eq!(struct_fields(&schema).len(), 5);

    let label = field(&schema, "label");
    assert!(matches!(label.schema.as_ref(), TypeShape::String));

    let pose = field(&schema, "pose");
    match pose.schema.as_ref() {
        TypeShape::Struct { name, fields } => {
            assert_eq!(name.as_str(), "message_derive::Position2D");
            assert_eq!(fields.len(), 2);
        }
        other => panic!("expected nested message field, got {:?}", other),
    }

    let temperatures = field(&schema, "temperatures");
    match temperatures.schema.as_ref() {
        TypeShape::Sequence { element, .. } => {
            assert!(matches!(
                element.as_ref(),
                TypeShape::Primitive(PrimitiveType::F32)
            ));
        }
        other => panic!("expected sequence field, got {:?}", other),
    }

    let flags = field(&schema, "flags");
    match flags.schema.as_ref() {
        TypeShape::Sequence { element, length } => {
            assert_eq!(*length, ros_z::dynamic::SequenceLength::Fixed(2));
            assert!(matches!(
                element.as_ref(),
                TypeShape::Primitive(PrimitiveType::Bool)
            ));
        }
        other => panic!("expected fixed array field, got {:?}", other),
    }

    let payload = field(&schema, "payload");
    match payload.schema.as_ref() {
        TypeShape::Sequence { element, .. } => {
            assert!(matches!(
                element.as_ref(),
                TypeShape::Primitive(PrimitiveType::U8)
            ));
        }
        other => panic!("expected byte sequence field, got {:?}", other),
    }

    let expected_hash = ros_z::dynamic::schema_tree_hash(RobotTelemetry::type_name(), &schema)
        .expect("schema hash");

    let reported_hash = RobotTelemetry::schema_hash();
    assert_eq!(reported_hash, expected_hash);
}

#[test]
fn derived_standard_message_hash_matches_runtime_bundle_hash() {
    let runtime_hash =
        ros_z::dynamic::schema_tree_hash(RobotTelemetry::type_name(), &RobotTelemetry::schema())
            .expect("runtime hash");

    assert_eq!(RobotTelemetry::schema_hash(), runtime_hash);
}

#[test]
fn derived_message_hash_matches_runtime_bundle_hash() {
    let expected =
        ros_z::dynamic::schema_tree_hash(RobotTelemetry::type_name(), &RobotTelemetry::schema())
            .unwrap();

    assert_eq!(RobotTelemetry::schema_hash(), expected);
}

#[test]
fn derive_generates_distinct_generic_type_info_per_instantiation() {
    let u32_schema = GenericTelemetry::<u32>::schema();
    let string_schema = GenericTelemetry::<String>::schema();

    assert_eq!(
        GenericTelemetry::<u32>::type_name(),
        "message_derive::GenericTelemetry<u32>"
    );
    assert_eq!(
        GenericTelemetry::<String>::type_name(),
        "message_derive::GenericTelemetry<String>"
    );
    assert_ne!(
        GenericTelemetry::<u32>::type_name(),
        GenericTelemetry::<String>::type_name()
    );

    assert_eq!(
        schema_type_name(&u32_schema),
        GenericTelemetry::<u32>::type_name()
    );
    assert_eq!(
        schema_type_name(&string_schema),
        GenericTelemetry::<String>::type_name()
    );
    assert_ne!(
        schema_type_name(&u32_schema),
        schema_type_name(&string_schema)
    );
    assert_ne!(
        GenericTelemetry::<u32>::schema_hash(),
        GenericTelemetry::<String>::schema_hash()
    );

    let foo = field(&u32_schema, "foo");
    assert!(matches!(
        foo.schema.as_ref(),
        TypeShape::Primitive(PrimitiveType::U32)
    ));

    let data = field(&string_schema, "data");
    match data.schema.as_ref() {
        TypeShape::Sequence { element, .. } => {
            assert!(matches!(element.as_ref(), TypeShape::String));
        }
        other => panic!("expected string sequence field, got {:?}", other),
    }
}

#[test]
fn derive_supports_nested_generic_message_fields() {
    let schema = GenericTelemetry::<Position2D>::schema();
    assert_eq!(
        GenericTelemetry::<Position2D>::type_name(),
        "message_derive::GenericTelemetry<message_derive::Position2D>"
    );
    assert_eq!(
        NestedGenericTelemetry::<Position2D>::type_name(),
        "message_derive::NestedGenericTelemetry<message_derive::Position2D>"
    );

    let foo = field(&schema, "foo");
    match foo.schema.as_ref() {
        TypeShape::Struct { name, .. } => {
            assert_eq!(name.as_str(), "message_derive::Position2D");
        }
        other => panic!("expected nested message field, got {:?}", other),
    }

    let nested_schema = NestedGenericTelemetry::<Position2D>::schema();
    let inner = field(&nested_schema, "inner");
    match inner.schema.as_ref() {
        TypeShape::Struct { name, .. } => {
            assert_eq!(name.as_str(), GenericTelemetry::<Position2D>::type_name());
        }
        other => panic!("expected nested generic message field, got {:?}", other),
    }
}

#[test]
fn derive_supports_nested_same_generic_instantiations() {
    assert_eq!(
        GenericTelemetry::<GenericTelemetry<u32>>::type_name(),
        "message_derive::GenericTelemetry<message_derive::GenericTelemetry<u32>>"
    );

    let schema = GenericTelemetry::<GenericTelemetry<u32>>::schema();
    assert_eq!(
        schema_type_name(&schema),
        "message_derive::GenericTelemetry<message_derive::GenericTelemetry<u32>>"
    );
}

#[test]
fn derive_generates_direct_enum_schema() {
    let schema = DriveMode::schema();
    let ros_z::dynamic::TypeShape::Enum { name, variants } = schema.as_ref() else {
        panic!("expected enum schema");
    };

    assert_eq!(name.as_str(), "message_derive::DriveMode");
    assert_eq!(variants.len(), 2);
    assert_eq!(variants[0].name, "Idle");
    assert_eq!(variants[1].name, "Manual");
}

#[test]
fn derive_uses_explicit_name_as_generic_base_name() {
    assert_eq!(
        ExplicitGenericTelemetry::<u32>::type_name(),
        "test_pkg::ExplicitGenericTelemetry<u32>"
    );
    assert_eq!(
        ExplicitGenericTelemetry::<Position2D>::type_name(),
        "test_pkg::ExplicitGenericTelemetry<message_derive::Position2D>"
    );
}

#[test]
fn derive_supports_enums_with_full_schema() {
    let schema = DriveMode::schema();

    assert_eq!(DriveMode::type_name(), "message_derive::DriveMode");
    match schema.as_ref() {
        TypeShape::Enum { name, variants } => {
            assert_eq!(name.as_str(), "message_derive::DriveMode");
            assert_eq!(variants.len(), 2);
        }
        other => panic!("expected enum field, got {:?}", other),
    }
}

#[test]
fn derive_supports_option_fields_with_type_info() {
    let schema = OptionalTelemetry::schema();

    let mode = field(&schema, "mode");
    match mode.schema.as_ref() {
        TypeShape::Optional(inner) => match inner.as_ref() {
            TypeShape::Enum { name, .. } => {
                assert_eq!(name.as_str(), "message_derive::DriveMode");
            }
            other => panic!("expected optional enum field, got {:?}", other),
        },
        other => panic!("expected optional field, got {:?}", other),
    }

    assert_ne!(OptionalTelemetry::schema_hash(), DriveMode::schema_hash());
}

#[test]
fn derive_only_special_cases_exact_builtin_container_paths() {
    let schema = ShadowedNameEnvelope::schema();

    let shadow_string = field(&schema, "shadow_string");
    match shadow_string.schema.as_ref() {
        TypeShape::Struct { name, .. } => {
            assert_eq!(name.as_str(), "message_derive::shadow_types::String");
        }
        other => panic!("expected shadowed String to be a message, got {:?}", other),
    }

    let shadow_vec = field(&schema, "shadow_vec");
    match shadow_vec.schema.as_ref() {
        TypeShape::Struct { name, .. } => {
            assert_eq!(name.as_str(), "message_derive::shadow_types::Vec");
        }
        other => panic!("expected shadowed Vec to be a message, got {:?}", other),
    }

    let shadow_option = field(&schema, "shadow_option");
    match shadow_option.schema.as_ref() {
        TypeShape::Struct { name, .. } => {
            assert_eq!(name.as_str(), "message_derive::shadow_types::Option<u32>");
        }
        other => panic!("expected shadowed Option to be a message, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn derived_message_schema_is_auto_registered_and_discoverable() {
    let router = TestRouter::new();

    let pub_ctx = create_context_with_router(&router)
        .await
        .expect("publisher context");
    let pub_node = pub_ctx
        .create_node("derived_talker")
        .build()
        .await
        .expect("publisher node");

    let publisher = pub_node
        .publisher::<RobotTelemetry>("/derived_topic")
        .build()
        .await
        .expect("publisher");

    let registered_hash = ros_z_schema::compute_hash(
        &schema_to_bundle(RobotTelemetry::type_name(), &RobotTelemetry::schema())
            .expect("schema bundle"),
    );
    let registered = pub_node
        .schema_service()
        .expect("schema service")
        .get_schema("message_derive::RobotTelemetry", &registered_hash)
        .expect("query registered schema");
    assert!(registered.is_some(), "schema should be auto-registered");

    let sub_ctx = create_context_with_router(&router)
        .await
        .expect("subscriber context");
    let sub_node = sub_ctx
        .create_node("derived_listener")
        .build()
        .await
        .expect("subscriber node");

    let publish_task = tokio::spawn(async move {
        for _ in 0..25 {
            let message = RobotTelemetry {
                label: "robot-1".to_string(),
                pose: Position2D { x: 1.25, y: -2.5 },
                temperatures: vec![20.5, 21.0, 21.5],
                flags: [true, false],
                payload: vec![1, 2, 3, 4],
            };
            publisher.publish(&message).await.expect("publish");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    tokio::time::sleep(Duration::from_millis(400)).await;

    let subscriber = sub_node
        .dynamic_subscriber_auto("/derived_topic", Duration::from_secs(10))
        .await
        .expect("dynamic subscriber with auto-discovery")
        .build()
        .await
        .expect("subscriber build");
    let discovered_schema = subscriber.schema().expect("discovered schema");

    assert_eq!(
        shape_type_name(discovered_schema),
        "message_derive::RobotTelemetry"
    );
    assert_eq!(shape_fields(discovered_schema).len(), 5);

    let message = tokio::time::timeout(Duration::from_secs(3), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");
    assert_eq!(payload_field(&message, "label").as_str(), Some("robot-1"));
    assert_eq!(
        nested_payload_field(&message, "pose", "x").as_f64(),
        Some(1.25)
    );
    assert_eq!(
        nested_payload_field(&message, "pose", "y").as_f64(),
        Some(-2.5)
    );

    publish_task.await.expect("publisher task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn derived_message_schema_discovery_works_across_namespaces() {
    let router = TestRouter::new();

    let pub_ctx = create_context_with_router(&router)
        .await
        .expect("publisher context");
    let pub_node = pub_ctx
        .create_node("derived_talker")
        .with_namespace("tools")
        .build()
        .await
        .expect("publisher node");

    let publisher = pub_node
        .publisher::<RobotTelemetry>("/derived_topic")
        .build()
        .await
        .expect("publisher");

    let sub_ctx = create_context_with_router(&router)
        .await
        .expect("subscriber context");
    let sub_node = sub_ctx
        .create_node("derived_listener")
        .with_namespace("ui")
        .build()
        .await
        .expect("subscriber node");

    let publish_task = tokio::spawn(async move {
        for _ in 0..25 {
            let message = RobotTelemetry {
                label: "robot-1".to_string(),
                pose: Position2D { x: 1.25, y: -2.5 },
                temperatures: vec![20.5, 21.0, 21.5],
                flags: [true, false],
                payload: vec![1, 2, 3, 4],
            };
            publisher.publish(&message).await.expect("publish");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    tokio::time::sleep(Duration::from_millis(400)).await;

    let subscriber = sub_node
        .dynamic_subscriber_auto("/derived_topic", Duration::from_secs(10))
        .await
        .expect("dynamic subscriber with auto-discovery")
        .build()
        .await
        .expect("subscriber build");
    let discovered_schema = subscriber.schema().expect("discovered schema");

    assert_eq!(
        shape_type_name(discovered_schema),
        "message_derive::RobotTelemetry"
    );

    let message = tokio::time::timeout(Duration::from_secs(3), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");
    assert_eq!(payload_field(&message, "label").as_str(), Some("robot-1"));

    publish_task.await.expect("publisher task");
}
