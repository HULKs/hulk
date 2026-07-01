use std::sync::Arc;
use std::time::Duration;

use ros_z::dynamic::{DiscoveredTopicSchema, Schema};
use ros_z::entity::TypeInfo;
use ros_z::node::Node;

use crate::{RecordingError, Result};

const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub(crate) struct ResolvedTopic {
    requested_topic: String,
    topic: String,
    type_info: TypeInfo,
    schema_hash: String,
    schema: Schema,
}

impl ResolvedTopic {
    pub(crate) fn from_discovery(
        requested_topic: String,
        discovered: DiscoveredTopicSchema,
    ) -> Self {
        let type_info = discovered.type_info();
        Self {
            requested_topic,
            topic: discovered.qualified_topic,
            schema_hash: discovered.schema_hash.to_hash_string(),
            type_info,
            schema: discovered.schema,
        }
    }

    pub(crate) fn requested_topic(&self) -> &str {
        &self.requested_topic
    }

    pub(crate) fn topic(&self) -> &str {
        &self.topic
    }

    pub(crate) fn type_info(&self) -> &TypeInfo {
        &self.type_info
    }

    pub(crate) fn type_name(&self) -> &str {
        &self.type_info.name
    }

    pub(crate) fn schema_hash(&self) -> &str {
        &self.schema_hash
    }

    pub(crate) fn schema(&self) -> &Schema {
        &self.schema
    }
}

pub(crate) async fn resolve_topics(
    node: Arc<Node>,
    topics: &[String],
) -> Result<Vec<ResolvedTopic>> {
    let mut resolved = Vec::with_capacity(topics.len());
    for topic in topics {
        let discovered = node
            .discover_topic_schema(topic, DISCOVERY_TIMEOUT)
            .await
            .map_err(|source| RecordingError::SchemaDiscovery {
                topic: topic.clone(),
                source,
            })?;
        resolved.push(ResolvedTopic::from_discovery(topic.clone(), discovered));
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ros_z::Message;

    use super::ResolvedTopic;

    #[test]
    fn resolved_topic_preserves_requested_and_qualified_topic() {
        let discovered = ros_z::dynamic::DiscoveredTopicSchema {
            qualified_topic: "/tools/chatter".to_string(),
            root_name: String::type_name(),
            schema: Arc::new(String::schema()),
            schema_hash: String::schema_hash(),
        };

        let topic = ResolvedTopic::from_discovery("chatter".to_string(), discovered);

        assert_eq!(topic.requested_topic(), "chatter");
        assert_eq!(topic.topic(), "/tools/chatter");
        assert_eq!(topic.type_name(), String::type_name());
        assert_eq!(topic.schema_hash(), String::schema_hash().to_hash_string());
    }
}
