use std::collections::{BTreeMap, BTreeSet};

use color_eyre::Result;
use ros_z::{entity::EndpointKind, graph::GraphView};
use ros_z_debug::{ProjectedTopicScope, TopicProjection};

use super::topic::normalize_namespace;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopicEntry {
    pub selector: String,
    pub resolved_topic: String,
    pub type_name: String,
    pub publishers: usize,
    pub subscribers: usize,
    pub in_target_namespace: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TopicCatalog {
    topics: Vec<TopicEntry>,
}

impl TopicCatalog {
    pub fn new(topics: Vec<TopicEntry>) -> Self {
        Self { topics }
    }

    #[cfg(test)]
    pub fn topics(&self) -> &[TopicEntry] {
        &self.topics
    }

    pub fn namespace_topics(&self) -> impl Iterator<Item = &TopicEntry> {
        self.topics.iter().filter(|topic| topic.in_target_namespace)
    }

    pub fn all_topics(&self) -> impl Iterator<Item = &TopicEntry> {
        self.topics.iter()
    }
}

pub fn build_topic_catalog(target_namespace: &str, view: &GraphView<'_>) -> Result<TopicCatalog> {
    let mut entries = BTreeMap::<String, (BTreeSet<String>, usize, usize)>::new();

    for endpoint in view.endpoints() {
        if !matches!(
            endpoint.kind,
            EndpointKind::Publisher | EndpointKind::Subscription
        ) {
            continue;
        }

        let entry = entries
            .entry(endpoint.topic.clone())
            .or_insert_with(|| (BTreeSet::new(), 0, 0));
        entry.0.insert(endpoint.type_info.name.clone());
        match endpoint.kind {
            EndpointKind::Publisher => entry.1 += 1,
            EndpointKind::Subscription => entry.2 += 1,
            EndpointKind::Service | EndpointKind::Client => {}
        }
    }

    build_topic_catalog_from_entries(
        target_namespace,
        entries
            .into_iter()
            .map(|(topic, (type_names, publishers, subscribers))| {
                (
                    topic,
                    display_type_name(type_names),
                    publishers,
                    subscribers,
                )
            }),
    )
}

fn display_type_name(type_names: BTreeSet<String>) -> String {
    if type_names.len() == 1 {
        return type_names.into_iter().next().unwrap_or_default();
    }

    format!(
        "conflicting types: {}",
        type_names.into_iter().collect::<Vec<_>>().join(", ")
    )
}

pub fn build_topic_catalog_from_entries(
    target_namespace: &str,
    entries: impl IntoIterator<Item = (String, String, usize, usize)>,
) -> Result<TopicCatalog> {
    let target_namespace = normalize_namespace(target_namespace)?;

    let mut topics = entries
        .into_iter()
        .map(|(resolved_topic, type_name, publishers, subscribers)| {
            let projected = TopicProjection::project(&target_namespace, [&resolved_topic])?
                .into_iter()
                .next()
                .expect("projection should return one topic for one input");
            let in_target_namespace = matches!(
                projected.scope,
                ProjectedTopicScope::RelativeToActiveNamespace
            );
            Ok(TopicEntry {
                selector: projected.display_name,
                resolved_topic: projected.resolved_topic,
                type_name,
                publishers,
                subscribers,
                in_target_namespace,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    topics.sort_by(|left, right| left.selector.cmp(&right.selector));
    Ok(TopicCatalog::new(topics))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use ros_z::{
        context::ContextBuilder,
        entity::{EndpointEntity, EndpointKind, Entity, NodeEntity, SchemaHash, TypeInfo},
    };

    use super::*;

    static NEXT_TEST_ID: AtomicUsize = AtomicUsize::new(1);

    fn unique_node_name(prefix: &str) -> String {
        let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
        format!("{prefix}_{id}")
    }

    fn endpoint(
        node: &NodeEntity,
        id: usize,
        kind: EndpointKind,
        topic: &str,
        type_name: &str,
    ) -> EndpointEntity {
        EndpointEntity {
            id,
            node: node.clone(),
            kind,
            topic: topic.to_string(),
            type_info: TypeInfo::new(type_name, SchemaHash::zero()),
            qos: Default::default(),
        }
    }

    #[test]
    fn catalog_displays_namespace_topics_as_relative_selectors() {
        let catalog = build_topic_catalog_from_entries(
            "/42",
            [
                (
                    "/42/ground_to_field".to_string(),
                    "geometry::Isometry2".to_string(),
                    1,
                    0,
                ),
                (
                    "/diagnostics".to_string(),
                    "std_msgs::String".to_string(),
                    1,
                    0,
                ),
            ],
        )
        .unwrap();

        assert_eq!(catalog.topics()[0].selector, "/diagnostics");
        assert_eq!(catalog.topics()[1].selector, "ground_to_field");
        assert_eq!(catalog.topics()[1].resolved_topic, "/42/ground_to_field");
        assert!(catalog.topics()[1].in_target_namespace);
        assert!(!catalog.topics()[0].in_target_namespace);
    }

    #[test]
    fn catalog_sorts_by_selector() {
        let catalog = build_topic_catalog_from_entries(
            "/42",
            [
                ("/42/zeta".to_string(), "Z".to_string(), 1, 0),
                ("/42/alpha".to_string(), "A".to_string(), 1, 0),
            ],
        )
        .unwrap();

        let selectors = catalog
            .topics()
            .iter()
            .map(|topic| topic.selector.as_str())
            .collect::<Vec<_>>();
        assert_eq!(selectors, ["alpha", "zeta"]);
    }

    #[test]
    fn catalog_rebuild_reflects_changed_entries() {
        let initial_catalog = build_topic_catalog_from_entries(
            "/42",
            [("/42/ground_to_field".to_string(), "Pose".to_string(), 1, 0)],
        )
        .unwrap();
        let rebuilt_catalog = build_topic_catalog_from_entries(
            "/42",
            [
                ("/42/ground_to_field".to_string(), "Pose".to_string(), 1, 0),
                ("/42/ball_position".to_string(), "Point".to_string(), 1, 0),
            ],
        )
        .unwrap();

        assert_eq!(
            initial_catalog.topics().len() + 1,
            rebuilt_catalog.topics().len()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn graph_catalog_aggregates_topic_endpoints_and_ignores_service_endpoints() -> Result<()>
    {
        let context = ContextBuilder::default()
            .disable_multicast_scouting()
            .without_graph_initial_query()
            .build()
            .await?;
        let node = context
            .create_node(unique_node_name("twix_catalog_graph"))
            .without_schema_service()
            .build()
            .await?;
        let graph = node.graph();
        let node_entity = node.node_entity();

        graph.add_local_entity(Entity::Endpoint(endpoint(
            node_entity,
            1,
            EndpointKind::Publisher,
            "/42/ball_position",
            "Point",
        )))?;
        graph.add_local_entity(Entity::Endpoint(endpoint(
            node_entity,
            2,
            EndpointKind::Subscription,
            "/42/ball_position",
            "Point",
        )))?;
        graph.add_local_entity(Entity::Endpoint(endpoint(
            node_entity,
            3,
            EndpointKind::Service,
            "/42/reset",
            "Reset",
        )))?;
        graph.add_local_entity(Entity::Endpoint(endpoint(
            node_entity,
            4,
            EndpointKind::Client,
            "/42/reset",
            "Reset",
        )))?;

        let catalog = build_topic_catalog("/42", &graph.view())?;

        assert_eq!(catalog.topics().len(), 1);
        let topic = &catalog.topics()[0];
        assert_eq!(topic.selector, "ball_position");
        assert_eq!(topic.resolved_topic, "/42/ball_position");
        assert_eq!(topic.type_name, "Point");
        assert_eq!(topic.publishers, 1);
        assert_eq!(topic.subscribers, 1);
        assert!(topic.in_target_namespace);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn graph_catalog_marks_conflicting_topic_types_deterministically() -> Result<()> {
        let context = ContextBuilder::default()
            .disable_multicast_scouting()
            .without_graph_initial_query()
            .build()
            .await?;
        let node = context
            .create_node(unique_node_name("twix_catalog_conflict"))
            .without_schema_service()
            .build()
            .await?;
        let graph = node.graph();
        let node_entity = node.node_entity();

        graph.add_local_entity(Entity::Endpoint(endpoint(
            node_entity,
            1,
            EndpointKind::Publisher,
            "/42/robot_pose",
            "Twist",
        )))?;
        graph.add_local_entity(Entity::Endpoint(endpoint(
            node_entity,
            2,
            EndpointKind::Subscription,
            "/42/robot_pose",
            "Pose",
        )))?;

        let catalog = build_topic_catalog("/42", &graph.view())?;

        assert_eq!(
            catalog.topics()[0].type_name,
            "conflicting types: Pose, Twist"
        );

        Ok(())
    }
}
