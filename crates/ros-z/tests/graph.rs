//! Graph API tests
//!
//! These tests verify the graph introspection functionality, including:
//! - Getting topic/service names and types
//! - Counting publishers/subscribers/clients/services
//! - Waiting for graph changes
//! - Node discovery and information
//! - Service availability checking

use std::{num::NonZeroUsize, time::Duration};

use ros_z::{
    Result, ServiceTypeInfo,
    context::ContextBuilder,
    entity::{
        EndpointEntity, EndpointKind, Entity, EntityKind, NodeEntity, NodeKey, SchemaHash, TypeInfo,
    },
    message::Service,
    qos::{QosCompatibility, QosDurability, QosHistory, QosProfile, QosReliability},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, ros_z::Message)]
#[message(name = "test_msgs::AddTwoIntsRequest")]
struct AddTwoIntsRequest {
    a: i64,
    b: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, ros_z::Message)]
#[message(name = "test_msgs::AddTwoIntsResponse")]
struct AddTwoIntsResponse {
    sum: i64,
}

struct AddTwoInts;

impl ServiceTypeInfo for AddTwoInts {
    fn service_type_info() -> std::result::Result<TypeInfo, ros_z_schema::SchemaError> {
        let descriptor = ros_z_schema::ServiceDef::new(
            "test_msgs::AddTwoInts",
            "test_msgs::AddTwoIntsRequest",
            "test_msgs::AddTwoIntsResponse",
        )?;
        Ok(TypeInfo::new(
            "test_msgs::AddTwoInts",
            Some(SchemaHash(ros_z_schema::compute_hash(&descriptor).0)),
        ))
    }
}

impl Service for AddTwoInts {
    type Request = AddTwoIntsRequest;
    type Response = AddTwoIntsResponse;
}

