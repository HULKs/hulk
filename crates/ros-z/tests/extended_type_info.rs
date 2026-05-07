use std::time::Duration;

use ros_z::{
    Message,
    context::ContextBuilder,
    dynamic::{DynamicValue, EnumPayloadValue, SchemaRegistry},
};
use ros_z_schema::{
    EnumVariantDef, FieldDef, PrimitiveTypeDef, SchemaBundle, SequenceLengthDef, TypeDef,
    TypeDefinition,
};
use serde::{Deserialize, Serialize};
use zenoh::{Wait, config::WhatAmI};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
struct TelemetryLite {
    label: String,
    temperatures: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
enum RobotState {
    Idle,
    Error(String),
    Charging { minutes_remaining: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
struct RobotEnvelope {
    label: String,
    mission_id: Option<u32>,
    state: RobotState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
struct GenericEnvelope<T> {
    payload: T,
    items: Vec<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
struct GenericOptionalEnvelope<T> {
    payload: Option<T>,
    items: Vec<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
struct DerivedEnvelope {
    mission_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
struct RecursiveTrace {
    name: String,
    children: Vec<RecursiveTrace>,
}

fn schema_type_name(schema: &SchemaBundle) -> &str {
    let TypeDef::Named(name) = &schema.root else {
        panic!("expected named schema root, got {:?}", schema.root);
    };
    name.as_str()
}

fn root_definition(schema: &SchemaBundle) -> &TypeDefinition {
    let TypeDef::Named(name) = &schema.root else {
        panic!("expected named schema root, got {:?}", schema.root);
    };
    schema
        .definitions
        .get(name)
        .unwrap_or_else(|| panic!("missing root definition {name}"))
}

fn struct_fields(schema: &SchemaBundle) -> &[FieldDef] {
    let TypeDefinition::Struct(definition) = root_definition(schema) else {
        panic!("expected struct schema, got {schema:?}");
    };
    &definition.fields
}

fn field<'a>(schema: &'a SchemaBundle, name: &str) -> &'a FieldDef {
    struct_fields(schema)
        .iter()
        .find(|field| field.name == name)
        .unwrap_or_else(|| panic!("{name} field"))
}

fn uses_extended_types(schema: &SchemaBundle) -> bool {
    definition_uses_extended_types(root_definition(schema), schema)
}

fn definition_uses_extended_types(definition: &TypeDefinition, schema: &SchemaBundle) -> bool {
    match definition {
        TypeDefinition::Struct(definition) => definition
            .fields
            .iter()
            .any(|field| shape_uses_extended_types(&field.shape, schema)),
        TypeDefinition::Enum(_) => true,
    }
}

fn shape_uses_extended_types(shape: &TypeDef, schema: &SchemaBundle) -> bool {
    match shape {
        TypeDef::Named(name) => definition_uses_extended_types(
            schema
                .definitions
                .get(name)
                .unwrap_or_else(|| panic!("missing definition {name}")),
            schema,
        ),
        TypeDef::Optional(_) | TypeDef::Map { .. } => true,
        TypeDef::Sequence { element, .. } => shape_uses_extended_types(element, schema),
        TypeDef::Primitive(_) | TypeDef::String => false,
    }
}

fn shape_variants(schema: &SchemaBundle) -> &[EnumVariantDef] {
    let TypeDefinition::Enum(definition) = root_definition(schema) else {
        panic!("expected enum schema, got {schema:?}");
    };
    &definition.variants
}

fn payload_field(payload: &ros_z::dynamic::DynamicPayload, name: &str) -> DynamicValue {
    let DynamicValue::Struct(message) = &payload.value else {
        panic!("expected struct payload, got {payload:?}");
    };
    message.get_dynamic(name).expect("payload field")
}

fn schema_has_recursive_children(schema: &SchemaBundle) -> bool {
    let TypeDef::Named(root_name) = &schema.root else {
        return false;
    };
    let TypeDefinition::Struct(definition) = root_definition(schema) else {
        return false;
    };
    definition.fields.iter().any(|field| {
        field.name == "children"
            && matches!(
                &field.shape,
                TypeDef::Sequence {
                    element,
                    length: SequenceLengthDef::Dynamic,
                } if matches!(element.as_ref(), TypeDef::Named(name) if name == root_name)
            )
    })
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
fn extended_derive_keeps_standard_schema_for_compatible_structs() {
    let schema = TelemetryLite::schema().unwrap();
    assert!(!uses_extended_types(&schema));
    assert_eq!(
        schema_type_name(&schema),
        "extended_type_info::TelemetryLite"
    );

    assert!(uses_extended_types(&RobotEnvelope::schema().unwrap()));
    assert!(uses_extended_types(&RobotState::schema().unwrap()));
}

#[test]
fn extended_derive_generates_distinct_generic_names_and_hashes() {
    let u32_schema = GenericEnvelope::<u32>::schema().unwrap();
    let message_schema = GenericEnvelope::<TelemetryLite>::schema().unwrap();

    assert_eq!(
        GenericEnvelope::<u32>::type_name(),
        "extended_type_info::GenericEnvelope<u32>"
    );
    assert_eq!(
        GenericEnvelope::<TelemetryLite>::type_name(),
        "extended_type_info::GenericEnvelope<extended_type_info::TelemetryLite>"
    );
    assert_ne!(
        GenericEnvelope::<u32>::schema_hash().unwrap(),
        GenericEnvelope::<TelemetryLite>::schema_hash().unwrap()
    );

    let payload = field(&message_schema, "payload");
    match &payload.shape {
        TypeDef::Named(name) => {
            assert_eq!(name.as_str(), "extended_type_info::TelemetryLite");
        }
        other => panic!("expected nested message payload, got {:?}", other),
    }

    assert_eq!(
        schema_type_name(&u32_schema),
        GenericEnvelope::<u32>::type_name()
    );
}

#[test]
fn derived_message_hash_matches_manual_runtime_bundle_hash() {
    let schema = DerivedEnvelope::schema().unwrap();
    let runtime_hash = ros_z_schema::compute_hash(&schema);

    assert_eq!(DerivedEnvelope::schema_hash().unwrap(), runtime_hash);
}

#[test]
fn extended_derive_keeps_extended_only_generic_instantiations_on_extended_path() {
    let schema = GenericOptionalEnvelope::<u32>::schema().unwrap();
    assert!(uses_extended_types(&schema));
    assert_eq!(
        GenericOptionalEnvelope::<u32>::type_name(),
        "extended_type_info::GenericOptionalEnvelope<u32>"
    );

    let payload = field(&schema, "payload");
    match &payload.shape {
        TypeDef::Optional(inner) => {
            assert!(matches!(
                inner.as_ref(),
                TypeDef::Primitive(PrimitiveTypeDef::U32)
            ));
        }
        other => panic!("expected optional payload field, got {:?}", other),
    }
}

#[test]
fn dynamic_registry_round_trips_recursive_bundle_by_type_name_and_hash() {
    let schema = std::sync::Arc::new(RecursiveTrace::schema().unwrap());
    schema.validate().unwrap();
    assert!(schema_has_recursive_children(&schema));
    let schema_hash = ros_z_schema::compute_hash(schema.as_ref());
    let mut registry = SchemaRegistry::new();

    registry
        .register_root_schema(&RecursiveTrace::type_name(), std::sync::Arc::clone(&schema))
        .expect("recursive schema should register");

    let retrieved = registry
        .get_root_with_hash(&RecursiveTrace::type_name(), &schema_hash)
        .expect("recursive schema should be retrievable by type name and hash");
    assert_eq!(retrieved.as_ref(), schema.as_ref());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discovery_uses_schema_service_for_standard_compatible_types() {
    let router = TestRouter::new();

    let pub_ctx = create_context_with_router(&router)
        .await
        .expect("publisher context");
    let pub_node = pub_ctx
        .create_node("telemetry_talker")
        .build()
        .await
        .expect("publisher node");

    let publisher = pub_node
        .publisher::<TelemetryLite>("/extended_standard_topic")
        .build()
        .await
        .expect("publisher");

    let registered_hash = ros_z_schema::compute_hash(&TelemetryLite::schema().unwrap());
    let registered = pub_node
        .schema_service()
        .expect("standard schema service")
        .get_schema("extended_type_info::TelemetryLite", &registered_hash)
        .expect("schema lookup");
    assert!(
        registered.is_some(),
        "standard-compatible schema should register"
    );

    let sub_ctx = create_context_with_router(&router)
        .await
        .expect("subscriber context");
    let sub_node = sub_ctx
        .create_node("telemetry_listener")
        .build()
        .await
        .expect("subscriber node");

    let publish_task = tokio::spawn(async move {
        for _ in 0..20 {
            let message = TelemetryLite {
                label: "robot-7".to_string(),
                temperatures: vec![20.0, 20.5],
            };
            publisher.publish(&message).await.expect("publish");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    tokio::time::sleep(Duration::from_millis(400)).await;

    let subscriber = sub_node
        .dynamic_subscriber_auto("/extended_standard_topic", Duration::from_secs(10))
        .await
        .expect("dynamic subscriber")
        .build()
        .await
        .expect("subscriber build");
    let schema = subscriber.schema().expect("discovered schema");

    assert_eq!(
        schema_type_name(schema),
        "extended_type_info::TelemetryLite"
    );
    assert!(!uses_extended_types(schema));

    let message = tokio::time::timeout(Duration::from_secs(3), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");
    assert_eq!(payload_field(&message, "label").as_str(), Some("robot-7"));

    publish_task.await.expect("publisher task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn schema_service_round_trips_recursive_bundle() {
    let router = TestRouter::new();

    let pub_ctx = create_context_with_router(&router)
        .await
        .expect("publisher context");
    let pub_node = pub_ctx
        .create_node("recursive_talker")
        .build()
        .await
        .expect("publisher node");

    let publisher = pub_node
        .publisher::<RecursiveTrace>("/recursive_trace_topic")
        .build()
        .await
        .expect("publisher");

    let recursive_schema = RecursiveTrace::schema().unwrap();
    recursive_schema.validate().unwrap();
    assert!(schema_has_recursive_children(&recursive_schema));
    let registered_hash = ros_z_schema::compute_hash(&recursive_schema);
    let registered = pub_node
        .schema_service()
        .expect("schema service")
        .get_schema(&RecursiveTrace::type_name(), &registered_hash)
        .expect("schema lookup")
        .expect("recursive schema should register");
    assert_eq!(registered.schema_hash, registered_hash);
    assert_eq!(registered.schema.as_ref(), &recursive_schema);

    let sub_ctx = create_context_with_router(&router)
        .await
        .expect("subscriber context");
    let sub_node = sub_ctx
        .create_node("recursive_listener")
        .build()
        .await
        .expect("subscriber node");

    let publish_task = tokio::spawn(async move {
        for _ in 0..20 {
            let message = RecursiveTrace {
                name: "root".to_string(),
                children: vec![RecursiveTrace {
                    name: "child".to_string(),
                    children: Vec::new(),
                }],
            };
            publisher.publish(&message).await.expect("publish");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    tokio::time::sleep(Duration::from_millis(400)).await;

    let subscriber = sub_node
        .dynamic_subscriber_auto("/recursive_trace_topic", Duration::from_secs(10))
        .await
        .expect("dynamic subscriber")
        .build()
        .await
        .expect("subscriber build");
    let schema = subscriber.schema().expect("discovered schema");
    assert_eq!(schema_type_name(schema), RecursiveTrace::type_name());
    assert!(schema_has_recursive_children(schema));

    let message = tokio::time::timeout(Duration::from_secs(3), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");
    assert_eq!(payload_field(&message, "name").as_str(), Some("root"));
    let DynamicValue::Sequence(children) = payload_field(&message, "children") else {
        panic!("expected recursive children sequence");
    };
    let DynamicValue::Struct(child) = &children[0] else {
        panic!("expected recursive child struct");
    };
    assert_eq!(child.get_dynamic("name").unwrap().as_str(), Some("child"));

    publish_task.await.expect("publisher task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn extended_discovery_should_fail_when_the_publisher_disabled_the_schema_service() {
    let router = TestRouter::new();

    let pub_ctx = create_context_with_router(&router)
        .await
        .expect("publisher context");
    let pub_node = pub_ctx
        .create_node("extended_talker")
        .without_schema_service()
        .build()
        .await
        .expect("publisher node");

    let publisher = pub_node
        .publisher::<RobotEnvelope>("/extended_robot_topic")
        .build()
        .await
        .expect("publisher");

    let sub_ctx = create_context_with_router(&router)
        .await
        .expect("subscriber context");
    let sub_node = sub_ctx
        .create_node("extended_listener")
        .build()
        .await
        .expect("subscriber node");

    let publish_task = tokio::spawn(async move {
        for _ in 0..20 {
            let message = RobotEnvelope {
                label: "robot-9".to_string(),
                mission_id: Some(42),
                state: RobotState::Charging {
                    minutes_remaining: 12,
                },
            };
            publisher.publish(&message).await.expect("publish");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    tokio::time::sleep(Duration::from_millis(400)).await;

    let result = sub_node
        .dynamic_subscriber_auto("/extended_robot_topic", Duration::from_secs(3))
        .await;
    assert!(
        result.is_err(),
        "extended discovery should fail when the publisher disabled the schema service"
    );

    publish_task.await.expect("publisher task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn extended_only_types_use_schema_service_when_enabled() {
    let router = TestRouter::new();

    let pub_ctx = create_context_with_router(&router)
        .await
        .expect("publisher context");
    let pub_node = pub_ctx
        .create_node("extended_talker")
        .build()
        .await
        .expect("publisher node");

    let publisher = pub_node
        .publisher::<RobotEnvelope>("/extended_robot_topic")
        .build()
        .await
        .expect("publisher");

    let registered_hash = ros_z_schema::compute_hash(&RobotEnvelope::schema().unwrap());
    let registered = pub_node
        .schema_service()
        .expect("schema service")
        .get_schema("extended_type_info::RobotEnvelope", &registered_hash)
        .expect("schema lookup");
    let registered = registered.expect("extended schema should register");
    assert_eq!(registered.schema_hash, registered_hash);

    let sub_ctx = create_context_with_router(&router)
        .await
        .expect("subscriber context");
    let sub_node = sub_ctx
        .create_node("extended_listener")
        .build()
        .await
        .expect("subscriber node");

    let publish_task = tokio::spawn(async move {
        for _ in 0..20 {
            let message = RobotEnvelope {
                label: "robot-9".to_string(),
                mission_id: Some(42),
                state: RobotState::Charging {
                    minutes_remaining: 12,
                },
            };
            publisher.publish(&message).await.expect("publish");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    tokio::time::sleep(Duration::from_millis(400)).await;

    let subscriber = sub_node
        .dynamic_subscriber_auto("/extended_robot_topic", Duration::from_secs(10))
        .await
        .expect("dynamic subscriber")
        .build()
        .await
        .expect("subscriber build");
    let schema = subscriber.schema().expect("discovered schema");

    assert!(uses_extended_types(schema));
    assert_eq!(
        schema_type_name(schema),
        "extended_type_info::RobotEnvelope"
    );

    let message = tokio::time::timeout(Duration::from_secs(3), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");
    assert_eq!(payload_field(&message, "label").as_str(), Some("robot-9"));

    match payload_field(&message, "mission_id") {
        DynamicValue::Optional(Some(value)) => {
            assert_eq!(value.as_ref().as_u32(), Some(42));
        }
        other => panic!("expected Some mission_id, got {other:?}"),
    }

    match payload_field(&message, "state") {
        DynamicValue::Enum(value) => {
            assert_eq!(value.variant_name, "Charging");
            match value.payload {
                EnumPayloadValue::Struct(fields) => {
                    assert_eq!(fields.len(), 1);
                    assert_eq!(fields[0].name, "minutes_remaining");
                    assert_eq!(fields[0].value.as_u32(), Some(12));
                }
                other => panic!("expected struct payload, got {other:?}"),
            }
        }
        other => panic!("expected enum value, got {other:?}"),
    }

    publish_task.await.expect("publisher task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn type_description_discovery_works_across_namespaces_for_extended_types() {
    let router = TestRouter::new();

    let pub_ctx = create_context_with_router(&router)
        .await
        .expect("publisher context");
    let pub_node = pub_ctx
        .create_node("extended_talker")
        .with_namespace("tools")
        .build()
        .await
        .expect("publisher node");

    let publisher = pub_node
        .publisher::<RobotEnvelope>("/extended_robot_topic")
        .build()
        .await
        .expect("publisher");

    let sub_ctx = create_context_with_router(&router)
        .await
        .expect("subscriber context");
    let sub_node = sub_ctx
        .create_node("extended_listener")
        .with_namespace("ui")
        .build()
        .await
        .expect("subscriber node");

    let publish_task = tokio::spawn(async move {
        for _ in 0..20 {
            let message = RobotEnvelope {
                label: "robot-9".to_string(),
                mission_id: Some(42),
                state: RobotState::Charging {
                    minutes_remaining: 12,
                },
            };
            publisher.publish(&message).await.expect("publish");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    tokio::time::sleep(Duration::from_millis(400)).await;

    let subscriber = sub_node
        .dynamic_subscriber_auto("/extended_robot_topic", Duration::from_secs(10))
        .await
        .expect("dynamic subscriber")
        .build()
        .await
        .expect("subscriber build");
    let schema = subscriber.schema().expect("discovered schema");

    assert!(uses_extended_types(schema));
    assert_eq!(
        schema_type_name(schema),
        "extended_type_info::RobotEnvelope"
    );

    let message = tokio::time::timeout(Duration::from_secs(3), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");
    assert_eq!(payload_field(&message, "label").as_str(), Some("robot-9"));

    publish_task.await.expect("publisher task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn top_level_enums_are_discoverable_through_the_schema_service() {
    let router = TestRouter::new();

    let pub_ctx = create_context_with_router(&router)
        .await
        .expect("publisher context");
    let pub_node = pub_ctx
        .create_node("state_talker")
        .build()
        .await
        .expect("publisher node");

    let publisher = pub_node
        .publisher::<RobotState>("/robot_state_topic")
        .build()
        .await
        .expect("publisher");

    let sub_ctx = create_context_with_router(&router)
        .await
        .expect("subscriber context");
    let sub_node = sub_ctx
        .create_node("state_listener")
        .build()
        .await
        .expect("subscriber node");

    let publish_task = tokio::spawn(async move {
        for _ in 0..20 {
            publisher
                .publish(&RobotState::Error("battery low".to_string()))
                .await
                .expect("publish");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    tokio::time::sleep(Duration::from_millis(400)).await;

    let subscriber = sub_node
        .dynamic_subscriber_auto("/robot_state_topic", Duration::from_secs(10))
        .await
        .expect("enum discovery")
        .build()
        .await
        .expect("subscriber build");
    let schema = subscriber.schema().expect("discovered schema");

    let variants = shape_variants(schema);
    assert_eq!(variants.len(), 3);
    assert_eq!(variants[1].name, "Error");

    let message = tokio::time::timeout(Duration::from_secs(3), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");

    match message.value {
        DynamicValue::Enum(value) => {
            assert_eq!(value.variant_name, "Error");
            match value.payload {
                EnumPayloadValue::Newtype(value) => {
                    assert_eq!(value.as_ref().as_str(), Some("battery low"));
                }
                other => panic!("expected newtype payload, got {other:?}"),
            }
        }
        other => panic!("expected enum field, got {other:?}"),
    }

    publish_task.await.expect("publisher task");
}
