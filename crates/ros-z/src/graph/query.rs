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
    pub(crate) fn type_incompatible_endpoints_for(
        &self,
        endpoint: &EndpointEntity,
    ) -> Vec<EndpointEntity> {
        self.view()
            .endpoints()
            .filter(|candidate| {
                candidate.topic == endpoint.topic && candidate.type_info != endpoint.type_info
            })
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Error, Result,
        entity::{Entity, NodeEntity, SchemaHash, TypeInfo},
    };

    use super::*;

    fn endpoint(
        node: NodeEntity,
        id: usize,
        kind: EndpointKind,
        topic: &str,
        type_name: &str,
    ) -> EndpointEntity {
        EndpointEntity {
            id,
            node,
            kind,
            topic: topic.to_string(),
            type_info: TypeInfo::new(type_name, SchemaHash::zero()),
            qos: Default::default(),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn type_incompatible_endpoints_for_returns_visible_type_mismatches_for_all_endpoint_kinds()
    -> Result<()> {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .map_err(|source| Error::zenoh("open Zenoh session", source))?;
        let graph = Graph::new(&session).await?;
        let node = NodeEntity::new(
            session.zid(),
            61,
            "type_mismatch_node".to_string(),
            String::new(),
        );
        let publisher = endpoint(
            node.clone(),
            62,
            EndpointKind::Publisher,
            "/type_mismatch_topic",
            "std_msgs::String",
        );
        let incompatible_subscription = endpoint(
            node.clone(),
            63,
            EndpointKind::Subscription,
            "/type_mismatch_topic",
            "std_msgs::Int32",
        );
        let compatible_subscription = endpoint(
            node.clone(),
            64,
            EndpointKind::Subscription,
            "/type_mismatch_topic",
            "std_msgs::String",
        );
        let different_topic_subscription = endpoint(
            node,
            65,
            EndpointKind::Subscription,
            "/other_type_mismatch_topic",
            "std_msgs::Float32",
        );
        let incompatible_publisher = endpoint(
            publisher.node.clone(),
            66,
            EndpointKind::Publisher,
            "/type_mismatch_topic",
            "std_msgs::Bool",
        );
        let incompatible_service = endpoint(
            publisher.node.clone(),
            67,
            EndpointKind::Service,
            "/type_mismatch_topic",
            "test_msgs::Service",
        );
        let incompatible_client = endpoint(
            publisher.node.clone(),
            68,
            EndpointKind::Client,
            "/type_mismatch_topic",
            "test_msgs::Client",
        );

        graph.add_local_entity(Entity::Endpoint(publisher.clone()))?;
        graph.add_local_entity(Entity::Endpoint(incompatible_subscription.clone()))?;
        graph.add_local_entity(Entity::Endpoint(compatible_subscription.clone()))?;
        graph.add_local_entity(Entity::Endpoint(different_topic_subscription))?;
        graph.add_local_entity(Entity::Endpoint(incompatible_publisher.clone()))?;
        graph.add_local_entity(Entity::Endpoint(incompatible_service.clone()))?;
        graph.add_local_entity(Entity::Endpoint(incompatible_client.clone()))?;

        let incompatible_endpoints = graph.type_incompatible_endpoints_for(&publisher);

        assert_eq!(incompatible_endpoints.len(), 4);
        assert!(incompatible_endpoints.contains(&incompatible_subscription));
        assert!(incompatible_endpoints.contains(&incompatible_publisher));
        assert!(incompatible_endpoints.contains(&incompatible_service));
        assert!(incompatible_endpoints.contains(&incompatible_client));

        let service_incompatible_endpoints =
            graph.type_incompatible_endpoints_for(&incompatible_service);

        assert_eq!(service_incompatible_endpoints.len(), 5);
        assert!(service_incompatible_endpoints.contains(&publisher));
        assert!(service_incompatible_endpoints.contains(&incompatible_subscription));
        assert!(service_incompatible_endpoints.contains(&compatible_subscription));
        assert!(service_incompatible_endpoints.contains(&incompatible_publisher));
        assert!(service_incompatible_endpoints.contains(&incompatible_client));

        session
            .close()
            .await
            .map_err(|source| Error::zenoh("close Zenoh session", source))?;
        Ok(())
    }
}
