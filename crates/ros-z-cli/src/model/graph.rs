use ros_z::graph::GraphRevision;
use serde::Serialize;

use crate::support::nodes::fully_qualified_node_name;

#[derive(Debug, Clone, Serialize)]
pub struct GraphSummary {
    pub revision: GraphRevision,
    pub topics: Vec<TopicSummary>,
    pub nodes: Vec<NodeSummary>,
    pub services: Vec<ServiceSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct TopicSummary {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub publishers: usize,
    pub subscribers: usize,
}

impl TopicSummary {
    pub fn new(name: String, type_name: String, publishers: usize, subscribers: usize) -> Self {
        Self {
            name,
            type_name,
            publishers,
            subscribers,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct NodeSummary {
    pub name: String,
    pub namespace: String,
    pub fqn: String,
}

impl NodeSummary {
    pub fn new(name: String, namespace: String) -> Self {
        let fqn = fully_qualified_node_name(&namespace, &name);
        Self {
            name,
            namespace,
            fqn,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ServiceSummary {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub servers: usize,
    pub clients: usize,
}

impl ServiceSummary {
    pub fn new(name: String, type_name: String, servers: usize, clients: usize) -> Self {
        Self {
            name,
            type_name,
            servers,
            clients,
        }
    }
}
