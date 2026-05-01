use std::{collections::HashSet, path::Path, sync::Arc};

use color_eyre::eyre::{Context, Result, bail, eyre};
use ros_z::{
    dynamic::DiscoveredTopicSchema,
    entity::{Entity, EntityKind, SchemaHash, TypeInfo},
    node::Node,
};

use crate::{
    PreparedRecording, TopicPlan,
    api::{RecorderOptions, RecordingStartup, ResolvedPublisher, ResolvedTopic},
};

pub async fn build(node: Arc<Node>, mut options: RecorderOptions) -> Result<PreparedRecording> {
    options.topics = normalize_topics(options.topics);

    if options.topics.is_empty() {
        bail!("at least one topic must be requested for recording");
    }

    validate_output_path(&options.output)?;

    let topics = discover_topics(node.as_ref(), &options.topics, options.discovery_timeout).await?;
    let startup = RecordingStartup {
        output: options.output.clone(),
        requested_topics: options.topics.clone(),
        resolved_topics: topics.iter().map(|topic| topic.startup.clone()).collect(),
    };

    Ok(PreparedRecording {
        node,
        options,
        startup,
        topics,
    })
}

pub fn normalize_topics<I>(topics: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for topic in topics {
        if seen.insert(topic.clone()) {
            normalized.push(topic);
        }
    }

    normalized
}

fn validate_output_path(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    if !parent.exists() {
        bail!("output directory does not exist: {}", parent.display());
    }

    if path.exists() {
        bail!("output file already exists: {}", path.display());
    }

    Ok(())
}

async fn discover_topics(
    node: &Node,
    requested_topics: &[String],
    discovery_timeout: std::time::Duration,
) -> Result<Vec<TopicPlan>> {
    let mut topics = Vec::with_capacity(requested_topics.len());

    for requested_topic in requested_topics {
        let discovered = node
            .discover_topic_schema(requested_topic, discovery_timeout)
            .await
            .map_err(|error| eyre!(error.to_string()))
            .with_context(|| format!("failed to discover schema for topic {requested_topic}"))?;

        topics.push(TopicPlan {
            startup: resolve_topic(node, requested_topic, &discovered)?,
            schema: discovered.schema,
        });
    }

    Ok(topics)
}

fn resolve_topic(
    node: &Node,
    requested_topic: &str,
    discovered: &DiscoveredTopicSchema,
) -> Result<ResolvedTopic> {
    let bundle = ros_z::dynamic::schema_bridge::message_schema_to_bundle(&discovered.schema)
        .map_err(|error| {
            eyre!(
                "failed to build schema bundle for topic {}: {}",
                requested_topic,
                error
            )
        })?;
    let schema_json = ros_z_schema::to_json(&bundle).map_err(|error| {
        eyre!(
            "failed to serialize schema for topic {}: {}",
            requested_topic,
            error
        )
    })?;

    Ok(ResolvedTopic {
        requested_topic: requested_topic.to_string(),
        qualified_topic: discovered.qualified_topic.clone(),
        type_name: discovered.schema.type_name_str().to_string(),
        schema_hash: resolved_topic_schema_hash(discovered.schema_hash),
        schema_json,
        publishers: collect_publishers(node, discovered),
    })
}

fn resolved_topic_schema_hash(schema_hash: SchemaHash) -> String {
    schema_hash.to_hash_string()
}

fn collect_publishers(node: &Node, discovered: &DiscoveredTopicSchema) -> Vec<ResolvedPublisher> {
    node.graph()
        .get_entities_by_topic(EntityKind::Publisher, &discovered.qualified_topic)
        .into_iter()
        .filter_map(|entity| match entity.as_ref() {
            Entity::Endpoint(endpoint) => Some(ResolvedPublisher {
                node_fqn: endpoint.node.as_ref().map(node_fqn),
                schema_hash: publisher_schema_hash(endpoint.type_info.as_ref()),
                qos: endpoint.qos.encode(),
            }),
            Entity::Node(_) => None,
        })
        .collect()
}

fn publisher_schema_hash(type_info: Option<&TypeInfo>) -> Option<String> {
    type_info.and_then(|type_info| type_info.hash.as_ref().map(|hash| hash.to_hash_string()))
}

fn node_fqn(node: &ros_z::entity::NodeEntity) -> String {
    let namespace = if node.namespace.is_empty() {
        "/"
    } else {
        node.namespace.as_str()
    };

    if namespace == "/" {
        format!("/{}", node.name)
    } else {
        format!("{}/{}", namespace.trim_end_matches('/'), node.name)
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_topics;

    #[test]
    fn normalize_topics_preserves_order() {
        let normalized = normalize_topics(vec![
            "/foo".to_string(),
            "/bar".to_string(),
            "/foo".to_string(),
            "/baz".to_string(),
        ]);

        assert_eq!(normalized, vec!["/foo", "/bar", "/baz"]);
    }
}
