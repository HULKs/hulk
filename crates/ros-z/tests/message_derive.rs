use std::time::Duration;

use ros_z::{
    Message,
    context::ContextBuilder,
    dynamic::{FieldType, message_schema_to_bundle},
};
use serde::{Deserialize, Serialize};
use zenoh::{Wait, config::WhatAmI};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
struct DerivedMessage {
    count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
#[message(name = "test_pkg::ExplicitNativeMessage")]
struct ExplicitNativeMessage {
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
enum DriveMode {
    Idle,
    Manual { speed_limit: u32 },
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
    assert_eq!(
        schema.type_name().expect("valid schema type name").as_str(),
        DerivedMessage::type_name()
    );
}

#[test]
fn derive_message_accepts_explicit_native_type_name() {
    assert_eq!(
        ExplicitNativeMessage::type_name(),
        "test_pkg::ExplicitNativeMessage"
    );
    let schema = ExplicitNativeMessage::schema();
    assert_eq!(schema.type_name_str(), "test_pkg::ExplicitNativeMessage");
}

#[test]
fn derive_generates_type_info_and_schema() {
    let schema = RobotTelemetry::schema();

    assert_eq!(
        RobotTelemetry::type_name(),
        "message_derive::RobotTelemetry"
    );
    assert_eq!(schema.type_name_str(), "message_derive::RobotTelemetry");
    assert_eq!(schema.field_count(), 5);

    let label = schema.field("label").expect("label field");
    assert!(matches!(label.field_type, FieldType::String));

    let pose = schema.field("pose").expect("pose field");
    match &pose.field_type {
        FieldType::Message(nested) => {
            assert_eq!(nested.type_name_str(), "message_derive::Position2D");
            assert_eq!(nested.field_count(), 2);
        }
        other => panic!("expected nested message field, got {:?}", other),
    }

    let temperatures = schema.field("temperatures").expect("temperatures field");
    match &temperatures.field_type {
        FieldType::Sequence(inner) => {
            assert!(matches!(inner.as_ref(), FieldType::Float32));
        }
        other => panic!("expected sequence field, got {:?}", other),
    }

    let flags = schema.field("flags").expect("flags field");
    match &flags.field_type {
        FieldType::Array(inner, len) => {
            assert_eq!(*len, 2);
            assert!(matches!(inner.as_ref(), FieldType::Bool));
        }
        other => panic!("expected fixed array field, got {:?}", other),
    }

    let payload = schema.field("payload").expect("payload field");
    match &payload.field_type {
        FieldType::Sequence(inner) => {
            assert!(matches!(inner.as_ref(), FieldType::Uint8));
        }
        other => panic!("expected byte sequence field, got {:?}", other),
    }

    let expected_hash = ros_z::dynamic::schema_hash(&schema).expect("schema hash");

    let reported_hash = RobotTelemetry::schema_hash();
    assert_eq!(reported_hash, expected_hash);
}

#[test]
fn derived_standard_message_hash_matches_runtime_bundle_hash() {
    let runtime = ros_z::dynamic::MessageSchema::builder("message_derive::RobotTelemetry")
        .field("label", ros_z::dynamic::FieldType::String)
        .field(
            "pose",
            ros_z::dynamic::FieldType::Message(Position2D::schema()),
        )
        .field(
            "temperatures",
            ros_z::dynamic::FieldType::Sequence(Box::new(ros_z::dynamic::FieldType::Float32)),
        )
        .field(
            "flags",
            ros_z::dynamic::FieldType::Array(Box::new(ros_z::dynamic::FieldType::Bool), 2),
        )
        .field(
            "payload",
            ros_z::dynamic::FieldType::Sequence(Box::new(ros_z::dynamic::FieldType::Uint8)),
        )
        .build()
        .expect("runtime schema");

    let runtime_hash = ros_z::dynamic::schema_hash(&runtime).expect("runtime hash");

    assert_eq!(RobotTelemetry::schema_hash(), runtime_hash);
}

#[test]
fn derived_message_hash_matches_runtime_bundle_hash() {
    let expected = ros_z::dynamic::schema_hash(&RobotTelemetry::schema()).unwrap();

    assert_eq!(RobotTelemetry::schema_hash(), expected);
}

#[test]
fn derive_generates_distinct_generic_type_info_per_instantiation() {
    let u32_schema = GenericTelemetry::<u32>::schema();
    let string_schema = GenericTelemetry::<String>::schema();

    assert_eq!(
        GenericTelemetry::<u32>::type_name(),
        "message_derive::GenericTelemetry__u32"
    );
    assert_eq!(
        GenericTelemetry::<String>::type_name(),
        "message_derive::GenericTelemetry__string"
    );
    assert_ne!(
        GenericTelemetry::<u32>::type_name(),
        GenericTelemetry::<String>::type_name()
    );

    assert_eq!(
        u32_schema.type_name_str(),
        GenericTelemetry::<u32>::type_name()
    );
    assert_eq!(
        string_schema.type_name_str(),
        GenericTelemetry::<String>::type_name()
    );
    assert_ne!(u32_schema.type_name_str(), string_schema.type_name_str());
    assert_ne!(
        GenericTelemetry::<u32>::schema_hash(),
        GenericTelemetry::<String>::schema_hash()
    );

    let foo = u32_schema.field("foo").expect("foo field");
    assert!(matches!(foo.field_type, FieldType::Uint32));

    let data = string_schema.field("data").expect("data field");
    match &data.field_type {
        FieldType::Sequence(inner) => {
            assert!(matches!(inner.as_ref(), FieldType::String));
        }
        other => panic!("expected string sequence field, got {:?}", other),
    }
}

#[test]
fn derive_supports_nested_generic_message_fields() {
    let schema = GenericTelemetry::<Position2D>::schema();
    assert_eq!(
        GenericTelemetry::<Position2D>::type_name(),
        "message_derive::GenericTelemetry__message_derive_position2d"
    );

    let foo = schema.field("foo").expect("foo field");
    match &foo.field_type {
        FieldType::Message(nested) => {
            assert_eq!(nested.type_name_str(), "message_derive::Position2D");
        }
        other => panic!("expected nested message field, got {:?}", other),
    }

    let nested_schema = NestedGenericTelemetry::<Position2D>::schema();
    let inner = nested_schema.field("inner").expect("inner field");
    match &inner.field_type {
        FieldType::Message(nested) => {
            assert_eq!(
                nested.type_name_str(),
                GenericTelemetry::<Position2D>::type_name()
            );
        }
        other => panic!("expected nested generic message field, got {:?}", other),
    }
}

#[test]
fn derive_supports_enums_with_full_schema() {
    let schema = DriveMode::schema();

    assert_eq!(DriveMode::type_name(), "message_derive::DriveMode");
    assert_eq!(schema.type_name_str(), "message_derive::DriveMode");
    assert_eq!(schema.field_count(), 1);

    let value = schema.field("value").expect("enum payload field");
    match &value.field_type {
        FieldType::Enum(enum_schema) => {
            assert_eq!(enum_schema.type_name, "message_derive::DriveMode");
            assert_eq!(enum_schema.variants.len(), 2);
        }
        other => panic!("expected enum field, got {:?}", other),
    }
}

#[test]
fn derive_supports_option_fields_with_type_info() {
    let schema = OptionalTelemetry::schema();

    let mode = schema.field("mode").expect("optional field");
    match &mode.field_type {
        FieldType::Optional(inner) => match inner.as_ref() {
            FieldType::Enum(enum_schema) => {
                assert_eq!(enum_schema.type_name, "message_derive::DriveMode");
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

    let shadow_string = schema.field("shadow_string").expect("shadow_string field");
    match &shadow_string.field_type {
        FieldType::Message(nested) => {
            assert_eq!(
                nested.type_name_str(),
                "message_derive::shadow_types::String"
            );
        }
        other => panic!("expected shadowed String to be a message, got {:?}", other),
    }

    let shadow_vec = schema.field("shadow_vec").expect("shadow_vec field");
    match &shadow_vec.field_type {
        FieldType::Message(nested) => {
            assert_eq!(nested.type_name_str(), "message_derive::shadow_types::Vec");
        }
        other => panic!("expected shadowed Vec to be a message, got {:?}", other),
    }

    let shadow_option = schema.field("shadow_option").expect("shadow_option field");
    match &shadow_option.field_type {
        FieldType::Message(nested) => {
            assert_eq!(
                nested.type_name_str(),
                "message_derive::shadow_types::Option__u32"
            );
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
        &message_schema_to_bundle(&RobotTelemetry::schema()).expect("schema bundle"),
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
        discovered_schema.type_name_str(),
        "message_derive::RobotTelemetry"
    );
    assert_eq!(discovered_schema.field_count(), 5);

    let message = tokio::time::timeout(Duration::from_secs(3), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");
    assert_eq!(
        message.get::<String>("label").expect("label field"),
        "robot-1".to_string()
    );
    assert_eq!(message.get::<f64>("pose.x").expect("nested pose.x"), 1.25);
    assert_eq!(message.get::<f64>("pose.y").expect("nested pose.y"), -2.5);

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
        discovered_schema.type_name_str(),
        "message_derive::RobotTelemetry"
    );

    let message = tokio::time::timeout(Duration::from_secs(3), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");
    assert_eq!(
        message.get::<String>("label").expect("label field"),
        "robot-1".to_string()
    );

    publish_task.await.expect("publisher task");
}
