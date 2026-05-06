use slab::Slab;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    sync::{Arc, Weak},
};
use tracing::debug;
use zenoh::Result;

use crate::entity::{EndpointKind, Entity, LivelinessKE, NodeKey, Topic};

const DEFAULT_SLAB_CAPACITY: usize = 128;

/// Type alias for entity parser function
pub(super) type EntityParser =
    Arc<dyn Fn(&zenoh::key_expr::KeyExpr) -> Result<Entity> + Send + Sync>;

pub(super) struct GraphData {
    cached: HashSet<LivelinessKE>,
    parsed: HashMap<LivelinessKE, Arc<Entity>>,
    by_topic: HashMap<Topic, Slab<Weak<Entity>>>,
    by_service: HashMap<Topic, Slab<Weak<Entity>>>,
    by_node: HashMap<NodeKey, Slab<Weak<Entity>>>,
    parser: EntityParser,
}

impl GraphData {
    pub(super) fn new_with_parser(parser: EntityParser) -> Self {
        Self {
            cached: HashSet::new(),
            parsed: HashMap::new(),
            by_topic: HashMap::new(),
            by_service: HashMap::new(),
            by_node: HashMap::new(),
            parser,
        }
    }

    pub(super) fn insert(&mut self, key_expr: LivelinessKE) {
        // Skip if already parsed to avoid duplicates
        if self.parsed.contains_key(&key_expr) {
            tracing::debug!("insert: Skipping already parsed key");
            return;
        }
        self.cached.insert(key_expr);
    }

    fn get_or_create_slab<K>(
        map: &mut HashMap<K, Slab<Weak<Entity>>>,
        key: K,
    ) -> &mut Slab<Weak<Entity>>
    where
        K: Eq + Hash,
    {
        map.entry(key)
            .or_insert_with(|| Slab::with_capacity(DEFAULT_SLAB_CAPACITY))
    }

    fn insert_weak_entity(slab: &mut Slab<Weak<Entity>>, weak: Weak<Entity>) {
        if slab.len() >= slab.capacity() {
            slab.retain(|_, weak_ptr| weak_ptr.upgrade().is_some());
        }
        slab.insert(weak);
    }

    fn retain_entities_not_matching_key(slab: &mut Slab<Weak<Entity>>, key_expr: &LivelinessKE) {
        slab.retain(|_, weak| {
            weak.upgrade().is_some_and(|arc| {
                crate::entity::entity_to_liveliness_key_expr(&arc)
                    .ok()
                    .as_ref()
                    != Some(key_expr)
            })
        });
    }

    fn remove_entity_from_indexes(&mut self, entity: &Entity, key_expr: &LivelinessKE) {
        match entity {
            Entity::Node(node_entity) => {
                if let Some(slab) = self.by_node.get_mut(&crate::entity::node_key(node_entity)) {
                    Self::retain_entities_not_matching_key(slab, key_expr);
                }
            }
            Entity::Endpoint(endpoint_entity) => {
                if matches!(
                    endpoint_entity.kind,
                    EndpointKind::Publisher | EndpointKind::Subscription
                ) && let Some(slab) = self.by_topic.get_mut(&endpoint_entity.topic)
                {
                    Self::retain_entities_not_matching_key(slab, key_expr);
                }
                if matches!(
                    endpoint_entity.kind,
                    EndpointKind::Service | EndpointKind::Client
                ) && let Some(slab) = self.by_service.get_mut(&endpoint_entity.topic)
                {
                    Self::retain_entities_not_matching_key(slab, key_expr);
                }
                if let Some(node) = endpoint_entity.node.as_ref()
                    && let Some(slab) = self.by_node.get_mut(&crate::entity::node_key(node))
                {
                    Self::retain_entities_not_matching_key(slab, key_expr);
                }
            }
        }
    }

    fn index_entity_arc(&mut self, entity: &Arc<Entity>) {
        let weak = Arc::downgrade(entity);

        match &**entity {
            Entity::Node(node) => {
                // Index maps own their keys so parsed entities can remain immutable and shared by Arc.
                let slab =
                    Self::get_or_create_slab(&mut self.by_node, crate::entity::node_key(node));
                Self::insert_weak_entity(slab, weak);
            }
            Entity::Endpoint(endpoint) => {
                if matches!(
                    endpoint.kind,
                    EndpointKind::Publisher | EndpointKind::Subscription
                ) {
                    // Index maps own their keys so parsed entities can remain immutable and shared by Arc.
                    let topic_slab =
                        Self::get_or_create_slab(&mut self.by_topic, endpoint.topic.clone());
                    Self::insert_weak_entity(topic_slab, weak.clone());
                }

                if matches!(endpoint.kind, EndpointKind::Service | EndpointKind::Client) {
                    // Index maps own their keys so parsed entities can remain immutable and shared by Arc.
                    let service_slab =
                        Self::get_or_create_slab(&mut self.by_service, endpoint.topic.clone());
                    Self::insert_weak_entity(service_slab, weak.clone());
                }

                if let Some(node) = endpoint.node.as_ref() {
                    // Index maps own their keys so parsed entities can remain immutable and shared by Arc.
                    let node_slab =
                        Self::get_or_create_slab(&mut self.by_node, crate::entity::node_key(node));
                    Self::insert_weak_entity(node_slab, weak);
                }
            }
        }
    }

