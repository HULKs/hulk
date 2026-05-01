//! Tests for dynamic pub/sub.

use std::sync::Arc;

use crate::dynamic::codec::DynamicCdrCodec;
use crate::dynamic::message::DynamicMessage;
use crate::dynamic::schema::{FieldType, MessageSchema};
use crate::msg::{WireDecoder, WireEncoder};
use zenoh_buffers::buffer::SplitBuffer;

fn create_test_schema() -> Arc<MessageSchema> {
    MessageSchema::builder("std_msgs::String")
        .field("data", FieldType::String)
        .build()
        .unwrap()
}

fn create_point_schema() -> Arc<MessageSchema> {
    MessageSchema::builder("geometry_msgs::Point")
        .field("x", FieldType::Float64)
        .field("y", FieldType::Float64)
        .field("z", FieldType::Float64)
        .build()
        .unwrap()
}

// Note: Full integration tests for DynamicPublisher/DynamicSubscriber require a Zenoh session.
// These are basic schema validation tests.

#[test]
fn test_builder_creation() {
    // This tests that the schema can be created properly for pub/sub
    let schema = create_test_schema();
    assert_eq!(schema.type_name_str(), "std_msgs::String");
    assert_eq!(schema.fields().len(), 1);
    assert_eq!(schema.fields()[0].name, "data");
}

#[test]
fn test_complex_schema_for_pubsub() {
    let vector3 = MessageSchema::builder("geometry_msgs::Vector3")
        .field("x", FieldType::Float64)
        .field("y", FieldType::Float64)
        .field("z", FieldType::Float64)
        .build()
        .unwrap();

    let twist = MessageSchema::builder("geometry_msgs::Twist")
        .field("linear", FieldType::Message(vector3.clone()))
        .field("angular", FieldType::Message(vector3))
        .build()
        .unwrap();

    assert_eq!(twist.type_name_str(), "geometry_msgs::Twist");
    assert_eq!(twist.fields().len(), 2);
}

// Tests for unified pub/sub using DynamicCdrCodec

#[test]
fn test_dynamic_cdr_codec_roundtrip() {
    let schema = create_point_schema();
    let mut message = DynamicMessage::new(&schema);
    message.set("x", 1.5f64).unwrap();
    message.set("y", 2.5f64).unwrap();
    message.set("z", 3.5f64).unwrap();

    // Serialize using DynamicCdrCodec (WireEncoder trait)
    let bytes = DynamicCdrCodec::serialize(&message);
    assert!(!bytes.is_empty());

    // Deserialize using DynamicCdrCodec (WireDecoder trait)
    let deserialized = DynamicCdrCodec::deserialize((&bytes, &schema)).unwrap();

    assert_eq!(deserialized.get::<f64>("x").unwrap(), 1.5);
    assert_eq!(deserialized.get::<f64>("y").unwrap(), 2.5);
    assert_eq!(deserialized.get::<f64>("z").unwrap(), 3.5);
}

#[test]
fn test_dynamic_cdr_codec_zbuf() {
    use zenoh_buffers::buffer::{Buffer, SplitBuffer};

    let schema = create_test_schema();
    let mut message = DynamicMessage::new(&schema);
    message.set("data", "Hello, unified pubsub!").unwrap();

    // Serialize to ZBuf
    let zbuf = DynamicCdrCodec::serialize_to_zbuf(&message);
    assert!(zbuf.len() > 0);

    // Convert to bytes and deserialize
    let bytes: Vec<u8> = zbuf.contiguous().to_vec();
    let deserialized = DynamicCdrCodec::deserialize((&bytes, &schema)).unwrap();

    assert_eq!(
        deserialized.get::<String>("data").unwrap(),
        "Hello, unified pubsub!"
    );
}

#[test]
fn test_dynamic_cdr_codec_to_buf() {
    let schema = create_point_schema();
    let mut message = DynamicMessage::new(&schema);
    message.set("x", 10.0f64).unwrap();
    message.set("y", 20.0f64).unwrap();
    message.set("z", 30.0f64).unwrap();

    // Serialize to existing buffer
    let mut buffer = Vec::new();
    DynamicCdrCodec::serialize_to_buf(&message, &mut buffer);

    // Should match serialize() output
    let direct = DynamicCdrCodec::serialize(&message);
    assert_eq!(buffer, direct);

    // Verify deserialize works
    let deserialized = DynamicCdrCodec::deserialize((&buffer, &schema)).unwrap();
    assert_eq!(deserialized.get::<f64>("x").unwrap(), 10.0);
}

