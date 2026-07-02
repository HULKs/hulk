use std::fmt;
use std::time::Duration;

use itertools::Itertools;
use tokio::time::Instant;

use crate::{
    dynamic::{DynamicCdrCodec, DynamicError, DynamicPayload, DynamicSubscriber, Schema},
    endpoint_builder::{EndpointBuilderContext, MessageEndpointType},
    entity::{EndpointEntity, EndpointKind, SchemaHash, TypeInfo},
    graph::Graph,
    pubsub::{RawPayload, RawPayloadCodec, RawSubscriber, SubscriberBuilder, SubscriberOptions},
    qos::QosProfile,
    topic_name::{qualify_remote_private_service_name, qualify_topic_name},
};
use tracing::info;

#[derive(Debug, Clone)]
pub struct DiscoveredTopicSchema {
    pub qualified_topic: String,
    pub root_name: String,
    pub schema: Schema,
    pub schema_hash: SchemaHash,
}

impl DiscoveredTopicSchema {
    pub fn type_info(&self) -> TypeInfo {
        TypeInfo::new(&self.root_name, self.schema_hash)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct TopicSchemaFingerprint {
    pub topic: String,
    pub node_namespace: String,
    pub node_name: String,
    pub type_info: TypeInfo,
}

impl TopicSchemaFingerprint {
    pub fn from_publisher(endpoint: &EndpointEntity) -> Self {
        Self {
            topic: endpoint.topic.clone(),
            node_namespace: endpoint.node.namespace.clone(),
            node_name: endpoint.node.name.clone(),
            type_info: endpoint.type_info.clone(),
        }
    }
}

impl fmt::Display for TopicSchemaFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{}:{}@{}",
            self.node_namespace, self.node_name, self.type_info.name, self.type_info.hash
        )
    }
}

pub fn topic_schema_fingerprints_from_publishers(
    publishers: &[EndpointEntity],
) -> Vec<TopicSchemaFingerprint> {
    let mut fingerprints = publishers
        .iter()
        .map(TopicSchemaFingerprint::from_publisher)
        .collect_vec();
    fingerprints.sort_by(|left, right| {
        (
            &left.topic,
            &left.node_namespace,
            &left.node_name,
            &left.type_info.name,
            &left.type_info.hash.0,
        )
            .cmp(&(
                &right.topic,
                &right.node_namespace,
                &right.node_name,
                &right.type_info.name,
                &right.type_info.hash.0,
            ))
    });
    fingerprints.dedup();
    fingerprints
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TopicSchemaCandidate {
    pub node_name: String,
    pub namespace: String,
    pub type_name: String,
    pub schema_hash: SchemaHash,
}

impl TopicSchemaCandidate {
    fn from_fingerprint(fingerprint: &TopicSchemaFingerprint) -> Self {
        Self {
            node_name: fingerprint.node_name.clone(),
            namespace: fingerprint.node_namespace.clone(),
            type_name: fingerprint.type_info.name.clone(),
            schema_hash: fingerprint.type_info.hash,
        }
    }

    pub(crate) fn schema_service_name(
        &self,
        operation: &'static str,
    ) -> Result<String, DynamicError> {
        qualify_remote_private_service_name("get_schema", &self.namespace, &self.node_name)
            .map_err(|source| DynamicError::name(operation, source))
    }
}

impl fmt::Display for TopicSchemaCandidate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{}:{}@{}",
            self.namespace, self.node_name, self.type_name, self.schema_hash
        )
    }
}

pub(crate) fn collect_topic_schema_candidates_from_publishers(
    publishers: &[EndpointEntity],
    qualified_topic: &str,
) -> Result<Vec<TopicSchemaCandidate>, DynamicError> {
    let candidates = topic_schema_fingerprints_from_publishers(publishers)
        .iter()
        .map(TopicSchemaCandidate::from_fingerprint)
        .collect_vec();

    if candidates.is_empty() {
        return Err(DynamicError::NoPublishers {
            topic: qualified_topic.to_string(),
        });
    }

    if !candidates
        .iter()
        .map(|candidate| (&candidate.type_name, candidate.schema_hash))
        .all_equal()
    {
        return Err(DynamicError::TopicTypeConflict {
            topic: qualified_topic.to_string(),
            candidates: candidates.iter().map(ToString::to_string).collect_vec(),
        });
    }

    Ok(candidates)
}

fn collect_topic_schema_candidates(
    graph: &Graph,
    qualified_topic: &str,
) -> Result<Vec<TopicSchemaCandidate>, DynamicError> {
    let data = graph.lock();
    let publishers = data
        .publishers_on(qualified_topic)
        .cloned()
        .collect::<Vec<_>>();

    collect_topic_schema_candidates_from_publishers(&publishers, qualified_topic)
}

