//! Graph API tests
//!
//! These tests verify the graph introspection functionality, including:
//! - Getting topic/service names and types
//! - Counting publishers/subscribers/clients/services
//! - Waiting for graph changes
//! - Node discovery and information
//! - Service availability checking

use std::{num::NonZeroUsize, time::Duration};

use ros_z::__private::ros_z_schema::TypeName;
use ros_z::{
    Message, Result, ServiceTypeInfo,
    context::ContextBuilder,
    dynamic::{RuntimeFieldSchema, Schema, TypeShape},
    entity::{EndpointEntity, EndpointKind, Entity, EntityKind, NodeEntity, NodeKey},
    msg::Service,
    qos::{QosCompatibility, QosDurability, QosHistory, QosProfile, QosReliability},
};
use serde::{Deserialize, Serialize};

fn struct_schema(name: &str, fields: Vec<RuntimeFieldSchema>) -> Schema {
    std::sync::Arc::new(TypeShape::Struct {
        name: TypeName::new(name.to_string()).expect("valid test type name"),
        fields,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct AddTwoIntsRequest {
    a: i64,
    b: i64,
}

impl Message for AddTwoIntsRequest {
    type Codec = ros_z::SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        "test_msgs::AddTwoIntsRequest"
    }

    fn schema_hash() -> ros_z::entity::SchemaHash {
        ros_z::entity::SchemaHash::zero()
    }

    fn schema() -> Schema {
        struct_schema(
            "test_msgs::AddTwoIntsRequest",
            vec![
                RuntimeFieldSchema::new("a", i64::schema()),
                RuntimeFieldSchema::new("b", i64::schema()),
            ],
        )
    }
}

impl ros_z::msg::WireMessage for AddTwoIntsRequest {
    type Codec = ros_z::msg::SerdeCdrCodec<AddTwoIntsRequest>;
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct AddTwoIntsResponse {
    sum: i64,
}

impl Message for AddTwoIntsResponse {
    type Codec = ros_z::SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        "test_msgs::AddTwoIntsResponse"
    }

    fn schema_hash() -> ros_z::entity::SchemaHash {
        ros_z::entity::SchemaHash::zero()
    }

    fn schema() -> Schema {
        struct_schema(
            "test_msgs::AddTwoIntsResponse",
            vec![RuntimeFieldSchema::new("sum", i64::schema())],
        )
    }
}

impl ros_z::msg::WireMessage for AddTwoIntsResponse {
    type Codec = ros_z::msg::SerdeCdrCodec<AddTwoIntsResponse>;
}

struct AddTwoInts;

impl ServiceTypeInfo for AddTwoInts {
    fn service_type_info() -> ros_z::entity::TypeInfo {
        ros_z::entity::TypeInfo::new("test_msgs::AddTwoInts", None)
    }
}

impl Service for AddTwoInts {
    type Request = AddTwoIntsRequest;
    type Response = AddTwoIntsResponse;
}

/// Helper to create a test context and node
async fn setup_test_node(node_name: &str) -> Result<(ros_z::context::Context, ros_z::node::Node)> {
    let context = ContextBuilder::default().build().await?;
    let node = context.create_node(node_name).build().await?;

    // Allow time for node discovery
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok((context, node))
}

fn unique_graph_name(prefix: &str) -> String {
    format!(
        "/{prefix}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

fn unique_node_name(prefix: &str) -> String {
    format!(
        "{prefix}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

/// Helper to wait for publishers on a topic
async fn wait_for_publishers(
    node: &ros_z::node::Node,
    topic: &str,
    expected_count: usize,
    timeout_ms: u64,
) -> Result<bool> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    loop {
        let count = node.graph().count(EntityKind::Publisher, topic);
        if count >= expected_count {
            return Ok(true);
        }
        if start.elapsed() >= timeout {
            return Ok(false);
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Helper to wait for subscribers on a topic
async fn wait_for_subscribers(
    node: &ros_z::node::Node,
    topic: &str,
    expected_count: usize,
    timeout_ms: u64,
) -> Result<bool> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    loop {
        let count = node.graph().count(EntityKind::Subscription, topic);
        if count >= expected_count {
            return Ok(true);
        }
        if start.elapsed() >= timeout {
            return Ok(false);
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Helper to wait for services on a service name
async fn wait_for_services(
    node: &ros_z::node::Node,
    service: &str,
    expected_count: usize,
    timeout_ms: u64,
) -> Result<bool> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    loop {
        let count = node.graph().count(EntityKind::Service, service);
        if count >= expected_count {
            return Ok(true);
        }
        if start.elapsed() >= timeout {
            return Ok(false);
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn node_exists_returns_false_after_only_node_removed() -> Result<()> {
        let session = zenoh::open(zenoh::Config::default()).await?;
        let graph = ros_z::graph::Graph::new(&session).await?;
        let node = NodeEntity::new(
            session.zid(),
            1,
            "removed_node".to_string(),
            String::new(),
            String::new(),
        );
        let node_key = ros_z::entity::node_key(&node);
        let entity = Entity::Node(node);

        graph.add_local_entity(entity.clone())?;
        assert!(graph.node_exists(node_key.clone()));

        graph.remove_local_entity(&entity)?;

        assert!(!graph.node_exists(node_key));
        session.close().await?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn context_can_skip_initial_graph_liveliness_query() -> Result<()> {
        let _ctx = ContextBuilder::default()
            .without_graph_initial_query()
            .build()
            .await?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn endpoint_queries_return_empty_for_entity_kind_node() -> Result<()> {
        let context = ContextBuilder::default().build().await?;
        let node = context
            .create_node("graph_node_kind_no_panic")
            .build()
            .await?;
        let graph = node.graph().clone();

        assert_eq!(graph.count(EntityKind::Node, "/anything"), 0);
        assert_eq!(graph.count_by_service(EntityKind::Node, "/anything"), 0);
        assert!(
            graph
                .get_entities_by_topic(EntityKind::Node, "/anything")
                .is_empty()
        );
        assert!(
            graph
                .get_entities_by_service(EntityKind::Node, "/anything")
                .is_empty()
        );
        assert!(
            graph
                .get_entities_by_node(
                    EntityKind::Node,
                    ("".into(), "graph_node_kind_no_panic".into())
                )
                .is_empty()
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "EntityKind::Service | EntityKind::Client")]
    async fn service_count_still_panics_for_publisher_kind() {
        let context = ContextBuilder::default().build().await.unwrap();
        let node = context
            .create_node("graph_service_count_invalid_kind")
            .build()
            .await
            .unwrap();

        let _ = node
            .graph()
            .count_by_service(EntityKind::Publisher, "/anything");
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "EntityKind::Service | EntityKind::Client")]
    async fn service_entities_still_panic_for_subscription_kind() {
        let context = ContextBuilder::default().build().await.unwrap();
        let node = context
            .create_node("graph_service_entities_invalid_kind")
            .build()
            .await
            .unwrap();

        let _ = node
            .graph()
            .get_entities_by_service(EntityKind::Subscription, "/anything");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn readding_local_entity_replaces_existing_index_entry() -> Result<()> {
        let context = ContextBuilder::default().build().await?;
        let node = context
            .create_node("graph_readd_local_entity")
            .build()
            .await?;
        let graph = node.graph().clone();
        let topic = unique_graph_name("graph_readd_local_entity_topic");
        let entity = Entity::Endpoint(EndpointEntity {
            id: 4242,
            node: Some(node.node_entity().clone()),
            kind: EndpointKind::Publisher,
            topic: topic.clone(),
            type_info: None,
            qos: Default::default(),
        });

        graph.add_local_entity(entity.clone())?;
        let held_entities = graph.get_entities_by_topic(EntityKind::Publisher, &topic);
        assert_eq!(held_entities.len(), 1);

        graph.add_local_entity(entity)?;
        let current_entities = graph.get_entities_by_topic(EntityKind::Publisher, &topic);
        assert_eq!(current_entities.len(), 1);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn graph_reports_incompatible_reliability_for_topic_endpoints() -> Result<()> {
        let context = ContextBuilder::default().build().await?;
        let pub_node = context
            .create_node(unique_node_name("qos_diag_pub"))
            .build()
            .await?;
        let sub_node = context
            .create_node(unique_node_name("qos_diag_sub"))
            .build()
            .await?;
        let topic = unique_graph_name("qos_diag_reliability");
        let pub_qos = QosProfile {
            reliability: QosReliability::BestEffort,
            history: QosHistory::KeepLast(NonZeroUsize::new(10).unwrap()),
            ..Default::default()
        };
        let sub_qos = QosProfile {
            reliability: QosReliability::Reliable,
            history: QosHistory::KeepLast(NonZeroUsize::new(10).unwrap()),
            ..Default::default()
        };

        let _publisher = pub_node
            .publisher::<String>(&topic)
            .qos(pub_qos)
            .build()
            .await?;
        let _subscriber = sub_node
            .subscriber::<String>(&topic)
            .qos(sub_qos)
            .build()
            .await?;

        assert!(wait_for_publishers(&pub_node, &topic, 1, 2_000).await?);
        assert!(wait_for_subscribers(&pub_node, &topic, 1, 2_000).await?);

        let diagnostics = pub_node.graph().qos_incompatibilities_for_topic(&topic);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].compatibility,
            QosCompatibility::IncompatibleReliability,
        );
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn graph_reports_incompatible_durability_for_topic_endpoints() -> Result<()> {
        let context = ContextBuilder::default().build().await?;
        let pub_node = context
            .create_node(unique_node_name("qos_diag_dur_pub"))
            .build()
            .await?;
        let sub_node = context
            .create_node(unique_node_name("qos_diag_dur_sub"))
            .build()
            .await?;
        let topic = unique_graph_name("qos_diag_durability");
        let pub_qos = QosProfile {
            durability: QosDurability::Volatile,
            history: QosHistory::KeepLast(NonZeroUsize::new(10).unwrap()),
            ..Default::default()
        };
        let sub_qos = QosProfile {
            durability: QosDurability::TransientLocal,
            history: QosHistory::KeepLast(NonZeroUsize::new(10).unwrap()),
            ..Default::default()
        };

        let _publisher = pub_node
            .publisher::<String>(&topic)
            .qos(pub_qos)
            .build()
            .await?;
        let _subscriber = sub_node
            .subscriber::<String>(&topic)
            .qos(sub_qos)
            .build()
            .await?;

        assert!(wait_for_publishers(&pub_node, &topic, 1, 2_000).await?);
        assert!(wait_for_subscribers(&pub_node, &topic, 1, 2_000).await?);

        let diagnostics = pub_node.graph().qos_incompatibilities_for_topic(&topic);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].compatibility,
            QosCompatibility::IncompatibleDurability,
        );
        Ok(())
    }

    /// Tests getting topic names and types from the graph
    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_topic_names_and_types() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let topic_name = unique_graph_name("test_graph_topic_names_and_types");

        let _pub = node.publisher::<String>(&topic_name).build().await?;
        assert!(
            wait_for_publishers(&node, &topic_name, 1, 1_000).await?,
            "Expected graph to discover publisher for {topic_name}"
        );

        let graph = node.graph().clone();
        let topics = graph.get_topic_names_and_types();

        assert!(
            topics.iter().any(|(name, _)| name == &topic_name),
            "Expected to find {topic_name} in discovered topics, got: {topics:?}"
        );

        Ok(())
    }

    /// Tests getting service names and types from the graph
    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_service_names_and_types() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let service_name = unique_graph_name("test_graph_service_names_and_types");

        let _service = node
            .create_service_server::<AddTwoInts>(&service_name)
            .build()
            .await?;
        assert!(
            wait_for_services(&node, &service_name, 1, 1_000).await?,
            "Expected graph to discover service for {service_name}"
        );

        let graph = node.graph().clone();
        let services = graph.get_service_names_and_types();

        assert!(
            services.iter().any(|(name, _)| name == &service_name),
            "Expected to find {service_name} in discovered services, got: {services:?}"
        );

        Ok(())
    }

    /// Tests counting publishers on a topic
    #[tokio::test(flavor = "multi_thread")]
    async fn test_count_publishers() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let topic_name = "/test_count_publishers";

        // Count publishers on a topic that doesn't exist yet
        let graph = node.graph().clone();
        let count = graph.count(EntityKind::Publisher, topic_name);

        // Should be 0 or at least return successfully
        assert_eq!(count, 0, "Expected 0 publishers on non-existent topic");

        // Create a publisher
        let _pub = node.publisher::<String>(topic_name).build().await?;

        // Allow discovery
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Count again - should see our publisher
        let count = graph.count(EntityKind::Publisher, topic_name);
        assert!(
            count >= 1,
            "Expected at least 1 publisher after creating one"
        );

        Ok(())
    }

    /// Tests counting subscribers on a topic
    #[tokio::test(flavor = "multi_thread")]
    async fn test_count_subscribers() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let topic_name = "/test_count_subscribers";

        // Count subscribers on a topic that doesn't exist yet
        let graph = node.graph().clone();
        let count = graph.count(EntityKind::Subscription, topic_name);
        assert_eq!(count, 0, "Expected 0 subscribers on non-existent topic");

        // Create a subscriber
        let _sub = node.subscriber::<String>(topic_name).build().await?;

        // Allow discovery
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Count again - should see our subscriber
        let count = graph.count(EntityKind::Subscription, topic_name);
        assert!(
            count >= 1,
            "Expected at least 1 subscriber after creating one"
        );

        Ok(())
    }

    /// Tests counting clients on a service
    #[tokio::test(flavor = "multi_thread")]
    async fn test_count_clients() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let service_name = "/test_count_clients";

        // Count clients on a service that doesn't exist yet
        let graph = node.graph().clone();
        let count = graph.count(EntityKind::Client, service_name);

        // Should be 0 or at least return successfully
        assert_eq!(count, 0, "Expected 0 clients on non-existent service");

        // Create a client
        let _client = node
            .create_service_client::<AddTwoInts>(service_name)
            .build()
            .await?;

        // Allow discovery
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Count again - should see our client
        let count = graph.count(EntityKind::Client, service_name);
        assert!(count >= 1, "Expected at least 1 client after creating one");

        Ok(())
    }

    /// Tests counting services on a service name
    #[tokio::test(flavor = "multi_thread")]
    async fn test_count_services() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let service_name = "/test_count_services";

        // Count services on a service that doesn't exist yet
        let graph = node.graph().clone();
        let count = graph.count(EntityKind::Service, service_name);

        // Should be 0 or at least return successfully
        assert_eq!(count, 0, "Expected 0 services on non-existent service");

        // Create a service
        let _service = node
            .create_service_server::<AddTwoInts>(service_name)
            .build()
            .await?;

        // Allow discovery
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Count again - should see our service
        let count = graph.count(EntityKind::Service, service_name);
        println!("Service count after creation: {}", count);
        assert!(count >= 1, "Expected at least 1 service after creating one");

        Ok(())
    }

    /// Tests getting service names and types for a specific node
    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_service_names_and_types_by_node() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let service_name = "/test_service_by_node";

        // Create a service
        let _service = node
            .create_service_server::<AddTwoInts>(service_name)
            .build()
            .await?;

        // Allow discovery
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Get services by node
        let graph = node.graph().clone();
        // Note: "/" namespace is normalized to "" in NodeEntity::key()
        let node_key: NodeKey = ("".to_string(), "test_graph_node".to_string());

        let entities = graph.get_entities_by_node(EntityKind::Service, node_key);

        // Should find our service
        assert!(!entities.is_empty(), "Expected to find service by node");
        assert!(
            entities
                .iter()
                .any(|e| e.topic.contains("test_service_by_node")),
            "Expected to find our specific service"
        );

        Ok(())
    }

    /// Tests getting client names and types for a specific node
    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_client_names_and_types_by_node() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let service_name = "/test_client_by_node";

        // Create a client
        let _client = node
            .create_service_client::<AddTwoInts>(service_name)
            .build()
            .await?;

        // Allow discovery
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Get clients by node
        let graph = node.graph().clone();
        // Note: "/" namespace is normalized to "" in NodeEntity::key()
        let node_key: NodeKey = ("".to_string(), "test_graph_node".to_string());

        let entities = graph.get_entities_by_node(EntityKind::Client, node_key);

        // Should find our client
        assert!(!entities.is_empty(), "Expected to find client by node");
        assert!(
            entities
                .iter()
                .any(|e| e.topic.contains("test_client_by_node")),
            "Expected to find our specific client"
        );

        Ok(())
    }

    /// Tests graph queries with a hand-crafted graph
    #[tokio::test(flavor = "multi_thread")]
    async fn test_graph_query_functions() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let topic_name = format!(
            "/test_graph_query_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );

        let graph = node.graph().clone();

        // Initially, topic should not exist
        let count_pubs = graph.count(EntityKind::Publisher, &topic_name);
        let count_subs = graph.count(EntityKind::Subscription, &topic_name);
        assert_eq!(count_pubs, 0, "Expected 0 publishers initially");
        assert_eq!(count_subs, 0, "Expected 0 subscribers initially");

        // Create a publisher
        let pub_handle = node.publisher::<String>(&topic_name).build().await?;

        tokio::time::sleep(Duration::from_millis(300)).await;

        // Should see 1 publisher
        let count_pubs = graph.count(EntityKind::Publisher, &topic_name);
        assert!(
            count_pubs >= 1,
            "Expected at least 1 publisher after creation"
        );

        // Create a subscriber
        let sub_handle = node.subscriber::<String>(&topic_name).build().await?;

        tokio::time::sleep(Duration::from_millis(300)).await;

        // Should see 1 publisher and 1 subscriber
        let count_pubs = graph.count(EntityKind::Publisher, &topic_name);
        let count_subs = graph.count(EntityKind::Subscription, &topic_name);
        assert!(count_pubs >= 1, "Expected at least 1 publisher");
        assert!(count_subs >= 1, "Expected at least 1 subscriber");

        // Drop publisher
        drop(pub_handle);
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Should see 0 publishers, 1 subscriber
        let count_pubs = graph.count(EntityKind::Publisher, &topic_name);
        let count_subs = graph.count(EntityKind::Subscription, &topic_name);
        assert_eq!(count_pubs, 0, "Expected 0 publishers after drop");
        assert!(count_subs >= 1, "Expected at least 1 subscriber still");

        // Drop subscriber
        drop(sub_handle);
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Should see 0 publishers, 0 subscribers
        let count_pubs = graph.count(EntityKind::Publisher, &topic_name);
        let count_subs = graph.count(EntityKind::Subscription, &topic_name);
        assert_eq!(count_pubs, 0, "Expected 0 publishers after all drops");
        assert_eq!(count_subs, 0, "Expected 0 subscribers after all drops");

        Ok(())
    }

    /// Tests getting all node names from the graph
    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_node_names() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;

        // Get node names
        let graph = node.graph().clone();
        let nodes = graph.get_node_names();

        // Should at least see our own node
        assert!(!nodes.is_empty(), "Expected to find at least one node");
        assert!(
            nodes.iter().any(|(name, _)| name == "test_graph_node"),
            "Expected to find our test node"
        );

        Ok(())
    }

    /// Tests getting all node names with enclaves from the graph
    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_node_names_with_enclaves() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;

        // Get node names with enclaves
        let graph = node.graph().clone();
        let nodes = graph.get_node_names_with_enclaves();

        // Should at least see our own node
        assert!(!nodes.is_empty(), "Expected to find at least one node");
        assert!(
            nodes.iter().any(|(name, _, _)| name == "test_graph_node"),
            "Expected to find our test node with enclave"
        );

        Ok(())
    }

    /// Tests discovering publishers from multiple nodes
    #[tokio::test(flavor = "multi_thread")]
    async fn test_multi_node_publishers() -> Result<()> {
        let context = ContextBuilder::default().build().await?;
        let node1 = context
            .create_node(unique_node_name("test_pub_node_1"))
            .build()
            .await?;
        let node2 = context
            .create_node(unique_node_name("test_pub_node_2"))
            .build()
            .await?;

        let topic_name = unique_graph_name("test_multi_node_pub");

        // Create publishers on both nodes
        let _pub1 = node1.publisher::<String>(&topic_name).build().await?;
        let _pub2 = node2.publisher::<String>(&topic_name).build().await?;

        assert!(
            wait_for_publishers(&node1, &topic_name, 2, 1_000).await?,
            "Expected node1 graph to discover both publishers for {topic_name}"
        );
        assert!(
            wait_for_publishers(&node2, &topic_name, 2, 1_000).await?,
            "Expected node2 graph to discover both publishers for {topic_name}"
        );

        Ok(())
    }

    /// Tests discovering subscribers from multiple nodes
    #[tokio::test(flavor = "multi_thread")]
    async fn test_multi_node_subscribers() -> Result<()> {
        let context = ContextBuilder::default().build().await?;
        let node1 = context
            .create_node(unique_node_name("test_sub_node_1"))
            .build()
            .await?;
        let node2 = context
            .create_node(unique_node_name("test_sub_node_2"))
            .build()
            .await?;

        let topic_name = unique_graph_name("test_multi_node_sub");

        // Create subscribers on both nodes
        let _sub1 = node1.subscriber::<String>(&topic_name).build().await?;
        let _sub2 = node2.subscriber::<String>(&topic_name).build().await?;

        assert!(
            wait_for_subscribers(&node1, &topic_name, 2, 1_000).await?,
            "Expected node1 graph to discover both subscribers for {topic_name}"
        );
        assert!(
            wait_for_subscribers(&node2, &topic_name, 2, 1_000).await?,
            "Expected node2 graph to discover both subscribers for {topic_name}"
        );

        Ok(())
    }

    /// Tests discovering services from multiple nodes
    #[tokio::test(flavor = "multi_thread")]
    async fn test_multi_node_services() -> Result<()> {
        // Create a single context and multiple nodes to share the graph
        let context = ContextBuilder::default().build().await?;
        let node1 = context.create_node("test_node_1").build().await?;
        let node2 = context.create_node("test_node_2").build().await?;

        let service_name1 = "/test_multi_node_service_1";
        let service_name2 = "/test_multi_node_service_2";

        // Create services on different nodes
        let _srv1 = node1
            .create_service_server::<AddTwoInts>(service_name1)
            .build()
            .await?;
        let _srv2 = node2
            .create_service_server::<AddTwoInts>(service_name2)
            .build()
            .await?;

        tokio::time::sleep(Duration::from_millis(300)).await;

        // Check service discovery from node1's perspective
        let graph1 = node1.graph();
        let services = graph1.get_service_names_and_types();

        // Should see both services
        assert!(
            services.iter().any(|(name, _)| name.contains("service_1")),
            "Expected to find service_1"
        );
        assert!(
            services.iter().any(|(name, _)| name.contains("service_2")),
            "Expected to find service_2"
        );

        Ok(())
    }

    /// Tests discovering clients from multiple nodes
    #[tokio::test(flavor = "multi_thread")]
    async fn test_multi_node_clients() -> Result<()> {
        // Create a single context and multiple nodes to share the graph
        let context = ContextBuilder::default().build().await?;
        let node1 = context.create_node("test_node_1").build().await?;
        let node2 = context.create_node("test_node_2").build().await?;

        let service_name = "/test_multi_node_client";

        // Create service and clients
        let _srv = node1
            .create_service_server::<AddTwoInts>(service_name)
            .build()
            .await?;
        let _client1 = node1
            .create_service_client::<AddTwoInts>(service_name)
            .build()
            .await?;
        let _client2 = node2
            .create_service_client::<AddTwoInts>(service_name)
            .build()
            .await?;

        tokio::time::sleep(Duration::from_millis(300)).await;

        // Check from graph
        let graph1 = node1.graph();
        let count = graph1.count(EntityKind::Client, service_name);
        assert!(count >= 2, "Expected at least 2 clients");

        Ok(())
    }

    /// Tests checking if a service server is available
    #[tokio::test(flavor = "multi_thread")]
    async fn test_service_server_is_available() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let service_name = "/test_service_available";

        // Create client
        let client = node
            .create_service_client::<AddTwoInts>(service_name)
            .build()
            .await?;

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Service should not be available yet
        let graph = node.graph().clone();
        let count = graph.count(EntityKind::Service, service_name);
        assert_eq!(count, 0, "Expected 0 services before creating server");

        // Create the service
        let _service = node
            .create_service_server::<AddTwoInts>(service_name)
            .build()
            .await?;

        tokio::time::sleep(Duration::from_millis(300)).await;

        // Service should now be available
        let count = graph.count(EntityKind::Service, service_name);
        assert!(count >= 1, "Expected at least 1 service after creation");

        // Drop service
        drop(_service);
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Service should no longer be available
        let count = graph.count(EntityKind::Service, service_name);
        assert_eq!(count, 0, "Expected 0 services after dropping server");

        drop(client);
        Ok(())
    }

    /// Tests the get_entities_by_topic functionality
    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_entities_by_topic() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let topic_name = "/test_entities_by_topic";

        // Create publisher and subscriber
        let _pub = node.publisher::<String>(topic_name).build().await?;
        let _sub = node.subscriber::<String>(topic_name).build().await?;

        tokio::time::sleep(Duration::from_millis(300)).await;

        // Get entities by topic
        let graph = node.graph().clone();
        let pubs = graph.get_entities_by_topic(EntityKind::Publisher, topic_name);
        let subs = graph.get_entities_by_topic(EntityKind::Subscription, topic_name);

        // Should find both
        assert!(!pubs.is_empty(), "Expected to find publishers");
        assert!(!subs.is_empty(), "Expected to find subscribers");

        Ok(())
    }

    /// Tests waiting for publishers on a topic
    #[tokio::test(flavor = "multi_thread")]
    async fn test_wait_for_publishers() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let topic_name = "/test_wait_for_publishers";

        // Valid call (expect timeout since there are no publishers)
        let success = wait_for_publishers(&node, topic_name, 1, 100).await?;
        assert!(!success, "Expected timeout since no publishers");

        Ok(())
    }

    /// Tests waiting for subscribers on a topic
    #[tokio::test(flavor = "multi_thread")]
    async fn test_wait_for_subscribers() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let topic_name = "/test_wait_for_subscribers";

        // Valid call (expect timeout since there are no subscribers)
        let success = wait_for_subscribers(&node, topic_name, 1, 100).await?;
        assert!(!success, "Expected timeout since no subscribers");

        Ok(())
    }
}