#[test]
fn dynamic_cdr_codec_roundtrips_with_explicit_schema() {
    let schema = create_point_schema();
    let mut message = DynamicMessage::new(&schema);
    message.set("x", 5.0f64).unwrap();
    message.set("y", 6.0f64).unwrap();
    message.set("z", 7.0f64).unwrap();

    let encoded = DynamicCdrCodec::encode(&message, &schema).unwrap();
    let decoded = DynamicCdrCodec::decode(&encoded.payload.contiguous(), &schema).unwrap();

    assert_eq!(decoded.get::<f64>("x").unwrap(), 5.0);
    assert_eq!(decoded.get::<f64>("y").unwrap(), 6.0);
    assert_eq!(decoded.get::<f64>("z").unwrap(), 7.0);
}

#[test]
fn test_type_aliases_exist() {
    // Verify type aliases are accessible
    use crate::dynamic::{
        DynamicPublisher, DynamicPublisherBuilder, DynamicSubscriber, DynamicSubscriberBuilder,
    };

    // These are compile-time checks that the types exist
    fn _check_types() {
        let _: Option<DynamicPublisher> = None;
        let _: Option<DynamicSubscriber> = None;
        let _: Option<DynamicPublisherBuilder> = None;
        let _: Option<DynamicSubscriberBuilder> = None;
    }
}

// Tests for PublisherBuilder dyn_schema support

#[tokio::test(flavor = "multi_thread")]
async fn test_zpub_builder_with_dyn_schema() {
    use crate::dynamic::{DynamicCdrCodec, DynamicMessage};
    use crate::pubsub::PublisherBuilder;
    use std::marker::PhantomData;

    let schema = create_point_schema();

    // Create a mock builder to test with_dyn_schema
    let session = zenoh::Wait::wait(zenoh::open(zenoh::Config::default())).unwrap();
    let graph = std::sync::Arc::new(crate::graph::Graph::new(&session, 0).await.unwrap());
    let builder: PublisherBuilder<DynamicMessage, DynamicCdrCodec> = PublisherBuilder {
        entity: crate::entity::EndpointEntity {
            id: 0,
            node: None,
            kind: crate::entity::EndpointKind::Publisher,
            topic: String::new(),
            type_info: None,
            qos: ros_z_protocol::qos::QosProfile::default(),
        },
        session: std::sync::Arc::new(session),
        graph,
        clock: crate::time::Clock::default(),
        attachment: true,
        shm_config: None,
        dyn_schema: None,
        _phantom_data: PhantomData,
    };

    // Add schema
    let builder = builder.dyn_schema(schema.clone());
    assert!(builder.dyn_schema.is_some());
    assert_eq!(
        builder.dyn_schema.as_ref().unwrap().type_name_str(),
        "geometry_msgs::Point"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_zpub_builder_with_codec_preserves_schema() {
    use crate::dynamic::{DynamicCdrCodec, DynamicMessage};
    use crate::pubsub::PublisherBuilder;
    use std::marker::PhantomData;

    let schema = create_test_schema();

    // Create builder with schema
    let session = zenoh::Wait::wait(zenoh::open(zenoh::Config::default())).unwrap();
    let graph = std::sync::Arc::new(crate::graph::Graph::new(&session, 0).await.unwrap());
    let builder: PublisherBuilder<DynamicMessage, DynamicCdrCodec> = PublisherBuilder {
        entity: crate::entity::EndpointEntity {
            id: 0,
            node: None,
            kind: crate::entity::EndpointKind::Publisher,
            topic: String::new(),
            type_info: None,
            qos: ros_z_protocol::qos::QosProfile::default(),
        },
        session: std::sync::Arc::new(session),
        graph,
        clock: crate::time::Clock::default(),
        attachment: true,
        shm_config: None,
        dyn_schema: Some(schema.clone()),
        _phantom_data: PhantomData,
    };

    // Convert codec type - schema should be preserved
    let builder: PublisherBuilder<DynamicMessage, DynamicCdrCodec> = builder.codec();
    assert!(builder.dyn_schema.is_some());
    assert_eq!(
        builder.dyn_schema.as_ref().unwrap().type_name_str(),
        "std_msgs::String"
    );
}