fn schema_query_timeout(deadline: Instant) -> Option<Duration> {
    let timeout = deadline.saturating_duration_since(Instant::now());
    if timeout.is_zero() {
        return None;
    }
    Some(timeout)
}

fn qualify_topic_for_discovery(
    topic: &str,
    namespace: &str,
    node_name: &str,
) -> Result<String, DynamicError> {
    qualify_topic_name(topic, namespace, node_name).map_err(|source| DynamicError::TopicName {
        topic: topic.to_string(),
        source,
    })
}

fn collect_visible_schema_service_candidates(
    candidates: &[TopicSchemaCandidate],
    visible_services: &[EndpointEntity],
) -> Result<Vec<TopicSchemaCandidate>, DynamicError> {
    let mut visible_candidates = Vec::new();

    for candidate in candidates {
        let service_name =
            candidate.schema_service_name("checking remote schema service visibility")?;
        if visible_services
            .iter()
            .any(|service| service.kind == EndpointKind::Service && service.topic == service_name)
        {
            visible_candidates.push(candidate.clone());
        }
    }

    Ok(visible_candidates)
}

struct SchemaServiceCandidateSnapshot {
    compatible: Vec<TopicSchemaCandidate>,
    visible: Vec<TopicSchemaCandidate>,
}

fn collect_schema_service_candidate_snapshot(
    graph: &Graph,
    qualified_topic: &str,
) -> Result<SchemaServiceCandidateSnapshot, DynamicError> {
    let data = graph.lock();
    let publishers = data
        .publishers_on(qualified_topic)
        .cloned()
        .collect::<Vec<_>>();
    let compatible = collect_topic_schema_candidates_from_publishers(&publishers, qualified_topic)?;
    let visible_services = data.services().cloned().collect_vec();
    let visible = collect_visible_schema_service_candidates(&compatible, &visible_services)?;

    Ok(SchemaServiceCandidateSnapshot {
        compatible,
        visible,
    })
}

fn no_schema_services_error(
    qualified_topic: &str,
    candidates: &[TopicSchemaCandidate],
) -> DynamicError {
    DynamicError::NoSchemaServices {
        topic: qualified_topic.to_string(),
        candidates: candidates.iter().map(ToString::to_string).collect_vec(),
    }
}

fn schema_discovery_timeout_error(
    qualified_topic: &str,
    candidates: &[TopicSchemaCandidate],
    source: Option<DynamicError>,
) -> DynamicError {
    DynamicError::SchemaDiscoveryTimeout {
        topic: qualified_topic.to_string(),
        candidates: candidates.iter().map(ToString::to_string).collect_vec(),
        source: source.map(|error| Box::new(error) as crate::error::BoxError),
    }
}

async fn wait_for_topic_schema_candidates(
    graph: &Graph,
    qualified_topic: &str,
    deadline: Instant,
) -> Result<Vec<TopicSchemaCandidate>, DynamicError> {
    let mut revisions = graph.watch_revisions();

    loop {
        revisions.mark_seen();

        match collect_topic_schema_candidates(graph, qualified_topic) {
            Ok(candidates) => return Ok(candidates),
            Err(DynamicError::NoPublishers { .. }) => {}
            Err(error) => return Err(error),
        }

        let Some(timeout) = schema_query_timeout(deadline) else {
            return collect_topic_schema_candidates(graph, qualified_topic);
        };

        match tokio::time::timeout(timeout, revisions.changed()).await {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => return collect_topic_schema_candidates(graph, qualified_topic),
        }
    }
}

async fn wait_for_visible_schema_service_candidates(
    graph: &Graph,
    qualified_topic: &str,
    deadline: Instant,
) -> Result<Vec<TopicSchemaCandidate>, DynamicError> {
    let mut revisions = graph.watch_revisions();

    loop {
        revisions.mark_seen();

        let snapshot = collect_schema_service_candidate_snapshot(graph, qualified_topic)?;
        if !snapshot.visible.is_empty() {
            return Ok(snapshot.visible);
        }

        let Some(timeout) = schema_query_timeout(deadline) else {
            return Err(no_schema_services_error(
                qualified_topic,
                &snapshot.compatible,
            ));
        };

        match tokio::time::timeout(timeout, revisions.changed()).await {
            Ok(Some(_)) | Err(_) => {}
            Ok(None) => {
                let snapshot = collect_schema_service_candidate_snapshot(graph, qualified_topic)?;
                if !snapshot.visible.is_empty() {
                    return Ok(snapshot.visible);
                }
                return Err(no_schema_services_error(
                    qualified_topic,
                    &snapshot.compatible,
                ));
            }
        }
    }
}

pub(crate) struct SchemaDiscovery {
    context: EndpointBuilderContext,
    timeout: Duration,
}

