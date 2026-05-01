#![cfg(feature = "nalgebra")]

use std::time::Duration;

use nalgebra::{
    Isometry2, Isometry3, Point2, Point3, Rotation2, Rotation3, Translation2, Translation3,
    UnitComplex, UnitQuaternion, Vector2, Vector3,
};
use ros_z::{
    Message,
    context::ContextBuilder,
    dynamic::{DynamicMessage, DynamicValue, EnumPayloadValue, EnumValue, FieldType},
    entity::EntityKind,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use zenoh::{Wait, config::WhatAmI};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
#[message(name = "custom_msgs::MathSnapshot")]
struct MathSnapshot {
    image_position: Point2<f32>,
    field_position: Point3<f64>,
    image_velocity: Vector2<f32>,
    field_velocity: Vector3<f64>,
    camera_offset: Translation3<f32>,
    odometry_offset: Translation2<f64>,
    planar_rotation: Rotation2<f64>,
    camera_rotation: Rotation3<f32>,
    planar_orientation: UnitComplex<f64>,
    torso_orientation: UnitQuaternion<f32>,
    support_foot_to_ground: Isometry2<f64>,
    camera_to_ground: Isometry3<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
#[message(name = "custom_msgs::MathCommand")]
struct MathCommand {
    target_position: Point3<f64>,
    camera_to_target: Isometry3<f32>,
    target_pose: Option<Isometry2<f64>>,
    mode: MotionMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
#[message(name = "custom_msgs::MotionMode")]
enum MotionMode {
    Approach,
    Search { sweep_cycles: u32 },
}

impl ros_z::msg::WireMessage for MathSnapshot {
    type Codec = ros_z::msg::SerdeCdrCodec<Self>;
}

impl ros_z::msg::WireMessage for MathCommand {
    type Codec = ros_z::msg::SerdeCdrCodec<Self>;
}

impl ros_z::msg::WireMessage for MotionMode {
    type Codec = ros_z::msg::SerdeCdrCodec<Self>;
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

fn example_math_snapshot() -> MathSnapshot {
    MathSnapshot {
        image_position: Point2::new(12.5, -3.0),
        field_position: Point3::new(4.0, -1.5, 0.25),
        image_velocity: Vector2::new(0.5, -0.25),
        field_velocity: Vector3::new(1.0, 0.0, -0.5),
        camera_offset: Translation3::new(0.1, 0.2, 0.3),
        odometry_offset: Translation2::new(-2.0, 1.0),
        planar_rotation: Rotation2::from_matrix_unchecked(nalgebra::Matrix2::new(
            0.0, -1.0, 1.0, 0.0,
        )),
        camera_rotation: Rotation3::from_matrix_unchecked(nalgebra::Matrix3::new(
            1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 1.0, 0.0,
        )),
        planar_orientation: UnitComplex::new(std::f64::consts::FRAC_PI_2),
        torso_orientation: UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(
            1.0, 0.0, 0.0, 0.0,
        )),
        support_foot_to_ground: Isometry2::from_parts(
            Translation2::new(0.5, -0.5),
            UnitComplex::new(std::f64::consts::FRAC_PI_4),
        ),
        camera_to_ground: Isometry3::from_parts(
            Translation3::new(0.0, 0.1, 0.2),
            UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(1.0, 0.0, 0.0, 0.0)),
        ),
    }
}

fn example_math_command() -> MathCommand {
    MathCommand {
        target_position: Point3::new(3.0, -2.0, 0.4),
        camera_to_target: Isometry3::from_parts(
            Translation3::new(0.3, 0.0, 0.9),
            UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(1.0, 0.0, 0.0, 0.0)),
        ),
        target_pose: Some(Isometry2::from_parts(
            Translation2::new(1.5, -0.25),
            UnitComplex::new(std::f64::consts::FRAC_PI_6),
        )),
        mode: MotionMode::Search { sweep_cycles: 3 },
    }
}

fn dynamic_message_to_json(message: &DynamicMessage) -> Value {
    let mut fields = Map::new();
    for (name, value) in message.iter() {
        fields.insert(name.to_string(), dynamic_value_to_json(value));
    }
    Value::Object(fields)
}

fn dynamic_value_to_json(value: &DynamicValue) -> Value {
    match value {
        DynamicValue::Bool(value) => Value::Bool(*value),
        DynamicValue::Int8(value) => Value::Number((*value).into()),
        DynamicValue::Int16(value) => Value::Number((*value).into()),
        DynamicValue::Int32(value) => Value::Number((*value).into()),
        DynamicValue::Int64(value) => Value::Number((*value).into()),
        DynamicValue::Uint8(value) => Value::Number((*value).into()),
        DynamicValue::Uint16(value) => Value::Number((*value).into()),
        DynamicValue::Uint32(value) => Value::Number((*value).into()),
        DynamicValue::Uint64(value) => Value::Number((*value).into()),
        DynamicValue::Float32(value) => serde_json::Number::from_f64(*value as f64)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        DynamicValue::Float64(value) => serde_json::Number::from_f64(*value)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        DynamicValue::String(value) => Value::String(value.clone()),
        DynamicValue::Bytes(value) => Value::Array(
            value
                .iter()
                .map(|byte| Value::Number((*byte).into()))
                .collect(),
        ),
        DynamicValue::Message(value) => dynamic_message_to_json(value),
        DynamicValue::Optional(None) => Value::Null,
        DynamicValue::Optional(Some(value)) => dynamic_value_to_json(value),
        DynamicValue::Enum(value) => enum_value_to_json(value),
        DynamicValue::Array(values) => {
            Value::Array(values.iter().map(dynamic_value_to_json).collect())
        }
        DynamicValue::Map(entries) => Value::Array(
            entries
                .iter()
                .map(|(key, value)| {
                    let mut entry = Map::new();
                    entry.insert("key".to_string(), dynamic_value_to_json(key));
                    entry.insert("value".to_string(), dynamic_value_to_json(value));
                    Value::Object(entry)
                })
                .collect(),
        ),
    }
}

fn enum_value_to_json(value: &EnumValue) -> Value {
    let mut fields = Map::new();
    fields.insert(
        "variant_index".to_string(),
        Value::Number(value.variant_index.into()),
    );
    fields.insert(
        "variant_name".to_string(),
        Value::String(value.variant_name.clone()),
    );
    fields.insert("payload".to_string(), enum_payload_to_json(&value.payload));
    Value::Object(fields)
}

fn enum_payload_to_json(payload: &EnumPayloadValue) -> Value {
    match payload {
        EnumPayloadValue::Unit => Value::Null,
        EnumPayloadValue::Newtype(value) => dynamic_value_to_json(value),
        EnumPayloadValue::Tuple(values) => {
            Value::Array(values.iter().map(dynamic_value_to_json).collect())
        }
        EnumPayloadValue::Struct(fields) => Value::Object(
            fields
                .iter()
                .map(|field| (field.name.clone(), dynamic_value_to_json(&field.value)))
                .collect(),
        ),
    }
}

async fn wait_for_publishers(
    node: &ros_z::node::Node,
    topic: &str,
    expected_count: usize,
    timeout: Duration,
) {
    let start = tokio::time::Instant::now();
    loop {
        if node.graph().count(EntityKind::Publisher, topic) >= expected_count {
            return;
        }
        assert!(
            start.elapsed() < timeout,
            "publisher for {topic} was not discovered within {timeout:?}"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[test]
fn standard_nalgebra_schema_is_basic_only() {
    let schema = MathSnapshot::schema();
    assert!(!schema.uses_extended_types());

    let image_position = schema
        .field("image_position")
        .expect("image_position field");
    assert_eq!(
        image_position.field_type,
        FieldType::Array(Box::new(FieldType::Float32), 2)
    );

    let planar_orientation = schema
        .field("planar_orientation")
        .expect("planar_orientation field");
    assert_eq!(
        planar_orientation.field_type,
        FieldType::Array(Box::new(FieldType::Float64), 2)
    );

    let camera_to_ground = schema
        .field("camera_to_ground")
        .expect("camera_to_ground field");
    let FieldType::Message(isometry_schema) = &camera_to_ground.field_type else {
        panic!("expected Isometry3 to map to a nested message schema");
    };
    assert_eq!(isometry_schema.type_name_str(), "nalgebra::Isometry3F32");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nalgebra_fields_roundtrip_via_standard_discovery() {
    let router = TestRouter::new();

    let pub_ctx = create_context_with_router(&router)
        .await
        .expect("publisher context");
    let pub_node = pub_ctx
        .create_node("math_snapshot_talker")
        .build()
        .await
        .expect("publisher node");

    let publisher = pub_node
        .publisher::<MathSnapshot>("/math_snapshot")
        .build()
        .await
        .expect("publisher");

    let expected = example_math_snapshot();
    let expected_json = serde_json::to_value(&expected).expect("serde snapshot json");
    let publish_task = tokio::spawn(async move {
        for _ in 0..20 {
            publisher.publish(&expected).await.expect("publish");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    let sub_ctx = create_context_with_router(&router)
        .await
        .expect("subscriber context");
    let sub_node = sub_ctx
        .create_node("math_snapshot_listener")
        .build()
        .await
        .expect("subscriber node");
    wait_for_publishers(&sub_node, "/math_snapshot", 1, Duration::from_secs(2)).await;
    let subscriber = sub_node
        .dynamic_subscriber_auto("/math_snapshot", Duration::from_secs(10))
        .await
        .expect("subscriber discovery")
        .build()
        .await
        .expect("subscriber build");
    let schema = subscriber.schema().expect("discovered schema");
    assert!(!schema.uses_extended_types());

    let message = tokio::time::timeout(Duration::from_secs(3), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");
    assert_eq!(dynamic_message_to_json(&message), expected_json);

    publish_task.await.expect("publisher task");
}

#[test]
fn single_schema_message_can_embed_basic_nalgebra_fields() {
    let schema = MathCommand::schema();
    assert!(schema.uses_extended_types());

    let target_position = schema
        .field("target_position")
        .expect("target_position field");
    assert_eq!(
        target_position.field_type,
        FieldType::Array(Box::new(FieldType::Float64), 3)
    );

    let camera_to_target = schema
        .field("camera_to_target")
        .expect("camera_to_target field");
    let FieldType::Message(nested) = &camera_to_target.field_type else {
        panic!("expected Isometry3 field to stay standard-compatible inside extended schemas");
    };
    assert_eq!(nested.type_name_str(), "nalgebra::Isometry3F32");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn single_schema_discovery_works_with_basic_nalgebra_fields() {
    let router = TestRouter::new();

    let pub_ctx = create_context_with_router(&router)
        .await
        .expect("publisher context");
    let pub_node = pub_ctx
        .create_node("math_command_talker")
        .build()
        .await
        .expect("publisher node");

    let publisher = pub_node
        .publisher::<MathCommand>("/math_command")
        .build()
        .await
        .expect("publisher");

    let expected = example_math_command();
    let expected_target_position =
        serde_json::to_value(expected.target_position).expect("serde target_position json");
    let expected_camera_to_target =
        serde_json::to_value(expected.camera_to_target).expect("serde camera_to_target json");
    let expected_target_pose =
        serde_json::to_value(expected.target_pose).expect("serde target_pose json");
    let publish_task = tokio::spawn(async move {
        for _ in 0..20 {
            publisher.publish(&expected).await.expect("publish");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    let sub_ctx = create_context_with_router(&router)
        .await
        .expect("subscriber context");
    let sub_node = sub_ctx
        .create_node("math_command_listener")
        .build()
        .await
        .expect("subscriber node");
    wait_for_publishers(&sub_node, "/math_command", 1, Duration::from_secs(2)).await;
    let subscriber = sub_node
        .dynamic_subscriber_auto("/math_command", Duration::from_secs(10))
        .await
        .expect("subscriber discovery")
        .build()
        .await
        .expect("subscriber build");

    let schema = subscriber.schema().expect("discovered schema");
    assert!(schema.uses_extended_types());

    let message = tokio::time::timeout(Duration::from_secs(3), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");
    assert_eq!(
        dynamic_value_to_json(
            &message
                .get_dynamic("target_position")
                .expect("target_position value")
        ),
        expected_target_position,
    );
    assert_eq!(
        dynamic_value_to_json(
            &message
                .get_dynamic("camera_to_target")
                .expect("camera_to_target value")
        ),
        expected_camera_to_target,
    );
    assert_eq!(
        dynamic_value_to_json(
            &message
                .get_dynamic("target_pose")
                .expect("target_pose value")
        ),
        expected_target_pose,
    );

    let mode_value = message.get_dynamic("mode").expect("mode value");
    let mode = mode_value.as_enum().expect("enum mode");
    assert_eq!(mode.variant_name, "Search");

    publish_task.await.expect("publisher task");
}
