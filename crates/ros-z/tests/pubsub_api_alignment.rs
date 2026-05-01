use std::sync::atomic::{AtomicUsize, Ordering};

use ros_z::{
    Message, Result, SchemaHash,
    context::ContextBuilder,
    dynamic::{DynamicCdrCodec, DynamicMessage, FieldType, MessageSchema},
    qos::QosProfile,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

static TEST_TOPIC_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn next_test_topic() -> String {
    format!(
        "/pubsub_api_alignment_{}",
        TEST_TOPIC_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestMessage {
    data: Vec<u8>,
    counter: u64,
}

impl Message for TestMessage {
    type Codec = ros_z::SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        "test_msgs::PubSubApiAlignmentTestMessage"
    }

    fn schema_hash() -> SchemaHash {
        SchemaHash::zero()
    }

    fn schema() -> std::sync::Arc<MessageSchema> {
        MessageSchema::builder("test_msgs::PubSubApiAlignmentTestMessage")
            .field("data", FieldType::Sequence(Box::new(FieldType::Uint8)))
            .field("counter", FieldType::Uint64)
            .build()
            .expect("schema should build")
    }
}

impl ros_z::msg::WireMessage for TestMessage {
    type Codec = ros_z::msg::SerdeCdrCodec<TestMessage>;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn approved_pubsub_entry_points_return_public_types() -> Result<()> {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await?;
    let node = context
        .create_node("pubsub_api_alignment_node")
        .build()
        .await?;
    let topic = next_test_topic();

    let publisher_builder: ros_z::pubsub::PublisherBuilder<TestMessage, _> =
        node.publisher::<TestMessage>(&topic);
    let _raw_builder = node.subscriber::<TestMessage>(&topic).raw();
    let subscriber_builder: ros_z::pubsub::SubscriberBuilder<TestMessage, _> =
        node.subscriber::<TestMessage>(&topic);

    let publisher: ros_z::pubsub::Publisher<TestMessage, _> = publisher_builder.build().await?;
    let _subscriber: ros_z::pubsub::Subscriber<TestMessage, _> = subscriber_builder.build().await?;

    let _publication: ros_z::pubsub::PreparedPublication<'_, TestMessage, _> = publisher.prepare();

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn pubsub_builders_use_concise_setter_names() -> Result<()> {
    let context = ros_z::context::ContextBuilder::default().build().await?;
    let node = context
        .create_node("pubsub_builder_api_alignment_node")
        .build()
        .await?;
    let schema = TestMessage::schema();
    let dynamic_schema = MessageSchema::builder("test_msgs::DynamicPubSubApiAlignment")
        .field("data", FieldType::String)
        .build()
        .expect("schema should build");
    let qos = QosProfile::default();

    let _publisher_builder = node
        .publisher::<TestMessage>("/pubsub_builder_api_alignment_publisher")
        .qos(qos)
        .attachment(true)
        .shm_config(ros_z::shm::ShmConfig::new(std::sync::Arc::new(
            ros_z::shm::ShmProviderBuilder::new(1024 * 1024).build()?,
        )))
        .codec::<ros_z::SerdeCdrCodec<TestMessage>>()
        .dyn_schema(schema.clone())
        .without_shm();
    let _subscriber_builder = node
        .subscriber::<TestMessage>("/pubsub_builder_api_alignment_subscriber")
        .qos(qos)
        .locality(zenoh::sample::Locality::Any)
        .transient_local_replay_timeout(std::time::Duration::from_millis(10))
        .codec::<ros_z::SerdeCdrCodec<TestMessage>>()
        .dyn_schema(schema.clone());
    let _raw_builder = node
        .subscriber::<TestMessage>("/pubsub_builder_api_alignment_raw")
        .raw()
        .qos(qos)
        .locality(zenoh::sample::Locality::Any)
        .transient_local_replay_timeout(std::time::Duration::from_millis(10));
    let _dynamic_publisher_builder: ros_z::pubsub::PublisherBuilder<
        DynamicMessage,
        DynamicCdrCodec,
    > = node.dynamic_publisher(
        "/pubsub_builder_api_alignment_dynamic_publisher",
        dynamic_schema,
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn async_pubsub_entry_points_use_new_names() -> Result<()> {
    let context = ros_z::context::ContextBuilder::default().build().await?;
    let node = context
        .create_node("async_pubsub_api_alignment_node")
        .build()
        .await?;
    let publisher = node
        .publisher::<TestMessage>("/async_pubsub_api_alignment")
        .build()
        .await?;
    let subscriber = node
        .subscriber::<TestMessage>("/async_pubsub_api_alignment")
        .build()
        .await?;

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    publisher
        .publish(&TestMessage {
            data: vec![1],
            counter: 1,
        })
        .await?;
    let received =
        tokio::time::timeout(std::time::Duration::from_secs(1), subscriber.recv()).await??;

    assert_eq!(received.counter, 1);
    Ok(())
}