impl SchemaDiscovery {
    pub(crate) fn new(context: EndpointBuilderContext, timeout: Duration) -> Self {
        Self { context, timeout }
    }

    async fn discover_candidates(
        &self,
        topic: &str,
    ) -> Result<(String, Vec<TopicSchemaCandidate>, tokio::time::Instant), DynamicError> {
        let qualified_topic = qualify_topic_for_discovery(
            topic,
            &self.context.node.namespace,
            &self.context.node.name,
        )?;

        self.discover_qualified_candidates(qualified_topic).await
    }

    async fn discover_qualified_candidates(
        &self,
        qualified_topic: String,
    ) -> Result<(String, Vec<TopicSchemaCandidate>, tokio::time::Instant), DynamicError> {
        let deadline = tokio::time::Instant::now() + self.timeout;
        let candidates = wait_for_topic_schema_candidates(
            self.context.graph.as_ref(),
            &qualified_topic,
            deadline,
        )
        .await?;

        Ok((qualified_topic, candidates, deadline))
    }

    pub(crate) async fn discover_qualified(
        &self,
        qualified_topic: String,
    ) -> Result<DiscoveredTopicSchema, DynamicError> {
        let (qualified_topic, _candidates, deadline) =
            self.discover_qualified_candidates(qualified_topic).await?;
        self.discover_from_candidates(qualified_topic, deadline)
            .await
    }

    pub(crate) async fn discover(
        &self,
        topic: &str,
    ) -> Result<DiscoveredTopicSchema, DynamicError> {
        let (qualified_topic, _candidates, deadline) = self.discover_candidates(topic).await?;
        self.discover_from_candidates(qualified_topic, deadline)
            .await
    }

    async fn discover_from_candidates(
        &self,
        qualified_topic: String,
        deadline: Instant,
    ) -> Result<DiscoveredTopicSchema, DynamicError> {
        let (root_name, schema, schema_hash) =
            self.try_schema_service(&qualified_topic, deadline).await?;

        Ok(DiscoveredTopicSchema {
            qualified_topic,
            root_name,
            schema,
            schema_hash,
        })
    }

    async fn try_schema_service(
        &self,
        qualified_topic: &str,
        deadline: Instant,
    ) -> Result<(String, Schema, SchemaHash), DynamicError> {
        let mut last_error = None;
        let candidates = wait_for_visible_schema_service_candidates(
            self.context.graph.as_ref(),
            qualified_topic,
            deadline,
        )
        .await?;

        for candidate in &candidates {
            let Some(timeout) = schema_query_timeout(deadline) else {
                return Err(schema_discovery_timeout_error(
                    qualified_topic,
                    &candidates,
                    last_error,
                ));
            };

            match tokio::time::timeout_at(
                deadline,
                super::schema_query::query_schema(&self.context, candidate, timeout),
            )
            .await
            {
                Ok(Ok(result)) => return Ok(result),
                Ok(Err(error)) => last_error = Some(error),
                Err(_) => {
                    return Err(schema_discovery_timeout_error(
                        qualified_topic,
                        &candidates,
                        last_error,
                    ));
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| schema_discovery_timeout_error(qualified_topic, &candidates, None)))
    }
}

/// Builder for dynamic subscribers that discover their schema at build time.
///
/// Create this with [`crate::node::Node::dynamic_subscriber_auto`]. Schema
/// discovery runs in [`build`](Self::build), so construction and option
/// configuration remain infallible. This builder exposes subscriber options
/// that do not require knowing the message schema up front. Use [`raw`](Self::raw)
/// for raw sample delivery after discovery.
#[derive(Debug)]
pub struct DynamicSubscriberDiscoveryBuilder {
    context: EndpointBuilderContext,
    topic: String,
    discovery_timeout: Duration,
    options: SubscriberOptions,
}

/// Builder for raw dynamic subscribers that discover type metadata at build time.
#[derive(Debug)]
pub struct DynamicRawSubscriberDiscoveryBuilder {
    context: EndpointBuilderContext,
    topic: String,
    discovery_timeout: Duration,
    options: SubscriberOptions,
}

async fn discover_dynamic_subscriber_builder(
    context: EndpointBuilderContext,
    topic: String,
    discovery_timeout: Duration,
    options: SubscriberOptions,
) -> crate::Result<SubscriberBuilder<DynamicPayload, DynamicCdrCodec>> {
    let qualified_topic = qualify_topic_name(&topic, &context.node.namespace, &context.node.name)
        .map_err(|source| crate::Error::topic_name(topic.clone(), source))?;

    let discovered = SchemaDiscovery::new(context.clone(), discovery_timeout)
        .discover_qualified(qualified_topic)
        .await?;

    info!(
        "[NOD] Discovered schema for topic {}: {} (hash: {})",
        discovered.qualified_topic,
        discovered.root_name,
        discovered.schema_hash.to_hash_string()
    );

    Ok(SubscriberBuilder::<DynamicPayload, DynamicCdrCodec>::new(
        context,
        topic,
        MessageEndpointType::prevalidated_dynamic(discovered.type_info(), discovered.schema),
    )
    .options(options))
}

async fn discover_raw_subscriber_builder(
    context: EndpointBuilderContext,
    topic: String,
    discovery_timeout: Duration,
    options: SubscriberOptions,
) -> crate::Result<SubscriberBuilder<RawPayload, RawPayloadCodec>> {
    let qualified_topic = qualify_topic_name(&topic, &context.node.namespace, &context.node.name)
        .map_err(|source| crate::Error::topic_name(topic.clone(), source))?;
    let (_, candidates, _) = SchemaDiscovery::new(context.clone(), discovery_timeout)
        .discover_qualified_candidates(qualified_topic.clone())
        .await?;
    let candidate = &candidates[0];
    let type_info = TypeInfo::new(&candidate.type_name, candidate.schema_hash);

    info!(
        "[NOD] Discovered raw topic {}: {} (hash: {})",
        qualified_topic,
        type_info.name,
        type_info.hash.to_hash_string()
    );

    Ok(SubscriberBuilder::<RawPayload, RawPayloadCodec>::new(
        context,
        topic,
        MessageEndpointType::type_info_only(type_info),
    )
    .options(options))
}

impl DynamicSubscriberDiscoveryBuilder {
    pub(crate) fn new(
        context: EndpointBuilderContext,
        topic: String,
        discovery_timeout: Duration,
    ) -> Self {
        Self {
            context,
            topic,
            discovery_timeout,
            options: SubscriberOptions::default(),
        }
    }

