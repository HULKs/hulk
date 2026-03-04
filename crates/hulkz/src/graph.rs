//! Graph plane types for discovery and liveliness.
//!
//! The graph plane tracks network topology through liveliness tokens:
//! - Sessions: `hulkz/graph/{domain_id}/{zenoh_id}/sessions/{namespace}/{session_id}`
//! - Nodes: `hulkz/graph/{domain_id}/{zenoh_id}/nodes/{namespace}/{node}`
//! - Publishers: `hulkz/graph/{domain_id}/{zenoh_id}/publishers/{namespace}/{node}/{topic_encoded}`
//! - Parameters: `hulkz/graph/{domain_id}/{zenoh_id}/parameters/{namespace}/{node}/{topic_encoded}`
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
use tracing::{error, warn};
use zenoh::handlers::FifoChannelHandler;
use zenoh::sample::{Sample, SampleKind};

use crate::error::Result;
use crate::key::{GraphKey, ROOT};
use crate::topic::decode_topic_segment;
use crate::{Error, Session};

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

struct ParsedGraphKey {
    domain_id: u32,
    zenoh_id: String,
    tail: Vec<String>,
}

fn parse_graph_key(key: &str, entity: &str, tail_len: usize) -> Result<ParsedGraphKey> {
    let parts: Vec<&str> = key.split('/').collect();
    let expected_parts = 5 + tail_len;

    if parts.len() != expected_parts {
        return Err(Error::GraphKeyParsing {
            key: key.to_string(),
            reason: format!("expected {expected_parts} parts, found {}", parts.len()),
        });
    }

    if parts[0] != ROOT || parts[1] != "graph" || parts[4] != entity {
        return Err(Error::GraphKeyParsing {
            key: key.to_string(),
            reason: "invalid prefix".to_string(),
        });
    }

    let domain_segment = parts[2];
    let domain_id = domain_segment
        .parse::<u32>()
        .map_err(|error| Error::GraphKeyParsing {
            key: key.to_string(),
            reason: format!("invalid domain id '{domain_segment}': {error}"),
        })?;

    Ok(ParsedGraphKey {
        domain_id,
        zenoh_id: parts[3].to_string(),
        tail: parts[5..].iter().map(|segment| (*segment).to_string()).collect(),
    })
}

/// Information about a discovered session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SessionInfo {
    /// ROS domain id extracted from graph key.
    pub domain_id: u32,
    /// Zenoh id extracted from graph key.
    pub zenoh_id: String,
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
        let parsed = parse_graph_key(key, "sessions", 2)?;
        Ok(Self {
            domain_id: parsed.domain_id,
            zenoh_id: parsed.zenoh_id,
            namespace: parsed.tail[0].clone(),
            id: parsed.tail[1].clone(),
        })
    }
}

/// Information about a discovered node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct NodeInfo {
    /// ROS domain id extracted from graph key.
    pub domain_id: u32,
    /// Zenoh id extracted from graph key.
    pub zenoh_id: String,
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
        let parsed = parse_graph_key(key, "nodes", 2)?;
        Ok(Self {
            domain_id: parsed.domain_id,
            zenoh_id: parsed.zenoh_id,
            namespace: parsed.tail[0].clone(),
            name: parsed.tail[1].clone(),
        })
    }
}

/// Information about a discovered publisher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublisherInfo {
    /// ROS domain id extracted from graph key.
    pub domain_id: u32,
    /// Zenoh id extracted from graph key.
    pub zenoh_id: String,
    /// The namespace this publisher belongs to.
    pub namespace: String,
    /// The node name that owns this publisher.
    pub node: String,
    /// Canonical resolved topic being published.
    pub topic: String,
}

impl GraphInfo for PublisherInfo {
    fn from_key(key: &str) -> Result<Self> {
        let parsed = parse_graph_key(key, "publishers", 3)?;
        let topic = decode_topic_segment(&parsed.tail[2])?;

        Ok(Self {
            domain_id: parsed.domain_id,
            zenoh_id: parsed.zenoh_id,
            namespace: parsed.tail[0].clone(),
            node: parsed.tail[1].clone(),
            topic,
        })
    }
}

