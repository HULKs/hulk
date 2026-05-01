use parking_lot::Mutex;
use serde::Serialize;
use slab::Slab;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    sync::{Arc, Weak},
    time::{Duration, SystemTime},
};
use tokio::sync::Notify;
use tracing::debug;

use crate::entity::{
    EndpointEntity, EndpointKind, Entity, EntityKind, LivelinessKE, NodeKey, Topic,
};
use crate::event::GraphEventManager;
use tracing;
use zenoh::{Result, Session, pubsub::Subscriber, sample::SampleKind, session::ZenohId};

/// A serializable snapshot of the native ros-z graph state
#[derive(Debug, Clone, Serialize)]
pub struct GraphSnapshot {
    pub timestamp: SystemTime,
    pub domain_id: usize,
    pub topics: Vec<TopicSnapshot>,
    pub nodes: Vec<NodeSnapshot>,
    pub services: Vec<ServiceSnapshot>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EndpointEntity, EndpointKind, NodeEntity, TypeInfo};

    fn native_endpoint_liveliness(kind: EndpointKind) -> LivelinessKE {
        let zid: ZenohId = "1234567890abcdef1234567890abcdef".parse().unwrap();
        let node = NodeEntity::new(42, zid, 1, "talker".into(), String::new(), String::new());
        let entity = EndpointEntity {
            id: 2,
            node: Some(node),
            kind,
            topic: "/chatter".into(),
            type_info: Some(TypeInfo::new("std_msgs::String", None)),
            qos: Default::default(),
        };
        crate::entity::endpoint_to_liveliness_key_expr(&entity).unwrap()
    }

    #[test]
    fn parse_native_publisher_liveliness() {
        let key_expr = native_endpoint_liveliness(EndpointKind::Publisher);

        let parsed = ros_z_protocol::format::parse_liveliness(&key_expr).unwrap();

        assert!(
            matches!(parsed, Entity::Endpoint(endpoint) if endpoint.kind == EndpointKind::Publisher && endpoint.topic == "/chatter")
        );
    }

    #[test]
    fn parse_native_subscriber_liveliness() {
        let key_expr = native_endpoint_liveliness(EndpointKind::Subscription);

        let parsed = ros_z_protocol::format::parse_liveliness(&key_expr).unwrap();

        assert!(
            matches!(parsed, Entity::Endpoint(endpoint) if endpoint.kind == EndpointKind::Subscription)
        );
    }

    #[test]
    fn parse_native_service_liveliness() {
        let key_expr = native_endpoint_liveliness(EndpointKind::Service);

        let parsed = ros_z_protocol::format::parse_liveliness(&key_expr).unwrap();

        assert!(
            matches!(parsed, Entity::Endpoint(endpoint) if endpoint.kind == EndpointKind::Service)
        );
    }

    #[test]
    fn parse_native_client_liveliness() {
        let key_expr = native_endpoint_liveliness(EndpointKind::Client);

        let parsed = ros_z_protocol::format::parse_liveliness(&key_expr).unwrap();

        assert!(
            matches!(parsed, Entity::Endpoint(endpoint) if endpoint.kind == EndpointKind::Client)
        );
    }

    #[test]
    fn parse_native_node_liveliness() {
        let zid: ZenohId = "1234567890abcdef1234567890abcdef".parse().unwrap();
        let node = NodeEntity::new(7, zid, 4, "node".into(), "/ns".into(), String::new());
        let key_expr = crate::entity::node_to_liveliness_key_expr(&node).unwrap();

        let parsed = ros_z_protocol::format::parse_liveliness(&key_expr).unwrap();

        assert!(
            matches!(parsed, Entity::Node(node) if node.z_id == zid && node.name == "node" && node.namespace == "/ns")
        );
    }

    #[test]
    fn reject_ros2_liveliness_prefix() {
        let key_expr: zenoh::key_expr::KeyExpr<'static> = concat!(
            "@ros2",
            "_lv/0/1234567890abcdef1234567890abcdef/1/1/MP/%/%/talker/chatter/std_msgs::String/EMPTY_SCHEMA_HASH/Q"
        )
            .try_into()
            .unwrap();

        assert!(ros_z_protocol::format::parse_liveliness(&key_expr).is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn node_exists_returns_false_after_only_node_removed() {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .expect("session should open");
        let graph = Graph::new(&session, 0).await.expect("graph should build");
        let node = NodeEntity::new(
            0,
            session.zid(),
            1,
            "removed_node".to_string(),
            String::new(),
            String::new(),
        );
        let node_key = crate::entity::node_key(&node);
        let entity = Entity::Node(node);

        graph
            .add_local_entity(entity.clone())
            .expect("node should be added");
        assert!(graph.node_exists(node_key.clone()));

        graph
            .remove_local_entity(&entity)
            .expect("node should be removed");

        assert!(!graph.node_exists(node_key));
        session.close().await.expect("session should close");
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QosIncompatibility {
    pub topic: String,
    pub publisher: EndpointEntity,
    pub subscription: EndpointEntity,
    pub compatibility: crate::qos::QosCompatibility,
}

const DEFAULT_SLAB_CAPACITY: usize = 128;

#[derive(Debug, Clone)]
pub struct GraphOptions {
    pub initial_liveliness_query_timeout: Option<Duration>,
}

impl Default for GraphOptions {
    fn default() -> Self {
        Self {
            initial_liveliness_query_timeout: Some(Duration::from_secs(3)),
        }
    }
}

/// Type alias for entity parser function
type EntityParser = Arc<dyn Fn(&zenoh::key_expr::KeyExpr) -> Result<Entity> + Send + Sync>;

pub struct GraphData {
    cached: HashSet<LivelinessKE>,
    parsed: HashMap<LivelinessKE, Arc<Entity>>,
    by_topic: HashMap<Topic, Slab<Weak<Entity>>>,
    by_service: HashMap<Topic, Slab<Weak<Entity>>>,
    by_node: HashMap<NodeKey, Slab<Weak<Entity>>>,
    parser: EntityParser,
}

impl GraphData {
    fn new_with_parser(parser: EntityParser) -> Self {
        Self {
            cached: HashSet::new(),
            parsed: HashMap::new(),
            by_topic: HashMap::new(),
            by_service: HashMap::new(),
            by_node: HashMap::new(),
            parser,
        }
    }

    fn insert(&mut self, key_expr: LivelinessKE) {
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

    fn remove(&mut self, key_expr: &LivelinessKE) {
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

    pub fn visit_by_node<F>(&mut self, node_key: NodeKey, mut f: F)
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

    pub fn visit_by_topic<F>(&mut self, topic: impl AsRef<str>, mut f: F)
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

    pub fn visit_by_service<F>(&mut self, service_name: impl AsRef<str>, mut f: F)
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
}

pub struct Graph {
    pub data: Arc<Mutex<GraphData>>,
    pub event_manager: Arc<GraphEventManager>,
    pub zid: ZenohId,
    /// Notified whenever an entity appears or disappears in the graph.
    ///
    /// Publishers use this to implement `wait_for_subscribers`: they register
    /// a `notified()` future before sampling the graph, then `await` it so no
    /// arrival is missed between the sample and the wait.
    pub change_notify: Arc<Notify>,
    _subscriber: Subscriber<()>,
}

impl std::fmt::Debug for Graph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Graph")
            .field("zid", &self.zid)
            .finish_non_exhaustive()
    }
}

impl Graph {
    /// Create a new Graph using the native ros-z liveliness protocol.
    pub async fn new(session: &Session, domain_id: usize) -> Result<Self> {
        Self::new_with_options(session, domain_id, GraphOptions::default()).await
    }

    pub async fn new_with_options(
        session: &Session,
        domain_id: usize,
        options: GraphOptions,
    ) -> Result<Self> {
        let liveliness_pattern = format!("{}/**", crate::entity::ADMIN_SPACE);

        Self::new_with_pattern_and_options(
            session,
            domain_id,
            liveliness_pattern,
            ros_z_protocol::format::parse_liveliness,
            options,
        )
        .await
    }

    async fn wait_until<F>(&self, timeout: Duration, predicate: F) -> bool
    where
        F: Fn(&Self) -> bool,
    {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let notified = self.change_notify.notified();
            tokio::pin!(notified);

            if predicate(self) {
                return true;
            }

            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return false;
            }

            if tokio::time::timeout(remaining, &mut notified)
                .await
                .is_err()
            {
                return predicate(self);
            }
        }
    }

    /// Create a new Graph with a custom liveliness subscription pattern and parser
    ///
    /// # Arguments
    /// * `session` - Zenoh session
    /// * `domain_id` - in-process deployment partition retained for existing APIs; native keys do not serialize it
    /// * `liveliness_pattern` - Liveliness key expression pattern to subscribe to
    /// * `parser` - Function to parse liveliness key expressions into Entity
    ///
    /// The default ros-z liveliness pattern is `@ros_z/**`.
    pub async fn new_with_pattern<F>(
        session: &Session,
        _domain_id: usize,
        liveliness_pattern: String,
        parser: F,
    ) -> Result<Self>
    where
        F: Fn(&zenoh::key_expr::KeyExpr) -> Result<Entity> + Send + Sync + 'static,
    {
        Self::new_with_pattern_and_options(
            session,
            _domain_id,
            liveliness_pattern,
            parser,
            GraphOptions::default(),
        )
        .await
    }

    async fn new_with_pattern_and_options<F>(
        session: &Session,
        _domain_id: usize,
        liveliness_pattern: String,
        parser: F,
        options: GraphOptions,
    ) -> Result<Self>
    where
        F: Fn(&zenoh::key_expr::KeyExpr) -> Result<Entity> + Send + Sync + 'static,
    {
        let zid = session.zid();
        let parser_arc = Arc::new(parser);
        let graph_data = Arc::new(Mutex::new(GraphData::new_with_parser(parser_arc.clone())));
        let event_manager = Arc::new(GraphEventManager::new());
        let change_notify = Arc::new(Notify::new());
        let c_graph_data = graph_data.clone();
        let c_event_manager = event_manager.clone();
        let c_change_notify = change_notify.clone();
        let c_zid = zid;
        let c_liveliness_pattern = liveliness_pattern.clone();
        let callback_parser = parser_arc.clone();
        tracing::debug!("Creating liveliness subscriber for {}", liveliness_pattern);
        let sub = session
            .liveliness()
            .declare_subscriber(&liveliness_pattern)
            .history(true)
            .callback(move |sample| {
                let key_expr = sample.key_expr().to_owned();
                let key_expr = LivelinessKE(key_expr.clone());
                tracing::debug!(
                    "Received liveliness token: {} kind={:?}",
                    key_expr.0,
                    sample.kind()
                );

                let graph_change = match sample.kind() {
                    SampleKind::Put => {
                        debug!("[GRF] Entity appeared: {}", key_expr.0);
                        tracing::debug!("Graph subscriber: PUT {}", key_expr.as_str());
                        let parsed_entity = match callback_parser(&key_expr) {
                            Ok(entity) => Some(entity),
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to parse liveliness token {}: {:?}",
                                    key_expr.0,
                                    e
                                );
                                None
                            }
                        };

                        {
                            let mut graph_data_guard = c_graph_data.lock();
                            // Only insert if not already parsed (avoid duplicates from liveliness query)
                            let already_parsed = graph_data_guard.parsed.contains_key(&key_expr);
                            let already_cached = graph_data_guard.cached.contains(&key_expr);
                            tracing::debug!(
                                "  Check: parsed={}, cached={}, parsed.len()={}, cached.len()={}",
                                already_parsed,
                                already_cached,
                                graph_data_guard.parsed.len(),
                                graph_data_guard.cached.len()
                            );
                            if already_parsed {
                                tracing::debug!("  Skipping - already in parsed");
                            } else if already_cached {
                                tracing::debug!("  Skipping - already in cached");
                            } else {
                                tracing::debug!("  Adding to cached");
                                graph_data_guard.insert(key_expr.clone());
                            }
                        }

                        parsed_entity.map(|entity| {
                            tracing::debug!("Successfully parsed entity: {:?}", entity);
                            (entity, true)
                        })
                    }
                    SampleKind::Delete => {
                        debug!("[GRF] Entity disappeared: {}", key_expr.0);
                        tracing::debug!("Graph subscriber: DELETE {}", key_expr.as_str());
                        let parsed_entity = callback_parser(&key_expr).ok();
                        c_graph_data.lock().remove(&key_expr);
                        parsed_entity.map(|entity| (entity, false))
                    }
                };

                if let Some((entity, appeared)) = graph_change {
                    c_event_manager.trigger_graph_change(&entity, appeared, c_zid);
                }
                // Wake any tasks waiting in wait_for_subscribers / wait_for_publishers.
                c_change_notify.notify_waiters();
            })
            .await?;

        // Query existing liveliness tokens from all connected sessions
        // This is crucial for cross-context discovery where entities from other sessions
        // were created before this session started
        if let Some(timeout) = options.initial_liveliness_query_timeout {
            let replies = session
                .liveliness()
                .get(&c_liveliness_pattern)
                .timeout(timeout)
                .await?;

            // Process all replies and add them to the graph.
            // At this point plain ros-z still relies on liveliness for local entity
            // visibility, so do not filter current-session entities here.
            let mut reply_count = 0;
            while let Ok(reply) = replies.recv_async().await {
                reply_count += 1;
                if let Ok(sample) = reply.into_result() {
                    let key_expr = sample.key_expr().to_owned();
                    let key_expr = LivelinessKE(key_expr.clone());

                    tracing::debug!("Graph: Caching liveliness entity: {}", key_expr.as_str());
                    graph_data.lock().insert(key_expr);
                }
            }
            tracing::debug!("Graph: Liveliness query received {} replies", reply_count);
        }

        Ok(Self {
            _subscriber: sub,
            data: graph_data,
            event_manager,
            change_notify,
            zid,
        })
    }

    /// Check if an entity belongs to the current session
    pub fn is_entity_local(&self, entity: &Entity) -> bool {
        match entity {
            Entity::Node(node) => node.z_id == self.zid,
            Entity::Endpoint(endpoint) => endpoint
                .node
                .as_ref()
                .is_some_and(|node| node.z_id == self.zid),
        }
    }

    /// Add a local entity to the graph for immediate discovery
    /// This is used to make local publishers/subscriptions/services/clients
    /// immediately visible in graph queries without waiting for Zenoh liveliness propagation
    pub fn add_local_entity(&self, entity: Entity) -> Result<()> {
        let mut data = self.data.lock();

        // Create LivelinessKE from entity
        let key_expr = crate::entity::entity_to_liveliness_key_expr(&entity)?;

        // Check if entity already exists (to avoid triggering duplicate graph change events)
        let already_exists = data.parsed.contains_key(&key_expr);

        // Store the local entity as shared immutable graph data.
        let arc = Arc::new(entity.clone());

        // Store in parsed HashMap
        if already_exists {
            data.remove_entity_from_indexes(&entity, &key_expr);
        }
        data.parsed.insert(key_expr, arc.clone());

        // Add to appropriate indexes
        data.index_entity_arc(&arc);

        // Release lock before triggering events
        drop(data);

        // Only trigger graph change event if this is a new entity
        // (to avoid double-counting when liveliness already triggered it)
        if !already_exists {
            self.event_manager
                .trigger_graph_change(&entity, true, self.zid);
        }

        Ok(())
    }

    /// Remove a local entity from the graph
    pub fn remove_local_entity(&self, entity: &Entity) -> Result<()> {
        let mut data = self.data.lock();

        // Create LivelinessKE from entity
        let key_expr = crate::entity::entity_to_liveliness_key_expr(entity)?;

        // Remove from both cached and parsed
        data.cached.remove(&key_expr);
        data.parsed.remove(&key_expr);

        // Also remove from the index slabs (by_topic, by_service, by_node).
        data.remove_entity_from_indexes(entity, &key_expr);

        // Release lock before triggering events
        drop(data);

        // Note: We do NOT call trigger_graph_change here because the liveliness
        // DELETE callback will fire when the entity's liveliness token is dropped,
        // which already triggers the graph change event. Calling it here too would
        // double-count the change. (Same pattern as add_local_entity's !already_exists guard.)

        Ok(())
    }

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
        let mut res = Vec::new();
        let mut data = self.data.lock();

        if !data.cached.is_empty() {
            data.parse();
        }

        // Iterate directly over all services in by_service index
        for (service_name, slab) in &mut data.by_service {
            let mut found_type = None;
            slab.retain(|_, weak| {
                if let Some(ent) = weak.upgrade() {
                    // Skip expensive get_endpoint() if we already found the type
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

    pub fn get_topic_names_and_types(&self) -> Vec<(String, String)> {
        let mut res = Vec::new();
        let mut data = self.data.lock();

        if !data.cached.is_empty() {
            data.parse();
        }

        // NOTE: Each topic has exactly one topic type
        // Iterate directly over all topics in by_topic index
        for (topic_name, slab) in &mut data.by_topic {
            let mut found_type = None;
            slab.retain(|_, weak| {
                if let Some(ent) = weak.upgrade() {
                    // Skip expensive get_endpoint() if we already found the type
                    if found_type.is_none()
                        && let Some(enp) = crate::entity::entity_get_endpoint(&ent)
                    {
                        // Include both publishers and subscribers
                        if matches!(
                            enp.kind,
                            EndpointKind::Publisher | EndpointKind::Subscription
                        ) && let Some(type_info) = &enp.type_info
                        {
                            found_type = Some(type_info.name.clone());
                        }
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

    pub fn get_names_and_types_by_node(
        &self,
        node_key: NodeKey,
        kind: EntityKind,
    ) -> Vec<(String, String)> {
        use std::collections::BTreeSet;

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

        if !data.cached.is_empty() {
            tracing::debug!(
                "get_names_and_types_by_node: Parsing {} cached entries",
                data.cached.len()
            );
            data.parse();
        }

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
        let mut data = self.data.lock();

        if !data.cached.is_empty() {
            data.parse();
        }

        data.by_node.get_mut(&node_key).is_some_and(|entities| {
            entities.retain(|_, weak| weak.upgrade().is_some());
            !entities.is_empty()
        })
    }

    /// Get all node names and namespaces discovered in the graph
    ///
    /// Returns a vector of tuples (node_name, node_namespace)
    pub fn get_node_names(&self) -> Vec<(String, String)> {
        let mut data = self.data.lock();

        if !data.cached.is_empty() {
            data.parse();
        }

        // Extract all nodes from by_node HashMap
        // Return one entry per node instance (even if multiple nodes have same name/namespace)
        // Denormalize namespace: empty string becomes "/"
        let mut result = Vec::new();
        for ((namespace, name), slab) in data.by_node.iter() {
            let denormalized_ns = if namespace.is_empty() {
                "/".to_string()
            } else if !namespace.starts_with('/') {
                format!("/{}", namespace)
            } else {
                namespace.clone()
            };

            // Count each Node entity separately (not Endpoint entities)
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

    /// Get all node names, namespaces, and enclaves discovered in the graph
    ///
    /// Returns a vector of tuples (node_name, node_namespace, enclave)
    pub fn get_node_names_with_enclaves(&self) -> Vec<(String, String, String)> {
        let mut data = self.data.lock();

        if !data.cached.is_empty() {
            data.parse();
        }

        // Extract all nodes from by_node HashMap
        // Return one entry per node instance (even if multiple nodes have same name/namespace)
        // Denormalize namespace: empty string becomes "/"
        let mut result = Vec::new();
        for ((namespace, name), slab) in data.by_node.iter() {
            let denormalized_ns = if namespace.is_empty() {
                "/".to_string()
            } else if !namespace.starts_with('/') {
                format!("/{}", namespace)
            } else {
                namespace.clone()
            };

            // Process each Node entity separately (not Endpoint entities)
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

    /// Get action client names and types by node
    ///
    /// Returns a vector of tuples (action_name, action_type) for action clients on the specified node
    ///
    /// Action clients subscribe to feedback topics, so we query subscribers and
    /// filter for native action feedback channels.
    pub fn get_action_client_names_and_types_by_node(
        &self,
        node_key: NodeKey,
    ) -> Vec<(String, String)> {
        // Get all subscribers for this node
        let subscribers = self.get_names_and_types_by_node(node_key, EntityKind::Subscription);

        // Filter for action feedback topics and extract action name/type
        self.filter_action_names_and_types(subscribers)
    }

    /// Get action server names and types by node
    ///
    /// Returns a vector of tuples (action_name, action_type) for action servers on the specified node
    ///
    /// Action servers publish feedback topics, so we query publishers and filter
    /// for native action feedback channels.
    pub fn get_action_server_names_and_types_by_node(
        &self,
        node_key: NodeKey,
    ) -> Vec<(String, String)> {
        // Get all publishers for this node
        let publishers = self.get_names_and_types_by_node(node_key, EntityKind::Publisher);

        // Filter for action feedback topics and extract action name/type
        self.filter_action_names_and_types(publishers)
    }

    /// Filter topic names and types to extract action names and types
    ///
    /// This helper method implements native action-channel filtering:
    /// - Looks for topics with the native action feedback suffix
    /// - Extracts the action name by removing the suffix
    /// - Extracts the action type by removing the native feedback suffix from the type
    fn filter_action_names_and_types(
        &self,
        topics: Vec<(String, String)>,
    ) -> Vec<(String, String)> {
        const ACTION_CHANNEL_PREFIX: &str = "_ros_z_action";
        const ACTION_TYPE_SUFFIX: &str = "FeedbackMessage";
        let action_name_suffix = format!("/{ACTION_CHANNEL_PREFIX}/feedback");

        topics
            .into_iter()
            .filter_map(|(topic_name, type_name)| {
                // Check if topic name ends with the native feedback suffix.
                if topic_name.ends_with(&action_name_suffix) {
                    // Extract action name by removing the suffix
                    let action_name = topic_name
                        .strip_suffix(&action_name_suffix)
                        .unwrap()
                        .to_string();

                    // Extract action type by removing the feedback suffix if present.
                    let action_type_base = type_name
                        .strip_suffix(ACTION_TYPE_SUFFIX)
                        .unwrap_or(&type_name);
                    let action_type = action_type_base
                        .strip_suffix("::")
                        .unwrap_or(action_type_base)
                        .to_string();

                    Some((action_name, action_type))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all action names and types discovered in the graph
    ///
    /// Returns a vector of tuples (action_name, action_type) for all action clients and servers
    ///
    /// Query all topics and filter for native action feedback channels.
    pub fn get_action_names_and_types(&self) -> Vec<(String, String)> {
        // Get all topics
        let topics = self.get_topic_names_and_types();

        // Filter for action feedback topics and extract action name/type
        let mut res = self.filter_action_names_and_types(topics);

        // Remove duplicates (same action name/type may appear on multiple nodes)
        res.sort();
        res.dedup();
        res
    }

    /// Wait for a full native ros-z action server (services + publishers) to appear.
    ///
    /// Waits for exactly one server to be ready. Multiple servers sharing the same
    /// action name is not a supported ros-z pattern, so a fixed threshold of 1 is
    /// intentional here (unlike `wait_for_service` which accepts an explicit `count`).
    pub(crate) async fn wait_for_action_server(
        &self,
        action_name: impl Into<String>,
        timeout: Duration,
    ) -> bool {
        let action_name = action_name.into();
        const ACTION_CHANNEL_PREFIX: &str = "_ros_z_action";
        let goal_service = format!("{action_name}/{ACTION_CHANNEL_PREFIX}/send_goal");
        let result_service = format!("{action_name}/{ACTION_CHANNEL_PREFIX}/get_result");
        let cancel_service = format!("{action_name}/{ACTION_CHANNEL_PREFIX}/cancel_goal");
        let feedback_topic = format!("{action_name}/{ACTION_CHANNEL_PREFIX}/feedback");
        let status_topic = format!("{action_name}/{ACTION_CHANNEL_PREFIX}/status");

        self.wait_until(timeout, move |graph| {
            graph.count_by_service(EntityKind::Service, &goal_service) >= 1
                && graph.count_by_service(EntityKind::Service, &result_service) >= 1
                && graph.count_by_service(EntityKind::Service, &cancel_service) >= 1
                && !graph
                    .get_entities_by_topic(EntityKind::Publisher, &feedback_topic)
                    .is_empty()
                && !graph
                    .get_entities_by_topic(EntityKind::Publisher, &status_topic)
                    .is_empty()
        })
        .await
    }

    /// Create a serializable snapshot of the current graph state
    ///
    /// This captures topics, nodes, and services with their metadata,
    /// suitable for JSON serialization or other export formats.
    pub fn snapshot(&self, domain_id: usize) -> GraphSnapshot {
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
            domain_id,
            topics,
            nodes,
            services,
        }
    }
}
