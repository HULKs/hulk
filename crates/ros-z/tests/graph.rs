//! Graph API tests
//!
//! These tests verify the graph introspection functionality, including:
//! - Getting topic/service names and types
//! - Counting publishers/subscribers/clients/services
//! - Waiting for graph changes
//! - Node discovery and information
//! - Service availability checking

use std::{net::TcpListener, num::NonZeroUsize, time::Duration};

use ros_z::{
    Message, ServiceTypeInfo,
    context::ContextBuilder,
    entity::{EndpointEntity, EndpointKind, Entity, NodeEntity, NodeKey, SchemaHash, TypeInfo},
    graph::{GraphChangeSubscription, GraphRevision, GraphView},
    message::Service,
    qos::{QosCompatibility, QosDurability, QosHistory, QosProfile, QosReliability},
};
use serde::{Deserialize, Serialize};
use zenoh::config::WhatAmI;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

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
    fn service_type_info() -> TypeInfo {
        let descriptor = ros_z_schema::ServiceDef::new(
            "test_msgs::AddTwoInts",
            AddTwoIntsRequest::type_name(),
            AddTwoIntsResponse::type_name(),
        )
        .expect("test service descriptor should be static and valid");
        let hash = ros_z_schema::compute_hash(&descriptor)
            .expect("test service hash should be static and valid");
        TypeInfo::new(descriptor.type_name.as_str(), hash)
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

fn unique_listen_endpoint() -> Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    Ok(format!("tcp/127.0.0.1:{port}"))
}