/// Information about a discovered parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterInfo {
    /// ROS domain id extracted from graph key.
    pub domain_id: u32,
    /// Zenoh id extracted from graph key.
    pub zenoh_id: String,
    /// The namespace this parameter belongs to.
    pub namespace: String,
    /// The node name that owns this parameter.
    pub node: String,
    /// Canonical resolved topic for this parameter.
    pub topic: String,
}

impl GraphInfo for ParameterInfo {
    fn from_key(key: &str) -> Result<Self> {
        let parsed = parse_graph_key(key, "parameters", 3)?;
        let topic = decode_topic_segment(&parsed.tail[2])?;

        Ok(Self {
            domain_id: parsed.domain_id,
            zenoh_id: parsed.zenoh_id,
            namespace: parsed.tail[0].clone(),
            node: parsed.tail[1].clone(),
            topic,
        })
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
            match T::from_key(sample.key_expr().as_str()) {
                Ok(info) => results.push(info),
                Err(err) => {
                    warn!("Failed to parse graph key '{}': {err}", sample.key_expr());
                }
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
        let key = "hulkz/graph/0/zid-1/sessions/chappie/abc123@robot1";
        let info = SessionInfo::from_key(key).unwrap();
        assert_eq!(info.domain_id, 0);
        assert_eq!(info.zenoh_id, "zid-1");
        assert_eq!(info.namespace, "chappie");
        assert_eq!(info.id, "abc123@robot1");
    }

    #[test]
    fn parse_session_key_invalid() {
        assert!(SessionInfo::from_key("invalid").is_err());
        assert!(SessionInfo::from_key("hulkz/graph/0/zid/nodes/ns/node").is_err());
    }

    #[test]
    fn parse_node_key_valid() {
        let key = "hulkz/graph/7/zid-9/nodes/robot/navigation";
        let info = NodeInfo::from_key(key).unwrap();
        assert_eq!(info.domain_id, 7);
        assert_eq!(info.zenoh_id, "zid-9");
        assert_eq!(info.namespace, "robot");
        assert_eq!(info.name, "navigation");
    }

    #[test]
    fn parse_node_key_invalid() {
        assert!(NodeInfo::from_key("invalid").is_err());
        assert!(NodeInfo::from_key("hulkz/graph/0/zid/sessions/ns/id").is_err());
    }

    #[test]
    fn publisher_info_from_key() {
        let key = "hulkz/graph/0/zid-1/publishers/chappie/vision/chappie%2Fcamera%2Ffront";
        let info = PublisherInfo::from_key(key).unwrap();
        assert_eq!(info.domain_id, 0);
        assert_eq!(info.zenoh_id, "zid-1");
        assert_eq!(info.namespace, "chappie");
        assert_eq!(info.node, "vision");
        assert_eq!(info.topic, "chappie/camera/front");
    }

    #[test]
    fn publisher_info_from_key_invalid() {
        assert!(PublisherInfo::from_key("invalid").is_err());
        assert!(PublisherInfo::from_key("hulkz/graph/0/zid/publishers/ns/node").is_err());
        assert!(PublisherInfo::from_key("hulkz/graph/0/zid/publishers/ns/node/%ZZ").is_err());
    }

    #[test]
    fn parameter_info_from_key() {
        let key = "hulkz/graph/0/zid-1/parameters/chappie/motor/chappie%2Fmotor%2Fmax_speed";
        let info = ParameterInfo::from_key(key).unwrap();
        assert_eq!(info.domain_id, 0);
        assert_eq!(info.zenoh_id, "zid-1");
        assert_eq!(info.namespace, "chappie");
        assert_eq!(info.node, "motor");
        assert_eq!(info.topic, "chappie/motor/max_speed");
    }

    #[test]
    fn parameter_info_from_key_invalid() {
        assert!(ParameterInfo::from_key("invalid").is_err());
        assert!(ParameterInfo::from_key("hulkz/graph/0/zid/parameters/ns/node").is_err());
    }
}
