use crate::entity::{EndpointEntity, EndpointKind, Entity, NodeEntity, NodeKey};
use crate::qos::{QosCompatibility, QosProfile};

use super::{Graph, GraphRevision, state::GraphData};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QosIncompatibility {
    pub topic: String,
    pub publisher: EndpointEntity,
    pub subscription: EndpointEntity,
    pub compatibility: QosCompatibility,
}

/// A publisher/subscriber type mismatch for one topic.
///
/// Each value represents one publisher/subscriber pair on the same topic whose
/// full [`TypeInfo`](crate::entity::TypeInfo) values differ. Services and
/// clients are not considered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeMismatch {
    pub topic: String,
    pub publisher: EndpointEntity,
    pub subscription: EndpointEntity,
}

/// Subscription to effective local graph changes.
///
/// The subscription stores only the latest graph revision. Treat each observed revision as a signal
/// to resync from [`Graph::lock`]; revisions do not carry per-change payloads and do not prove that
/// the distributed graph is complete.
pub struct GraphRevisionWatch {
    changes: tokio::sync::watch::Receiver<GraphRevision>,
}

impl GraphRevisionWatch {
    pub(super) fn new(changes: tokio::sync::watch::Receiver<GraphRevision>) -> Self {
        Self { changes }
    }

    /// Return the currently observed graph revision without marking it as seen.
    pub fn current(&self) -> GraphRevision {
        *self.changes.borrow()
    }

    /// Mark the current revision as seen and return it.
    pub fn mark_seen(&mut self) -> GraphRevision {
        *self.changes.borrow_and_update()
    }

    /// Wait for a later graph revision.
    pub async fn changed(&mut self) -> Option<GraphRevision> {
        self.changes.changed().await.ok()?;
        Some(*self.changes.borrow_and_update())
    }
}

impl Graph {
    pub(crate) async fn wait_until<F>(&self, mut predicate: F) -> bool
    where
        F: FnMut(&GraphData) -> bool,
    {
        let mut revisions = self.watch_revisions();
        loop {
            revisions.mark_seen();
            let satisfied = {
                let data = self.lock();
                predicate(&data)
            };
            if satisfied {
                return true;
            }

            if revisions.changed().await.is_none() {
                return false;
            }
        }
    }

    pub(crate) fn type_incompatible_endpoints_for(
        &self,
        endpoint: &EndpointEntity,
    ) -> Vec<EndpointEntity> {
        self.lock()
            .endpoints()
            .filter(|candidate| {
                candidate.topic == endpoint.topic && candidate.type_info != endpoint.type_info
            })
            .cloned()
            .collect()
    }
}

impl GraphData {
    pub fn entities(&self) -> impl Iterator<Item = &Entity> + '_ {
        self.entities_raw()
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

