//! Graph plane types for discovery and liveliness.
//!
//! The graph plane tracks network topology through liveliness tokens:
//! - Sessions: `hulkz/graph/sessions/{namespace}/{session_id}`
//! - Nodes: `hulkz/graph/nodes/{namespace}/{node}`
//! - Publishers: `hulkz/graph/publishers/{namespace}/{node}/{scope}/{path}`
//! - Parameters: `hulkz/graph/parameters/{namespace}/{node}/{scope}/{path}`
//!
//! # Graph Access API
//!
//! Use [`Session::graph()`](crate::Session::graph) to access the graph plane:
//!
//! ```rust,no_run
//! # use hulkz::{Session, Result};
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! let session = Session::create("robot").await?;
//!
//! // List nodes in current namespace
//! let nodes = session.graph().nodes().list().await?;
//!
//! // List publishers in a specific namespace
//! let pubs = session.graph().in_namespace("other").publishers().list().await?;
//!
//! // Watch all nodes across all namespaces
//! let (mut watcher, driver) = session.graph().all_namespaces().nodes().watch().await?;
//! tokio::spawn(driver);
//! while let Some(event) = watcher.recv().await {
//!     match event {
//!         hulkz::GraphEvent::Joined(info) => println!("+ {}", info.name),
//!         hulkz::GraphEvent::Left(info) => println!("- {}", info.name),
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use std::fmt;
use std::future::Future;
use std::marker::PhantomData;

use serde::Serialize;
use tokio::sync::mpsc;
use tracing::error;
use zenoh::handlers::FifoChannelHandler;
use zenoh::sample::{Sample, SampleKind};

use crate::error::Result;
use crate::key::GraphKey;
use crate::{Error, Scope, Session};

/// Event for any graph entity appearing or disappearing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphEvent<T> {
    /// An entity has joined/appeared.
    Joined(T),
    /// An entity has left/disappeared.
    Left(T),
}

/// Trait for types that can be parsed from graph keys.
pub trait GraphInfo: Sized + Clone + Send + 'static {
    /// Parse from a Zenoh key expression.
    fn from_key(key: &str) -> Result<Self>;
}

/// Information about a discovered session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SessionInfo {
    /// The namespace this session belongs to.
    pub namespace: String,
    /// The session ID (format: `{uuid}@{hostname}`).
    pub id: String,
}

impl fmt::Display for SessionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl GraphInfo for SessionInfo {
    fn from_key(key: &str) -> Result<Self> {
        let parts: Vec<&str> = key.split('/').collect();
        if parts.len() != 5 {
            return Err(Error::GraphKeyParsing {
                key: key.to_string(),
                reason: format!("expected 5 parts, found {}", parts.len()),
            });
        }
        if parts[0] != "hulkz" || parts[1] != "graph" || parts[2] != "sessions" {
            return Err(Error::GraphKeyParsing {
                key: key.to_string(),
                reason: "invalid prefix".to_string(),
            });
        }
        Ok(Self {
            namespace: parts[3].to_string(),
            id: parts[4].to_string(),
        })
    }
}

/// Information about a discovered node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct NodeInfo {
    /// The namespace this node belongs to.
    pub namespace: String,
    /// The node name.
    pub name: String,
}

impl fmt::Display for NodeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl GraphInfo for NodeInfo {
    fn from_key(key: &str) -> Result<Self> {
        let parts: Vec<&str> = key.split('/').collect();
        if parts.len() != 5 {
            return Err(Error::GraphKeyParsing {
                key: key.to_string(),
                reason: format!("expected 5 parts, found {}", parts.len()),
            });
        }
        if parts[0] != "hulkz" || parts[1] != "graph" || parts[2] != "nodes" {
            return Err(Error::GraphKeyParsing {
                key: key.to_string(),
                reason: "invalid prefix".to_string(),
            });
        }
        Ok(Self {
            namespace: parts[3].to_string(),
            name: parts[4].to_string(),
        })
    }
}

/// Information about a discovered publisher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublisherInfo {
    /// The namespace this publisher belongs to.
    pub namespace: String,
    /// The node name that owns this publisher.
    pub node: String,
    /// The scope of the published topic.
    pub scope: Scope,
    /// The path/topic being published.
    pub path: String,
}

