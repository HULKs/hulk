//! Graph plane types for discovery and liveliness.
//!
//! The graph plane tracks network topology through liveliness tokens:
//! - Sessions: `hulkz/graph/sessions/{namespace}/{session_id}`
//! - Nodes: `hulkz/graph/nodes/{namespace}/{node}`
//! - Publishers: `hulkz/graph/publishers/{namespace}/{node}/{scope}/{path}`
//! - Parameters: `hulkz/graph/parameters/{namespace}/{node}/{scope}/{path}`

use tokio::sync::mpsc;

use crate::key::Scope;

/// Event indicating a session joining or leaving the network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    /// A new session has joined.
    Joined(String),
    /// A session has left.
    Left(String),
}

/// Event indicating a node joining or leaving the network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeEvent {
    /// A new node has joined.
    Joined(String),
    /// A node has left.
    Left(String),
}

/// Event indicating a publisher appearing or disappearing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublisherEvent {
    /// A new publisher has been advertised.
    Advertised(PublisherInfo),
    /// A publisher has been unadvertised.
    Unadvertised(PublisherInfo),
}

/// Event indicating a parameter being declared or undeclared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterEvent {
    /// A new parameter has been declared.
    Declared(ParameterInfo),
    /// A parameter has been undeclared.
    Undeclared(ParameterInfo),
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

impl PublisherInfo {
    /// Parses publisher info from a graph publisher key.
    ///
    /// Expected format: `hulkz/graph/publishers/{namespace}/{node}/{scope}/{path...}`
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        let parts: Vec<&str> = key.split('/').collect();
        // Minimum: hulkz/graph/publishers/ns/node/scope/path (7 parts)
        if parts.len() < 7 {
            return None;
        }
        if parts[0] != "hulkz" || parts[1] != "graph" || parts[2] != "publishers" {
            return None;
        }

        let namespace = parts[3].to_string();
        let node = parts[4].to_string();
        let scope = match parts[5] {
            "global" => Scope::Global,
            "local" => Scope::Local,
            "private" => Scope::Private,
            _ => return None,
        };
        // Path is everything after the scope, joined back together
        let path = parts[6..].join("/");

        Some(Self {
            namespace,
            node,
            scope,
            path,
        })
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

impl ParameterInfo {
    /// Parses parameter info from a graph parameter key.
    ///
    /// Expected format: `hulkz/graph/parameters/{namespace}/{node}/{scope}/{path...}`
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        let parts: Vec<&str> = key.split('/').collect();
        // Minimum: hulkz/graph/parameters/ns/node/scope/path (7 parts)
        if parts.len() < 7 {
            return None;
        }
        if parts[0] != "hulkz" || parts[1] != "graph" || parts[2] != "parameters" {
            return None;
        }

        let namespace = parts[3].to_string();
        let node = parts[4].to_string();
        let scope = match parts[5] {
            "global" => Scope::Global,
            "local" => Scope::Local,
            "private" => Scope::Private,
            _ => return None,
        };
        // Path is everything after the scope, joined back together
        let path = parts[6..].join("/");

        Some(Self {
            namespace,
            node,
            scope,
            path,
        })
    }

    /// Returns the display path with scope prefix.
    ///
    /// - Global: `/path`
    /// - Local: `path`
    /// - Private: `~/path`
    pub fn display_path(&self) -> String {
        match self.scope {
            Scope::Global => format!("/{}", self.path),
            Scope::Local => self.path.clone(),
            Scope::Private => format!("~/{}", self.path),
        }
    }
}

/// Watcher for session events.
///
/// Receives events when sessions join or leave the network.
pub struct SessionWatcher {
    receiver: mpsc::Receiver<SessionEvent>,
}

impl SessionWatcher {
    pub(crate) fn new(receiver: mpsc::Receiver<SessionEvent>) -> Self {
        Self { receiver }
    }

    /// Receives the next session event.
    ///
    /// Returns `None` if the watcher has been closed.
    pub async fn recv(&mut self) -> Option<SessionEvent> {
        self.receiver.recv().await
    }

    /// Tries to receive a session event without blocking.
    pub fn try_recv(&mut self) -> Option<SessionEvent> {
        self.receiver.try_recv().ok()
    }
}

/// Watcher for node events.
///
/// Receives events when nodes join or leave the network.
pub struct NodeWatcher {
    receiver: mpsc::Receiver<NodeEvent>,
}