    pub fn publishers(&self) -> impl Iterator<Item = &EndpointEntity> + '_ {
        self.endpoints()
            .filter(|endpoint| endpoint.kind == EndpointKind::Publisher)
    }

    pub fn subscriptions(&self) -> impl Iterator<Item = &EndpointEntity> + '_ {
        self.endpoints()
            .filter(|endpoint| endpoint.kind == EndpointKind::Subscription)
    }

    pub fn services(&self) -> impl Iterator<Item = &EndpointEntity> + '_ {
        self.endpoints()
            .filter(|endpoint| endpoint.kind == EndpointKind::Service)
    }

    pub fn clients(&self) -> impl Iterator<Item = &EndpointEntity> + '_ {
        self.endpoints()
            .filter(|endpoint| endpoint.kind == EndpointKind::Client)
    }

    pub fn endpoints_for_node<'a>(
        &'a self,
        node: &'a NodeKey,
    ) -> impl Iterator<Item = &'a EndpointEntity> + 'a {
        self.endpoints()
            .filter(move |endpoint| endpoint.node.key() == *node)
    }

    pub fn endpoints_on<'a>(
        &'a self,
        name: &'a str,
    ) -> impl Iterator<Item = &'a EndpointEntity> + 'a {
        self.endpoints()
            .filter(move |endpoint| endpoint.topic == name)
    }

    pub fn publishers_on<'a>(
        &'a self,
        topic: &'a str,
    ) -> impl Iterator<Item = &'a EndpointEntity> + 'a {
        self.publishers()
            .filter(move |endpoint| endpoint.topic == topic)
    }

    pub fn subscriptions_on<'a>(
        &'a self,
        topic: &'a str,
    ) -> impl Iterator<Item = &'a EndpointEntity> + 'a {
        self.subscriptions()
            .filter(move |endpoint| endpoint.topic == topic)
    }

    pub fn services_named<'a>(
        &'a self,
        service: &'a str,
    ) -> impl Iterator<Item = &'a EndpointEntity> + 'a {
        self.services()
            .filter(move |endpoint| endpoint.topic == service)
    }

    pub fn clients_named<'a>(
        &'a self,
        service: &'a str,
    ) -> impl Iterator<Item = &'a EndpointEntity> + 'a {
        self.clients()
            .filter(move |endpoint| endpoint.topic == service)
    }

    pub fn node_exists(&self, node: &NodeKey) -> bool {
        self.entities().any(|entity| match entity {
            Entity::Node(node_entity) => node_entity.key() == *node,
            Entity::Endpoint(endpoint) => endpoint.node.key() == *node,
        })
    }

    fn collect_pub_sub_pair_diagnostics<T>(
        &self,
        topic: &str,
        mut diagnostic_for_pair: impl FnMut(&str, &EndpointEntity, &EndpointEntity) -> Option<T>,
    ) -> Vec<T> {
        let publishers = self.publishers_on(topic).collect::<Vec<_>>();
        let subscriptions = self.subscriptions_on(topic).collect::<Vec<_>>();

        let mut diagnostics = Vec::new();
        for publisher in &publishers {
            for subscription in &subscriptions {
                if let Some(diagnostic) = diagnostic_for_pair(topic, publisher, subscription) {
                    diagnostics.push(diagnostic);
                }
            }
        }

        diagnostics
    }

    /// Return pairwise QoS incompatibilities for publishers and subscribers on `topic`.
    pub fn qos_incompatibilities_for_topic(
        &self,
        topic: impl AsRef<str>,
    ) -> Vec<QosIncompatibility> {
        let topic = topic.as_ref();

        self.collect_pub_sub_pair_diagnostics(topic, |topic, publisher, subscription| {
            let Ok(offered) = QosProfile::try_from(publisher.qos) else {
                return None;
            };
            let Ok(requested) = QosProfile::try_from(subscription.qos) else {
                return None;
            };

            let compatibility = requested.compatibility_with_offered(&offered);
            if compatibility == QosCompatibility::Compatible {
                return None;
            }

            tracing::warn!(
                topic = %topic,
                publisher_qos = ?publisher.qos,
                subscription_qos = ?subscription.qos,
                compatibility = ?compatibility,
                "QoS incompatibility detected"
            );
            Some(QosIncompatibility {
                topic: topic.to_string(),
                publisher: publisher.clone(),
                subscription: subscription.clone(),
                compatibility,
            })
        })
    }

    /// Return pairwise publisher/subscriber type mismatches for `topic`.
    pub fn pub_sub_type_mismatches_for_topic(&self, topic: impl AsRef<str>) -> Vec<TypeMismatch> {
        let topic = topic.as_ref();

        self.collect_pub_sub_pair_diagnostics(topic, |topic, publisher, subscription| {
            if publisher.type_info == subscription.type_info {
                return None;
            }

            Some(TypeMismatch {
                topic: topic.to_string(),
                publisher: publisher.clone(),
                subscription: subscription.clone(),
            })
        })
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

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_data_pub_sub_type_mismatches_for_topic_returns_pub_sub_pairs_only() -> Result<()>
    {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .map_err(|source| Error::zenoh("open Zenoh session", source))?;
        let graph = Graph::new(&session).await?;
        let pub_node = NodeEntity::new(
            session.zid(),
            71,
            "type_mismatch_pub".to_string(),
            String::new(),
        );
        let sub_node = NodeEntity::new(
            session.zid(),
            72,
            "type_mismatch_sub".to_string(),
            String::new(),
        );
        let publisher = endpoint(
            pub_node.clone(),
            73,
            EndpointKind::Publisher,
            "/doctor_type_mismatch",
            "std_msgs::String",
        );
        let incompatible_subscription = endpoint(
            sub_node.clone(),
            74,
            EndpointKind::Subscription,
            "/doctor_type_mismatch",
            "std_msgs::Int32",
        );
        let compatible_subscription = endpoint(
            sub_node,
            75,
            EndpointKind::Subscription,
            "/doctor_type_mismatch",
            "std_msgs::String",
        );
        let incompatible_publisher = endpoint(
            pub_node,
            76,
            EndpointKind::Publisher,
            "/doctor_type_mismatch",
            "std_msgs::Bool",
        );

        graph.add_local_entity(Entity::Endpoint(publisher.clone()))?;
        graph.add_local_entity(Entity::Endpoint(incompatible_subscription.clone()))?;
        graph.add_local_entity(Entity::Endpoint(compatible_subscription.clone()))?;
        graph.add_local_entity(Entity::Endpoint(incompatible_publisher.clone()))?;

        let data = graph.lock();
        let mismatches = data.pub_sub_type_mismatches_for_topic("/doctor_type_mismatch");

        assert_eq!(mismatches.len(), 3);
        assert!(mismatches.contains(&TypeMismatch {
            topic: "/doctor_type_mismatch".to_string(),
            publisher: publisher.clone(),
            subscription: incompatible_subscription.clone(),
        }));
        assert!(mismatches.contains(&TypeMismatch {
            topic: "/doctor_type_mismatch".to_string(),
            publisher: incompatible_publisher.clone(),
            subscription: incompatible_subscription,
        }));
        assert!(mismatches.contains(&TypeMismatch {
            topic: "/doctor_type_mismatch".to_string(),
            publisher: incompatible_publisher,
            subscription: compatible_subscription,
        }));

        drop(data);
        session
            .close()
            .await
            .map_err(|source| Error::zenoh("close Zenoh session", source))?;
        Ok(())
    }

    #[tokio::test]
    async fn graph_revision_watch_changed_returns_none_after_sender_closes() {
        let (sender, receiver) = tokio::sync::watch::channel(GraphRevision::INITIAL);
        let mut changes = GraphRevisionWatch::new(receiver);
        changes.mark_seen();

        drop(sender);

        assert_eq!(changes.changed().await, None);
    }
}
