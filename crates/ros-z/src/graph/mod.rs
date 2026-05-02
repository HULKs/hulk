use parking_lot::Mutex;
use std::{sync::Arc, time::Duration};
use tokio::sync::Notify;

mod discovery;
mod query;
mod snapshot;
mod state;

use discovery::install_liveliness;
pub use query::QosIncompatibility;
pub use snapshot::{GraphSnapshot, NodeSnapshot, ServiceSnapshot, TopicSnapshot};
use state::GraphData;

use crate::entity::Entity;
use crate::event::GraphEventManager;
use zenoh::{Result, Session, pubsub::Subscriber, session::ZenohId};

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

pub struct Graph {
    data: Arc<Mutex<GraphData>>,
    event_manager: Arc<GraphEventManager>,
    pub zid: ZenohId,
    /// Notified whenever an entity appears or disappears in the graph.
    ///
    /// Publishers use this to implement `wait_for_subscribers`: they register
    /// a `notified()` future before sampling the graph, then `await` it so no
    /// arrival is missed between the sample and the wait.
    pub(crate) change_notify: Arc<Notify>,
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
    pub async fn new(session: &Session) -> Result<Self> {
        Self::new_with_options(session, GraphOptions::default()).await
    }

    pub async fn new_with_options(session: &Session, options: GraphOptions) -> Result<Self> {
        let liveliness_pattern = format!("{}/**", crate::entity::ADMIN_SPACE);

        Self::new_with_pattern_and_options(
            session,
            liveliness_pattern,
            ros_z_protocol::format::parse_liveliness,
            options,
        )
        .await
    }

    /// Create a new Graph with a custom liveliness subscription pattern and parser
    ///
    /// # Arguments
    /// * `session` - Zenoh session
    /// * `liveliness_pattern` - Liveliness key expression pattern to subscribe to
    /// * `parser` - Function to parse liveliness key expressions into Entity
    ///
    /// The default ros-z liveliness pattern is `@ros_z/**`.
    pub async fn new_with_pattern<F>(
        session: &Session,
        liveliness_pattern: String,
        parser: F,
    ) -> Result<Self>
    where
        F: Fn(&zenoh::key_expr::KeyExpr) -> Result<Entity> + Send + Sync + 'static,
    {
        Self::new_with_pattern_and_options(
            session,
            liveliness_pattern,
            parser,
            GraphOptions::default(),
        )
        .await
    }

    async fn new_with_pattern_and_options<F>(
        session: &Session,
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
        let sub = install_liveliness(
            session,
            &liveliness_pattern,
            parser_arc,
            &options,
            graph_data.clone(),
            event_manager.clone(),
            change_notify.clone(),
            zid,
        )
        .await?;

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
        let key_expr = crate::entity::entity_to_liveliness_key_expr(&entity)?;
        let is_new = data.insert_local_entity(entity.clone(), key_expr);
        drop(data);

        if is_new {
            self.event_manager
                .trigger_graph_change(&entity, true, self.zid);
        }

        Ok(())
    }

    /// Remove a local entity from the graph
    pub fn remove_local_entity(&self, entity: &Entity) -> Result<()> {
        let mut data = self.data.lock();
        let key_expr = crate::entity::entity_to_liveliness_key_expr(entity)?;
        data.remove_local_entity(entity, &key_expr);
        drop(data);
        Ok(())
    }
}