impl GraphInfo for PublisherInfo {
    fn from_key(key: &str) -> Result<Self> {
        let parts: Vec<&str> = key.split('/').collect();
        if parts.len() < 7 {
            return Err(Error::GraphKeyParsing {
                key: key.to_string(),
                reason: format!("expected at least 7 parts, found {}", parts.len()),
            });
        }
        if parts[0] != "hulkz" || parts[1] != "graph" || parts[2] != "publishers" {
            return Err(Error::GraphKeyParsing {
                key: key.to_string(),
                reason: "invalid prefix".to_string(),
            });
        }

        let namespace = parts[3].to_string();
        let node = parts[4].to_string();
        let scope = match parts[5] {
            "global" => Scope::Global,
            "local" => Scope::Local,
            "private" => Scope::Private,
            _ => {
                return Err(Error::GraphKeyParsing {
                    key: key.to_string(),
                    reason: format!("invalid scope '{}'", parts[5]),
                })
            }
        };
        let path = parts[6..].join("/");

        Ok(Self {
            namespace,
            node,
            scope,
            path,
        })
    }
}

impl PublisherInfo {
    /// Returns the display path with scope prefix.
    pub fn display_path(&self) -> String {
        match self.scope {
            Scope::Global => format!("/{}", self.path),
            Scope::Local => self.path.clone(),
            Scope::Private => format!("~/{}", self.path),
        }
    }
}

/// Information about a discovered parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterInfo {
    /// The namespace this parameter belongs to.
    pub namespace: String,
    /// The node name that owns this parameter.
    pub node: String,
    /// The scope of the parameter.
    pub scope: Scope,
    /// The parameter path.
    pub path: String,
}

impl GraphInfo for ParameterInfo {
    fn from_key(key: &str) -> Result<Self> {
        let parts: Vec<&str> = key.split('/').collect();
        if parts.len() < 7 {
            return Err(Error::GraphKeyParsing {
                key: key.to_string(),
                reason: format!("expected at least 7 parts, found {}", parts.len()),
            });
        }
        if parts[0] != "hulkz" || parts[1] != "graph" || parts[2] != "parameters" {
            return Err(Error::GraphKeyParsing {
                key: key.to_string(),
                reason: "invalid prefix".to_string(),
            });
        }

        let namespace = parts[3].to_string();
        let node = parts[4].to_string();
        let scope = match parts[5] {
            "global" => Scope::Global,
            "local" => Scope::Local,
            "private" => Scope::Private,
            _ => {
                return Err(Error::GraphKeyParsing {
                    key: key.to_string(),
                    reason: format!("invalid scope '{}'", parts[5]),
                })
            }
        };
        let path = parts[6..].join("/");

        Ok(Self {
            namespace,
            node,
            scope,
            path,
        })
    }
}

impl ParameterInfo {
    /// Returns the display path with scope prefix.
    pub fn display_path(&self) -> String {
        match self.scope {
            Scope::Global => format!("/{}", self.path),
            Scope::Local => self.path.clone(),
            Scope::Private => format!("~/{}", self.path),
        }
    }
}

/// Namespace scope for graph queries.
#[derive(Clone)]
enum NamespaceScope {
    /// Use session's namespace.
    Session,
    /// Use a specific namespace.
    Specific(String),
    /// Query all namespaces.
    All,
}

/// Entry point for graph plane operations.
///
/// Created via [`Session::graph()`](crate::Session::graph).
pub struct GraphAccess<'session> {
    session: &'session Session,
    namespace: NamespaceScope,
}

impl<'session> GraphAccess<'session> {
    pub(crate) fn new(session: &'session Session) -> Self {
        Self {
            session,
            namespace: NamespaceScope::Session,
        }
    }

    /// Query a specific namespace instead of the session's namespace.
    pub fn in_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = NamespaceScope::Specific(namespace.into());
        self
    }

    /// Query all namespaces.
    pub fn all_namespaces(mut self) -> Self {
        self.namespace = NamespaceScope::All;
        self
    }

    /// Access session discovery operations.
    pub fn sessions(self) -> EntityAccess<'session, SessionInfo> {
        EntityAccess::new(self.session, self.namespace, GraphEntity::Sessions)
    }

    /// Access node discovery operations.
    pub fn nodes(self) -> EntityAccess<'session, NodeInfo> {
        EntityAccess::new(self.session, self.namespace, GraphEntity::Nodes)
    }

    /// Access publisher discovery operations.
    pub fn publishers(self) -> EntityAccess<'session, PublisherInfo> {
        EntityAccess::new(self.session, self.namespace, GraphEntity::Publishers)
    }

    /// Access parameter discovery operations.
    pub fn parameters(self) -> EntityAccess<'session, ParameterInfo> {
        EntityAccess::new(self.session, self.namespace, GraphEntity::Parameters)
    }
}