    /// Set the QoS profile used by the built dynamic subscriber.
    ///
    /// This does not affect the schema discovery request timeout. Use
    /// [`transient_local_replay_timeout`](Self::transient_local_replay_timeout)
    /// to configure transient-local replay after the subscriber has been built.
    pub fn qos(mut self, qos: QosProfile) -> Self {
        self.options = self.options.qos(qos);
        self
    }

    /// Limit accepted samples by their zenoh origin locality.
    ///
    /// The locality filter is applied to the dynamic subscriber created after
    /// schema discovery succeeds.
    pub fn locality(mut self, locality: zenoh::sample::Locality) -> Self {
        self.options = self.options.locality(locality);
        self
    }

    /// Set how long transient-local subscribers wait for replay responses.
    ///
    /// This timeout is separate from the schema discovery timeout passed to
    /// [`crate::node::Node::dynamic_subscriber_auto`]. It only applies when the
    /// subscriber QoS requests transient-local durability.
    pub fn transient_local_replay_timeout(mut self, timeout: Duration) -> Self {
        self.options = self.options.transient_local_replay_timeout(timeout);
        self
    }

    /// Switch this discovery builder to raw sample delivery.
    ///
    /// Publisher type discovery still runs at build time so the subscriber
    /// advertises the discovered dynamic type metadata, but received samples are
    /// returned without deserialization and schema-service lookup.
    pub fn raw(self) -> DynamicRawSubscriberDiscoveryBuilder {
        DynamicRawSubscriberDiscoveryBuilder {
            context: self.context,
            topic: self.topic,
            discovery_timeout: self.discovery_timeout,
            options: self.options,
        }
    }

    /// Discover the topic schema and build the dynamic subscriber.
    ///
    /// This performs the fallible work deferred by the builder: topic
    /// qualification, schema discovery, schema validation, and subscriber
    /// declaration. The returned subscriber decodes payloads using the
    /// discovered schema.
    pub async fn build(self) -> crate::Result<DynamicSubscriber> {
        let Self {
            context,
            topic,
            discovery_timeout,
            options,
        } = self;

        discover_dynamic_subscriber_builder(context, topic, discovery_timeout, options)
            .await?
            .build()
            .await
    }
}

impl DynamicRawSubscriberDiscoveryBuilder {
    /// Set the QoS profile used by the built raw dynamic subscriber.
    pub fn qos(mut self, qos: QosProfile) -> Self {
        self.options = self.options.qos(qos);
        self
    }

    /// Limit accepted samples by their zenoh origin locality.
    pub fn locality(mut self, locality: zenoh::sample::Locality) -> Self {
        self.options = self.options.locality(locality);
        self
    }

    /// Set how long transient-local subscribers wait for replay responses.
    pub fn transient_local_replay_timeout(mut self, timeout: Duration) -> Self {
        self.options = self.options.transient_local_replay_timeout(timeout);
        self
    }

