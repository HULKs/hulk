//! Key expression building for the hulkz key space.
//!
//! Provides type-safe construction of Zenoh key expressions following the hulkz plane
//! architecture: Data, View, Param, and Graph.

use std::fmt;

/// Root prefix for all hulkz key expressions.
pub(crate) const ROOT: &str = "hulkz";

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
pub(crate) fn graph_session_key(namespace: &str, session_id: &str) -> String {
    format!("{ROOT}/graph/sessions/{namespace}/{session_id}")
}

/// Builds a graph node liveliness key.
pub(crate) fn graph_node_key(namespace: &str, node: &str) -> String {
    format!("{ROOT}/graph/nodes/{namespace}/{node}")
}

/// Pattern for querying graph sessions.
pub(crate) fn graph_sessions_pattern(namespace: &str) -> String {
    format!("{ROOT}/graph/sessions/{namespace}/*")
}

/// Pattern for querying graph nodes.
pub(crate) fn graph_nodes_pattern(namespace: &str) -> String {
    format!("{ROOT}/graph/nodes/{namespace}/*")
}

/// Pattern for querying all publishers in a namespace.
pub(crate) fn graph_publishers_pattern(namespace: &str) -> String {
    format!("{ROOT}/graph/publishers/{namespace}/**")
}

/// Pattern for querying all parameters in a namespace.
pub(crate) fn graph_parameters_pattern(namespace: &str) -> String {
    format!("{ROOT}/graph/parameters/{namespace}/**")
}

/// Pattern for discovering all sessions across all namespaces.
pub(crate) fn graph_all_sessions_pattern() -> String {
    format!("{ROOT}/graph/sessions/*/*")
}

/// Pattern for discovering all nodes across all namespaces.
pub(crate) fn graph_all_nodes_pattern() -> String {
    format!("{ROOT}/graph/nodes/*/*")
}

/// Pattern for discovering all publishers across all namespaces.
pub(crate) fn graph_all_publishers_pattern() -> String {
    format!("{ROOT}/graph/publishers/**")
}

/// Pattern for discovering all parameters across all namespaces.
pub(crate) fn graph_all_parameters_pattern() -> String {
    format!("{ROOT}/graph/parameters/**")
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
