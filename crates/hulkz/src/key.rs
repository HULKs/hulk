//! Key expression building for the Hulkz key space.
//!
//! Provides type-safe construction of Zenoh key expressions following the
//! Hulkz 5-plane architecture: Data, View, Param, and Graph.

use std::fmt;

pub use crate::error::KeyError;

/// Root prefix for all Hulkz key expressions.
pub const ROOT: &str = "hulkz";

/// Functional planes in the Hulkz architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Plane {
    /// High-bandwidth, low-latency production data (CDR encoded).
    Data,
    /// Human-readable JSON mirror of Data for debugging.
    View,
    /// Configuration state with read/write branches (JSON encoded).
    Param,
    /// Network topology, node discovery, and heartbeats (JSON encoded).
    Graph,
}

impl Plane {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Plane::Data => "data",
            Plane::View => "view",
            Plane::Param => "param",
            Plane::Graph => "graph",
        }
    }
}

impl fmt::Display for Plane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Hierarchical scope for data visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    /// Fleet-wide shared data.
    Global,
    /// Robot-wide public data (namespaced).
    Local,
    /// Node-internal debug data (namespaced + node name).
    Private,
}

impl Scope {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Scope::Global => "global",
            Scope::Local => "local",
            Scope::Private => "private",
        }
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Intent for parameter access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamIntent {
    /// Read parameter value (query response + publish updates).
    Read,
    /// Write parameter value (accepts updates).
    Write,
}

impl ParamIntent {
    pub const fn as_str(&self) -> &'static str {
        match self {
            ParamIntent::Read => "read",
            ParamIntent::Write => "write",
        }
    }
}

impl fmt::Display for ParamIntent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Builder for constructing Hulkz key expressions.
///
/// # Key Space Schema
///
/// ```text
/// hulkz/{plane}/{scope}/[{namespace}/[{node}/]]{key}
/// ```
///
/// # Example
///
/// ```
/// use hulkz::key::{KeyExpr, Plane, Scope};
///
/// let key = KeyExpr::new(Plane::Data, Scope::Local, "camera/front")
///     .namespace("chappie")
///     .build()
///     .unwrap();
///
/// assert_eq!(key, "hulkz/data/local/chappie/camera/front");
/// ```
#[derive(Debug, Clone)]
pub struct KeyExpr {
    plane: Plane,
    scope: Scope,
    namespace: Option<String>,
    node: Option<String>,
    key: String,
}

impl KeyExpr {
    pub fn new(plane: Plane, scope: Scope, key: impl Into<String>) -> Self {
        Self {
            plane,
            scope,
            namespace: None,
            node: None,
            key: key.into(),
        }
    }

    /// Sets the namespace (robot identifier). Required for Local and Private scopes.
    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Sets the node name. Required for Private scope.
    pub fn node(mut self, node: impl Into<String>) -> Self {
        self.node = Some(node.into());
        self
    }

    /// Builds the complete key expression string.
    pub fn build(self) -> Result<String, KeyError> {
        match self.scope {
            Scope::Global => Ok(format!(
                "{}/{}/{}/{}",
                ROOT, self.plane, self.scope, self.key
            )),
            Scope::Local => {
                let namespace = self.namespace.ok_or(KeyError::MissingNamespace)?;
                Ok(format!(
                    "{}/{}/{}/{}/{}",
                    ROOT, self.plane, self.scope, namespace, self.key
                ))
            }
            Scope::Private => {
                let namespace = self.namespace.ok_or(KeyError::MissingNamespace)?;
                let node = self.node.ok_or(KeyError::MissingNode)?;
                Ok(format!(
                    "{}/{}/{}/{}/{}/{}",
                    ROOT, self.plane, self.scope, namespace, node, self.key
                ))
            }
        }
    }
}

/// Builds a graph session liveliness key.
///
/// Schema: `hulkz/graph/sessions/{namespace}/{session_id}`
pub fn graph_session_key(namespace: &str, session_id: &str) -> String {
    format!("{}/graph/sessions/{}/{}", ROOT, namespace, session_id)
}

/// Builds a graph node liveliness key.
///
/// Schema: `hulkz/graph/nodes/{namespace}/{node}`
pub fn graph_node_key(namespace: &str, node: &str) -> String {
    format!("{}/graph/nodes/{}/{}", ROOT, namespace, node)
}

/// Builds a graph publisher liveliness key.
///
/// Schema: `hulkz/graph/publishers/{namespace}/{node}/{scope}/{path}`
pub fn graph_publisher_key(namespace: &str, node: &str, scope: Scope, path: &str) -> String {
    format!(
        "{}/graph/publishers/{}/{}/{}/{}",
        ROOT, namespace, node, scope, path
    )
}

/// Pattern for querying graph sessions.
///
/// Schema: `hulkz/graph/sessions/{namespace}/*`
pub fn graph_sessions_pattern(namespace: &str) -> String {
    format!("{}/graph/sessions/{}/*", ROOT, namespace)
}

/// Pattern for querying graph nodes.
///
/// Schema: `hulkz/graph/nodes/{namespace}/*`
pub fn graph_nodes_pattern(namespace: &str) -> String {
    format!("{}/graph/nodes/{}/*", ROOT, namespace)
}

