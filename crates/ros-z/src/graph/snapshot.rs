use std::time::SystemTime;

use serde::Serialize;

use crate::entity::EntityKind;

use super::Graph;

/// A serializable snapshot of the native ros-z graph state
#[derive(Debug, Clone, Serialize)]
pub struct GraphSnapshot {
    pub timestamp: SystemTime,
    pub topics: Vec<TopicSnapshot>,
    pub nodes: Vec<NodeSnapshot>,
    pub services: Vec<ServiceSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopicSnapshot {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub publishers: usize,
    pub subscribers: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeSnapshot {
    pub name: String,
    pub namespace: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceSnapshot {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
}

impl Graph {
    /// Create a serializable snapshot of the current graph state
    ///
    /// This captures topics, nodes, and services with their metadata,
    /// suitable for JSON serialization or other export formats.
    pub fn snapshot(&self) -> GraphSnapshot {
        let topics: Vec<TopicSnapshot> = self
            .get_topic_names_and_types()
            .into_iter()
            .map(|(name, type_name)| {
                let publishers = self
                    .get_entities_by_topic(EntityKind::Publisher, &name)
                    .len();
                let subscribers = self
                    .get_entities_by_topic(EntityKind::Subscription, &name)
                    .len();
                TopicSnapshot {
                    name,
                    type_name,
                    publishers,
                    subscribers,
                }
            })
            .collect();

        let nodes: Vec<NodeSnapshot> = self
            .get_node_names()
            .into_iter()
            .map(|(name, namespace)| NodeSnapshot { name, namespace })
            .collect();

        let services: Vec<ServiceSnapshot> = self
            .get_service_names_and_types()
            .into_iter()
            .map(|(name, type_name)| ServiceSnapshot { name, type_name })
            .collect();

        GraphSnapshot {
            timestamp: SystemTime::now(),
            topics,
            nodes,
            services,
        }
    }
}
