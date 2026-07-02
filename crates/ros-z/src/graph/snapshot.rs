use std::time::SystemTime;

use serde::Serialize;

use crate::entity::EndpointKind;

use super::{Graph, GraphRevision};

/// A serializable point-in-time observation of the local native ros-z graph state.
#[derive(Debug, Clone, Serialize)]
pub struct GraphSnapshot {
    /// Revision of the local graph state used to produce this snapshot.
    ///
    /// This is a monotonic local progress token. It does not mean the distributed graph is complete
    /// or settled.
    pub revision: GraphRevision,
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
    /// Create a serializable point-in-time observation of the current local graph state.
    ///
    /// This captures topics, nodes, services, and the local revision used to produce the snapshot.
    /// The snapshot does not imply that the distributed graph is complete or settled.
    pub fn snapshot(&self) -> GraphSnapshot {
        let (revision, topics, nodes, services) = {
            let view = self.view();
            let revision = view.revision();
            let topics: Vec<TopicSnapshot> = view
                .topic_names_and_types()
                .into_iter()
                .map(|(name, type_name)| {
                    let publishers = view
                        .endpoints()
                        .filter(|endpoint| {
                            endpoint.kind == EndpointKind::Publisher && endpoint.topic == name
                        })
                        .count();
                    let subscribers = view
                        .endpoints()
                        .filter(|endpoint| {
                            endpoint.kind == EndpointKind::Subscription && endpoint.topic == name
                        })
                        .count();
                    TopicSnapshot {
                        name,
                        type_name,
                        publishers,
                        subscribers,
                    }
                })
                .collect();

            let nodes: Vec<NodeSnapshot> = view
                .node_names()
                .into_iter()
                .map(|(name, namespace)| NodeSnapshot { name, namespace })
                .collect();

            let services: Vec<ServiceSnapshot> = view
                .service_names_and_types()
                .into_iter()
                .map(|(name, type_name)| ServiceSnapshot { name, type_name })
                .collect();

            (revision, topics, nodes, services)
        };

        GraphSnapshot {
            revision,
            timestamp: SystemTime::now(),
            topics,
            nodes,
            services,
        }
    }
}