async fn setup_test_node(node_name: &str) -> Result<(ros_z::context::Context, ros_z::node::Node)> {
    let context = ContextBuilder::default().build().await?;
    let node = context.create_node(node_name).build().await?;

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

async fn wait_for_count(
    node: &ros_z::node::Node,
    kind: EntityKind,
    name: &str,
    expected_count: usize,
    timeout_ms: u64,
) -> Result<bool> {
    wait_for_count_matching(node, kind, name, timeout_ms, |count| {
        count >= expected_count
    })
    .await
}

async fn wait_for_count_at_most(
    node: &ros_z::node::Node,
    kind: EntityKind,
    name: &str,
    max_count: usize,
    timeout_ms: u64,
) -> Result<bool> {
    wait_for_count_matching(node, kind, name, timeout_ms, |count| count <= max_count).await
}

async fn wait_for_count_matching(
    node: &ros_z::node::Node,
    kind: EntityKind,
    name: &str,
    timeout_ms: u64,
    matches: impl Fn(usize) -> bool,
) -> Result<bool> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    loop {
        let count = node.graph().count(kind, name);
        if matches(count) {
            return Ok(true);
        }
        if start.elapsed() >= timeout {
            return Ok(false);
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn wait_for_publishers(
    node: &ros_z::node::Node,
    topic: &str,
    expected_count: usize,
    timeout_ms: u64,
) -> Result<bool> {
    wait_for_count(
        node,
        EntityKind::Publisher,
        topic,
        expected_count,
        timeout_ms,
    )
    .await
}

async fn wait_for_subscribers(
    node: &ros_z::node::Node,
    topic: &str,
    expected_count: usize,
    timeout_ms: u64,
) -> Result<bool> {
    wait_for_count(
        node,
        EntityKind::Subscription,
        topic,
        expected_count,
        timeout_ms,
    )
    .await
}

async fn wait_for_services(
    node: &ros_z::node::Node,
    service: &str,
    expected_count: usize,
    timeout_ms: u64,
) -> Result<bool> {
    wait_for_count(
        node,
        EntityKind::Service,
        service,
        expected_count,
        timeout_ms,
    )
    .await
}

async fn wait_for_clients(
    node: &ros_z::node::Node,
    service: &str,
    expected_count: usize,
    timeout_ms: u64,
) -> Result<bool> {
    wait_for_count(
        node,
        EntityKind::Client,
        service,
        expected_count,
        timeout_ms,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn node_exists_returns_false_after_only_node_removed() -> Result<()> {
        let session = zenoh::open(zenoh::Config::default()).await?;
        let graph = ros_z::graph::Graph::new(&session).await?;
        let node = NodeEntity::new(session.zid(), 1, "removed_node".to_string(), String::new());
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
    async fn service_count_panics_for_non_service_endpoint_kind() {
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
    async fn service_entities_panic_for_non_service_endpoint_kind() {
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

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_reports_topic_names_and_types() -> Result<()> {
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

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_reports_service_names_and_types() -> Result<()> {
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

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_counts_publishers_after_discovery() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let topic_name = unique_graph_name("graph_count_publishers");

        let graph = node.graph().clone();
        let count = graph.count(EntityKind::Publisher, &topic_name);

        assert_eq!(count, 0, "Expected 0 publishers on non-existent topic");

        let _pub = node.publisher::<String>(&topic_name).build().await?;

        assert!(wait_for_publishers(&node, &topic_name, 1, 1_000).await?);

        let count = graph.count(EntityKind::Publisher, &topic_name);
        assert!(
            count >= 1,
            "Expected at least 1 publisher after creating one"
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_counts_subscribers_after_discovery() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let topic_name = unique_graph_name("graph_count_subscribers");

        let graph = node.graph().clone();
        let count = graph.count(EntityKind::Subscription, &topic_name);
        assert_eq!(count, 0, "Expected 0 subscribers on non-existent topic");

        let _sub = node.subscriber::<String>(&topic_name).build().await?;

        assert!(wait_for_subscribers(&node, &topic_name, 1, 1_000).await?);

        let count = graph.count(EntityKind::Subscription, &topic_name);
        assert!(
            count >= 1,
            "Expected at least 1 subscriber after creating one"
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_counts_clients_after_discovery() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let service_name = unique_graph_name("graph_count_clients");

        let graph = node.graph().clone();
        let count = graph.count(EntityKind::Client, &service_name);

        assert_eq!(count, 0, "Expected 0 clients on non-existent service");

        let _client = node
            .create_service_client::<AddTwoInts>(&service_name)
            .build()
            .await?;

        assert!(wait_for_clients(&node, &service_name, 1, 1_000).await?);

        let count = graph.count(EntityKind::Client, &service_name);
        assert!(count >= 1, "Expected at least 1 client after creating one");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_counts_services_after_discovery() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let service_name = unique_graph_name("graph_count_services");

        let graph = node.graph().clone();
        let count = graph.count(EntityKind::Service, &service_name);

        assert_eq!(count, 0, "Expected 0 services on non-existent service");

        let _service = node
            .create_service_server::<AddTwoInts>(&service_name)
            .build()
            .await?;

        assert!(wait_for_services(&node, &service_name, 1, 1_000).await?);

        let count = graph.count(EntityKind::Service, &service_name);
        assert!(count >= 1, "Expected at least 1 service after creating one");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_reports_services_by_node() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let service_name = unique_graph_name("graph_service_by_node");

        let _service = node
            .create_service_server::<AddTwoInts>(&service_name)
            .build()
            .await?;

        assert!(wait_for_services(&node, &service_name, 1, 1_000).await?);

        let graph = node.graph().clone();
        let node_key: NodeKey = ("".to_string(), "test_graph_node".to_string());

        let entities = graph.get_entities_by_node(EntityKind::Service, node_key);

        assert!(!entities.is_empty(), "Expected to find service by node");
        assert!(
            entities.iter().any(|e| e.topic == service_name),
            "Expected to find our specific service"
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_reports_clients_by_node() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let service_name = unique_graph_name("graph_client_by_node");

        let _client = node
            .create_service_client::<AddTwoInts>(&service_name)
            .build()
            .await?;

        assert!(wait_for_clients(&node, &service_name, 1, 1_000).await?);

        let graph = node.graph().clone();
        let node_key: NodeKey = ("".to_string(), "test_graph_node".to_string());

        let entities = graph.get_entities_by_node(EntityKind::Client, node_key);

        assert!(!entities.is_empty(), "Expected to find client by node");
        assert!(
            entities.iter().any(|e| e.topic == service_name),
            "Expected to find our specific client"
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_counts_entities_added_and_removed_by_topic() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let topic_name = format!(
            "/test_graph_query_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );

        let graph = node.graph().clone();

        let count_pubs = graph.count(EntityKind::Publisher, &topic_name);
        let count_subs = graph.count(EntityKind::Subscription, &topic_name);
        assert_eq!(count_pubs, 0, "Expected 0 publishers initially");
        assert_eq!(count_subs, 0, "Expected 0 subscribers initially");

        let pub_handle = node.publisher::<String>(&topic_name).build().await?;

        assert!(wait_for_publishers(&node, &topic_name, 1, 1_000).await?);

        let count_pubs = graph.count(EntityKind::Publisher, &topic_name);
        assert!(
            count_pubs >= 1,
            "Expected at least 1 publisher after creation"
        );

        let sub_handle = node.subscriber::<String>(&topic_name).build().await?;

        assert!(wait_for_subscribers(&node, &topic_name, 1, 1_000).await?);

        let count_pubs = graph.count(EntityKind::Publisher, &topic_name);
        let count_subs = graph.count(EntityKind::Subscription, &topic_name);
        assert!(count_pubs >= 1, "Expected at least 1 publisher");
        assert!(count_subs >= 1, "Expected at least 1 subscriber");

        drop(pub_handle);
        assert!(wait_for_count_at_most(&node, EntityKind::Publisher, &topic_name, 0, 1_000).await?);

        let count_pubs = graph.count(EntityKind::Publisher, &topic_name);
        let count_subs = graph.count(EntityKind::Subscription, &topic_name);
        assert_eq!(count_pubs, 0, "Expected 0 publishers after drop");
        assert!(count_subs >= 1, "Expected at least 1 subscriber still");

        drop(sub_handle);
        assert!(
            wait_for_count_at_most(&node, EntityKind::Subscription, &topic_name, 0, 1_000).await?
        );

        let count_pubs = graph.count(EntityKind::Publisher, &topic_name);
        let count_subs = graph.count(EntityKind::Subscription, &topic_name);
        assert_eq!(count_pubs, 0, "Expected 0 publishers after all drops");
        assert_eq!(count_subs, 0, "Expected 0 subscribers after all drops");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_lists_node_names() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;

        let graph = node.graph().clone();
        let nodes = graph.get_node_names();

        assert!(!nodes.is_empty(), "Expected to find at least one node");
        assert!(
            nodes.iter().any(|(name, _)| name == "test_graph_node"),
            "Expected to find our test node"
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_discovers_publishers_from_multiple_nodes() -> Result<()> {
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

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_discovers_subscribers_from_multiple_nodes() -> Result<()> {
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

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_discovers_services_from_multiple_nodes() -> Result<()> {
        let context = ContextBuilder::default().build().await?;
        let node1 = context
            .create_node(unique_node_name("graph_service_node_1"))
            .build()
            .await?;
        let node2 = context
            .create_node(unique_node_name("graph_service_node_2"))
            .build()
            .await?;

        let service_name1 = unique_graph_name("graph_multi_node_service_1");
        let service_name2 = unique_graph_name("graph_multi_node_service_2");

        let _srv1 = node1
            .create_service_server::<AddTwoInts>(&service_name1)
            .build()
            .await?;
        let _srv2 = node2
            .create_service_server::<AddTwoInts>(&service_name2)
            .build()
            .await?;

        assert!(wait_for_services(&node1, &service_name1, 1, 1_000).await?);
        assert!(wait_for_services(&node1, &service_name2, 1, 1_000).await?);

        let graph1 = node1.graph();
        let services = graph1.get_service_names_and_types();

        assert!(
            services.iter().any(|(name, _)| name == &service_name1),
            "Expected to find {service_name1}"
        );
        assert!(
            services.iter().any(|(name, _)| name == &service_name2),
            "Expected to find {service_name2}"
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_discovers_clients_from_multiple_nodes() -> Result<()> {
        let context = ContextBuilder::default().build().await?;
        let node1 = context
            .create_node(unique_node_name("graph_client_node_1"))
            .build()
            .await?;
        let node2 = context
            .create_node(unique_node_name("graph_client_node_2"))
            .build()
            .await?;

        let service_name = unique_graph_name("graph_multi_node_client");

        let _srv = node1
            .create_service_server::<AddTwoInts>(&service_name)
            .build()
            .await?;
        let _client1 = node1
            .create_service_client::<AddTwoInts>(&service_name)
            .build()
            .await?;
        let _client2 = node2
            .create_service_client::<AddTwoInts>(&service_name)
            .build()
            .await?;

        assert!(wait_for_clients(&node1, &service_name, 2, 1_000).await?);

        let graph1 = node1.graph();
        let count = graph1.count(EntityKind::Client, &service_name);
        assert!(count >= 2, "Expected at least 2 clients");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_reports_service_server_availability() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let service_name = unique_graph_name("graph_service_available");

        let client = node
            .create_service_client::<AddTwoInts>(&service_name)
            .build()
            .await?;

        let graph = node.graph().clone();
        let count = graph.count(EntityKind::Service, &service_name);
        assert_eq!(count, 0, "Expected 0 services before creating server");

        let service = node
            .create_service_server::<AddTwoInts>(&service_name)
            .build()
            .await?;

        assert!(wait_for_services(&node, &service_name, 1, 1_000).await?);

        let count = graph.count(EntityKind::Service, &service_name);
        assert!(count >= 1, "Expected at least 1 service after creation");

        drop(service);
        assert!(wait_for_count_at_most(&node, EntityKind::Service, &service_name, 0, 1_000).await?);

        let count = graph.count(EntityKind::Service, &service_name);
        assert_eq!(count, 0, "Expected 0 services after dropping server");

        drop(client);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_returns_entities_by_topic() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let topic_name = unique_graph_name("graph_entities_by_topic");

        let _pub = node.publisher::<String>(&topic_name).build().await?;
        let _sub = node.subscriber::<String>(&topic_name).build().await?;

        assert!(wait_for_publishers(&node, &topic_name, 1, 1_000).await?);
        assert!(wait_for_subscribers(&node, &topic_name, 1, 1_000).await?);

        let graph = node.graph().clone();
        let pubs = graph.get_entities_by_topic(EntityKind::Publisher, &topic_name);
        let subs = graph.get_entities_by_topic(EntityKind::Subscription, &topic_name);

        assert!(!pubs.is_empty(), "Expected to find publishers");
        assert!(!subs.is_empty(), "Expected to find subscribers");

        Ok(())
    }
}