impl NodeWatcher {
    pub(crate) fn new(receiver: mpsc::Receiver<NodeEvent>) -> Self {
        Self { receiver }
    }

    /// Receives the next node event.
    ///
    /// Returns `None` if the watcher has been closed.
    pub async fn recv(&mut self) -> Option<NodeEvent> {
        self.receiver.recv().await
    }

    /// Tries to receive a node event without blocking.
    pub fn try_recv(&mut self) -> Option<NodeEvent> {
        self.receiver.try_recv().ok()
    }
}

/// Watcher for publisher events.
///
/// Receives events when publishers are advertised or unadvertised.
pub struct PublisherWatcher {
    receiver: mpsc::Receiver<PublisherEvent>,
}

impl PublisherWatcher {
    pub(crate) fn new(receiver: mpsc::Receiver<PublisherEvent>) -> Self {
        Self { receiver }
    }

    /// Receives the next publisher event.
    ///
    /// Returns `None` if the watcher has been closed.
    pub async fn recv(&mut self) -> Option<PublisherEvent> {
        self.receiver.recv().await
    }

    /// Tries to receive a publisher event without blocking.
    pub fn try_recv(&mut self) -> Option<PublisherEvent> {
        self.receiver.try_recv().ok()
    }
}

/// Watcher for parameter events.
///
/// Receives events when parameters are declared or undeclared.
pub struct ParameterWatcher {
    receiver: mpsc::Receiver<ParameterEvent>,
}

impl ParameterWatcher {
    pub(crate) fn new(receiver: mpsc::Receiver<ParameterEvent>) -> Self {
        Self { receiver }
    }

    /// Receives the next parameter event.
    ///
    /// Returns `None` if the watcher has been closed.
    pub async fn recv(&mut self) -> Option<ParameterEvent> {
        self.receiver.recv().await
    }

    /// Tries to receive a parameter event without blocking.
    pub fn try_recv(&mut self) -> Option<ParameterEvent> {
        self.receiver.try_recv().ok()
    }
}

/// Extracts the session ID from a graph session key.
///
/// Expected format: `hulkz/graph/sessions/{namespace}/{session_id}`
pub(crate) fn parse_session_key(key: &str) -> Option<String> {
    let parts: Vec<&str> = key.split('/').collect();
    if parts.len() != 5 {
        return None;
    }
    if parts[0] != "hulkz" || parts[1] != "graph" || parts[2] != "sessions" {
        return None;
    }
    Some(parts[4].to_string())
}

/// Extracts the node name from a graph node key.
///
/// Expected format: `hulkz/graph/nodes/{namespace}/{node}`
pub(crate) fn parse_node_key(key: &str) -> Option<String> {
    let parts: Vec<&str> = key.split('/').collect();
    if parts.len() != 5 {
        return None;
    }
    if parts[0] != "hulkz" || parts[1] != "graph" || parts[2] != "nodes" {
        return None;
    }
    Some(parts[4].to_string())
}

/// Information about a discovered session including its namespace.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionInfo {
    /// The namespace this session belongs to.
    pub namespace: String,
    /// The session ID (format: `{uuid}@{hostname}`).
    pub session_id: String,
}

impl SessionInfo {
    /// Parse a session info from a graph key.
    ///
    /// Expected format: `hulkz/graph/sessions/{namespace}/{session_id}`
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        let parts: Vec<&str> = key.split('/').collect();
        if parts.len() != 5 {
            return None;
        }
        if parts[0] != "hulkz" || parts[1] != "graph" || parts[2] != "sessions" {
            return None;
        }
        Some(Self {
            namespace: parts[3].to_string(),
            session_id: parts[4].to_string(),
        })
    }
}

/// Information about a discovered node including its namespace.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeInfo {
    /// The namespace this node belongs to.
    pub namespace: String,
    /// The node name.
    pub node: String,
}

