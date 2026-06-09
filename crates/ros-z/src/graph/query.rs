use std::collections::BTreeMap;

use parking_lot::MutexGuard;

use crate::entity::{EndpointEntity, EndpointKind, Entity, NodeEntity, NodeKey};
use crate::qos::{QosCompatibility, QosProfile};

use super::{Graph, state::GraphData};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QosIncompatibility {
    pub topic: String,
    pub publisher: EndpointEntity,
    pub subscription: EndpointEntity,
    pub compatibility: QosCompatibility,
}

/// A snapshot view of graph entities while the graph lock is held.
///
/// Do not hold a `GraphView` across `.await` points or while calling other `Graph` methods.
pub struct GraphView<'a> {
    data: MutexGuard<'a, GraphData>,
}

impl Graph {
    /// Returns a view of the graph while holding the graph lock.
    ///
    /// Drop the returned `GraphView` before any `.await` point or before calling other `Graph`
    /// methods; those operations may need the same lock.
    pub fn view(&self) -> GraphView<'_> {
        GraphView {
            data: self.data.lock(),
        }
    }
}

impl GraphView<'_> {
    pub fn entities(&self) -> impl Iterator<Item = &Entity> + '_ {
        self.data.entities()
    }

    pub fn nodes(&self) -> impl Iterator<Item = &NodeEntity> + '_ {
        self.entities().filter_map(|entity| match entity {
            Entity::Node(node) => Some(node),
            Entity::Endpoint(_) => None,
        })
    }

    pub fn endpoints(&self) -> impl Iterator<Item = &EndpointEntity> + '_ {
        self.entities().filter_map(|entity| match entity {
            Entity::Node(_) => None,
            Entity::Endpoint(endpoint) => Some(endpoint),
        })
    }

    pub fn publishers_on(&self, topic: impl AsRef<str>) -> Vec<EndpointEntity> {
        self.endpoints_named(EndpointKind::Publisher, topic)
    }

    pub fn subscriptions_on(&self, topic: impl AsRef<str>) -> Vec<EndpointEntity> {
        self.endpoints_named(EndpointKind::Subscription, topic)
    }

    pub fn services_named(&self, service_name: impl AsRef<str>) -> Vec<EndpointEntity> {
        self.endpoints_named(EndpointKind::Service, service_name)
    }

    pub fn clients_named(&self, service_name: impl AsRef<str>) -> Vec<EndpointEntity> {
        self.endpoints_named(EndpointKind::Client, service_name)
    }

    pub fn endpoints_for_node(&self, node: NodeKey) -> Vec<EndpointEntity> {
        self.endpoints()
            .filter(|endpoint| endpoint.node.key() == node)
            .cloned()
            .collect()
    }

    pub fn node_exists(&self, node: &NodeKey) -> bool {
        self.entities().any(|entity| match entity {
            Entity::Node(node_entity) => node_entity.key() == *node,
            Entity::Endpoint(endpoint) => endpoint.node.key() == *node,
        })
    }

    pub fn topic_names_and_types(&self) -> Vec<(String, String)> {
        self.names_and_types_for(|endpoint| {
            matches!(
                endpoint.kind,
                EndpointKind::Publisher | EndpointKind::Subscription
            )
        })
    }

    pub fn service_names_and_types(&self) -> Vec<(String, String)> {
        self.names_and_types_for(|endpoint| endpoint.kind == EndpointKind::Service)
    }

    pub fn node_names(&self) -> Vec<(String, String)> {
        self.nodes()
            .map(|node| {
                let namespace = if node.namespace.is_empty() {
                    "/".to_string()
                } else if node.namespace.starts_with('/') {
                    node.namespace.clone()
                } else {
                    format!("/{}", node.namespace)
                };
                (node.name.clone(), namespace)
            })
            .collect()
    }

    fn endpoints_named(&self, kind: EndpointKind, name: impl AsRef<str>) -> Vec<EndpointEntity> {
        let name = name.as_ref();
        self.endpoints()
            .filter(|endpoint| endpoint.kind == kind && endpoint.topic == name)
            .cloned()
            .collect()
    }

    fn names_and_types_for(
        &self,
        include: impl Fn(&EndpointEntity) -> bool,
    ) -> Vec<(String, String)> {
        self.endpoints()
            .filter(|endpoint| include(endpoint))
            .fold(BTreeMap::new(), |mut names, endpoint| {
                names
                    .entry(endpoint.topic.clone())
                    .or_insert_with(|| endpoint.type_info.name.clone());
                names
            })
            .into_iter()
            .collect()
    }
}

impl Graph {
    pub fn qos_incompatibilities_for_topic(
        &self,
        topic: impl AsRef<str>,
    ) -> Vec<QosIncompatibility> {
        let topic = topic.as_ref();
        let view = self.view();
        let publishers = view.publishers_on(topic);
        let subscriptions = view.subscriptions_on(topic);
        drop(view);

        let mut diagnostics = Vec::new();
        for publisher in publishers {
            let Ok(offered) = QosProfile::try_from(publisher.qos) else {
                continue;
            };

            for subscription in &subscriptions {
                let Ok(requested) = QosProfile::try_from(subscription.qos) else {
                    continue;
                };
                let compatibility = requested.compatibility_with_offered(&offered);
                if compatibility != QosCompatibility::Compatible {
                    tracing::warn!(
                        topic = %topic,
                        publisher_qos = ?publisher.qos,
                        subscription_qos = ?subscription.qos,
                        compatibility = ?compatibility,
                        "QoS incompatibility detected"
                    );
                    diagnostics.push(QosIncompatibility {
                        topic: topic.to_string(),
                        publisher: publisher.clone(),
                        subscription: subscription.clone(),
                        compatibility,
                    });
                }
            }
        }

        diagnostics
    }
}