/// Pattern for querying all publishers in a namespace.
///
/// Schema: `hulkz/graph/publishers/{namespace}/**`
pub fn graph_publishers_pattern(namespace: &str) -> String {
    format!("{}/graph/publishers/{}/**", ROOT, namespace)
}

/// Pattern for querying all parameters in a namespace.
///
/// This queries the read branch for parameter discovery.
/// Schema: `hulkz/param/read/**` (all scopes)
pub fn param_read_pattern(namespace: &str) -> String {
    // Query global, local for this namespace, and private for this namespace
    // We use alternation: global + local/ns + private/ns
    // Zenoh doesn't support OR in patterns, so we return local/private pattern
    // and handle global separately if needed
    format!("{}/param/read/local/{}/**", ROOT, namespace)
}

/// Pattern for querying global parameters.
///
/// Schema: `hulkz/param/read/global/**`
pub fn param_read_global_pattern() -> String {
    format!("{}/param/read/global/**", ROOT)
}

/// Pattern for querying private parameters in a namespace.
///
/// Schema: `hulkz/param/read/private/{namespace}/**`
pub fn param_read_private_pattern(namespace: &str) -> String {
    format!("{}/param/read/private/{}/**", ROOT, namespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_global() {
        let key = KeyExpr::new(Plane::Data, Scope::Global, "imu")
            .build()
            .unwrap();
        assert_eq!(key, "hulkz/data/global/imu");
    }

    #[test]
    fn data_local() {
        let key = KeyExpr::new(Plane::Data, Scope::Local, "camera/front")
            .namespace("chappie")
            .build()
            .unwrap();
        assert_eq!(key, "hulkz/data/local/chappie/camera/front");
    }

    #[test]
    fn data_private() {
        let key = KeyExpr::new(Plane::Data, Scope::Private, "debug/state")
            .namespace("chappie")
            .node("nav")
            .build()
            .unwrap();
        assert_eq!(key, "hulkz/data/private/chappie/nav/debug/state");
    }

    #[test]
    fn view_plane() {
        let key = KeyExpr::new(Plane::View, Scope::Local, "odometry")
            .namespace("robot1")
            .build()
            .unwrap();
        assert_eq!(key, "hulkz/view/local/robot1/odometry");
    }

    #[test]
    fn param_plane() {
        let key = KeyExpr::new(Plane::Param, Scope::Private, "max_speed")
            .namespace("chappie")
            .node("motor")
            .build()
            .unwrap();
        assert_eq!(key, "hulkz/param/private/chappie/motor/max_speed");
    }

    #[test]
    fn graph_plane() {
        let key = KeyExpr::new(Plane::Graph, Scope::Local, "nodes")
            .namespace("chappie")
            .build()
            .unwrap();
        assert_eq!(key, "hulkz/graph/local/chappie/nodes");
    }

    #[test]
    fn missing_namespace() {
        let result = KeyExpr::new(Plane::Data, Scope::Local, "topic").build();
        assert!(matches!(result, Err(KeyError::MissingNamespace)));
    }

    #[test]
    fn missing_node() {
        let result = KeyExpr::new(Plane::Data, Scope::Private, "topic")
            .namespace("ns")
            .build();
        assert!(matches!(result, Err(KeyError::MissingNode)));
    }

    #[test]
    fn graph_session() {
        let key = graph_session_key("chappie", "abc123@robot1");
        assert_eq!(key, "hulkz/graph/sessions/chappie/abc123@robot1");
    }

    #[test]
    fn graph_node() {
        let key = graph_node_key("chappie", "navigation");
        assert_eq!(key, "hulkz/graph/nodes/chappie/navigation");
    }

    #[test]
    fn graph_publisher_local() {
        let key = graph_publisher_key("chappie", "vision", Scope::Local, "camera/front");
        assert_eq!(
            key,
            "hulkz/graph/publishers/chappie/vision/local/camera/front"
        );
    }

    #[test]
    fn graph_publisher_private() {
        let key = graph_publisher_key("chappie", "nav", Scope::Private, "debug/state");
        assert_eq!(
            key,
            "hulkz/graph/publishers/chappie/nav/private/debug/state"
        );
    }

    #[test]
    fn graph_publisher_global() {
        let key = graph_publisher_key("chappie", "coordinator", Scope::Global, "fleet_status");
        assert_eq!(
            key,
            "hulkz/graph/publishers/chappie/coordinator/global/fleet_status"
        );
    }

    #[test]
    fn graph_sessions_pattern_test() {
        let pattern = graph_sessions_pattern("chappie");
        assert_eq!(pattern, "hulkz/graph/sessions/chappie/*");
    }

    #[test]
    fn graph_nodes_pattern_test() {
        let pattern = graph_nodes_pattern("chappie");
        assert_eq!(pattern, "hulkz/graph/nodes/chappie/*");
    }

    #[test]
    fn graph_publishers_pattern_test() {
        let pattern = graph_publishers_pattern("chappie");
        assert_eq!(pattern, "hulkz/graph/publishers/chappie/**");
    }
}
