use std::{num::NonZeroUsize, time::Duration};

use ros_z::{
    Message, SchemaHash, ZBuf,
    attachment::Attachment,
    context::ContextBuilder,
    dynamic::{FieldType, MessageSchema},
    entity::{Entity, EntityKind, TypeInfo},
    qos::{QosDurability, QosHistory, QosProfile, QosReliability},
    time::{Clock, Time},
};
use ros_z_msgs::std_msgs::ByteMultiArray;
use serde::{Deserialize, Serialize};
use serde_json::json;
use zenoh_buffers::buffer::{Buffer, SplitBuffer};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestMessage {
    data: Vec<u8>,
    counter: u64,
}

impl Message for TestMessage {
    type Codec = ros_z::SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        "test_msgs::TestMessage"
    }

    fn schema_hash() -> SchemaHash {
        SchemaHash::zero()
    }

    fn schema() -> std::sync::Arc<MessageSchema> {
        MessageSchema::builder("test_msgs::TestMessage")
            .field("data", FieldType::Sequence(Box::new(FieldType::Uint8)))
            .field("counter", FieldType::Uint64)
            .build()
            .expect("schema should build")
    }
}

impl ros_z::msg::WireMessage for TestMessage {
    type Codec = ros_z::msg::SerdeCdrCodec<TestMessage>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct CacheSchemaHashMessage {
    data: String,
}

impl Message for CacheSchemaHashMessage {
    type Codec = ros_z::SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        "test_msgs::CacheSchemaHashMessage"
    }

    fn schema_hash() -> SchemaHash {
        SchemaHash([0x55; 32])
    }

    fn type_info() -> TypeInfo {
        TypeInfo::with_hash(Self::type_name(), Self::schema_hash())
    }

    fn schema() -> std::sync::Arc<MessageSchema> {
        MessageSchema::builder("test_msgs::CacheSchemaHashMessage")
            .field("data", FieldType::String)
            .build()
            .expect("schema should build")
    }
}

impl ros_z::msg::WireMessage for CacheSchemaHashMessage {
    type Codec = ros_z::msg::SerdeCdrCodec<CacheSchemaHashMessage>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct AdvertisedTypeInfoSchemaMessage {
    data: String,
}

impl Message for AdvertisedTypeInfoSchemaMessage {
    type Codec = ros_z::SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        "test_msgs::AdvertisedTypeInfoSchemaMessage"
    }

    fn schema_hash() -> SchemaHash {
        SchemaHash([0x77; 32])
    }

    fn type_info() -> TypeInfo {
        TypeInfo::with_hash(
            "test_msgs::AdvertisedTypeInfoSchemaMessageAlias",
            Self::schema_hash(),
        )
    }

    fn schema() -> std::sync::Arc<MessageSchema> {
        MessageSchema::builder("test_msgs::AdvertisedTypeInfoSchemaMessage")
            .field("data", FieldType::String)
            .build()
            .expect("schema should build")
    }
}

impl ros_z::msg::WireMessage for AdvertisedTypeInfoSchemaMessage {
    type Codec = ros_z::msg::SerdeCdrCodec<AdvertisedTypeInfoSchemaMessage>;
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
async fn test_create_pub_with_type_info_accepts_explicit_type_info_without_message_derive() {
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct WireOnlyMessage {
        data: Vec<u8>,
        counter: u64,
    }

    impl ros_z::msg::WireMessage for WireOnlyMessage {
        type Codec = ros_z::msg::SerdeCdrCodec<WireOnlyMessage>;
    }

    let context = ContextBuilder::default()
        .build()
        .await
        .expect("Failed to create context");
    let node = context
        .create_node("explicit_type_info_node")
        .build()
        .await
        .expect("Failed to create node");

    let _publisher = node
        .publisher_with_type_info::<WireOnlyMessage>(
            "/explicit_type_info_topic",
            Some(TypeInfo::new(
                "test_msgs::TestMessage",
                Some(SchemaHash::zero()),
            )),
        )
        .build()
        .await
        .unwrap();
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

    let expected_hash = CacheSchemaHashMessage::schema_hash();
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

    let advertised = AdvertisedTypeInfoSchemaMessage::type_info();
    let advertised_hash = advertised.hash.expect("type info should include a hash");
    let canonical_hash = ros_z::dynamic::schema_hash(&AdvertisedTypeInfoSchemaMessage::schema())
        .expect("schema hash should exist");

    let advertised_lookup = node
        .schema_service()
        .expect("schema service")
        .get_schema(&advertised.name, &advertised_hash)
        .expect("schema lookup should succeed");
    let registered = node
        .schema_service()
        .expect("schema service")
        .get_schema(
            AdvertisedTypeInfoSchemaMessage::schema().type_name_str(),
            &canonical_hash,
        )
        .expect("schema lookup should succeed")
        .expect("schema should be registered under the canonical schema key");

    assert!(advertised_lookup.is_none());
    assert_ne!(advertised_hash, canonical_hash);
    assert_eq!(registered.schema_hash, canonical_hash);
    assert_eq!(
        registered.schema.type_name_str(),
        AdvertisedTypeInfoSchemaMessage::schema().type_name_str()
    );
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
    let explicit_hash = SchemaHash([0x99; 32]);
    let schema = MessageSchema::builder("test_msgs::DynamicExplicitHash")
        .field("data", FieldType::String)
        .schema_hash(explicit_hash)
        .build()
        .expect("schema should build");

    let _publisher = node
        .dynamic_publisher(topic, schema.clone())
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
        .get_schema(schema.type_name_str(), &explicit_hash)
        .expect("schema lookup should succeed")
        .expect("schema should be registered under explicit hash");

    assert_eq!(advertised.name, schema.type_name_str());
    assert_eq!(advertised.hash, Some(explicit_hash));
    assert_eq!(registered.schema_hash, explicit_hash);
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
async fn test_bytemultiarray_pubsub_with_zbuf() {
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
                .publisher::<ByteMultiArray>("zbuf_topic")
                .build()
                .await
                .expect("Failed to create publisher");

            let mut buffer = vec![0xAA; 16];
            buffer[0..8].copy_from_slice(&42u64.to_le_bytes());

            let message = ByteMultiArray {
                data: ZBuf::from(buffer),
                ..Default::default()
            };

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
                .subscriber::<ByteMultiArray>("zbuf_topic")
                .build()
                .await
                .expect("Failed to create subscriber");

            let received_msg = tokio::time::timeout(Duration::from_secs(2), subscriber.recv())
                .await
                .expect("receive should not time out")
                .expect("receive should succeed");

            assert_eq!(received_msg.data.len(), 16);
            let timestamp_bytes = &received_msg.data.contiguous()[0..8];
            let timestamp = u64::from_le_bytes(timestamp_bytes.try_into().unwrap());
            assert_eq!(timestamp, 42);
        }
    });

    publisher_handle.await.expect("Publisher task panicked");
    subscriber_handle.await.expect("Subscriber task panicked");
}
