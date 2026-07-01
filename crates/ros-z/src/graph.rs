mod discovery;
mod query;
mod state;

use std::{ops::Deref, sync::Arc};

use discovery::install_liveliness;
pub use query::{GraphRevisionWatch, QosIncompatibility, TypeMismatch};
pub use state::GraphData;
use state::GraphInner;

use crate::Result;
use crate::entity::{ADMIN_SPACE, Entity};
use zenoh::{Session, pubsub::Subscriber, session::ZenohId};

/// Opaque token identifying a local graph state revision.
///
/// Values support equality and monotonic recency comparisons within one [`Graph`]. Ordering is
/// intentionally exposed for revisions from the same graph instance or snapshot/subscription
/// stream. Do not compare revisions from independently created graphs.
///
/// A changed revision means consumers should resync from [`Graph::lock`]. Do not treat revisions as
/// stable event counts or as proof that the distributed graph is complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct GraphRevision(u64);

impl GraphRevision {
    /// Baseline revision before any observed effective graph changes.
    pub const INITIAL: Self = Self(0);

    pub(super) fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

pub struct Graph {
    inner: Arc<GraphInner>,
    pub zid: ZenohId,
    _subscriber: Subscriber<()>,
}

pub struct GraphLock<'a> {
    data: parking_lot::MutexGuard<'a, GraphData>,
}

