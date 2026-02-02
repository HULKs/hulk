//! Key expression building for the Hulkz key space.
//!
//! Provides type-safe construction of Zenoh key expressions following the
//! Hulkz 5-plane architecture: Data, View, Param, and Graph.
//!
//! Most types in this module are crate-internal. Only [`Scope`] is public,
//! as it's used in discovery results ([`PublisherInfo`](crate::PublisherInfo),
//! [`ParameterInfo`](crate::ParameterInfo)).

use std::fmt;

/// Root prefix for all Hulkz key expressions.
pub(crate) const ROOT: &str = "hulkz";

/// Functional planes in the Hulkz architecture.
///
/// Note: `Param` and `Graph` variants are included for conceptual completeness
/// and potential future use, though current code builds those keys directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)] // Param/Graph kept for architecture documentation
pub(crate) enum Plane {
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
    pub(crate) const fn as_str(&self) -> &'static str {
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
pub(crate) enum ParamIntent {
    /// Read parameter value (query response + publish updates).
    Read,
    /// Write parameter value (accepts updates).
    Write,
}

impl ParamIntent {
    pub(crate) const fn as_str(&self) -> &'static str {
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

/// Builds a graph session liveliness key.
///
/// Schema: `hulkz/graph/sessions/{namespace}/{session_id}`
pub(crate) fn graph_session_key(namespace: &str, session_id: &str) -> String {
    format!("{}/graph/sessions/{}/{}", ROOT, namespace, session_id)
}

/// Builds a graph node liveliness key.
///
/// Schema: `hulkz/graph/nodes/{namespace}/{node}`
pub(crate) fn graph_node_key(namespace: &str, node: &str) -> String {
    format!("{}/graph/nodes/{}/{}", ROOT, namespace, node)
}

/// Pattern for querying graph sessions.
///
/// Schema: `hulkz/graph/sessions/{namespace}/*`
pub(crate) fn graph_sessions_pattern(namespace: &str) -> String {
    format!("{}/graph/sessions/{}/*", ROOT, namespace)
}

/// Pattern for querying graph nodes.
///
/// Schema: `hulkz/graph/nodes/{namespace}/*`
pub(crate) fn graph_nodes_pattern(namespace: &str) -> String {
    format!("{}/graph/nodes/{}/*", ROOT, namespace)
}

/// Pattern for querying all publishers in a namespace.
///
/// Schema: `hulkz/graph/publishers/{namespace}/**`
pub(crate) fn graph_publishers_pattern(namespace: &str) -> String {
    format!("{}/graph/publishers/{}/**", ROOT, namespace)
}

/// Pattern for querying all parameters in a namespace.
///
/// Schema: `hulkz/graph/parameters/{namespace}/**`
pub(crate) fn graph_parameters_pattern(namespace: &str) -> String {
    format!("{}/graph/parameters/{}/**", ROOT, namespace)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn graph_parameters_pattern_test() {
        let pattern = graph_parameters_pattern("chappie");
        assert_eq!(pattern, "hulkz/graph/parameters/chappie/**");
    }
}
