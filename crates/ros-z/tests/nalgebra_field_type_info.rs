#![cfg(feature = "nalgebra")]

use std::time::Duration;

use nalgebra::{
    Isometry2, Isometry3, Point2, Point3, Rotation2, Rotation3, Translation2, Translation3,
    UnitComplex, UnitQuaternion, Vector2, Vector3,
};
use ros_z::{
    Message,
    context::ContextBuilder,
    dynamic::{
        DynamicStruct, DynamicValue, EnumPayloadValue, EnumValue, PrimitiveType,
        RuntimeFieldSchema, SequenceLength, TypeShape,
    },
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

fn dynamic_message_to_json(message: &DynamicStruct) -> Value {
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
        DynamicValue::Struct(value) => dynamic_message_to_json(value),
        DynamicValue::Optional(None) => Value::Null,
        DynamicValue::Optional(Some(value)) => dynamic_value_to_json(value),
        DynamicValue::Enum(value) => enum_value_to_json(value),
        DynamicValue::Sequence(values) => {
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

fn schema_uses_extended_types(schema: &TypeShape) -> bool {
    match schema {
        TypeShape::Struct { fields, .. } => fields
            .iter()
            .any(|field| schema_uses_extended_types(field.schema.as_ref())),
        TypeShape::Enum { .. } | TypeShape::Optional(_) | TypeShape::Map { .. } => true,
        TypeShape::Sequence { element, .. } => schema_uses_extended_types(element.as_ref()),
        TypeShape::Primitive(_) | TypeShape::String => false,
    }
}

fn field<'a>(schema: &'a TypeShape, name: &str) -> &'a RuntimeFieldSchema {
    let TypeShape::Struct { fields, .. } = schema else {
        panic!("expected struct schema");
    };
    fields
        .iter()
        .find(|field| field.name == name)
        .expect("field")
}

fn fixed_sequence(schema: &TypeShape) -> Option<(PrimitiveType, usize)> {
    let TypeShape::Sequence {
        element,
        length: SequenceLength::Fixed(length),
    } = schema
    else {
        return None;
    };
    let TypeShape::Primitive(primitive) = element.as_ref() else {
        return None;
    };
    Some((*primitive, *length))
}

fn dynamic_payload_to_json(message: &ros_z::dynamic::DynamicPayload) -> Value {
    dynamic_value_to_json(&message.value)
}

fn dynamic_payload_field(
    message: &ros_z::dynamic::DynamicPayload,
    name: &str,
) -> Option<DynamicValue> {
    let DynamicValue::Struct(message) = &message.value else {
        return None;
    };
    message.get_dynamic(name).ok()
}

#[test]
fn vector3_f64_schema_is_fixed_float_sequence() {
    let schema = Vector3::<f64>::schema();

    assert_eq!(Vector3::<f64>::type_name(), "nalgebra::Vector3<f64>");
    assert!(matches!(
        schema.as_ref(),
        TypeShape::Sequence {
            element,
            length: SequenceLength::Fixed(3),
        } if matches!(element.as_ref(), TypeShape::Primitive(PrimitiveType::F64))
    ));
}

#[test]
fn isometry3_f32_schema_has_rotation_and_translation_fields() {
    let schema = Isometry3::<f32>::schema();

    assert_eq!(Isometry3::<f32>::type_name(), "nalgebra::Isometry3<f32>");
    let TypeShape::Struct { name, fields } = schema.as_ref() else {
        panic!("isometry schema should be a struct");
    };

    assert_eq!(name.as_str(), "nalgebra::Isometry3<f32>");
    assert_eq!(fields[0].name, "rotation");
    assert_eq!(fields[1].name, "translation");
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
    assert!(!schema_uses_extended_types(schema.as_ref()));

    assert_eq!(
        fixed_sequence(field(schema.as_ref(), "image_position").schema.as_ref()),
        Some((PrimitiveType::F32, 2))
    );

    assert_eq!(
        fixed_sequence(field(schema.as_ref(), "planar_orientation").schema.as_ref()),
        Some((PrimitiveType::F64, 2))
    );

    let TypeShape::Struct { name, .. } = field(schema.as_ref(), "camera_to_ground").schema.as_ref()
    else {
        panic!("expected Isometry3 to map to a nested message schema");
    };
    assert_eq!(name.as_str(), "nalgebra::Isometry3<f32>");
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
    assert!(!schema_uses_extended_types(schema));

    let message = tokio::time::timeout(Duration::from_secs(3), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");
    assert_eq!(dynamic_payload_to_json(&message), expected_json);

    publish_task.await.expect("publisher task");
}

#[test]
fn single_schema_message_can_embed_basic_nalgebra_fields() {
    let schema = MathCommand::schema();
    assert!(schema_uses_extended_types(schema.as_ref()));

    assert_eq!(
        fixed_sequence(field(schema.as_ref(), "target_position").schema.as_ref()),
        Some((PrimitiveType::F64, 3))
    );

    let TypeShape::Struct { name, .. } = field(schema.as_ref(), "camera_to_target").schema.as_ref()
    else {
        panic!("expected Isometry3 field to stay standard-compatible inside extended schemas");
    };
    assert_eq!(name.as_str(), "nalgebra::Isometry3<f32>");
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
    assert!(schema_uses_extended_types(schema));

    let message = tokio::time::timeout(Duration::from_secs(3), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");
    assert_eq!(
        dynamic_value_to_json(
            &dynamic_payload_field(&message, "target_position").expect("target_position value")
        ),
        expected_target_position,
    );
    assert_eq!(
        dynamic_value_to_json(
            &dynamic_payload_field(&message, "camera_to_target").expect("camera_to_target value")
        ),
        expected_camera_to_target,
    );
    assert_eq!(
        dynamic_value_to_json(
            &dynamic_payload_field(&message, "target_pose").expect("target_pose value")
        ),
        expected_target_pose,
    );

    let mode_value = dynamic_payload_field(&message, "mode").expect("mode value");
    let mode = mode_value.as_enum().expect("enum mode");
    assert_eq!(mode.variant_name, "Search");

    publish_task.await.expect("publisher task");
}
