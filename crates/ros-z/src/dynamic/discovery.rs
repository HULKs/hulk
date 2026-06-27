use std::fmt;
use std::time::Duration;

use itertools::Itertools;
use tokio::time::Instant;

use crate::{
    dynamic::{DynamicCdrCodec, DynamicError, DynamicPayload, DynamicSubscriber, Schema},
    endpoint_builder::{EndpointBuilderContext, MessageEndpointType},
    entity::{EndpointEntity, SchemaHash, TypeInfo},
    graph::Graph,
    pubsub::{RawSubscriber, SubscriberBuilder, SubscriberOptions},
    qos::QosProfile,
    topic_name::qualify_topic_name,
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
pub(crate) struct TopicSchemaCandidate {
    pub node_name: String,
    pub namespace: String,
    pub type_name: String,
    pub schema_hash: SchemaHash,
}

impl TopicSchemaCandidate {
    fn from_endpoint(endpoint: &EndpointEntity) -> Self {
        Self {
            node_name: endpoint.node.name.clone(),
            namespace: endpoint.node.namespace.clone(),
            type_name: endpoint.type_info.name.clone(),
            schema_hash: endpoint.type_info.hash,
        }
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
    let candidates = publishers
        .iter()
        .map(TopicSchemaCandidate::from_endpoint)
        .unique()
        .collect_vec();

    if candidates.is_empty() {
        return Err(DynamicError::SchemaNotFound(format!(
            "No publishers found for topic: {qualified_topic}"
        )));
    }

    if !candidates
        .iter()
        .map(|candidate| (&candidate.type_name, candidate.schema_hash))
        .all_equal()
    {
        return Err(DynamicError::SchemaConflict {
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
    let publishers = graph.view().publishers_on(qualified_topic);

    collect_topic_schema_candidates_from_publishers(&publishers, qualified_topic)
}

fn schema_query_timeout(deadline: Instant) -> Option<Duration> {
    let timeout = deadline.saturating_duration_since(Instant::now());
    if timeout.is_zero() {
        return None;
    }
    Some(timeout)
}

fn schema_query_timeout_error(
    qualified_topic: &str,
    last_error: Option<&DynamicError>,
) -> DynamicError {
    let mut message = format!("Timed out while discovering schema for topic: {qualified_topic}");
    if let Some(error) = last_error {
        message.push_str(&format!("; last candidate error: {error}"));
    }
    DynamicError::SchemaNotFound(message)
}

pub(crate) struct SchemaDiscovery {
    context: EndpointBuilderContext,
    timeout: Duration,
}

impl SchemaDiscovery {
    pub(crate) fn new(context: EndpointBuilderContext, timeout: Duration) -> Self {
        Self { context, timeout }
    }

    pub(crate) async fn discover(
        &self,
        topic: &str,
    ) -> Result<DiscoveredTopicSchema, DynamicError> {
        let qualified_topic =
            qualify_topic_name(topic, &self.context.node.namespace, &self.context.node.name)
                .map_err(|error| DynamicError::name("discovering topic schema", error))?;

        self.discover_qualified(qualified_topic).await
    }

    async fn discover_qualified(
        &self,
        qualified_topic: String,
    ) -> Result<DiscoveredTopicSchema, DynamicError> {
        let deadline = Instant::now() + self.timeout;
        let candidates = self
            .wait_for_topic_schema_candidates(&qualified_topic, deadline)
            .await?;

        let (root_name, schema, schema_hash) = self
            .try_schema_service(&qualified_topic, &candidates[..], deadline)
            .await?;

        Ok(DiscoveredTopicSchema {
            qualified_topic,
            root_name,
            schema,
            schema_hash,
        })
    }

    async fn wait_for_topic_schema_candidates(
        &self,
        qualified_topic: &str,
        deadline: Instant,
    ) -> Result<Vec<TopicSchemaCandidate>, DynamicError> {
        let graph = &self.context.graph;
        if tokio::time::timeout_at(
            deadline,
            graph.wait_until(|view| view.has_publishers_on(qualified_topic)),
        )
        .await
        .is_err()
        {
            return Err(schema_query_timeout_error(qualified_topic, None));
        }

        collect_topic_schema_candidates(graph.as_ref(), qualified_topic)
    }

    async fn try_schema_service(
        &self,
        qualified_topic: &str,
        candidates: &[TopicSchemaCandidate],
        deadline: Instant,
    ) -> Result<(String, Schema, SchemaHash), DynamicError> {
        let mut last_error = None;

        for candidate in candidates {
            let Some(timeout) = schema_query_timeout(deadline) else {
                return Err(schema_query_timeout_error(
                    qualified_topic,
                    last_error.as_ref(),
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
                    return Err(schema_query_timeout_error(
                        qualified_topic,
                        last_error.as_ref(),
                    ));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            DynamicError::SchemaNotFound("No schema service source succeeded".to_string())
        }))
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

/// Builder for raw dynamic subscribers that discover schema metadata at build time.
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
    /// Schema discovery still runs at build time so the subscriber advertises
    /// the discovered dynamic type metadata, but received samples are returned
    /// without deserialization.
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

    /// Discover the topic schema and build a raw dynamic subscriber.
    pub async fn build(self) -> crate::Result<RawSubscriber> {
        let Self {
            context,
            topic,
            discovery_timeout,
            options,
        } = self;

        discover_dynamic_subscriber_builder(context, topic, discovery_timeout, options)
            .await?
            .raw()
            .build()
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::context::ContextBuilder;
    use crate::entity::{EndpointKind, NodeEntity, SchemaHash, TypeInfo};

    fn unique_node_name(prefix: &str) -> String {
        format!(
            "{prefix}_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after Unix epoch")
                .as_nanos()
        )
    }

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

    #[test]
    fn schema_query_timeout_error_reports_timeout_without_candidate_error() {
        let error = schema_query_timeout_error("/chatter", None);

        let DynamicError::SchemaNotFound(message) = error else {
            panic!("expected schema not found timeout, got {error:?}");
        };
        assert!(message.contains("Timed out"));
        assert!(message.contains("/chatter"));
    }

    #[test]
    fn schema_query_timeout_error_keeps_candidate_error_as_context() {
        let error = schema_query_timeout_error(
            "/chatter",
            Some(&DynamicError::SerializationError(
                "candidate failed".to_string(),
            )),
        );

        let DynamicError::SchemaNotFound(message) = error else {
            panic!("expected timeout schema not found, got {error:?}");
        };
        assert!(message.contains("Timed out"));
        assert!(message.contains("/chatter"));
        assert!(message.contains("candidate failed"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn schema_candidate_wait_reports_timeout_when_no_publishers_arrive() {
        let context = ContextBuilder::default()
            .build()
            .await
            .expect("test context should build");
        let node = context
            .create_node(unique_node_name("schema_candidate_timeout"))
            .build()
            .await
            .expect("test node should build");
        let error = node
            .discover_topic_schema("/missing_schema_timeout", Duration::from_millis(1))
            .await
            .expect_err("elapsed publisher wait should report timeout");

        let DynamicError::SchemaNotFound(message) = error else {
            panic!("expected timeout schema not found, got {error:?}");
        };
        assert!(message.contains("Timed out"));
        assert!(message.contains("/missing_schema_timeout"));
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
    fn schema_candidates_reject_different_hashes_for_same_topic() {
        let publishers = vec![
            publisher_with_hash("std_msgs::String", SchemaHash([1; 32])),
            publisher_with_hash("std_msgs::String", SchemaHash([2; 32])),
        ];

        let error = collect_topic_schema_candidates_from_publishers(&publishers, "/chatter")
            .expect_err("different schema hashes should conflict");

        let DynamicError::SchemaConflict { topic, candidates } = error else {
            panic!("expected schema conflict, got {error:?}");
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

        let DynamicError::SchemaConflict { topic, candidates } = error else {
            panic!("expected schema conflict, got {error:?}");
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
