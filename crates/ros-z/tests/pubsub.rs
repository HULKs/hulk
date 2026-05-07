use std::{num::NonZeroUsize, time::Duration};

use ros_z::{
    Message, SchemaHash,
    attachment::Attachment,
    context::ContextBuilder,
    entity::{Entity, EntityKind, TypeInfo},
    message::SerdeCdrCodec,
    qos::{QosDurability, QosHistory, QosProfile, QosReliability},
    schema::{MessageSchema, SchemaBuilder},
    time::{Clock, Time},
};
use ros_z_schema::{PrimitiveTypeDef, SchemaError, SequenceLengthDef, TypeDef, TypeName};
use ros_z_schema::{SchemaBundle, StructDef, TypeDefinition, TypeDefinitions};
use serde::{Deserialize, Serialize};
use serde_json::json;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestMessage {
    data: Vec<u8>,
    counter: u64,
}

impl Message for TestMessage {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        "test_msgs::TestMessage".to_string()
    }

    fn schema_hash() -> Result<SchemaHash, SchemaError> {
        Ok(SchemaHash::zero())
    }
}

impl MessageSchema for TestMessage {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        builder.define_message_struct::<Self>(|fields| {
            fields.field_with_shape(
                "data",
                TypeDef::Sequence {
                    element: Box::new(TypeDef::Primitive(PrimitiveTypeDef::U8)),
                    length: SequenceLengthDef::Dynamic,
                },
            );
            fields.field_with_shape("counter", TypeDef::Primitive(PrimitiveTypeDef::U64));
            Ok(())
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct CacheSchemaHashMessage {
    data: String,
}

impl Message for CacheSchemaHashMessage {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        "test_msgs::CacheSchemaHashMessage".to_string()
    }

    fn schema_hash() -> Result<SchemaHash, SchemaError> {
        Ok(SchemaHash([0x55; 32]))
    }

    fn type_info() -> Result<TypeInfo, SchemaError> {
        Ok(TypeInfo::with_hash(
            &Self::type_name(),
            Self::schema_hash()?,
        ))
    }
}

impl MessageSchema for CacheSchemaHashMessage {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        builder.define_message_struct::<Self>(|fields| {
            fields.field::<String>("data")?;
            Ok(())
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct AdvertisedTypeInfoSchemaMessage {
    data: String,
}

impl Message for AdvertisedTypeInfoSchemaMessage {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        "test_msgs::AdvertisedTypeInfoSchemaMessage".to_string()
    }

    fn schema_hash() -> Result<SchemaHash, SchemaError> {
        Ok(SchemaHash([0x77; 32]))
    }

    fn type_info() -> Result<TypeInfo, SchemaError> {
        Ok(TypeInfo::with_hash(
            "test_msgs::AdvertisedTypeInfoSchemaMessageAlias",
            Self::schema_hash()?,
        ))
    }
}

impl MessageSchema for AdvertisedTypeInfoSchemaMessage {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        builder.define_message_struct::<Self>(|fields| {
            fields.field::<String>("data")?;
            Ok(())
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct InvalidSchemaMessage;

impl MessageSchema for InvalidSchemaMessage {
    fn build_schema(_builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        TypeName::new("").map(TypeDef::Named)
    }
}

impl Message for InvalidSchemaMessage {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        "test_msgs::InvalidSchemaMessage".to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct InvalidSchemaManualTypeInfoMessage;

impl MessageSchema for InvalidSchemaManualTypeInfoMessage {
    fn build_schema(_builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        TypeName::new("").map(TypeDef::Named)
    }
}

impl Message for InvalidSchemaManualTypeInfoMessage {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        "test_msgs::InvalidSchemaManualTypeInfoMessage".to_string()
    }

    fn type_info() -> Result<TypeInfo, SchemaError> {
        Ok(TypeInfo::new(&Self::type_name(), None))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct MismatchedRootSchemaMessage {
    data: String,
}

impl MessageSchema for MismatchedRootSchemaMessage {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new("test_msgs::ActualRoot")?;
        builder.define_struct(name, |fields| {
            fields.field::<String>("data")?;
            Ok(())
        })
    }
}

impl Message for MismatchedRootSchemaMessage {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        "test_msgs::AdvertisedRoot".to_string()
    }
}

fn mismatched_dynamic_schema() -> ros_z::dynamic::Schema {
    let actual = TypeName::new("test_msgs::ActualDynamicRoot").unwrap();
    std::sync::Arc::new(SchemaBundle {
        root: TypeDef::Named(actual.clone()),
        definitions: TypeDefinitions::from([(
            actual,
            TypeDefinition::Struct(StructDef { fields: vec![] }),
        )]),
    })
}

async fn test_context() -> zenoh::Result<ros_z::context::Context> {
    ContextBuilder::default().build().await
}

#[tokio::test(flavor = "multi_thread")]
async fn node_builder_enables_schema_service_by_default() -> zenoh::Result<()> {
    let context = test_context().await?;
    let node = context
        .create_node("default_schema_service")
        .build()
        .await?;

    assert!(node.schema_service().is_some());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn node_builder_can_disable_schema_service_explicitly() -> zenoh::Result<()> {
    let context = test_context().await?;
    let node = context
        .create_node("no_schema_service")
        .without_schema_service()
        .build()
        .await?;

    assert!(node.schema_service().is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn raw_subscriber_receives_sample_payload() -> zenoh::Result<()> {
    let context = ContextBuilder::default().build().await?;
    let node = context.create_node("raw_subscriber_node").build().await?;
    let publisher = node.publisher::<TestMessage>("/raw_topic").build().await?;
    let mut subscriber = node
        .subscriber::<TestMessage>("/raw_topic")
        .raw()
        .build()
        .await?;

    tokio::time::sleep(Duration::from_millis(100)).await;
    publisher
        .publish(&TestMessage {
            data: vec![4, 2],
            counter: 42,
        })
        .await?;

    let sample = tokio::time::timeout(Duration::from_secs(1), subscriber.recv()).await??;
    assert!(!sample.payload().to_bytes().is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn typed_pubsub_builders_return_schema_errors() {
    let context = ContextBuilder::default()
        .build()
        .await
        .expect("Failed to create context");
    let node = context
        .create_node("invalid_schema_builders")
        .build()
        .await
        .expect("Failed to create node");

    let publisher = node
        .publisher::<InvalidSchemaMessage>("/invalid_schema_publisher")
        .build()
        .await;
    assert!(publisher.is_err());

    let subscriber = node
        .subscriber::<InvalidSchemaMessage>("/invalid_schema_subscriber")
        .build()
        .await;
    assert!(subscriber.is_err());

    let cache = node
        .create_cache::<InvalidSchemaMessage>("/invalid_schema_cache", 1)
        .build()
        .await;
    assert!(cache.is_err());

    let manual_publisher = node
        .publisher::<InvalidSchemaManualTypeInfoMessage>("/invalid_manual_schema_publisher")
        .build()
        .await;
    assert!(manual_publisher.is_err());

    let manual_subscriber = node
        .subscriber::<InvalidSchemaManualTypeInfoMessage>("/invalid_manual_schema_subscriber")
        .build()
        .await;
    assert!(manual_subscriber.is_err());

    let manual_cache = node
        .create_cache::<InvalidSchemaManualTypeInfoMessage>("/invalid_manual_schema_cache", 1)
        .build()
        .await;
    assert!(manual_cache.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn typed_publisher_rejects_schema_root_that_differs_from_type_name() {
    let context = ContextBuilder::default()
        .build()
        .await
        .expect("Failed to create context");
    let node = context
        .create_node("mismatched_schema_root")
        .build()
        .await
        .expect("Failed to create node");

    let error = node
        .publisher::<MismatchedRootSchemaMessage>("/mismatched_schema_root")
        .build()
        .await
        .expect_err("mismatched schema root should fail publisher build");
    let message = error.to_string();

    assert!(message.contains("schema root"));
    assert!(message.contains("test_msgs::AdvertisedRoot"));
    assert!(message.contains("test_msgs::ActualRoot"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn dynamic_publisher_rejects_schema_root_that_differs_from_type_info() {
    let context = ContextBuilder::default()
        .build()
        .await
        .expect("Failed to create context");
    let node = context
        .create_node("mismatched_dynamic_schema_root")
        .build()
        .await
        .expect("Failed to create node");
    let type_info = TypeInfo::new("test_msgs::AdvertisedDynamicRoot", None);

    let error = node
        .dynamic_publisher(
            "/mismatched_dynamic_schema_root",
            type_info,
            mismatched_dynamic_schema(),
        )
        .build()
        .await
        .expect_err("mismatched schema root should fail dynamic publisher build");
    let message = error.to_string();

    assert!(message.contains("schema root"));
    assert!(message.contains("test_msgs::AdvertisedDynamicRoot"));
    assert!(message.contains("test_msgs::ActualDynamicRoot"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_basic_pubsub() {
    let context = ContextBuilder::default()
        .build()
        .await
        .expect("Failed to create context");
    let node = context
        .create_node("test_node")
        .build()
        .await
        .expect("Failed to create node");

    let publisher = node
        .publisher::<TestMessage>("/test_topic")
        .build()
        .await
        .unwrap();

    let subscriber = node
        .subscriber::<TestMessage>("/test_topic")
        .build()
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let message = TestMessage {
        data: vec![1, 2, 3, 4, 5],
        counter: 42,
    };
    publisher.publish(&message).await.unwrap();

    let received_msg = tokio::time::timeout(Duration::from_secs(1), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");
    assert_eq!(received_msg, message);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transient_local_build_waits_for_initial_replay() -> zenoh::Result<()> {
    let context = ContextBuilder::default().build().await?;
    let pub_node = context
        .create_node("transient_build_order_pub")
        .build()
        .await?;
    let sub_node = context
        .create_node("transient_build_order_sub")
        .build()
        .await?;
    let topic = "/transient_local_build_waits_for_initial_replay";
    let qos = QosProfile {
        durability: QosDurability::TransientLocal,
        reliability: QosReliability::Reliable,
        history: QosHistory::KeepLast(NonZeroUsize::new(1).unwrap()),
        ..Default::default()
    };

    let publisher = pub_node
        .publisher::<TestMessage>(topic)
        .qos(qos)
        .build()
        .await?;
    let message = TestMessage {
        data: vec![1, 3, 5],
        counter: 13,
    };
    publisher.publish(&message).await?;

    let subscriber = sub_node
        .subscriber::<TestMessage>(topic)
        .qos(qos)
        .build()
        .await?;
    let received = tokio::select! {
        biased;
        result = subscriber.recv() => result?,
        () = std::future::ready(()) => {
            panic!("initial replay should be queued before build returns")
        }
    };

    assert_eq!(received, message);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_multiple_messages() {
    let context = ContextBuilder::default()
        .build()
        .await
        .expect("Failed to create context");
    let node = context
        .create_node("multi_msg_node")
        .build()
        .await
        .expect("Failed to create node");

    let publisher = node
        .publisher::<TestMessage>("/multi_topic")
        .build()
        .await
        .unwrap();

    let subscriber = node
        .subscriber::<TestMessage>("/multi_topic")
        .build()
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    for i in 0..5 {
        let message = TestMessage {
            data: vec![i as u8; 100],
            counter: i,
        };
        publisher.publish(&message).await.unwrap();
    }

    for i in 0..5 {
        let received_msg = tokio::time::timeout(Duration::from_secs(1), subscriber.recv())
            .await
            .expect("receive should not time out")
            .expect("receive should succeed");
        assert_eq!(received_msg.counter, i);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn create_cache_uses_schema_hash_method_when_type_info_is_available() {
    let context = ContextBuilder::default()
        .build()
        .await
        .expect("Failed to create context");
    let node = context
        .create_node("test_cache_schema_hash")
        .build()
        .await
        .expect("Failed to create node");
    let topic = "/test_cache_schema_hash";

    let _cache = node
        .create_cache::<CacheSchemaHashMessage>(topic, 4)
        .build()
        .await
        .expect("cache should build");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let expected_hash = CacheSchemaHashMessage::schema_hash().unwrap();
    assert_ne!(expected_hash, SchemaHash::zero());
    let endpoint = node
        .graph()
        .get_entities_by_topic(EntityKind::Subscription, topic)
        .into_iter()
        .find_map(|entity| match &*entity {
            Entity::Endpoint(endpoint) => Some(endpoint.clone()),
            _ => None,
        })
        .expect("cache subscriber should be discoverable");
    let advertised = endpoint.type_info.expect("cache subscriber type info");

    assert_eq!(advertised.name, CacheSchemaHashMessage::type_name());
    assert_eq!(advertised.hash, Some(expected_hash));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn publisher_schema_service_uses_schema_derived_key_when_type_info_diverges() {
    let context = ContextBuilder::default()
        .build()
        .await
        .expect("Failed to create context");
    let node = context
        .create_node("advertised_schema_service")
        .build()
        .await
        .expect("Failed to create node");

    let _publisher = node
        .publisher::<AdvertisedTypeInfoSchemaMessage>("/advertised_schema_service")
        .build()
        .await
        .expect("publisher should build");

    let advertised = AdvertisedTypeInfoSchemaMessage::type_info().unwrap();
    let advertised_hash = advertised.hash.expect("type info should include a hash");
    let canonical_schema = AdvertisedTypeInfoSchemaMessage::schema().unwrap();
    let canonical_hash = ros_z_schema::compute_hash(&canonical_schema);

    let advertised_lookup = node
        .schema_service()
        .expect("schema service")
        .get_schema(&advertised.name, &advertised_hash)
        .expect("schema lookup should succeed");
    let registered = node
        .schema_service()
        .expect("schema service")
        .get_schema(
            &AdvertisedTypeInfoSchemaMessage::type_name(),
            &canonical_hash,
        )
        .expect("schema lookup should succeed")
        .expect("schema should be registered under the canonical schema key");

    assert!(advertised_lookup.is_none());
    assert_ne!(advertised_hash, canonical_hash);
    assert_eq!(registered.schema_hash, canonical_hash);
    assert_eq!(
        registered.root_name,
        AdvertisedTypeInfoSchemaMessage::type_name()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn typed_pubsub_advertises_canonical_schema_hash_when_schema_hash_override_is_stale() {
    let context = ContextBuilder::default()
        .build()
        .await
        .expect("Failed to create context");
    let node = context
        .create_node("typed_canonical_schema_hash")
        .build()
        .await
        .expect("Failed to create node");
    let topic = "/typed_canonical_schema_hash";

    let _publisher = node
        .publisher::<AdvertisedTypeInfoSchemaMessage>(topic)
        .build()
        .await
        .expect("publisher should build");
    let _subscriber = node
        .subscriber::<AdvertisedTypeInfoSchemaMessage>(topic)
        .build()
        .await
        .expect("subscriber should build");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let canonical_schema = AdvertisedTypeInfoSchemaMessage::schema().unwrap();
    let canonical_hash = ros_z_schema::compute_hash(&canonical_schema);
    assert_ne!(
        canonical_hash,
        AdvertisedTypeInfoSchemaMessage::schema_hash().unwrap()
    );

    for kind in [EntityKind::Publisher, EntityKind::Subscription] {
        let endpoint = node
            .graph()
            .get_entities_by_topic(kind, topic)
            .into_iter()
            .find_map(|entity| match &*entity {
                Entity::Endpoint(endpoint) => Some(endpoint.clone()),
                _ => None,
            })
            .expect("endpoint should be discoverable");
        let advertised = endpoint.type_info.expect("endpoint type info");

        assert_eq!(
            advertised.name,
            AdvertisedTypeInfoSchemaMessage::type_name()
        );
        assert_eq!(advertised.hash, Some(canonical_hash));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn dynamic_publisher_advertises_explicit_schema_hash() {
    let context = ContextBuilder::default()
        .build()
        .await
        .expect("Failed to create context");
    let node = context
        .create_node("dynamic_explicit_schema_hash")
        .build()
        .await
        .expect("Failed to create node");
    let topic = "/dynamic_explicit_schema_hash";
    let root_name = "test_msgs::DynamicExplicitHash".to_string();
    let root_type_name = TypeName::new(&root_name).unwrap();
    let mut builder = SchemaBuilder::new();
    let root = builder
        .define_struct(root_type_name, |fields| {
            fields.field::<String>("data")?;
            Ok(())
        })
        .unwrap();
    let root_schema = std::sync::Arc::new(builder.finish(root).unwrap());
    let schema_hash = ros_z_schema::compute_hash(root_schema.as_ref());

    let _publisher = node
        .dynamic_publisher(
            topic,
            TypeInfo::with_hash(&root_name, schema_hash),
            root_schema,
        )
        .build()
        .await
        .expect("publisher should build");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let endpoint = node
        .graph()
        .get_entities_by_topic(EntityKind::Publisher, topic)
        .into_iter()
        .find_map(|entity| match &*entity {
            Entity::Endpoint(endpoint) => Some(endpoint.clone()),
            _ => None,
        })
        .expect("dynamic publisher should be discoverable");
    let advertised = endpoint.type_info.expect("publisher type info");
    let registered = node
        .schema_service()
        .expect("schema service")
        .get_schema(&root_name, &schema_hash)
        .expect("schema lookup should succeed")
        .expect("schema should be registered under root hash");

    assert_eq!(advertised.name, root_name);
    assert_eq!(advertised.hash, Some(schema_hash));
    assert_eq!(registered.schema_hash, schema_hash);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_recv_with_metadata_preserves_receive_context() {
    let context = ContextBuilder::default()
        .build()
        .await
        .expect("Failed to create context");
    let node = context
        .create_node("metadata_node")
        .build()
        .await
        .expect("Failed to create node");

    let publisher = node
        .publisher::<TestMessage>("/metadata_topic")
        .build()
        .await
        .unwrap();

    let subscriber = node
        .subscriber::<TestMessage>("/metadata_topic")
        .build()
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let message = TestMessage {
        data: vec![9, 8, 7],
        counter: 7,
    };
    publisher.publish(&message).await.unwrap();

    let received = tokio::time::timeout(Duration::from_secs(1), subscriber.recv_with_metadata())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");
    assert_eq!(received.message, message);
    assert!(received.transport_time.is_some());
    assert!(received.source_time.is_some());
    assert!(received.sequence_number.is_some());
    assert!(received.source_global_id.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_large_payload() {
    let context = ContextBuilder::default()
        .build()
        .await
        .expect("Failed to create context");
    let node = context
        .create_node("large_payload_node")
        .build()
        .await
        .expect("Failed to create node");

    let publisher = node
        .publisher::<TestMessage>("/large_topic")
        .build()
        .await
        .unwrap();

    let subscriber = node
        .subscriber::<TestMessage>("/large_topic")
        .build()
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let message = TestMessage {
        data: vec![0xAB; 1024 * 1024],
        counter: 999,
    };
    publisher.publish(&message).await.unwrap();

    let received_msg = tokio::time::timeout(Duration::from_secs(2), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");
    assert_eq!(received_msg.counter, 999);
    assert_eq!(received_msg.data.len(), 1024 * 1024);
    assert_eq!(received_msg.data[0], 0xAB);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_logical_clock_is_used_for_attachment_timestamps() {
    let clock = Clock::logical(Time::zero());
    clock.advance(Duration::from_secs(5)).unwrap();

    let context = ContextBuilder::default()
        .with_clock(clock.clone())
        .build()
        .await
        .expect("Failed to create context");
    let node = context
        .create_node("sim_clock_node")
        .build()
        .await
        .expect("Failed to create node");

    let publisher = node
        .publisher::<TestMessage>("/sim_clock")
        .build()
        .await
        .unwrap();
    let mut subscriber = node
        .subscriber::<TestMessage>("/sim_clock")
        .raw()
        .build()
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    publisher
        .publish(&TestMessage {
            data: vec![1],
            counter: 1,
        })
        .await
        .unwrap();

    let sample = tokio::time::timeout(Duration::from_secs(1), subscriber.recv())
        .await
        .expect("receive should not time out")
        .expect("receive should succeed");
    let attachment = sample
        .attachment()
        .and_then(|att| Attachment::try_from(att).ok())
        .expect("Missing attachment");

    assert_eq!(attachment.source_time(), clock.now());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_vec_u8_pubsub() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");

    let publisher_handle = tokio::spawn({
        let context = context.clone();
        async move {
            let node = context
                .create_node("test_publisher")
                .build()
                .await
                .expect("Failed to create node");

            let publisher = node
                .publisher::<Vec<u8>>("zbuf_topic")
                .build()
                .await
                .expect("Failed to create publisher");

            let mut buffer = vec![0xAA; 16];
            buffer[0..8].copy_from_slice(&42u64.to_le_bytes());

            let message = buffer;

            // Give the subscriber time to declare before publishing
            tokio::time::sleep(Duration::from_millis(500)).await;
            publisher
                .publish(&message)
                .await
                .expect("Failed to publish");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    let subscriber_handle = tokio::spawn({
        let context = context.clone();
        async move {
            let node = context
                .create_node("test_subscriber")
                .build()
                .await
                .expect("Failed to create node");

            let subscriber = node
                .subscriber::<Vec<u8>>("zbuf_topic")
                .build()
                .await
                .expect("Failed to create subscriber");

            let received_msg = tokio::time::timeout(Duration::from_secs(2), subscriber.recv())
                .await
                .expect("receive should not time out")
                .expect("receive should succeed");

            assert_eq!(received_msg.len(), 16);
            let timestamp_bytes = &received_msg[0..8];
            let timestamp = u64::from_le_bytes(timestamp_bytes.try_into().unwrap());
            assert_eq!(timestamp, 42);
        }
    });

    publisher_handle.await.expect("Publisher task panicked");
    subscriber_handle.await.expect("Subscriber task panicked");
}