    /// Discover topic type metadata and build a raw dynamic subscriber.
    pub async fn build(self) -> crate::Result<RawSubscriber> {
        let Self {
            context,
            topic,
            discovery_timeout,
            options,
        } = self;

        discover_raw_subscriber_builder(context, topic, discovery_timeout, options)
            .await?
            .raw()
            .build()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EndpointKind, NodeEntity, SchemaHash, TypeInfo};

    fn publisher(type_name: &str) -> EndpointEntity {
        EndpointEntity {
            id: 1,
            node: NodeEntity {
                z_id: Default::default(),
                id: 2,
                name: "talker".to_string(),
                namespace: "/".to_string(),
            },
            kind: EndpointKind::Publisher,
            topic: "/chatter".to_string(),
            type_info: TypeInfo::new(type_name, SchemaHash::zero()),
            qos: Default::default(),
        }
    }

    fn publisher_with_hash(type_name: &str, hash: SchemaHash) -> EndpointEntity {
        publisher_with_hash_from_node(type_name, hash, "talker", 2)
    }

    fn publisher_with_hash_from_node(
        type_name: &str,
        hash: SchemaHash,
        node_name: &str,
        node_id: usize,
    ) -> EndpointEntity {
        EndpointEntity {
            id: 1,
            node: NodeEntity {
                z_id: Default::default(),
                id: node_id,
                name: node_name.to_string(),
                namespace: "/".to_string(),
            },
            kind: EndpointKind::Publisher,
            topic: "/chatter".to_string(),
            type_info: TypeInfo::new(type_name, hash),
            qos: Default::default(),
        }
    }

    fn schema_service_for_node(node_name: &str, node_id: usize) -> EndpointEntity {
        EndpointEntity {
            id: 100 + node_id,
            node: NodeEntity {
                z_id: Default::default(),
                id: node_id,
                name: node_name.to_string(),
                namespace: "/".to_string(),
            },
            kind: EndpointKind::Service,
            topic: format!("/{node_name}/get_schema"),
            type_info: TypeInfo::new("ros_z::GetSchema", SchemaHash::zero()),
            qos: Default::default(),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn topic_candidate_wait_returns_existing_publishers_at_expired_deadline() {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .expect("open Zenoh session");
        let graph = Graph::new(&session).await.expect("create graph");
        graph
            .add_local_entity(crate::entity::Entity::Endpoint(publisher(
                "std_msgs::String",
            )))
            .expect("add publisher");

        let candidates =
            wait_for_topic_schema_candidates(&graph, "/chatter", tokio::time::Instant::now())
                .await
                .expect("publisher is visible at expired deadline");

        assert_eq!(candidates.len(), 1);

        session.close().await.expect("close Zenoh session");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn visible_schema_service_wait_returns_existing_services_at_expired_deadline() {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .expect("open Zenoh session");
        let graph = Graph::new(&session).await.expect("create graph");
        let hash = SchemaHash([1; 32]);
        graph
            .add_local_entity(crate::entity::Entity::Endpoint(
                publisher_with_hash_from_node("std_msgs::String", hash, "talker", 2),
            ))
            .expect("add publisher");
        graph
            .add_local_entity(crate::entity::Entity::Endpoint(schema_service_for_node(
                "talker", 2,
            )))
            .expect("add schema service");

        let visible = wait_for_visible_schema_service_candidates(
            &graph,
            "/chatter",
            tokio::time::Instant::now(),
        )
        .await
        .expect("schema service is visible at expired deadline");

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].node_name, "talker");

        session.close().await.expect("close Zenoh session");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn visible_schema_service_wait_reports_no_schema_services_at_deadline() {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .expect("open Zenoh session");
        let graph = Graph::new(&session).await.expect("create graph");
        let hash = SchemaHash([1; 32]);
        graph
            .add_local_entity(crate::entity::Entity::Endpoint(
                publisher_with_hash_from_node(
                    "std_msgs::String",
                    hash,
                    "talker_without_service",
                    2,
                ),
            ))
            .expect("add publisher");

        let error = wait_for_visible_schema_service_candidates(
            &graph,
            "/chatter",
            tokio::time::Instant::now(),
        )
        .await
        .expect_err("publisher without visible schema service should fail at deadline");

        let DynamicError::NoSchemaServices { topic, candidates } = error else {
            panic!("expected no schema services error, got {error:?}");
        };
        assert_eq!(topic, "/chatter");
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].contains("talker_without_service"));
        assert!(candidates[0].contains("std_msgs::String"));

        session.close().await.expect("close Zenoh session");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn try_schema_service_reports_timeout_when_visible_service_budget_elapsed() {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .expect("open Zenoh session");
        let graph = std::sync::Arc::new(Graph::new(&session).await.expect("create graph"));
        let hash = SchemaHash([1; 32]);
        graph
            .add_local_entity(crate::entity::Entity::Endpoint(
                publisher_with_hash_from_node("std_msgs::String", hash, "talker", 2),
            ))
            .expect("add publisher");
        graph
            .add_local_entity(crate::entity::Entity::Endpoint(schema_service_for_node(
                "talker", 2,
            )))
            .expect("add visible schema service");
        let context = EndpointBuilderContext::new(
            session.clone(),
            graph,
            std::sync::Arc::new(crate::context::GlobalCounter::default()),
            NodeEntity {
                z_id: Default::default(),
                id: 10,
                name: "listener".to_string(),
                namespace: "/".to_string(),
            },
            crate::time::Clock::default(),
            None,
            None,
        );
        let discovery = SchemaDiscovery::new(context, Duration::ZERO);

        let error = discovery
            .try_schema_service("/chatter", tokio::time::Instant::now())
            .await
            .expect_err("elapsed query budget should fail without denying service visibility");

        let DynamicError::SchemaDiscoveryTimeout {
            topic,
            candidates,
            source,
        } = error
        else {
            panic!("expected schema discovery timeout, got {error:?}");
        };
        assert_eq!(topic, "/chatter");
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].contains("talker"));
        assert!(candidates[0].contains("std_msgs::String"));
        assert!(source.is_none());