fn isolated_peer_config(
    listen_endpoint: Option<&str>,
    connect_endpoints: &[&str],
) -> Result<zenoh::Config> {
    let mut config = zenoh::Config::default();
    config
        .set_mode(Some(WhatAmI::Peer))
        .map_err(|mode| format!("failed to set Zenoh mode to peer: {mode:?}"))?;
    let listen_endpoints = listen_endpoint
        .map(|endpoint| format!("[\"{endpoint}\"]"))
        .unwrap_or_else(|| "[\"tcp/127.0.0.1:0\"]".to_string());
    let connect_endpoints = if connect_endpoints.is_empty() {
        "[]".to_string()
    } else {
        format!(
            "[{}]",
            connect_endpoints
                .iter()
                .map(|endpoint| format!("\"{endpoint}\""))
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    config.insert_json5("listen/endpoints", &listen_endpoints)?;
    config.insert_json5("connect/endpoints", &connect_endpoints)?;
    config.insert_json5("scouting/multicast/enabled", "false")?;
    Ok(config)
}

async fn try_open_isolated_graph_sessions() -> Result<(zenoh::Session, zenoh::Session)> {
    let graph_endpoint = unique_listen_endpoint()?;
    let graph_session = zenoh::open(isolated_peer_config(Some(&graph_endpoint), &[])?).await?;
    let remote_session = zenoh::open(isolated_peer_config(None, &[&graph_endpoint])?).await?;
    Ok((graph_session, remote_session))
}

async fn open_isolated_graph_sessions() -> Result<(zenoh::Session, zenoh::Session)> {
    let mut last_error = None;
    for _ in 0..8 {
        match try_open_isolated_graph_sessions().await {
            Ok(sessions) => return Ok(sessions),
            Err(error) => last_error = Some(error),
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    Err(last_error.unwrap_or_else(|| "failed to open isolated graph sessions".into()))
}

fn remote_node_entity(session: &zenoh::Session, id: usize, prefix: &str) -> Entity {
    Entity::Node(NodeEntity::new(
        session.zid(),
        id,
        unique_node_name(prefix),
        String::new(),
    ))
}

async fn declare_liveliness_token(
    session: &zenoh::Session,
    entity: &Entity,
) -> Result<zenoh::liveliness::LivelinessToken> {
    let liveliness_key_expr = entity.liveliness_key_expr()?;
    session
        .liveliness()
        .declare_token(liveliness_key_expr.0)
        .await
}

async fn wait_for_liveliness_token(
    session: &zenoh::Session,
    entity: &Entity,
    timeout: Duration,
) -> Result<()> {
    let liveliness_key_expr = entity.liveliness_key_expr()?;
    let started_at = std::time::Instant::now();
    loop {
        if started_at.elapsed() >= timeout {
            return Err(format!(
                "timed out waiting for liveliness token {}",
                liveliness_key_expr.as_str()
            )
            .into());
        }
        let query_timeout = timeout
            .saturating_sub(started_at.elapsed())
            .min(Duration::from_millis(50));
        let replies = session
            .liveliness()
            .get(liveliness_key_expr.as_str())
            .timeout(query_timeout)
            .await?;
        while let Ok(reply) = replies.recv_async().await {
            if let Ok(sample) = reply.into_result()
                && sample.key_expr().as_str() == liveliness_key_expr.as_str()
            {
                return Ok(());
            }
        }
    }
}

async fn wait_for_graph_revision_change(
    changes: &mut GraphChangeSubscription,
    previous_revision: GraphRevision,
    timeout: Duration,
) -> Result<GraphRevision> {
    let revision = changes.mark_seen();
    if revision != previous_revision {
        return Ok(revision);
    }

    tokio::time::timeout(timeout, async {
        loop {
            let revision = changes
                .changed()
                .await
                .ok_or("graph change subscription closed")?;
            if revision != previous_revision {
                return Ok(revision);
            }
        }
    })
    .await
    .map_err(|_| {
        format!("timed out waiting for graph revision change from {previous_revision:?}")
    })?
}

async fn wait_for_count(
    node: &ros_z::node::Node,
    name: &str,
    expected_count: usize,
    timeout_ms: u64,
    count: for<'a> fn(&GraphView<'a>, &str) -> usize,
) -> Result<bool> {
    wait_for_count_matching(node, name, timeout_ms, count, |count| {
        count >= expected_count
    })
    .await
}

async fn wait_for_count_at_most(
    node: &ros_z::node::Node,
    name: &str,
    max_count: usize,
    timeout_ms: u64,
    count: for<'a> fn(&GraphView<'a>, &str) -> usize,
) -> Result<bool> {
    wait_for_count_matching(node, name, timeout_ms, count, |count| count <= max_count).await
}

async fn wait_for_count_matching(
    node: &ros_z::node::Node,
    name: &str,
    timeout_ms: u64,
    count_entities: for<'a> fn(&GraphView<'a>, &str) -> usize,
    matches: impl Fn(usize) -> bool,
) -> Result<bool> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    loop {
        let count = {
            let view = node.graph().view();
            count_entities(&view, name)
        };
        if matches(count) {
            return Ok(true);
        }
        if start.elapsed() >= timeout {
            return Ok(false);
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

fn publisher_count(view: &GraphView<'_>, topic: &str) -> usize {
    view.publisher_count_on(topic)
}

fn subscriber_count(view: &GraphView<'_>, topic: &str) -> usize {
    view.subscription_count_on(topic)
}

fn service_count(view: &GraphView<'_>, service: &str) -> usize {
    view.services_named(service).len()
}

fn client_count(view: &GraphView<'_>, service: &str) -> usize {
    view.clients_named(service).len()
}

async fn wait_for_publishers(
    node: &ros_z::node::Node,
    topic: &str,
    expected_count: usize,
    timeout_ms: u64,
) -> Result<bool> {
    wait_for_count(node, topic, expected_count, timeout_ms, publisher_count).await
}

async fn wait_for_subscribers(
    node: &ros_z::node::Node,
    topic: &str,
    expected_count: usize,
    timeout_ms: u64,
) -> Result<bool> {
    wait_for_count(node, topic, expected_count, timeout_ms, subscriber_count).await
}

async fn wait_for_services(
    node: &ros_z::node::Node,
    service: &str,
    expected_count: usize,
    timeout_ms: u64,
) -> Result<bool> {
    wait_for_count(node, service, expected_count, timeout_ms, service_count).await
}

async fn wait_for_clients(
    node: &ros_z::node::Node,
    service: &str,
    expected_count: usize,
    timeout_ms: u64,
) -> Result<bool> {
    wait_for_count(node, service, expected_count, timeout_ms, client_count).await
}

async fn wait_for_publishers_at_most(
    node: &ros_z::node::Node,
    topic: &str,
    max_count: usize,
    timeout_ms: u64,
) -> Result<bool> {
    wait_for_count_at_most(node, topic, max_count, timeout_ms, publisher_count).await
}

async fn wait_for_subscribers_at_most(
    node: &ros_z::node::Node,
    topic: &str,
    max_count: usize,
    timeout_ms: u64,
) -> Result<bool> {
    wait_for_count_at_most(node, topic, max_count, timeout_ms, subscriber_count).await
}

async fn wait_for_services_at_most(
    node: &ros_z::node::Node,
    service: &str,
    max_count: usize,
    timeout_ms: u64,
) -> Result<bool> {
    wait_for_count_at_most(node, service, max_count, timeout_ms, service_count).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_view_supports_iterators_and_focused_helpers() -> Result<()> {
        let session = zenoh::open(zenoh::Config::default()).await?;
        let graph = ros_z::graph::Graph::new(&session).await?;
        let node = NodeEntity::new(
            session.zid(),
            31,
            unique_node_name("graph_view_node"),
            String::new(),
        );
        let topic = unique_graph_name("graph_view_topic");
        let service = unique_graph_name("graph_view_service");
        let publisher = EndpointEntity {
            id: 32,
            node: node.clone(),
            kind: EndpointKind::Publisher,
            topic: topic.clone(),
            type_info: TypeInfo::new("std_msgs::String", SchemaHash::zero()),
            qos: Default::default(),
        };
        let subscription = EndpointEntity {
            id: 33,
            node: node.clone(),
            kind: EndpointKind::Subscription,
            topic: topic.clone(),
            type_info: TypeInfo::new("std_msgs::String", SchemaHash::zero()),
            qos: Default::default(),
        };
        let service_endpoint = EndpointEntity {
            id: 34,
            node: node.clone(),
            kind: EndpointKind::Service,
            topic: service.clone(),
            type_info: TypeInfo::new("test_msgs::AddTwoInts", SchemaHash::zero()),
            qos: Default::default(),
        };
        let client_endpoint = EndpointEntity {
            id: 35,
            node: node.clone(),
            kind: EndpointKind::Client,
            topic: service.clone(),
            type_info: TypeInfo::new("test_msgs::AddTwoInts", SchemaHash::zero()),
            qos: Default::default(),
        };

        graph.add_local_entity(Entity::Node(node.clone()))?;
        graph.add_local_entity(Entity::Endpoint(publisher.clone()))?;
        graph.add_local_entity(Entity::Endpoint(subscription.clone()))?;
        graph.add_local_entity(Entity::Endpoint(service_endpoint.clone()))?;
        graph.add_local_entity(Entity::Endpoint(client_endpoint.clone()))?;

        let graph_revision = graph.revision();
        let view: ros_z::graph::GraphView<'_> = graph.view();
        assert_eq!(view.revision(), graph_revision);
        assert!(
            view.entities()
                .any(|candidate| candidate == &Entity::Node(node.clone()))
        );
        assert!(
            view.entities()
                .any(|candidate| candidate == &Entity::Endpoint(publisher.clone()))
        );
        assert!(view.nodes().any(|candidate| candidate == &node));
        assert!(view.endpoints().any(|candidate| candidate == &publisher));
        assert_eq!(view.publishers_on(&topic), vec![publisher.clone()]);
        assert_eq!(view.subscriptions_on(&topic), vec![subscription]);
        assert_eq!(view.publisher_count_on(&topic), 1);
        assert_eq!(view.subscription_count_on(&topic), 1);
        assert!(view.has_publishers_on(&topic));
        assert!(view.has_subscriptions_on(&topic));
        assert_eq!(view.publisher_count_on("/missing_topic"), 0);
        assert_eq!(view.subscription_count_on("/missing_topic"), 0);
        assert!(!view.has_publishers_on("/missing_topic"));
        assert!(!view.has_subscriptions_on("/missing_topic"));
        assert_eq!(
            view.services_named(&service),
            vec![service_endpoint.clone()]
        );
        assert_eq!(view.clients_named(&service), vec![client_endpoint]);
        let node_key = node.key();
        assert_eq!(view.endpoints_for_node(node_key.clone()).len(), 4);
        assert!(view.node_exists(&node_key));
        assert!(
            view.topic_names_and_types()
                .contains(&(topic, "std_msgs::String".to_string()))
        );
        assert!(
            view.service_names_and_types()
                .contains(&(service, "test_msgs::AddTwoInts".to_string()))
        );
        assert!(
            view.node_names()
                .contains(&(node.name.clone(), "/".to_string()))
        );

        drop(view);
        session.close().await?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_view_names_and_types_return_one_type_per_name() -> Result<()> {
        let session = zenoh::open(zenoh::Config::default()).await?;
        let graph = ros_z::graph::Graph::new(&session).await?;
        let node = NodeEntity::new(
            session.zid(),
            41,
            unique_node_name("graph_view_summary_node"),
            String::new(),
        );
        let topic = unique_graph_name("graph_view_summary_topic");
        let service = unique_graph_name("graph_view_summary_service");

        graph.add_local_entity(Entity::Node(node.clone()))?;
        graph.add_local_entity(Entity::Endpoint(EndpointEntity {
            id: 42,
            node: node.clone(),
            kind: EndpointKind::Publisher,
            topic: topic.clone(),
            type_info: TypeInfo::new("test_msgs::FirstTopicType", SchemaHash::zero()),
            qos: Default::default(),
        }))?;
        graph.add_local_entity(Entity::Endpoint(EndpointEntity {
            id: 43,
            node: node.clone(),
            kind: EndpointKind::Subscription,
            topic: topic.clone(),
            type_info: TypeInfo::new("test_msgs::SecondTopicType", SchemaHash::zero()),
            qos: Default::default(),
        }))?;
        graph.add_local_entity(Entity::Endpoint(EndpointEntity {
            id: 44,
            node: node.clone(),
            kind: EndpointKind::Service,
            topic: service.clone(),
            type_info: TypeInfo::new("test_msgs::FirstServiceType", SchemaHash::zero()),
            qos: Default::default(),
        }))?;
        graph.add_local_entity(Entity::Endpoint(EndpointEntity {
            id: 45,
            node,
            kind: EndpointKind::Service,
            topic: service.clone(),
            type_info: TypeInfo::new("test_msgs::SecondServiceType", SchemaHash::zero()),
            qos: Default::default(),
        }))?;

        let view = graph.view();
        assert_eq!(
            view.topic_names_and_types()
                .iter()
                .filter(|(name, _)| name == &topic)
                .count(),
            1
        );
        assert_eq!(
            view.service_names_and_types()
                .iter()
                .filter(|(name, _)| name == &service)
                .count(),
            1
        );

        drop(view);
        session.close().await?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_view_deduplicates_names_and_types() -> Result<()> {
        let session = zenoh::open(zenoh::Config::default()).await?;
        let graph = ros_z::graph::Graph::new(&session).await?;
        let node = NodeEntity::new(
            session.zid(),
            41,
            unique_node_name("graph_view_summary_node"),
            String::new(),
        );
        let topic = unique_graph_name("graph_view_summary_topic");
        let service = unique_graph_name("graph_view_summary_service");
        let type_info = TypeInfo::new("std_msgs::String", SchemaHash::zero());

        for (id, kind, name) in [
            (42, EndpointKind::Publisher, topic.clone()),
            (43, EndpointKind::Subscription, topic.clone()),
            (44, EndpointKind::Service, service.clone()),
            (45, EndpointKind::Client, service.clone()),
        ] {
            graph.add_local_entity(Entity::Endpoint(EndpointEntity {
                id,
                node: node.clone(),
                kind,
                topic: name,
                type_info: type_info.clone(),
                qos: Default::default(),
            }))?;
        }

        let view = graph.view();
        assert_eq!(
            view.topic_names_and_types()
                .into_iter()
                .filter(|(name, _)| name == &topic)
                .count(),
            1,
        );
        assert_eq!(
            view.service_names_and_types()
                .into_iter()
                .filter(|(name, _)| name == &service)
                .count(),
            1,
        );

        drop(view);
        session.close().await?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn node_exists_returns_false_after_only_node_removed() -> Result<()> {
        let session = zenoh::open(zenoh::Config::default()).await?;
        let graph = ros_z::graph::Graph::new(&session).await?;
        let node = NodeEntity::new(
            session.zid(),
            1,
            "removed_node".to_string(),
            "/".to_string(),
        );
        let node_key = node.key();
        let entity = Entity::Node(node);

        graph.add_local_entity(entity.clone())?;
        assert!(graph.view().node_exists(&node_key));

        graph.remove_local_entity(&entity)?;

        assert!(!graph.view().node_exists(&node_key));
        session.close().await?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn node_exists_returns_true_when_only_endpoint_exists() -> Result<()> {
        let session = zenoh::open(zenoh::Config::default()).await?;
        let graph = ros_z::graph::Graph::new(&session).await?;
        let node = NodeEntity::new(
            session.zid(),
            2,
            unique_node_name("endpoint_only_node"),
            String::new(),
        );
        let node_key = node.key();

        graph.add_local_entity(Entity::Endpoint(EndpointEntity {
            id: 3,
            node,
            kind: EndpointKind::Publisher,
            topic: unique_graph_name("endpoint_only_topic"),
            type_info: TypeInfo::new("std_msgs::String", SchemaHash::zero()),
            qos: Default::default(),
        }))?;

        assert!(graph.view().node_exists(&node_key));
        session.close().await?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_view_endpoint_queries_are_endpoint_typed() -> Result<()> {
        let context = ContextBuilder::default().build().await?;
        let node = context
            .create_node("graph_node_kind_no_panic")
            .build()
            .await?;
        let graph = node.graph();

        assert!(graph.view().publishers_on("/anything").is_empty());
        assert!(graph.view().services_named("/anything").is_empty());
        assert!(
            graph
                .view()
                .endpoints_for_node(("".into(), "absent_graph_node".into()))
                .is_empty()
        );

        Ok(())
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
            node: node.node_entity().clone(),
            kind: EndpointKind::Publisher,
            topic: topic.clone(),
            type_info: TypeInfo::new("std_msgs::String", SchemaHash::zero()),
            qos: Default::default(),
        });

        graph.add_local_entity(entity.clone())?;
        let held_entities = graph.view().publishers_on(&topic);
        assert_eq!(held_entities.len(), 1);

        graph.add_local_entity(entity)?;
        let current_entities = graph.view().publishers_on(&topic);
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
        let topics = graph.view().topic_names_and_types();

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
            .service_server::<AddTwoInts>(&service_name)
            .build()
            .await?;
        assert!(
            wait_for_services(&node, &service_name, 1, 1_000).await?,
            "Expected graph to discover service for {service_name}"
        );

        let graph = node.graph().clone();
        let services = graph.view().service_names_and_types();

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
        let count = graph.view().publishers_on(&topic_name).len();

        assert_eq!(count, 0, "Expected 0 publishers on non-existent topic");

        let _pub = node.publisher::<String>(&topic_name).build().await?;

        assert!(wait_for_publishers(&node, &topic_name, 1, 1_000).await?);

        let count = graph.view().publishers_on(&topic_name).len();
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
        let count = graph.view().subscriptions_on(&topic_name).len();
        assert_eq!(count, 0, "Expected 0 subscribers on non-existent topic");

        let _sub = node.subscriber::<String>(&topic_name).build().await?;

        assert!(wait_for_subscribers(&node, &topic_name, 1, 1_000).await?);

        let count = graph.view().subscriptions_on(&topic_name).len();
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
        let count = graph.view().clients_named(&service_name).len();

        assert_eq!(count, 0, "Expected 0 clients on non-existent service");

        let _client = node
            .service_client::<AddTwoInts>(&service_name)
            .build()
            .await?;

        assert!(wait_for_clients(&node, &service_name, 1, 1_000).await?);

        let count = graph.view().clients_named(&service_name).len();
        assert!(count >= 1, "Expected at least 1 client after creating one");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_counts_services_after_discovery() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let service_name = unique_graph_name("graph_count_services");

        let graph = node.graph().clone();
        let count = graph.view().services_named(&service_name).len();

        assert_eq!(count, 0, "Expected 0 services on non-existent service");

        let _service = node
            .service_server::<AddTwoInts>(&service_name)
            .build()
            .await?;

        assert!(wait_for_services(&node, &service_name, 1, 1_000).await?);

        let count = graph.view().services_named(&service_name).len();
        assert!(count >= 1, "Expected at least 1 service after creating one");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_reports_services_by_node() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let service_name = unique_graph_name("graph_service_by_node");

        let _service = node
            .service_server::<AddTwoInts>(&service_name)
            .build()
            .await?;

        assert!(wait_for_services(&node, &service_name, 1, 1_000).await?);

        let graph = node.graph().clone();
        let node_key: NodeKey = ("".to_string(), "test_graph_node".to_string());

        let entities: Vec<_> = graph
            .view()
            .endpoints_for_node(node_key)
            .into_iter()
            .filter(|endpoint| endpoint.kind == EndpointKind::Service)
            .collect();

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
            .service_client::<AddTwoInts>(&service_name)
            .build()
            .await?;

        assert!(wait_for_clients(&node, &service_name, 1, 1_000).await?);

        let graph = node.graph().clone();
        let node_key: NodeKey = ("".to_string(), "test_graph_node".to_string());

        let entities: Vec<_> = graph
            .view()
            .endpoints_for_node(node_key)
            .into_iter()
            .filter(|endpoint| endpoint.kind == EndpointKind::Client)
            .collect();

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

        let count_pubs = graph.view().publishers_on(&topic_name).len();
        let count_subs = graph.view().subscriptions_on(&topic_name).len();
        assert_eq!(count_pubs, 0, "Expected 0 publishers initially");
        assert_eq!(count_subs, 0, "Expected 0 subscribers initially");

        let pub_handle = node.publisher::<String>(&topic_name).build().await?;

        assert!(wait_for_publishers(&node, &topic_name, 1, 1_000).await?);

        let count_pubs = graph.view().publishers_on(&topic_name).len();
        assert!(
            count_pubs >= 1,
            "Expected at least 1 publisher after creation"
        );

        let sub_handle = node.subscriber::<String>(&topic_name).build().await?;

        assert!(wait_for_subscribers(&node, &topic_name, 1, 1_000).await?);

        let count_pubs = graph.view().publishers_on(&topic_name).len();
        let count_subs = graph.view().subscriptions_on(&topic_name).len();
        assert!(count_pubs >= 1, "Expected at least 1 publisher");
        assert!(count_subs >= 1, "Expected at least 1 subscriber");

        drop(pub_handle);
        assert!(wait_for_publishers_at_most(&node, &topic_name, 0, 1_000).await?);

        let count_pubs = graph.view().publishers_on(&topic_name).len();
        let count_subs = graph.view().subscriptions_on(&topic_name).len();
        assert_eq!(count_pubs, 0, "Expected 0 publishers after drop");
        assert!(count_subs >= 1, "Expected at least 1 subscriber still");

        drop(sub_handle);
        assert!(wait_for_subscribers_at_most(&node, &topic_name, 0, 1_000).await?);

        let count_pubs = graph.view().publishers_on(&topic_name).len();
        let count_subs = graph.view().subscriptions_on(&topic_name).len();
        assert_eq!(count_pubs, 0, "Expected 0 publishers after all drops");
        assert_eq!(count_subs, 0, "Expected 0 subscribers after all drops");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_lists_node_names() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;

        let graph = node.graph().clone();
        let nodes = graph.view().node_names();

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
            .service_server::<AddTwoInts>(&service_name1)
            .build()
            .await?;
        let _srv2 = node2
            .service_server::<AddTwoInts>(&service_name2)
            .build()
            .await?;

        assert!(wait_for_services(&node1, &service_name1, 1, 1_000).await?);
        assert!(wait_for_services(&node1, &service_name2, 1, 1_000).await?);

        let graph1 = node1.graph();
        let services = graph1.view().service_names_and_types();

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
            .service_server::<AddTwoInts>(&service_name)
            .build()
            .await?;
        let _client1 = node1
            .service_client::<AddTwoInts>(&service_name)
            .build()
            .await?;
        let _client2 = node2
            .service_client::<AddTwoInts>(&service_name)
            .build()
            .await?;

        assert!(wait_for_clients(&node1, &service_name, 2, 1_000).await?);

        let graph1 = node1.graph();
        let count = graph1.view().clients_named(&service_name).len();
        assert!(count >= 2, "Expected at least 2 clients");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_reports_service_server_availability() -> Result<()> {
        let (_ctx, node) = setup_test_node("test_graph_node").await?;
        let service_name = unique_graph_name("graph_service_available");

        let client = node
            .service_client::<AddTwoInts>(&service_name)
            .build()
            .await?;

        let graph = node.graph().clone();
        let count = graph.view().services_named(&service_name).len();
        assert_eq!(count, 0, "Expected 0 services before creating server");

        let service = node
            .service_server::<AddTwoInts>(&service_name)
            .build()
            .await?;

        assert!(wait_for_services(&node, &service_name, 1, 1_000).await?);

        let count = graph.view().services_named(&service_name).len();
        assert!(count >= 1, "Expected at least 1 service after creation");

        drop(service);
        assert!(wait_for_services_at_most(&node, &service_name, 0, 1_000).await?);

        let count = graph.view().services_named(&service_name).len();
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
        let pubs = graph.view().publishers_on(&topic_name);
        let subs = graph.view().subscriptions_on(&topic_name);

        assert!(!pubs.is_empty(), "Expected to find publishers");
        assert!(!subs.is_empty(), "Expected to find subscribers");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn graph_change_subscription_reports_historical_liveliness_put_as_revision() -> Result<()>
    {
        let (graph_session, remote_session) = open_isolated_graph_sessions().await?;
        let entity = remote_node_entity(&remote_session, 91, "graph_revision_history_remote");
        let _token = declare_liveliness_token(&remote_session, &entity).await?;
        wait_for_liveliness_token(&graph_session, &entity, Duration::from_secs(2)).await?;

        let graph = ros_z::graph::Graph::new(&graph_session).await?;
        let mut changes = graph.subscribe_changes();

        let history_revision = wait_for_graph_revision_change(
            &mut changes,
            GraphRevision::INITIAL,
            Duration::from_secs(2),
        )
        .await?;

        assert_ne!(history_revision, GraphRevision::INITIAL);
        assert!(
            graph
                .view()
                .entities()
                .any(|candidate| candidate == &entity)
        );

        graph_session.close().await?;
        remote_session.close().await?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn graph_change_subscription_reports_live_remote_put_and_delete() -> Result<()> {
        let (graph_session, remote_session) = open_isolated_graph_sessions().await?;
        let graph = ros_z::graph::Graph::new(&graph_session).await?;
        let mut changes = graph.subscribe_changes();
        let initial_revision = graph.revision();
        let entity = remote_node_entity(&remote_session, 92, "graph_revision_live_put_delete");

        let token = declare_liveliness_token(&remote_session, &entity).await?;

        let put_revision =
            wait_for_graph_revision_change(&mut changes, initial_revision, Duration::from_secs(2))
                .await?;
        assert_ne!(put_revision, initial_revision);
        assert!(
            graph
                .view()
                .entities()
                .any(|candidate| candidate == &entity)
        );

        drop(token);
        let delete_revision =
            wait_for_graph_revision_change(&mut changes, put_revision, Duration::from_secs(2))
                .await?;
        assert_ne!(delete_revision, put_revision);
        assert!(
            !graph
                .view()
                .entities()
                .any(|candidate| candidate == &entity)
        );

        graph_session.close().await?;
        remote_session.close().await?;
        Ok(())
    }
}