/// Graph entity type for pattern generation.
#[derive(Clone, Copy)]
enum GraphEntity {
    Sessions,
    Nodes,
    Publishers,
    Parameters,
}

/// Access to a specific entity type for list/watch operations.
pub struct EntityAccess<'session, T> {
    session: &'session Session,
    namespace: NamespaceScope,
    entity: GraphEntity,
    _phantom: PhantomData<T>,
}

impl<'session, T: GraphInfo> EntityAccess<'session, T> {
    fn new(session: &'session Session, namespace: NamespaceScope, entity: GraphEntity) -> Self {
        Self {
            session,
            namespace,
            entity,
            _phantom: PhantomData,
        }
    }

    /// Get the pattern for this entity/namespace combination.
    fn pattern(&self) -> String {
        let ns = match &self.namespace {
            NamespaceScope::Session => self.session.namespace(),
            NamespaceScope::Specific(ns) => ns.as_str(),
            NamespaceScope::All => "",
        };

        match (&self.namespace, self.entity) {
            (NamespaceScope::All, GraphEntity::Sessions) => GraphKey::all_sessions(),
            (NamespaceScope::All, GraphEntity::Nodes) => GraphKey::all_nodes(),
            (NamespaceScope::All, GraphEntity::Publishers) => GraphKey::all_publishers(),
            (NamespaceScope::All, GraphEntity::Parameters) => GraphKey::all_parameters(),
            (_, GraphEntity::Sessions) => GraphKey::sessions_in(ns),
            (_, GraphEntity::Nodes) => GraphKey::nodes_in(ns),
            (_, GraphEntity::Publishers) => GraphKey::publishers_in(ns),
            (_, GraphEntity::Parameters) => GraphKey::parameters_in(ns),
        }
    }

    /// List all entities matching this query.
    pub async fn list(&self) -> Result<Vec<T>> {
        let pattern = self.pattern();
        let replies = self.session.zenoh().liveliness().get(&pattern).await?;
        let mut results = Vec::new();

        while let Ok(reply) = replies.recv_async().await {
            let sample = match reply.result() {
                Ok(sample) => sample,
                Err(error) => {
                    error!("Error receiving graph sample: {}", error);
                    continue;
                }
            };
            if let Ok(info) = T::from_key(sample.key_expr().as_str()) {
                results.push(info);
            }
        }

        Ok(results)
    }

    /// Watch for entity events.
    ///
    /// Returns a watcher and a driver future that must be spawned.
    pub async fn watch(
        &self,
    ) -> Result<(
        Watcher<GraphEvent<T>>,
        impl Future<Output = Result<()>> + Send,
    )> {
        let pattern = self.pattern();
        let subscriber = self
            .session
            .zenoh()
            .liveliness()
            .declare_subscriber(&pattern)
            .await?;

        let (tx, rx) = mpsc::channel(32);
        let watcher = Watcher::new(rx);

        let driver = drive_watcher(subscriber, tx);

        Ok((watcher, driver))
    }
}

/// Generic watcher for graph events.
///
/// Receives events when entities join or leave the network.
pub struct Watcher<E> {
    receiver: mpsc::Receiver<E>,
}

impl<E> Watcher<E> {
    pub(crate) fn new(receiver: mpsc::Receiver<E>) -> Self {
        Self { receiver }
    }

    /// Receives the next event.
    ///
    /// Returns `None` if the watcher has been closed.
    pub async fn recv(&mut self) -> Option<E> {
        self.receiver.recv().await
    }

    /// Tries to receive an event without blocking.
    pub fn try_recv(&mut self) -> Option<E> {
        self.receiver.try_recv().ok()
    }
}