impl Deref for GraphLock<'_> {
    type Target = GraphData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
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
    ///
    /// The graph is a live local observation of Zenoh liveliness. This returns after the
    /// liveliness subscription has been declared; historical liveliness samples may still arrive
    /// later and advance the graph revision.
    pub async fn new(session: &Session) -> Result<Self> {
        let liveliness_pattern = format!("{}/**", ADMIN_SPACE);
        let zid = session.zid();
        let inner = GraphInner::new();
        let sub = install_liveliness(session, &liveliness_pattern, Arc::clone(&inner)).await?;

        Ok(Self {
            inner,
            _subscriber: sub,
            zid,
        })
    }

    /// Return the current local graph change revision.
    ///
    /// The initial revision is `0`. Zenoh liveliness history and live events are reported through
    /// the same revision stream. A revision change is a resync signal, not a graph-completeness
    /// guarantee.
    pub fn revision(&self) -> GraphRevision {
        self.inner.revision()
    }

    /// Lock and inspect the current local graph data.
    ///
    /// The returned guard holds the graph mutex. Keep it short-lived: do not hold it across
    /// `.await`, UI rendering, or calls that may need graph updates. Use
    /// [`Graph::watch_revisions`] to wait for change signals and then inspect a fresh lock.
    pub fn lock(&self) -> GraphLock<'_> {
        GraphLock {
            data: self.inner.lock(),
        }
    }

    /// Subscribe to effective local graph changes.
    ///
    /// Watches start from the current revision and keep only the latest revision. Treat each
    /// observed change as a signal to inspect [`Graph::lock`] again.
    pub fn watch_revisions(&self) -> GraphRevisionWatch {
        GraphRevisionWatch::new(self.inner.watch_revisions())
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
        let key_expr = entity.liveliness_key_expr()?;
        self.inner.insert(key_expr, entity);
        Ok(())
    }

    /// Remove a local entity from the graph
    pub fn remove_local_entity(&self, entity: &Entity) -> Result<()> {
        let key_expr = entity.liveliness_key_expr()?;
        self.inner.remove(&key_expr);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::Arc,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

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
    async fn graph_wait_until_returns_immediately_when_predicate_is_true() -> Result<()> {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .map_err(|source| Error::zenoh("open Zenoh session", source))?;
        let graph = Graph::new(&session).await?;
        let entity = Entity::Node(NodeEntity::new(
            session.zid(),
            55,
            unique_node_name("wait_until_immediate"),
            String::new(),
        ));
        let expected = entity.clone();
        graph.add_local_entity(entity)?;

        let observed = tokio::time::timeout(
            Duration::from_millis(100),
            graph.wait_until(|view| view.entities().any(|candidate| candidate == &expected)),
        )
        .await
        .expect("wait_until should return immediately for a true predicate");

        assert!(observed);
        session
            .close()
            .await
            .map_err(|source| Error::zenoh("close Zenoh session", source))?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_wait_until_observes_later_graph_revision() -> Result<()> {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .map_err(|source| Error::zenoh("open Zenoh session", source))?;
        let graph = Arc::new(Graph::new(&session).await?);
        let entity = Entity::Node(NodeEntity::new(
            session.zid(),
            56,
            unique_node_name("wait_until_later"),
            String::new(),
        ));
        let expected = entity.clone();

        let waiter = tokio::spawn({
            let graph = Arc::clone(&graph);
            async move {
                graph
                    .wait_until(|view| view.entities().any(|candidate| candidate == &expected))
                    .await
            }
        });

        tokio::time::sleep(Duration::from_millis(10)).await;
        graph.add_local_entity(entity)?;

        let observed = tokio::time::timeout(Duration::from_millis(100), waiter)
            .await
            .expect("wait_until should observe the graph revision")
            .expect("wait_until task should not panic");

        assert!(observed);
        session
            .close()
            .await
            .map_err(|source| Error::zenoh("close Zenoh session", source))?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn graph_revision_watch_current_does_not_block_revision_publication() -> Result<()> {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .map_err(|source| Error::zenoh("open Zenoh session", source))?;
        let graph = Arc::new(Graph::new(&session).await?);
        let changes = graph.watch_revisions();
        let _held_revision = changes.current();
        let entity = Entity::Node(NodeEntity::new(
            session.zid(),
            54,
            unique_node_name("copied_revision_publish"),
            String::new(),
        ));

        let update = tokio::task::spawn_blocking({
            let graph = Arc::clone(&graph);
            move || graph.add_local_entity(entity)
        });

        tokio::time::timeout(Duration::from_millis(100), update)
            .await
            .expect("copied public revisions must not block graph updates")
            .expect("graph update task should not panic")?;

        session
            .close()
            .await
            .map_err(|source| Error::zenoh("close Zenoh session", source))?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn local_entity_add_and_remove_advance_graph_revision() -> Result<()> {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .map_err(|source| Error::zenoh("open Zenoh session", source))?;
        let graph = Graph::new(&session).await?;
        let mut changes = graph.watch_revisions();
        let initial_revision: GraphRevision = graph.revision();
        assert_eq!(initial_revision, GraphRevision::INITIAL);
        let entity = Entity::Node(NodeEntity::new(
            session.zid(),
            51,
            unique_node_name("local_update_revision"),
            String::new(),
        ));

        graph.add_local_entity(entity.clone())?;

        let add_revision = tokio::time::timeout(Duration::from_millis(100), changes.changed())
            .await
            .expect("local add should publish a graph revision")
            .expect("graph revision sender should stay alive");
        assert_ne!(add_revision, initial_revision);
        assert_eq!(graph.revision(), add_revision);

        graph.remove_local_entity(&entity)?;

        let remove_revision = tokio::time::timeout(Duration::from_millis(100), changes.changed())
            .await
            .expect("local remove should publish a graph revision")
            .expect("graph revision sender should stay alive");
        assert_ne!(remove_revision, add_revision);
        assert_eq!(graph.revision(), remove_revision);
        session
            .close()
            .await
            .map_err(|source| Error::zenoh("close Zenoh session", source))?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cloned_graph_data_captures_current_graph_revision() -> Result<()> {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .map_err(|source| Error::zenoh("open Zenoh session", source))?;
        let graph = Graph::new(&session).await?;
        let entity = Entity::Node(NodeEntity::new(
            session.zid(),
            53,
            unique_node_name("snapshot_revision"),
            String::new(),
        ));

        let initial_data = graph.lock().clone();
        assert_eq!(initial_data.revision(), GraphRevision::INITIAL);

        graph.add_local_entity(entity)?;

        let data_revision = graph.lock().revision();
        assert_eq!(data_revision, graph.revision());
        session
            .close()
            .await
            .map_err(|source| Error::zenoh("close Zenoh session", source))?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unchanged_local_readd_does_not_advance_graph_revision() -> Result<()> {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .map_err(|source| Error::zenoh("open Zenoh session", source))?;
        let graph = Graph::new(&session).await?;
        let mut changes = graph.watch_revisions();
        let entity = Entity::Node(NodeEntity::new(
            session.zid(),
            52,
            unique_node_name("local_readd_revision"),
            String::new(),
        ));

        graph.add_local_entity(entity.clone())?;
        tokio::time::timeout(Duration::from_millis(100), changes.changed())
            .await
            .expect("first local add should publish a graph revision")
            .expect("graph revision sender should stay alive");
        let revision_after_first_add = graph.revision();

        graph.add_local_entity(entity)?;

        assert!(
            tokio::time::timeout(Duration::from_millis(50), changes.changed())
                .await
                .is_err(),
            "unchanged local re-add must not publish a graph revision"
        );
        assert_eq!(graph.revision(), revision_after_first_add);
        session
            .close()
            .await
            .map_err(|source| Error::zenoh("close Zenoh session", source))?;
        Ok(())
    }
}
