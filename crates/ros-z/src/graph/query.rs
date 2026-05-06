use std::{collections::BTreeSet, sync::Arc};

use crate::entity::{EndpointEntity, Entity, EntityKind, NodeKey};

use super::Graph;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QosIncompatibility {
    pub topic: String,
    pub publisher: EndpointEntity,
    pub subscription: EndpointEntity,
    pub compatibility: crate::qos::QosCompatibility,
}

impl Graph {
    pub fn count(&self, kind: EntityKind, name: impl AsRef<str>) -> usize {
        if kind == EntityKind::Node {
            return 0;
        }

        let mut total = 0;
        match kind {
            EntityKind::Publisher | EntityKind::Subscription => {
                self.data.lock().visit_by_topic(name, |ent| {
                    if crate::entity::entity_kind(&ent) == kind {
                        total += 1;
                    }
                });
            }
            EntityKind::Service | EntityKind::Client => {
                self.data.lock().visit_by_service(name, |ent| {
                    if crate::entity::entity_kind(&ent) == kind {
                        total += 1;
                    }
                });
            }
            _ => unreachable!(),
        }
        total
    }

    pub fn get_entities_by_topic(
        &self,
        kind: EntityKind,
        topic: impl AsRef<str>,
    ) -> Vec<Arc<Entity>> {
        if kind == EntityKind::Node {
            return Vec::new();
        }

        let mut res = Vec::new();
        self.data.lock().visit_by_topic(topic, |ent| {
            if crate::entity::entity_kind(&ent) == kind {
                res.push(ent);
            }
        });
        res
    }

    pub fn qos_incompatibilities_for_topic(
        &self,
        topic: impl AsRef<str>,
    ) -> Vec<QosIncompatibility> {
        let topic = topic.as_ref();
        let publishers = self.get_entities_by_topic(EntityKind::Publisher, topic);
        let subscriptions = self.get_entities_by_topic(EntityKind::Subscription, topic);
        let mut diagnostics = Vec::new();

        for publisher in publishers {
            let Some(publisher) = crate::entity::entity_get_endpoint(&publisher) else {
                continue;
            };
            let Ok(offered) = crate::qos::QosProfile::try_from(publisher.qos) else {
                continue;
            };

            for subscription in &subscriptions {
                let Some(subscription) = crate::entity::entity_get_endpoint(subscription) else {
                    continue;
                };
                let Ok(requested) = crate::qos::QosProfile::try_from(subscription.qos) else {
                    continue;
                };
                let compatibility = requested.compatibility_with_offered(&offered);
                if compatibility != crate::qos::QosCompatibility::Compatible {
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

    pub fn get_entities_by_node(&self, kind: EntityKind, node: NodeKey) -> Vec<EndpointEntity> {
        if kind == EntityKind::Node {
            return Vec::new();
        }

        let mut res = Vec::new();
        self.data.lock().visit_by_node(node, |ent| {
            if crate::entity::entity_kind(&ent) == kind
                && let Entity::Endpoint(endpoint) = &*ent
            {
                res.push(endpoint.clone());
            }
        });
        res
    }

    pub fn count_by_service(&self, kind: EntityKind, service_name: impl AsRef<str>) -> usize {
        if kind == EntityKind::Node {
            return 0;
        }
        assert!(matches!(kind, EntityKind::Service | EntityKind::Client));
        let mut total = 0;
        self.data.lock().visit_by_service(service_name, |ent| {
            if crate::entity::entity_kind(&ent) == kind {
                total += 1;
            }
        });
        total
    }

    pub fn get_entities_by_service(
        &self,
        kind: EntityKind,
        service_name: impl AsRef<str>,
    ) -> Vec<Arc<Entity>> {
        if kind == EntityKind::Node {
            return Vec::new();
        }
        assert!(matches!(kind, EntityKind::Service | EntityKind::Client));
        let mut res = Vec::new();
        self.data.lock().visit_by_service(service_name, |ent| {
            if crate::entity::entity_kind(&ent) == kind {
                res.push(ent);
            }
        });
        res
    }

    pub fn get_service_names_and_types(&self) -> Vec<(String, String)> {
        self.data.lock().service_names_and_types()
    }

    pub fn get_topic_names_and_types(&self) -> Vec<(String, String)> {
        self.data.lock().topic_names_and_types()
    }

    pub fn get_names_and_types_by_node(
        &self,
        node_key: NodeKey,
        kind: EntityKind,
    ) -> Vec<(String, String)> {
        // Use BTreeSet to deduplicate and sort results by (topic, type)
        // BTreeSet gives stable ordering for deterministic graph snapshots.
        let mut res_set = BTreeSet::new();
        let mut data = self.data.lock();

        let node_ns = node_key.0.clone();
        let node_name = node_key.1.clone();

        tracing::debug!(
            "get_names_and_types_by_node: Looking for node_key=({:?}, {:?}), kind={:?}",
            node_ns,
            node_name,
            kind
        );

        data.parse_pending();

        data.visit_by_node(node_key, |ent| {
            if let Some(enp) = crate::entity::entity_get_endpoint(&ent)
                && enp.entity_kind() == kind
                && let Some(type_info) = &enp.type_info
            {
                // Insert into set for automatic deduplication
                res_set.insert((enp.topic.clone(), type_info.name.clone()));
            }
        });

        let res: Vec<_> = res_set.into_iter().collect();

        tracing::debug!(
            "get_names_and_types_by_node: Returning {} topics for node ({:?}, {:?}), kind={:?}: {:?}",
            res.len(),
            node_ns,
            node_name,
            kind,
            res
        );

        res
    }

    /// Check if a node exists in the graph
    ///
    /// Returns true if the node exists, false otherwise
    pub fn node_exists(&self, node_key: NodeKey) -> bool {
        self.data.lock().node_exists(node_key)
    }

    /// Get all node names and namespaces discovered in the graph
    ///
    /// Returns a vector of tuples (node_name, node_namespace)
    pub fn get_node_names(&self) -> Vec<(String, String)> {
        self.data.lock().node_names()
    }

    /// Get all node names, namespaces, and enclaves discovered in the graph
    ///
    /// Returns a vector of tuples (node_name, node_namespace, enclave)
    pub fn get_node_names_with_enclaves(&self) -> Vec<(String, String, String)> {
        self.data.lock().node_names_with_enclaves()
    }
}
