use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use itertools::Itertools;

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
    fn from_entity(entity: &Entity) -> Option<Self> {
        let Entity::Endpoint(endpoint) = entity else {
            return None;
        };

        Some(Self {
            node_name: endpoint.node.name.clone(),
            namespace: endpoint.node.namespace.clone(),
            type_name: endpoint.type_info.name.clone(),
            schema_hash: endpoint.type_info.hash,
        })
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
    publishers: &[Arc<Entity>],
    qualified_topic: &str,
) -> Result<Vec<TopicSchemaCandidate>, DynamicError> {
    let candidates = publishers
        .iter()
        .filter_map(|publisher| TopicSchemaCandidate::from_entity(publisher))
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
    let publishers = graph.get_entities_by_topic(EntityKind::Publisher, qualified_topic);

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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::entity::{EndpointEntity, EndpointKind, Entity, NodeEntity, SchemaHash, TypeInfo};

    fn publisher(type_name: &str) -> Arc<Entity> {
        Arc::new(Entity::Endpoint(EndpointEntity {
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
    fn publisher_schema_candidates_use_strict_endpoint_type_info() {
        let hash = SchemaHash([0x42; 32]);
        let publisher = Arc::new(Entity::Endpoint(EndpointEntity {
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
        }));

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
