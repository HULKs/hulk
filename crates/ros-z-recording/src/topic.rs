use std::sync::Arc;
use std::time::Duration;

use ros_z::dynamic::{GetSchema, GetSchemaRequest, Schema, schema_from_response_with_hash};
use ros_z::entity::{EndpointEntity, TypeInfo};
use ros_z::graph::Graph;
use ros_z::node::Node;
use serde::Serialize;

use crate::{RecordingError, Result};

const SCHEMA_QUERY_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct ResolvedTopic {
    topic: String,
    type_info: TypeInfo,
    schema_hash: String,
    schema: Schema,
    publishers: Vec<PublisherInfo>,
}

impl ResolvedTopic {
    pub(crate) fn new(
        topic: String,
        type_info: TypeInfo,
        schema: Schema,
        publishers: Vec<PublisherInfo>,
    ) -> Result<Self> {
        if publishers.is_empty() {
            return Err(RecordingError::TopicWithoutPublishers { topic });
        }

        Ok(Self {
            topic,
            schema_hash: type_info.hash.to_hash_string(),
            type_info,
            schema,
            publishers,
        })
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }

    pub fn type_name(&self) -> &str {
        &self.type_info.name
    }

    pub fn schema_hash(&self) -> &str {
        &self.schema_hash
    }

    pub fn type_info(&self) -> TypeInfo {
        self.type_info.clone()
    }

    pub(crate) fn schema(&self) -> &Schema {
        &self.schema
    }

