use std::cmp::Ordering;
use std::time::Duration;
use std::{collections::BTreeSet, sync::Arc};

use crate::{
    dynamic::{DynamicError, Schema},
    entity::{Entity, EntityKind, SchemaHash, TypeInfo},
    graph::Graph,
    node::Node,
    topic_name::qualify_topic_name,
};

#[derive(Debug, Clone)]
pub struct DiscoveredTopicSchema {
    pub qualified_topic: String,
    pub root_name: String,
    pub schema: Schema,
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
    let mut saw_missing_schema_hash = false;
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
            saw_missing_schema_hash = true;
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
        return ensure_unambiguous_candidates(qualified_topic, candidates.into_iter().collect());
    }

    if saw_missing_node_identity {
        return Err(DynamicError::SchemaUnavailable {
            topic: qualified_topic.to_string(),
            reason: "publisher node identity is unavailable".to_string(),
        });
    }

    if saw_missing_type_info {
        return Err(DynamicError::SchemaUnavailable {
            topic: qualified_topic.to_string(),
            reason: "no publisher advertised type information".to_string(),
        });
    }

    if saw_missing_schema_hash {
        return Err(DynamicError::SchemaUnavailable {
            topic: qualified_topic.to_string(),
            reason: "no publisher advertised a schema hash".to_string(),
        });
    }

    Err(DynamicError::SchemaUnavailable {
        topic: qualified_topic.to_string(),
        reason: "no usable publishers found".to_string(),
    })
}

fn schema_identity(candidate: &TopicSchemaCandidate) -> (&str, SchemaHash) {
    (candidate.type_name.as_str(), candidate.schema_hash)
}

fn candidate_display(candidate: &TopicSchemaCandidate) -> String {
    format!(
        "{}/{}:{}@{}",
        candidate.namespace,
        candidate.node_name,
        candidate.type_name,
        candidate.schema_hash.to_hash_string()
    )
}

fn ensure_unambiguous_candidates(
    topic: &str,
    candidates: Vec<TopicSchemaCandidate>,
) -> Result<Vec<TopicSchemaCandidate>, DynamicError> {
    let Some(first) = candidates.first() else {
        return Err(DynamicError::SchemaUnavailable {
            topic: topic.to_string(),
            reason: "no publisher advertised type information and schema hash".to_string(),
        });
    };
    let first_identity = schema_identity(first);
    if candidates
        .iter()
        .any(|candidate| schema_identity(candidate) != first_identity)
    {
        return Err(DynamicError::SchemaConflict {
            topic: topic.to_string(),
            candidates: candidates.iter().map(candidate_display).collect(),
        });
    }

    Ok(candidates)
}

pub(crate) fn collect_topic_schema_candidates(
    graph: &Graph,
    qualified_topic: &str,
) -> Result<Vec<TopicSchemaCandidate>, DynamicError> {
    let publishers = graph.get_entities_by_topic(EntityKind::Publisher, qualified_topic);
    if publishers.is_empty() {
        return Err(DynamicError::SchemaUnavailable {
            topic: qualified_topic.to_string(),
            reason: "no active publishers found".to_string(),
        });
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

        let (root_name, schema, schema_hash) = self.try_schema_service(&candidates[..]).await?;

        Ok(DiscoveredTopicSchema {
            qualified_topic,
            root_name,
            schema,
            schema_hash,
        })
    }

    async fn try_schema_service(
        &self,
        candidates: &[TopicSchemaCandidate],
    ) -> Result<(String, Schema, SchemaHash), DynamicError> {
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

pub(crate) fn discovered_schema_type_info(discovered: &DiscoveredTopicSchema) -> TypeInfo {
    TypeInfo::with_hash(&discovered.root_name, discovered.schema_hash)
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

    fn publisher_with_hash(type_name: &str, hash: SchemaHash) -> Arc<Entity> {
        publisher_with_hash_from_node(type_name, hash, "talker", 2)
    }

    fn publisher_with_hash_from_node(
        type_name: &str,
        hash: SchemaHash,
        node_name: &str,
        node_id: usize,
    ) -> Arc<Entity> {
        Arc::new(Entity::Endpoint(EndpointEntity {
            id: 1,
            node: Some(NodeEntity {
                z_id: Default::default(),
                id: node_id,
                name: node_name.to_string(),
                namespace: "/".to_string(),
                enclave: String::new(),
            }),
            kind: EndpointKind::Publisher,
            topic: "/chatter".to_string(),
            type_info: Some(TypeInfo::with_hash(type_name, hash)),
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
        assert_eq!(candidates[0].type_name, "std_msgs::String");
        assert_eq!(candidates[0].schema_hash, hash);
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

        assert!(matches!(error, DynamicError::SchemaConflict { .. }));
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

        assert!(matches!(error, DynamicError::SchemaConflict { .. }));
    }
}