        session.close().await.expect("close Zenoh session");
    }

    #[test]
    fn schema_discovery_timeout_error_preserves_previous_candidate_error_as_source() {
        let candidate = TopicSchemaCandidate {
            node_name: "talker".to_string(),
            namespace: "/".to_string(),
            type_name: "std_msgs::String".to_string(),
            schema_hash: SchemaHash([1; 32]),
        };
        let previous_error = DynamicError::NoSchemaServices {
            topic: "/chatter".to_string(),
            candidates: vec![candidate.to_string()],
        };

        let error = schema_discovery_timeout_error("/chatter", &[candidate], Some(previous_error));

        let DynamicError::SchemaDiscoveryTimeout {
            topic,
            candidates,
            source,
        } = error
        else {
            panic!("expected schema discovery timeout, got {error:?}");
        };
        assert_eq!(topic, "/chatter");
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].contains("talker"));
        assert!(source.is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn visible_schema_service_wait_rejects_current_conflicts_before_visible_service() {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .expect("open Zenoh session");
        let graph = Graph::new(&session).await.expect("create graph");
        let hash = SchemaHash([3; 32]);
        graph
            .add_local_entity(crate::entity::Entity::Endpoint(
                publisher_with_hash_from_node("std_msgs::String", hash, "talker", 2),
            ))
            .expect("add first publisher");
        graph
            .add_local_entity(crate::entity::Entity::Endpoint(
                publisher_with_hash_from_node("custom_msgs::StringLike", hash, "other_talker", 3),
            ))
            .expect("add conflicting publisher");
        graph
            .add_local_entity(crate::entity::Entity::Endpoint(schema_service_for_node(
                "talker", 2,
            )))
            .expect("add schema service");

        let error = wait_for_visible_schema_service_candidates(
            &graph,
            "/chatter",
            tokio::time::Instant::now(),
        )
        .await
        .expect_err("current publisher conflicts should win over visible services");

        let DynamicError::TopicTypeConflict { topic, candidates } = error else {
            panic!("expected topic type conflict, got {error:?}");
        };
        assert_eq!(topic, "/chatter");
        assert_eq!(candidates.len(), 2);
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.contains("std_msgs::String"))
        );
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.contains("custom_msgs::StringLike"))
        );

        session.close().await.expect("close Zenoh session");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn visible_schema_service_wait_rechecks_topic_candidates_after_graph_change() {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .expect("open Zenoh session");
        let graph = Graph::new(&session).await.expect("create graph");
        let hash = SchemaHash([1; 32]);
        graph
            .add_local_entity(crate::entity::Entity::Endpoint(
                publisher_with_hash_from_node("std_msgs::String", hash, "talker", 2),
            ))
            .expect("add initial publisher");

        let wait = wait_for_visible_schema_service_candidates(
            &graph,
            "/chatter",
            tokio::time::Instant::now() + Duration::from_secs(1),
        );
        tokio::pin!(wait);

        tokio::select! {
            result = &mut wait => panic!("wait finished before late schema service appeared: {result:?}"),
            _ = tokio::time::sleep(Duration::from_millis(10)) => {}
        }

        graph
            .add_local_entity(crate::entity::Entity::Endpoint(
                publisher_with_hash_from_node("std_msgs::String", hash, "late_talker", 3),
            ))
            .expect("add late publisher");
        graph
            .add_local_entity(crate::entity::Entity::Endpoint(schema_service_for_node(
                "late_talker",
                3,
            )))
            .expect("add late schema service");

        let visible = wait.await.expect("late schema service becomes visible");

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].node_name, "late_talker");

        session.close().await.expect("close Zenoh session");
    }

    #[test]
    fn discovery_topic_qualification_errors_preserve_original_topic() {
        let error = qualify_topic_for_discovery("", "/", "listener")
            .expect_err("empty topic fails discovery topic qualification");

        let DynamicError::TopicName { topic, source } = error else {
            panic!("expected topic-name error");
        };

        assert_eq!(topic, "");
        assert_eq!(source, crate::topic_name::TopicNameError::Empty);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn discover_topic_schema_rejects_empty_topic_name() {
        let context = crate::context::ContextBuilder::default()
            .disable_multicast_scouting()
            .with_json("connect/endpoints", serde_json::json!([]))
            .build()
            .await
            .expect("create test context");
        let node = context
            .create_node("listener")
            .without_schema_service()
            .build()
            .await
            .expect("create test node");

        let result = node
            .discover_topic_schema("", Duration::from_millis(1))
            .await;
        context.shutdown().expect("shutdown test context");
        let error = result.expect_err("empty topic fails discovery API");

        let DynamicError::TopicName { topic, source } = error else {
            panic!("expected topic-name error");
        };

        assert_eq!(topic, "");
        assert_eq!(source, crate::topic_name::TopicNameError::Empty);
    }

    #[test]
    fn candidate_schema_service_name_uses_get_schema_private_service() {
        let candidate = TopicSchemaCandidate {
            node_name: "talker".to_string(),
            namespace: "/robot".to_string(),
            type_name: "std_msgs::String".to_string(),
            schema_hash: SchemaHash::zero(),
        };

        let service_name = candidate
            .schema_service_name("testing schema service helper")
            .expect("valid candidate service name");

        assert_eq!(service_name, "/robot/talker/get_schema");
    }

    #[test]
    fn topic_schema_fingerprints_from_publishers_returns_canonical_schema_identity() {
        let hash = SchemaHash([7; 32]);
        let talker_b = publisher_with_hash_from_node("std_msgs::String", hash, "talker_b", 3);
        let mut duplicate_talker_b =
            publisher_with_hash_from_node("std_msgs::String", hash, "talker_b", 3);
        duplicate_talker_b.id = 99;
        let talker_a = publisher_with_hash_from_node("std_msgs::String", hash, "talker_a", 2);

        let fingerprints =
            topic_schema_fingerprints_from_publishers(&[talker_b, duplicate_talker_b, talker_a]);

        assert_eq!(fingerprints.len(), 2);
        assert_eq!(fingerprints[0].topic, "/chatter");
        assert_eq!(fingerprints[0].node_namespace, "/");
        assert_eq!(fingerprints[0].node_name, "talker_a");
        assert_eq!(
            fingerprints[0].type_info,
            TypeInfo::new("std_msgs::String", hash)
        );
        assert_eq!(fingerprints[1].node_name, "talker_b");
    }

    #[test]
    fn schema_query_timeout_returns_none_for_elapsed_deadline() {
        let deadline = Instant::now() - Duration::from_millis(1);

        assert_eq!(schema_query_timeout(deadline), None);
    }

    #[test]
    fn schema_query_timeout_returns_remaining_duration_for_future_deadline() {
        let timeout = schema_query_timeout(Instant::now() + Duration::from_secs(1))
            .expect("future deadline should leave time for one schema query");

        assert!(timeout > Duration::ZERO);
        assert!(timeout <= Duration::from_secs(1));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn schema_candidate_wait_reports_no_publishers_when_none_arrive() {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .expect("open Zenoh session");
        let graph = Graph::new(&session).await.expect("create graph");

        let error =
            wait_for_topic_schema_candidates(&graph, "/missing_schema_timeout", Instant::now())
                .await
                .expect_err("elapsed publisher wait should report missing publishers");

        let DynamicError::NoPublishers { topic } = error else {
            panic!("expected no publishers, got {error:?}");
        };
        assert_eq!(topic, "/missing_schema_timeout");

        session.close().await.expect("close Zenoh session");
    }

    #[test]
    fn publisher_schema_candidates_keep_native_advertised_type_name() {
        let candidates = collect_topic_schema_candidates_from_publishers(
            &[publisher("std_msgs::String")],
            "/chatter",
        )
        .expect("candidate");

        assert_eq!(candidates[0].type_name, "std_msgs::String");
    }

    #[test]
    fn publisher_schema_candidates_do_not_normalize_dds_shaped_names() {
        let candidates = collect_topic_schema_candidates_from_publishers(
            &[publisher("std_msgs::msg::dds_::String_")],
            "/chatter",
        )
        .expect("candidate");

        assert_eq!(candidates[0].type_name, "std_msgs::msg::dds_::String_");
    }

    #[test]
    fn publisher_schema_candidates_use_strict_endpoint_type_info() {
        let hash = SchemaHash([0x42; 32]);
        let publisher = EndpointEntity {
            id: 1,
            node: NodeEntity {
                z_id: Default::default(),
                id: 2,
                name: "talker".to_string(),
                namespace: "/".to_string(),
            },
            kind: EndpointKind::Publisher,
            topic: "/chatter".to_string(),
            type_info: TypeInfo::new("std_msgs::String", hash),
            qos: Default::default(),
        };

        let candidates = collect_topic_schema_candidates_from_publishers(&[publisher], "/chatter")
            .expect("candidate");

        assert_eq!(candidates[0].type_name, "std_msgs::String");
        assert_eq!(candidates[0].schema_hash, hash);
    }

    #[test]
    fn schema_candidates_keep_compatible_publishers_for_service_fallback() {
        let hash = SchemaHash([1; 32]);
        let publishers = vec![
            publisher_with_hash_from_node("std_msgs::String", hash, "talker_a", 2),
            publisher_with_hash_from_node("std_msgs::String", hash, "talker_b", 3),
        ];

        let candidates = collect_topic_schema_candidates_from_publishers(&publishers, "/chatter")
            .expect("matching publishers should remain query candidates");

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].node_name, "talker_a");
        assert_eq!(candidates[0].type_name, "std_msgs::String");
        assert_eq!(candidates[0].schema_hash, hash);
        assert_eq!(candidates[1].node_name, "talker_b");
        assert_eq!(candidates[1].type_name, "std_msgs::String");
        assert_eq!(candidates[1].schema_hash, hash);
    }

    #[test]
    fn visible_schema_services_filter_compatible_candidates_before_querying() {
        let hash = SchemaHash([1; 32]);
        let publishers = vec![
            publisher_with_hash_from_node("std_msgs::String", hash, "talker_without_service", 2),
            publisher_with_hash_from_node("std_msgs::String", hash, "talker_with_service", 3),
        ];
        let candidates = collect_topic_schema_candidates_from_publishers(&publishers, "/chatter")
            .expect("matching publishers should remain query candidates");
        let visible_services = vec![schema_service_for_node("talker_with_service", 3)];

        let visible_candidates =
            collect_visible_schema_service_candidates(&candidates, &visible_services)
                .expect("valid candidate service names");

        assert_eq!(visible_candidates.len(), 1);
        assert_eq!(visible_candidates[0].node_name, "talker_with_service");
        assert_eq!(
            TypeInfo::new(
                &visible_candidates[0].type_name,
                visible_candidates[0].schema_hash,
            ),
            TypeInfo::new("std_msgs::String", hash)
        );
    }

    #[test]
    fn schema_candidates_reject_missing_publishers_with_topic_error() {
        let error = collect_topic_schema_candidates_from_publishers(&[], "/missing")
            .expect_err("missing publishers should fail");

        assert!(matches!(
            &error,
            DynamicError::NoPublishers { topic } if topic == "/missing"
        ));
        assert_eq!(
            error.to_string(),
            "no publishers found for topic '/missing'"
        );
    }

    #[test]
    fn schema_candidates_reject_different_hashes_for_same_topic() {
        let publishers = vec![
            publisher_with_hash("std_msgs::String", SchemaHash([1; 32])),
            publisher_with_hash("std_msgs::String", SchemaHash([2; 32])),
        ];

        let error = collect_topic_schema_candidates_from_publishers(&publishers, "/chatter")
            .expect_err("different schema hashes should conflict");

        let DynamicError::TopicTypeConflict { topic, candidates } = error else {
            panic!("expected topic type conflict, got {error:?}");
        };

        assert_eq!(topic, "/chatter");
        assert_eq!(candidates.len(), 2);
        assert!(candidates.iter().any(|candidate| {
            candidate.contains("std_msgs::String")
                && candidate.contains(&SchemaHash([1; 32]).to_hash_string())
        }));
        assert!(candidates.iter().any(|candidate| {
            candidate.contains("std_msgs::String")
                && candidate.contains(&SchemaHash([2; 32]).to_hash_string())
        }));
    }

    #[test]
    fn schema_candidates_reject_different_type_names_for_same_hash() {
        let hash = SchemaHash([3; 32]);
        let publishers = vec![
            publisher_with_hash("std_msgs::String", hash),
            publisher_with_hash("custom_msgs::StringLike", hash),
        ];

        let error = collect_topic_schema_candidates_from_publishers(&publishers, "/chatter")
            .expect_err("different type names should conflict even with same hash");

        let DynamicError::TopicTypeConflict { topic, candidates } = error else {
            panic!("expected topic type conflict, got {error:?}");
        };

        assert_eq!(topic, "/chatter");
        assert_eq!(candidates.len(), 2);
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.contains("std_msgs::String"))
        );
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.contains("custom_msgs::StringLike"))
        );
    }
}