/// Generic driver for graph watchers.
async fn drive_watcher<T: GraphInfo>(
    subscriber: zenoh::pubsub::Subscriber<FifoChannelHandler<Sample>>,
    tx: mpsc::Sender<GraphEvent<T>>,
) -> Result<()> {
    loop {
        let sample = subscriber.recv_async().await?;
        match T::from_key(sample.key_expr().as_str()) {
            Ok(info) => {
                let event = if sample.kind() == SampleKind::Put {
                    GraphEvent::Joined(info)
                } else {
                    GraphEvent::Left(info)
                };
                if tx.send(event).await.is_err() {
                    break;
                }
            }
            Err(err) => {
                error!("Error parsing graph key '{}': {}", sample.key_expr(), err);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_session_key_valid() {
        let key = "hulkz/graph/sessions/chappie/abc123@robot1";
        let info = SessionInfo::from_key(key).unwrap();
        assert_eq!(info.namespace, "chappie");
        assert_eq!(info.id, "abc123@robot1");
    }

    #[test]
    fn parse_session_key_invalid() {
        assert!(SessionInfo::from_key("invalid").is_err());
        assert!(SessionInfo::from_key("hulkz/graph/nodes/ns/node").is_err());
    }

    #[test]
    fn parse_node_key_valid() {
        let key = "hulkz/graph/nodes/robot/navigation";
        let info = NodeInfo::from_key(key).unwrap();
        assert_eq!(info.namespace, "robot");
        assert_eq!(info.name, "navigation");
    }

    #[test]
    fn parse_node_key_invalid() {
        assert!(NodeInfo::from_key("invalid").is_err());
        assert!(NodeInfo::from_key("hulkz/graph/sessions/ns/id").is_err());
    }

    #[test]
    fn publisher_info_from_key_local() {
        let key = "hulkz/graph/publishers/chappie/vision/local/camera/front";
        let info = PublisherInfo::from_key(key).unwrap();
        assert_eq!(info.namespace, "chappie");
        assert_eq!(info.node, "vision");
        assert_eq!(info.scope, Scope::Local);
        assert_eq!(info.path, "camera/front");
    }

    #[test]
    fn publisher_info_from_key_global() {
        let key = "hulkz/graph/publishers/robot/sensors/global/fleet_status";
        let info = PublisherInfo::from_key(key).unwrap();
        assert_eq!(info.scope, Scope::Global);
        assert_eq!(info.path, "fleet_status");
    }

    #[test]
    fn publisher_info_from_key_private() {
        let key = "hulkz/graph/publishers/ns/node/private/debug/state";
        let info = PublisherInfo::from_key(key).unwrap();
        assert_eq!(info.scope, Scope::Private);
        assert_eq!(info.path, "debug/state");
    }

    #[test]
    fn publisher_info_from_key_invalid() {
        assert!(PublisherInfo::from_key("invalid").is_err());
        assert!(PublisherInfo::from_key("hulkz/graph/publishers/ns/node").is_err());
        assert!(PublisherInfo::from_key("hulkz/graph/publishers/ns/node/bad_scope/path").is_err());
    }

    #[test]
    fn parameter_info_from_key_local() {
        let key = "hulkz/graph/parameters/chappie/motor/local/max_speed";
        let info = ParameterInfo::from_key(key).unwrap();
        assert_eq!(info.namespace, "chappie");
        assert_eq!(info.node, "motor");
        assert_eq!(info.scope, Scope::Local);
        assert_eq!(info.path, "max_speed");
    }

    #[test]
    fn parameter_info_from_key_global() {
        let key = "hulkz/graph/parameters/robot/config/global/fleet_id";
        let info = ParameterInfo::from_key(key).unwrap();
        assert_eq!(info.scope, Scope::Global);
        assert_eq!(info.path, "fleet_id");
    }

    #[test]
    fn parameter_info_from_key_private() {
        let key = "hulkz/graph/parameters/ns/node/private/debug/level";
        let info = ParameterInfo::from_key(key).unwrap();
        assert_eq!(info.scope, Scope::Private);
        assert_eq!(info.path, "debug/level");
    }

    #[test]
    fn parameter_info_from_key_nested_path() {
        let key = "hulkz/graph/parameters/ns/node/local/a/b/c/d";
        let info = ParameterInfo::from_key(key).unwrap();
        assert_eq!(info.path, "a/b/c/d");
    }

    #[test]
    fn parameter_info_from_key_invalid() {
        assert!(ParameterInfo::from_key("invalid").is_err());
        assert!(ParameterInfo::from_key("hulkz/graph/parameters/ns/node").is_err());
    }
}