impl NodeInfo {
    /// Parse a node info from a graph key.
    ///
    /// Expected format: `hulkz/graph/nodes/{namespace}/{node}`
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        let parts: Vec<&str> = key.split('/').collect();
        if parts.len() != 5 {
            return None;
        }
        if parts[0] != "hulkz" || parts[1] != "graph" || parts[2] != "nodes" {
            return None;
        }
        Some(Self {
            namespace: parts[3].to_string(),
            node: parts[4].to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_session_key_valid() {
        let key = "hulkz/graph/sessions/chappie/abc123@robot1";
        assert_eq!(parse_session_key(key), Some("abc123@robot1".to_string()));
    }

    #[test]
    fn parse_session_key_invalid() {
        assert_eq!(parse_session_key("hulkz/graph/sessions/chappie"), None);
        assert_eq!(parse_session_key("hulkz/graph/nodes/chappie/nav"), None);
    }

    #[test]
    fn parse_node_key_valid() {
        let key = "hulkz/graph/nodes/chappie/navigation";
        assert_eq!(parse_node_key(key), Some("navigation".to_string()));
    }

    #[test]
    fn parse_node_key_invalid() {
        assert_eq!(parse_node_key("hulkz/graph/nodes/chappie"), None);
        assert_eq!(parse_node_key("hulkz/graph/sessions/chappie/abc123"), None);
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
    fn publisher_info_from_key_private() {
        let key = "hulkz/graph/publishers/chappie/nav/private/debug/state";
        let info = PublisherInfo::from_key(key).unwrap();
        assert_eq!(info.namespace, "chappie");
        assert_eq!(info.node, "nav");
        assert_eq!(info.scope, Scope::Private);
        assert_eq!(info.path, "debug/state");
    }

    #[test]
    fn publisher_info_from_key_global() {
        let key = "hulkz/graph/publishers/chappie/coordinator/global/fleet_status";
        let info = PublisherInfo::from_key(key).unwrap();
        assert_eq!(info.namespace, "chappie");
        assert_eq!(info.node, "coordinator");
        assert_eq!(info.scope, Scope::Global);
        assert_eq!(info.path, "fleet_status");
    }

    #[test]
    fn publisher_info_from_key_invalid() {
        assert!(PublisherInfo::from_key("hulkz/graph/nodes/chappie/nav").is_none());
        assert!(PublisherInfo::from_key("hulkz/graph/publishers/chappie/nav").is_none());
    }

    #[test]
    fn parameter_info_from_key_global() {
        let key = "hulkz/graph/parameters/chappie/coordinator/global/fleet_id";
        let info = ParameterInfo::from_key(key).unwrap();
        assert_eq!(info.namespace, "chappie");
        assert_eq!(info.node, "coordinator");
        assert_eq!(info.scope, Scope::Global);
        assert_eq!(info.path, "fleet_id");
        assert_eq!(info.display_path(), "/fleet_id");
    }

    #[test]
    fn parameter_info_from_key_local() {
        let key = "hulkz/graph/parameters/chappie/motor/local/max_speed";
        let info = ParameterInfo::from_key(key).unwrap();
        assert_eq!(info.namespace, "chappie");
        assert_eq!(info.node, "motor");
        assert_eq!(info.scope, Scope::Local);
        assert_eq!(info.path, "max_speed");
        assert_eq!(info.display_path(), "max_speed");
    }

    #[test]
    fn parameter_info_from_key_private() {
        let key = "hulkz/graph/parameters/chappie/navigation/private/debug_level";
        let info = ParameterInfo::from_key(key).unwrap();
        assert_eq!(info.namespace, "chappie");
        assert_eq!(info.node, "navigation");
        assert_eq!(info.scope, Scope::Private);
        assert_eq!(info.path, "debug_level");
        assert_eq!(info.display_path(), "~/debug_level");
    }

    #[test]
    fn parameter_info_from_key_nested_path() {
        let key = "hulkz/graph/parameters/chappie/motor/local/wheel/radius";
        let info = ParameterInfo::from_key(key).unwrap();
        assert_eq!(info.namespace, "chappie");
        assert_eq!(info.node, "motor");
        assert_eq!(info.path, "wheel/radius");
        assert_eq!(info.display_path(), "wheel/radius");
    }

    #[test]
    fn parameter_info_from_key_invalid() {
        // Wrong prefix
        assert!(ParameterInfo::from_key("hulkz/param/read/local/chappie/speed").is_none());
        // Too few parts
        assert!(ParameterInfo::from_key("hulkz/graph/parameters/chappie/motor").is_none());
        // Wrong type
        assert!(ParameterInfo::from_key("hulkz/graph/nodes/chappie/nav").is_none());
        // Wrong plane
        assert!(
            ParameterInfo::from_key("hulkz/graph/publishers/chappie/nav/local/topic").is_none()
        );
    }
}