    pub(crate) fn publishers(&self) -> &[PublisherInfo] {
        &self.publishers
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PublisherInfo {
    pub node: String,
    pub schema_hash: String,
    pub endpoint_id: String,
}

#[derive(Debug, Clone)]
pub(crate) struct TopicPublisherResolution {
    pub topic: String,
    pub type_info: TypeInfo,
    pub publishers: Vec<PublisherInfo>,
    pub schema_candidates: Vec<EndpointEntity>,
}

#[cfg(test)]
impl TopicPublisherResolution {
    fn topic(&self) -> &str {
        &self.topic
    }

    fn type_info(&self) -> &TypeInfo {
        &self.type_info
    }

    fn publisher_count(&self) -> usize {
        self.publishers.len()
    }

    fn publisher_node(&self, index: usize) -> Option<&str> {
        self.publishers
            .get(index)
            .map(|publisher| publisher.node.as_str())
    }

    fn publisher_endpoint_id(&self, index: usize) -> Option<&str> {
        self.publishers
            .get(index)
            .map(|publisher| publisher.endpoint_id.as_str())
    }
}

pub async fn resolve_topics(
    node: Arc<Node>,
    graph: &Graph,
    topics: &[String],
) -> Result<Vec<ResolvedTopic>> {
    let mut resolved = Vec::with_capacity(topics.len());

    for topic in topics {
        let publishers = graph.view().publishers_on(topic);
        let publisher_resolution = resolve_topic_from_publishers(topic, publishers)?;
        let schema = query_schema(Arc::clone(&node), &publisher_resolution).await?;
        resolved.push(ResolvedTopic::new(
            publisher_resolution.topic,
            publisher_resolution.type_info,
            schema,
            publisher_resolution.publishers,
        )?);
    }

    Ok(resolved)
}

pub(crate) fn resolve_topic_from_publishers(
    topic: &str,
    publishers: Vec<EndpointEntity>,
) -> Result<TopicPublisherResolution> {
    let Some(first) = publishers.first().cloned() else {
        return Err(RecordingError::TopicWithoutPublishers {
            topic: topic.to_string(),
        });
    };
    let type_info = first.type_info.clone();
    let mut conflicts = Vec::new();
    let mut publisher_infos = Vec::with_capacity(publishers.len());

    for publisher in &publishers {
        if publisher.type_info.name != type_info.name || publisher.type_info.hash != type_info.hash
        {
            conflicts.push(format!(
                "{} has {} [{}]",
                publisher.node.fully_qualified_name(),
                publisher.type_info.name,
                publisher.type_info.hash.to_hash_string()
            ));
        }
        publisher_infos.push(PublisherInfo {
            node: publisher.node.fully_qualified_name(),
            schema_hash: publisher.type_info.hash.to_hash_string(),
            endpoint_id: endpoint_id_hex(publisher),
        });
    }

    if !conflicts.is_empty() {
        return Err(RecordingError::ConflictingTopicTypes {
            topic: topic.to_string(),
            details: conflicts,
        });
    }

    Ok(TopicPublisherResolution {
        topic: topic.to_string(),
        type_info,
        publishers: publisher_infos,
        schema_candidates: publishers,
    })
}

async fn query_schema(node: Arc<Node>, resolved: &TopicPublisherResolution) -> Result<Schema> {
    let mut last_error = None;
    for publisher in &resolved.schema_candidates {
        match query_schema_from_publisher(&node, resolved, publisher).await {
            Ok(schema) => return Ok(schema),
            Err(error) => last_error = Some(error),
        }
    }

    Err(
        last_error.unwrap_or_else(|| RecordingError::TopicWithoutPublishers {
            topic: resolved.topic.clone(),
        }),
    )
}

async fn query_schema_from_publisher(
    node: &Arc<Node>,
    resolved: &TopicPublisherResolution,
    publisher: &EndpointEntity,
) -> Result<Schema> {
    let service = format!("{}/get_schema", publisher.node.fully_qualified_name());
    let client = node
        .service_client::<GetSchema>(&service)
        .build()
        .await
        .map_err(|source| RecordingError::SchemaClient {
            service: service.clone(),
            source,
        })?;
    let response = client
        .call_with_timeout_async(
            &GetSchemaRequest {
                root_type_name: resolved.type_info.name.clone(),
                schema_hash: resolved.type_info.hash.to_hash_string(),
            },
            SCHEMA_QUERY_TIMEOUT,
        )
        .await
        .map_err(|source| RecordingError::SchemaCall {
            service: service.clone(),
            source,
        })?;

    if !response.successful {
        return Err(RecordingError::SchemaRejected {
            service,
            reason: response.failure_reason,
        });
    }

    schema_from_response_with_hash(&response, resolved.type_info.hash).map_err(|source| {
        RecordingError::SchemaResponse {
            topic: resolved.topic.clone(),
            source,
        }
    })
}

fn endpoint_id_hex(endpoint: &EndpointEntity) -> String {
    hex::encode(ros_z::entity::EndpointGlobalId::from(endpoint).as_bytes())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use ros_z::Message;
    use ros_z::context::ContextBuilder;
    use ros_z::entity::{
        EndpointEntity, EndpointGlobalId, EndpointKind, NodeEntity, SchemaHash, TypeInfo,
    };

    use super::{ResolvedTopic, query_schema, resolve_topic_from_publishers};
    use crate::RecordingError;

    fn publisher(topic: &str, type_name: &str, hash_byte: u8, node_id: usize) -> EndpointEntity {
        EndpointEntity {
            id: node_id,
            node: NodeEntity {
                z_id: Default::default(),
                id: node_id,
                name: format!("publisher_{node_id}"),
                namespace: "/test".to_string(),
            },
            kind: EndpointKind::Publisher,
            topic: topic.to_string(),
            type_info: TypeInfo::new(type_name, SchemaHash([hash_byte; 32])),
            qos: Default::default(),
        }
    }

    #[test]
    fn rejects_topic_without_publishers() {
        let error = resolve_topic_from_publishers("/missing", Vec::new())
            .expect_err("missing publisher must fail");

        assert!(
            matches!(error, RecordingError::TopicWithoutPublishers { topic } if topic == "/missing")
        );
    }

    #[test]
    fn rejects_publishers_with_different_type_names() {
        let error = resolve_topic_from_publishers(
            "/demo",
            vec![
                publisher("/demo", "test_msgs::Alpha", 1, 1),
                publisher("/demo", "test_msgs::Beta", 1, 2),
            ],
        )
        .expect_err("mixed type names must fail");

        assert!(
            matches!(error, RecordingError::ConflictingTopicTypes { topic, .. } if topic == "/demo")
        );
    }

    #[test]
    fn rejects_publishers_with_different_schema_hashes() {
        let error = resolve_topic_from_publishers(
            "/demo",
            vec![
                publisher("/demo", "test_msgs::Alpha", 1, 1),
                publisher("/demo", "test_msgs::Alpha", 2, 2),
            ],
        )
        .expect_err("mixed schema hashes must fail");

        assert!(
            matches!(error, RecordingError::ConflictingTopicTypes { topic, .. } if topic == "/demo")
        );
    }

    #[test]
    fn resolves_publishers_that_share_type_and_hash() {
        let resolved = resolve_topic_from_publishers(
            "/demo",
            vec![
                publisher("/demo", "test_msgs::Alpha", 1, 1),
                publisher("/demo", "test_msgs::Alpha", 1, 2),
            ],
        )
        .expect("matching publishers resolve");

        assert_eq!(resolved.topic(), "/demo");
        assert_eq!(resolved.type_info().name, "test_msgs::Alpha");
        assert_eq!(resolved.publisher_count(), 2);
        assert_eq!(resolved.publisher_node(0), Some("/test/publisher_1"));
        assert_eq!(
            resolved
                .publisher_endpoint_id(0)
                .expect("publisher id")
                .len(),
            EndpointGlobalId::from([1; 16]).as_bytes().len() * 2
        );
    }

    #[test]
    fn resolved_topic_rejects_empty_publisher_metadata() {
        let error = ResolvedTopic::new(
            "/demo".to_string(),
            TypeInfo::new(String::type_name(), String::schema_hash()),
            Arc::new(String::schema()),
            Vec::new(),
        )
        .expect_err("resolved topics must keep at least one publisher");

        assert!(matches!(
            error,
            RecordingError::TopicWithoutPublishers { topic } if topic == "/demo"
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn query_schema_tries_later_matching_publishers_after_first_service_fails() {
        let context = ContextBuilder::default()
            .build()
            .await
            .expect("context builds");
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let bad_node = context
            .create_node(format!("schema_fallback_bad_{unique}"))
            .with_namespace("/recording_tests")
            .without_schema_service()
            .build()
            .await
            .expect("schema-less node builds");
        let good_node = context
            .create_node(format!("schema_fallback_good_{unique}"))
            .with_namespace("/recording_tests")
            .build()
            .await
            .expect("schema node builds");
        let topic = format!("/recording_tests/schema_fallback_{unique}");
        let bad_publisher = bad_node
            .publisher::<String>(&topic)
            .build()
            .await
            .expect("bad publisher builds");
        let good_publisher = good_node
            .publisher::<String>(&topic)
            .build()
            .await
            .expect("good publisher builds");

        tokio::time::sleep(Duration::from_millis(100)).await;

        let resolved = resolve_topic_from_publishers(
            &topic,
            vec![
                bad_publisher.entity().clone(),
                good_publisher.entity().clone(),
            ],
        )
        .expect("matching publishers resolve");
        let schema = query_schema(Arc::new(good_node), &resolved)
            .await
            .expect("later publisher schema should resolve");

        assert_eq!(schema.root, String::schema().root);
    }
}
