use std::{num::NonZeroUsize, time::Duration};

use ros_z::{
    Message,
    attachment::Attachment,
    context::ContextBuilder,
    entity::{EndpointEntity, EndpointKind, Entity, EntityKind, TypeInfo},
    message::{SerdeCdrCodec, WireEncoder},
    qos::{QosDurability, QosHistory, QosProfile, QosReliability},
    schema::SchemaBuilder,
    time::{Clock, Time},
};
use ros_z_schema::{SchemaBundle, StructDef, TypeDef, TypeDefinition, TypeDefinitions, TypeName};
use serde::{Deserialize, Serialize};
use serde_json::json;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Message)]
struct TestMessage {
    data: Vec<u8>,
    counter: u64,
}

fn topic_key_expr_for<T: Message>(node: &ros_z::node::Node, topic: &str) -> String {
    let qualified_topic = ros_z::topic_name::qualify_topic_name(
        topic,
        &node.node_entity().namespace,
        &node.node_entity().name,
    )
    .expect("topic should qualify");

    let entity = EndpointEntity {
        id: 0,
        node: node.node_entity().clone(),
        kind: EndpointKind::Publisher,
        topic: qualified_topic,
        type_info: T::type_info(),
        qos: Default::default(),
    };

    ros_z_protocol::format::topic_key_expr(&entity)
        .expect("topic key expression should build")
        .to_string()
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

async fn test_context() -> ros_z::Result<ros_z::context::Context> {
    ContextBuilder::default().build().await
}

#[tokio::test(flavor = "multi_thread")]
async fn node_builder_enables_schema_service_by_default() -> ros_z::Result<()> {
    let context = test_context().await?;
    let node = context
        .create_node("default_schema_service")
        .build()
        .await?;

    assert!(node.schema_service().is_some());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn node_builder_can_disable_schema_service_explicitly() -> ros_z::Result<()> {
    let context = test_context().await?;
    let node = context
        .create_node("no_schema_service")
        .without_schema_service()
        .build()
        .await?;

    assert!(node.schema_service().is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn node_builder_accepts_zenoh_native_identity() -> ros_z::Result<()> {
    let context = test_context().await?;
    let node = context
        .create_node("123node")
        .with_namespace("/42/robot-01")
        .build()
        .await?;

    assert_eq!(node.name(), "123node");
    assert_eq!(node.namespace(), "/42/robot-01");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn node_builder_rejects_invalid_node_names_before_protocol_formatting() -> ros_z::Result<()> {
    let context = test_context().await?;

    for invalid_name in [
        "",
        "node/name",
        "node%name",
        "node#name",
        "node$name",
        "node?name",
        "node*name",
    ] {
        let error = context
            .create_node(invalid_name)
            .build()
            .await
            .expect_err("invalid node name should fail during node build");

        assert!(
            matches!(
                error,
                ros_z::Error::Name {
                    kind: ros_z::error::NameKind::Node,
                    source: ros_z::topic_name::TopicNameError::InvalidNodeName(_),
                    ..
                }
            ),
            "unexpected error for node name {invalid_name:?}: {error:?}"
        );
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn node_builder_rejects_invalid_namespaces_before_protocol_formatting() -> ros_z::Result<()> {
    let context = test_context().await?;

    for invalid_namespace in [
        "/robot/",
        "/robot//ns",
        "/robot%ns",
        "/robot#ns",
        "/robot$ns",
        "/robot?ns",
        "/robot*ns",
    ] {
        let error = context
            .create_node("node")
            .with_namespace(invalid_namespace)
            .build()
            .await
            .expect_err("invalid namespace should fail during node build");

        assert!(
            matches!(
                error,
                ros_z::Error::Name {
                    kind: ros_z::error::NameKind::Namespace,
                    source: ros_z::topic_name::TopicNameError::InvalidNamespace(_),
                    ..
                }
            ),
            "unexpected error for namespace {invalid_namespace:?}: {error:?}"
        );
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn raw_subscriber_receives_sample_payload() -> zenoh::Result<()> {
    let context = ContextBuilder::default().build().await?;
    let node = context.create_node("raw_subscriber_node").build().await?;
    let publisher = node.publisher::<TestMessage>("/raw_topic")?.build().await?;
    let mut subscriber = node
        .subscriber::<TestMessage>("/raw_topic")?
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
async fn dynamic_publisher_factory_rejects_schema_root_that_differs_from_type_info() {
    let context = ContextBuilder::default()
        .build()
        .await
        .expect("Failed to create context");
    let node = context
        .create_node("mismatched_dynamic_schema_root")
        .build()
        .await
        .expect("Failed to create node");
    let schema = mismatched_dynamic_schema();
    let type_info = TypeInfo::new(
        "test_msgs::AdvertisedDynamicRoot",
        ros_z_schema::compute_hash(schema.as_ref()).unwrap(),
    );

    let Err(error) = node.dynamic_publisher("/mismatched_dynamic_schema_root", type_info, schema)
    else {
        panic!("mismatched schema root should fail dynamic publisher factory");
    };
    match error {
        ros_z::Error::Wire(source) => match source.as_ref() {
            ros_z::error::WireError::DynamicSchema {
                endpoint_kind,
                topic,
                source,
            } => {
                assert_eq!(*endpoint_kind, "publisher");
                assert_eq!(topic, "/mismatched_dynamic_schema_root");
                assert!(source.to_string().contains("schema root"));
                assert!(
                    source
                        .to_string()
                        .contains("test_msgs::AdvertisedDynamicRoot")
                );
                assert!(source.to_string().contains("test_msgs::ActualDynamicRoot"));
            }
            other => panic!("expected dynamic schema wire error, got {other:?}"),
        },
        other => panic!("expected wire error, got {other:?}"),
    }
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
        .expect("publisher factory should succeed")
        .build()
        .await
        .unwrap();

    let subscriber = node
        .subscriber::<TestMessage>("/test_topic")
        .expect("subscriber factory should succeed")
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
async fn transient_local_build_waits_for_initial_replay() -> ros_z::Result<()> {
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
        .publisher::<TestMessage>(topic)?
        .qos(qos)
        .build()
        .await?;
    let message = TestMessage {
        data: vec![1, 3, 5],
        counter: 13,
    };
    publisher.publish(&message).await?;

    let subscriber = sub_node
        .subscriber::<TestMessage>(topic)?
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
        .expect("publisher factory should succeed")
        .build()
        .await
        .unwrap();

    let subscriber = node
        .subscriber::<TestMessage>("/multi_topic")
        .expect("subscriber factory should succeed")
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
    let schema_hash = ros_z_schema::compute_hash(root_schema.as_ref()).unwrap();

    let _publisher = node
        .dynamic_publisher(topic, TypeInfo::new(&root_name, schema_hash), root_schema)
        .expect("dynamic publisher factory should succeed")
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
    let advertised = endpoint.type_info;
    let registered = node
        .schema_service()
        .expect("schema service")
        .get_schema(&root_name, &schema_hash)
        .expect("schema lookup should succeed")
        .expect("schema should be registered under root hash");

    assert_eq!(advertised.name, root_name);
    assert_eq!(advertised.hash, schema_hash);
    assert_eq!(registered.schema_hash, schema_hash);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn recv_with_metadata_includes_transport_and_source_timestamps() {
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
        .expect("publisher factory should succeed")
        .build()
        .await
        .unwrap();

    let subscriber = node
        .subscriber::<TestMessage>("/metadata_topic")
        .expect("subscriber factory should succeed")
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
    assert_eq!(received.sequence_number, 0);
    assert_ne!(received.source_global_id, [0; 16]);
    assert_eq!(
        received.publication_id().sequence_number(),
        received.sequence_number
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn typed_subscriber_errors_when_sample_has_no_attachment() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");
    let node = context
        .create_node("missing_attachment_subscriber")
        .build()
        .await
        .expect("Failed to create node");

    let topic = "/missing_attachment_pubsub";
    let subscriber = node
        .subscriber::<TestMessage>(topic)
        .expect("subscriber factory should succeed")
        .build()
        .await
        .expect("Failed to create subscriber");
    let key_expr = topic_key_expr_for::<TestMessage>(&node, topic);
    let message = TestMessage {
        data: vec![1, 2, 3],
        counter: 99,
    };

    tokio::time::sleep(Duration::from_millis(100)).await;
    node.session()
        .put(
            key_expr,
            SerdeCdrCodec::<TestMessage>::serialize(&message).unwrap(),
        )
        .await
        .expect("raw put should succeed");

    let error = tokio::time::timeout(Duration::from_secs(1), subscriber.recv_with_metadata())
        .await
        .expect("receive should not time out")
        .expect_err("typed receive should reject samples without attachments");

    match error {
        ros_z::Error::Wire(source) => match source.as_ref() {
            ros_z::error::WireError::MissingSampleAttachment => {}
            other => panic!("expected missing sample attachment, got {other:?}"),
        },
        other => panic!("expected wire error, got {other:?}"),
    }
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
        .expect("publisher factory should succeed")
        .build()
        .await
        .unwrap();

    let subscriber = node
        .subscriber::<TestMessage>("/large_topic")
        .expect("subscriber factory should succeed")
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
        .expect("publisher factory should succeed")
        .build()
        .await
        .unwrap();
    let mut subscriber = node
        .subscriber::<TestMessage>("/sim_clock")
        .expect("subscriber factory should succeed")
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
                .expect("publisher factory should succeed")
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
                .expect("subscriber factory should succeed")
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
