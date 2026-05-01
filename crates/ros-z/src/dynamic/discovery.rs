use std::cmp::Ordering;
use std::time::Duration;
use std::{collections::BTreeSet, sync::Arc};

use crate::{
    dynamic::{DynamicError, MessageSchema},
    entity::{Entity, EntityKind, SchemaHash},
    graph::Graph,
    node::Node,
    topic_name::qualify_topic_name,
};

use super::type_info::schema_type_info_with_hash;

#[derive(Debug, Clone)]
pub struct DiscoveredTopicSchema {
    pub qualified_topic: String,
    pub schema: Arc<MessageSchema>,
    pub schema_hash: SchemaHash,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TopicSchemaCandidate {
    pub node_name: String,
    pub namespace: String,
    pub type_name: String,
    pub schema_hash: SchemaHash,
}

impl PartialOrd for TopicSchemaCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TopicSchemaCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        (
            &self.node_name,
            &self.namespace,
            &self.type_name,
            self.schema_hash.0,
        )
            .cmp(&(
                &other.node_name,
                &other.namespace,
                &other.type_name,
                other.schema_hash.0,
            ))
    }
}

pub(crate) fn collect_topic_schema_candidates_from_publishers(
    publishers: &[Arc<Entity>],
    qualified_topic: &str,
) -> Result<Vec<TopicSchemaCandidate>, DynamicError> {
    let mut saw_missing_node_identity = false;
    let mut saw_missing_type_info = false;
    let mut candidates = BTreeSet::new();

    for publisher in publishers {
        let Entity::Endpoint(endpoint) = &**publisher else {
            continue;
        };
        let Some(node) = endpoint.node.as_ref() else {
            saw_missing_node_identity = true;
            continue;
        };
        let Some(type_info) = endpoint.type_info.as_ref() else {
            saw_missing_type_info = true;
            continue;
        };
        let Some(schema_hash) = type_info.hash else {
            continue;
        };

        candidates.insert(TopicSchemaCandidate {
            node_name: node.name.clone(),
            namespace: node.namespace.clone(),
            type_name: type_info.name.clone(),
            schema_hash,
        });
    }

    if !candidates.is_empty() {
        return Ok(candidates.into_iter().collect());
    }

    if saw_missing_node_identity {
        return Err(DynamicError::MissingNodeIdentity {
            topic: qualified_topic.to_string(),
        });
    }

    if saw_missing_type_info {
        return Err(DynamicError::SchemaNotFound(format!(
            "No publishers with type information found for topic: {}",
            qualified_topic
        )));
    }

    Err(DynamicError::SchemaNotFound(format!(
        "No usable publishers found for topic: {}",
        qualified_topic
    )))
}

pub(crate) fn collect_topic_schema_candidates(
    graph: &Graph,
    qualified_topic: &str,
) -> Result<Vec<TopicSchemaCandidate>, DynamicError> {
    let publishers = graph.get_entities_by_topic(EntityKind::Publisher, qualified_topic);
    if publishers.is_empty() {
        return Err(DynamicError::SchemaNotFound(format!(
            "No publishers found for topic: {}",
            qualified_topic
        )));
    }

    collect_topic_schema_candidates_from_publishers(&publishers, qualified_topic)
}

pub(crate) struct SchemaDiscovery<'a> {
    node: &'a Node,
    timeout: Duration,
}

impl<'a> SchemaDiscovery<'a> {
    pub(crate) fn new(node: &'a Node, timeout: Duration) -> Self {
        Self { node, timeout }
    }

    pub(crate) async fn discover(
        &self,
        topic: &str,
    ) -> Result<DiscoveredTopicSchema, DynamicError> {
        let qualified_topic = qualify_topic_name(topic, self.node.namespace(), self.node.name())
            .map_err(|error| {
                DynamicError::SchemaNotFound(format!("Failed to qualify topic: {error}"))
            })?;
        let candidates =
            collect_topic_schema_candidates(self.node.graph().as_ref(), &qualified_topic)?;

        let (schema, schema_hash) = self.try_schema_service(&candidates[..]).await?;

        Ok(DiscoveredTopicSchema {
            qualified_topic,
            schema,
            schema_hash,
        })
    }

    async fn try_schema_service(
        &self,
        candidates: &[TopicSchemaCandidate],
    ) -> Result<(Arc<MessageSchema>, SchemaHash), DynamicError> {
        let mut last_error = None;

        for candidate in candidates {
            match super::schema_query::query_schema(self.node, candidate, self.timeout).await {
                Ok(result) => return Ok(result),
                Err(error) => last_error = Some(error),
            }
        }

        Err(last_error.unwrap_or_else(|| {
            DynamicError::SchemaNotFound("No schema service source succeeded".to_string())
        }))
    }
}

pub(crate) fn discovered_schema_type_info(
    discovered: &DiscoveredTopicSchema,
) -> crate::entity::TypeInfo {
    schema_type_info_with_hash(&discovered.schema, &discovered.schema_hash)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::entity::{EndpointEntity, EndpointKind, Entity, NodeEntity, SchemaHash, TypeInfo};

    fn publisher(type_name: &str) -> Arc<Entity> {
        Arc::new(Entity::Endpoint(EndpointEntity {
            id: 1,
            node: Some(NodeEntity {
                domain_id: 0,
                z_id: Default::default(),
                id: 2,
                name: "talker".to_string(),
                namespace: "/".to_string(),
                enclave: String::new(),
            }),
            kind: EndpointKind::Publisher,
            topic: "/chatter".to_string(),
            type_info: Some(TypeInfo::with_hash(type_name, SchemaHash::zero())),
            qos: Default::default(),
        }))
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
}