    pub(super) fn remove(&mut self, key_expr: &LivelinessKE) {
        let was_cached = self.cached.remove(key_expr);
        let parsed = self.parsed.remove(key_expr);
        let was_parsed = parsed.is_some();
        debug!(
            "[GRF] Removed KE: {}, cached={}, parsed={}",
            key_expr.0, was_cached, was_parsed
        );

        if was_parsed {
            tracing::debug!("remove: Removed from parsed");
        }

        // Note: We don't eagerly remove from by_topic/by_service/by_node maps here.
        // The weak references will naturally fail to upgrade when entities are dropped,
        // and the retain() calls in visit_by_* functions will clean them up lazily.
        // Lazy cleanup keeps removal O(1) and prunes stale weak references on reads.

        match (was_cached, parsed) {
            // Both should not be present at the same time
            (true, Some(_)) => {
                tracing::warn!(
                    liveliness_key = %key_expr.0,
                    was_cached,
                    was_parsed,
                    "liveliness key was present in both graph stores"
                );
            }
            // If not in either set, it might have been already removed or never existed
            (false, None) => {
                // This can happen due to duplicate removal events or race conditions
                // Log but don't panic
            }
            // Expected cases: either in cached (not yet parsed) or in parsed
            _ => {}
        }
    }

    fn parse(&mut self) {
        let count = self.cached.len();
        debug!("[GRF] Parsing {} cached entities", count);

        let cached = self.cached.drain().collect::<Vec<_>>();
        for key_expr in cached {
            // Skip if already parsed (e.g., added via add_local_entity)
            if self.parsed.contains_key(&key_expr) {
                tracing::debug!("parse: Skipping already parsed key");
                continue;
            }

            // Parse using the graph's configured liveliness parser.
            let entity = match (self.parser)(&key_expr.0) {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("Failed to parse liveliness key {}: {:?}", key_expr.0, e);
                    continue;
                }
            };
            let arc = Arc::new(entity);
            match &*arc {
                Entity::Node(x) => {
                    debug!("[GRF] Parsed node: {}/{}", x.namespace, x.name);

                    let node_key = crate::entity::node_key(x);
                    tracing::debug!(
                        "parse: Storing Node entity with key=({:?}, {:?})",
                        node_key.0,
                        node_key.1
                    );
                }
                Entity::Endpoint(x) => {
                    let node_desc = x
                        .node
                        .as_ref()
                        .map(|node| format!("{}/{}", node.namespace, node.name))
                        .unwrap_or_else(|| "<unavailable>".to_string());
                    debug!(
                        "[GRF] Parsed endpoint: kind={:?}, topic={}, node={}",
                        x.kind, x.topic, node_desc
                    );
                    let type_str = x
                        .type_info
                        .as_ref()
                        .map(|t| t.name.as_str())
                        .unwrap_or("unknown");
                    if let Some(node) = x.node.as_ref() {
                        let node_key = crate::entity::node_key(node);
                        tracing::debug!(
                            "parse: Storing Endpoint ({:?}) for node_key=({:?}, {:?}), topic={}, type={}, id={}",
                            x.kind,
                            node_key.0,
                            node_key.1,
                            x.topic,
                            type_str,
                            x.id
                        );
                    } else {
                        tracing::debug!(
                            "parse: Storing Endpoint ({:?}) without node identity, topic={}, type={}, id={}",
                            x.kind,
                            x.topic,
                            type_str,
                            x.id
                        );
                    }
                }
            }
            self.index_entity_arc(&arc);
            self.parsed.insert(key_expr, arc);
        }
    }

    pub(super) fn visit_by_node<F>(&mut self, node_key: NodeKey, mut f: F)
    where
        F: FnMut(Arc<Entity>),
    {
        if !self.cached.is_empty() {
            self.parse();
        }

        if let Some(entities) = self.by_node.get_mut(&node_key) {
            tracing::debug!(
                "visit_by_node: Found {} entities in slab for node ({:?}, {:?})",
                entities.len(),
                node_key.0,
                node_key.1
            );
            let mut upgraded = 0;
            let mut failed = 0;
            entities.retain(|_, weak| {
                if let Some(rc) = weak.upgrade() {
                    f(rc);
                    upgraded += 1;
                    true
                } else {
                    failed += 1;
                    false
                }
            });
            tracing::debug!(
                "visit_by_node: Upgraded {} entities, failed to upgrade {}",
                upgraded,
                failed
            );
        } else {
            tracing::debug!(
                "visit_by_node: No entities found for node ({:?}, {:?})",
                node_key.0,
                node_key.1
            );
        }
    }

    pub(super) fn visit_by_topic<F>(&mut self, topic: impl AsRef<str>, mut f: F)
    where
        F: FnMut(Arc<Entity>),
    {
        if !self.cached.is_empty() {
            self.parse();
        }

        if let Some(entities) = self.by_topic.get_mut(topic.as_ref()) {
            entities.retain(|_, weak| {
                if let Some(rc) = weak.upgrade() {
                    f(rc);
                    true
                } else {
                    false
                }
            });
        }
    }

    pub(super) fn visit_by_service<F>(&mut self, service_name: impl AsRef<str>, mut f: F)
    where
        F: FnMut(Arc<Entity>),
    {
        if !self.cached.is_empty() {
            self.parse();
        }

        if let Some(entities) = self.by_service.get_mut(service_name.as_ref()) {
            entities.retain(|_, weak| {
                if let Some(rc) = weak.upgrade() {
                    f(rc);
                    true
                } else {
                    false
                }
            });
        }
    }

    pub(super) fn insert_local_entity(&mut self, entity: Entity, key_expr: LivelinessKE) -> bool {
        let already_exists = self.parsed.contains_key(&key_expr);

        if already_exists {
            self.remove_entity_from_indexes(&entity, &key_expr);
        }

        let arc = Arc::new(entity);
        self.parsed.insert(key_expr, arc.clone());
        self.index_entity_arc(&arc);
        !already_exists
    }

    pub(super) fn remove_local_entity(&mut self, entity: &Entity, key_expr: &LivelinessKE) {
        self.cached.remove(key_expr);
        self.parsed.remove(key_expr);
        self.remove_entity_from_indexes(entity, key_expr);
    }

    pub(super) fn parse_pending(&mut self) {
        if !self.cached.is_empty() {
            self.parse();
        }
    }

    pub(super) fn service_names_and_types(&mut self) -> Vec<(String, String)> {
        self.parse_pending();
        let mut res = Vec::new();
        for (service_name, slab) in &mut self.by_service {
            let mut found_type = None;
            slab.retain(|_, weak| {
                if let Some(ent) = weak.upgrade() {
                    if let Some(enp) = crate::entity::entity_get_endpoint(&ent)
                        && found_type.is_none()
                        && enp.kind == EndpointKind::Service
                    {
                        found_type = enp.type_info.as_ref().map(|x| x.name.clone());
                    }
                    true
                } else {
                    false
                }
            });
            if let Some(type_name) = found_type {
                res.push((service_name.clone(), type_name));
            }
        }
        res
    }

    pub(super) fn topic_names_and_types(&mut self) -> Vec<(String, String)> {
        self.parse_pending();
        let mut res = Vec::new();
        for (topic_name, slab) in &mut self.by_topic {
            let mut found_type = None;
            slab.retain(|_, weak| {
                if let Some(ent) = weak.upgrade() {
                    if found_type.is_none()
                        && let Some(enp) = crate::entity::entity_get_endpoint(&ent)
                        && matches!(
                            enp.kind,
                            EndpointKind::Publisher | EndpointKind::Subscription
                        )
                        && let Some(type_info) = &enp.type_info
                    {
                        found_type = Some(type_info.name.clone());
                    }
                    true
                } else {
                    false
                }
            });
            if let Some(type_name) = found_type {
                res.push((topic_name.clone(), type_name));
            }
        }
        res
    }

    pub(super) fn node_exists(&mut self, node_key: NodeKey) -> bool {
        self.parse_pending();
        self.by_node.get_mut(&node_key).is_some_and(|entities| {
            entities.retain(|_, weak| weak.upgrade().is_some());
            !entities.is_empty()
        })
    }

    pub(super) fn node_names(&mut self) -> Vec<(String, String)> {
        self.parse_pending();
        let mut result = Vec::new();
        for ((namespace, name), slab) in self.by_node.iter() {
            let denormalized_ns = if namespace.is_empty() {
                "/".to_string()
            } else if !namespace.starts_with('/') {
                format!("/{namespace}")
            } else {
                namespace.clone()
            };
            for (_, weak_entity) in slab.iter() {
                if let Some(entity_arc) = weak_entity.upgrade()
                    && matches!(&*entity_arc, Entity::Node(_))
                {
                    result.push((name.clone(), denormalized_ns.clone()));
                }
            }
        }
        result
    }

    pub(super) fn node_names_with_enclaves(&mut self) -> Vec<(String, String, String)> {
        self.parse_pending();
        let mut result = Vec::new();
        for ((namespace, name), slab) in self.by_node.iter() {
            let denormalized_ns = if namespace.is_empty() {
                "/".to_string()
            } else if !namespace.starts_with('/') {
                format!("/{namespace}")
            } else {
                namespace.clone()
            };
            for (_, weak_entity) in slab.iter() {
                if let Some(entity_arc) = weak_entity.upgrade()
                    && let Entity::Node(node) = &*entity_arc
                {
                    let enclave = if node.enclave.is_empty() {
                        "/".to_string()
                    } else if !node.enclave.starts_with('/') {
                        format!("/{}", node.enclave)
                    } else {
                        node.enclave.clone()
                    };
                    result.push((name.clone(), denormalized_ns.clone(), enclave));
                }
            }
        }
        result
    }
}
