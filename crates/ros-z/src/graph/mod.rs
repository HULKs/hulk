use parking_lot::Mutex;
use std::{sync::Arc, time::Duration};
use tokio::sync::Notify;

mod discovery;
mod query;
mod snapshot;
mod state;

use discovery::install_liveliness;
pub use query::{GraphView, QosIncompatibility};
pub use snapshot::{GraphSnapshot, NodeSnapshot, ServiceSnapshot, TopicSnapshot};
use state::{GraphData, GraphMutation};

use crate::Result;
use crate::entity::{ADMIN_SPACE, Entity, entity_to_liveliness_key_expr};
use crate::event::GraphEventManager;
use zenoh::{Session, pubsub::Subscriber, session::ZenohId};

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

pub(in crate::graph) fn dispatch_graph_mutation(
    mutation: GraphMutation,
    event_manager: &GraphEventManager,
    change_notify: &Notify,
    zid: ZenohId,
) {
    let changed = !matches!(mutation, GraphMutation::Unchanged);
    match mutation {
        GraphMutation::Inserted(entity) => {
            event_manager.trigger_graph_change(&entity, true, zid);
        }
        GraphMutation::Removed(entity) => {
            event_manager.trigger_graph_change(&entity, false, zid);
        }
        GraphMutation::Replaced { old, new } => {
            event_manager.trigger_graph_change(&old, false, zid);
            event_manager.trigger_graph_change(&new, true, zid);
        }
        GraphMutation::Unchanged => {}
    }

    if changed {
        change_notify.notify_waiters();
    }
}

impl Graph {
    /// Create a new Graph using the native ros-z liveliness protocol.
    pub async fn new(session: &Session) -> Result<Self> {
        Self::new_with_options(session, GraphOptions::default()).await
    }

    pub async fn new_with_options(session: &Session, options: GraphOptions) -> Result<Self> {
        let liveliness_pattern = format!("{}/**", ADMIN_SPACE);

        Self::new_with_pattern_and_options(
            session,
            liveliness_pattern,
            |key_expr| Ok(ros_z_protocol::format::parse_liveliness(key_expr)?),
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
        F: Fn(&zenoh::key_expr::KeyExpr) -> crate::Result<Entity> + Send + Sync + 'static,
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
        F: Fn(&zenoh::key_expr::KeyExpr) -> crate::Result<Entity> + Send + Sync + 'static,
    {
        let zid = session.zid();
        let parser_arc = Arc::new(parser);
        let graph_data = Arc::new(Mutex::new(GraphData::new()));
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
            Entity::Endpoint(endpoint) => endpoint.node.z_id == self.zid,
        }
    }

    /// Add a local entity to the graph for immediate discovery
    /// This is used to make local publishers/subscriptions/services/clients
    /// immediately visible in graph queries without waiting for Zenoh liveliness propagation
    pub fn add_local_entity(&self, entity: Entity) -> Result<()> {
        let key_expr = entity_to_liveliness_key_expr(&entity)?;
        let mutation = self.data.lock().insert(key_expr, entity);
        dispatch_graph_mutation(mutation, &self.event_manager, &self.change_notify, self.zid);
        Ok(())
    }

    /// Remove a local entity from the graph
    pub fn remove_local_entity(&self, entity: &Entity) -> Result<()> {
        let key_expr = entity_to_liveliness_key_expr(entity)?;
        let mutation = self.data.lock().remove(&key_expr);
        dispatch_graph_mutation(mutation, &self.event_manager, &self.change_notify, self.zid);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use crate::{
        Error, Result,
        entity::{Entity, NodeEntity},
    };

    use super::*;

    fn unique_node_name(prefix: &str) -> String {
        format!(
            "{prefix}_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after Unix epoch")
                .as_nanos()
        )
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn local_entity_add_notifies_graph_waiters() -> Result<()> {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .map_err(|source| Error::zenoh("open Zenoh session", source))?;
        let graph = Graph::new(&session).await?;
        let entity = Entity::Node(NodeEntity::new(
            session.zid(),
            51,
            unique_node_name("local_add_notify"),
            String::new(),
        ));

        let notified = graph.change_notify.notified();
        graph.add_local_entity(entity)?;

        tokio::time::timeout(Duration::from_millis(100), notified)
            .await
            .expect("local add should notify graph waiters");
        session
            .close()
            .await
            .map_err(|source| Error::zenoh("close Zenoh session", source))?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn local_entity_remove_notifies_graph_waiters() -> Result<()> {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .map_err(|source| Error::zenoh("open Zenoh session", source))?;
        let graph = Graph::new(&session).await?;
        let entity = Entity::Node(NodeEntity::new(
            session.zid(),
            52,
            unique_node_name("local_remove_notify"),
            String::new(),
        ));

        graph.add_local_entity(entity.clone())?;
        let notified = graph.change_notify.notified();
        graph.remove_local_entity(&entity)?;

        tokio::time::timeout(Duration::from_millis(100), notified)
            .await
            .expect("local remove should notify graph waiters");
        session
            .close()
            .await
            .map_err(|source| Error::zenoh("close Zenoh session", source))?;
        Ok(())
    }
}
