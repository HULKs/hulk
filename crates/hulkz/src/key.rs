//! Key expression building for the hulkz key space.
//!
//! Provides type-safe construction of Zenoh key expressions following the hulkz plane
//! architecture: Data, View, Param, and Graph.

use std::fmt;

use crate::Scope;

/// Root prefix for all hulkz key expressions.
pub(crate) const ROOT: &str = "hulkz";

/// Builds data plane key expressions.
pub(crate) struct DataKey;

impl DataKey {
    /// Global scope: `hulkz/data/global/{path}`
    pub fn global(path: &str) -> String {
        format!("{ROOT}/data/global/{path}")
    }

    /// Local scope: `hulkz/data/local/{namespace}/{path}`
    pub fn local(namespace: &str, path: &str) -> String {
        format!("{ROOT}/data/local/{namespace}/{path}")
    }

    /// Private scope: `hulkz/data/private/{namespace}/{node}/{path}`
    pub fn private(namespace: &str, node: &str, path: &str) -> String {
        format!("{ROOT}/data/private/{namespace}/{node}/{path}")
    }

    /// Build key from scope (convenience for ScopedPath).
    pub fn from_scope(scope: Scope, namespace: &str, node: &str, path: &str) -> String {
        match scope {
            Scope::Global => Self::global(path),
            Scope::Local => Self::local(namespace, path),
            Scope::Private => Self::private(namespace, node, path),
        }
    }
}

/// Builds view plane key expressions (JSON debug mirror).
pub(crate) struct ViewKey;

impl ViewKey {
    pub fn global(path: &str) -> String {
        format!("{ROOT}/view/global/{path}")
    }

    pub fn local(namespace: &str, path: &str) -> String {
        format!("{ROOT}/view/local/{namespace}/{path}")
    }

    pub fn private(namespace: &str, node: &str, path: &str) -> String {
        format!("{ROOT}/view/private/{namespace}/{node}/{path}")
    }

    pub fn from_scope(scope: Scope, namespace: &str, node: &str, path: &str) -> String {
        match scope {
            Scope::Global => Self::global(path),
            Scope::Local => Self::local(namespace, path),
            Scope::Private => Self::private(namespace, node, path),
        }
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

/// Builds parameter plane key expressions.
pub(crate) struct ParamKey;

impl ParamKey {
    pub fn global(intent: ParamIntent, path: &str) -> String {
        format!("{ROOT}/param/{intent}/global/{path}")
    }

    pub fn local(intent: ParamIntent, namespace: &str, path: &str) -> String {
        format!("{ROOT}/param/{intent}/local/{namespace}/{path}")
    }

    pub fn private(intent: ParamIntent, namespace: &str, node: &str, path: &str) -> String {
        format!("{ROOT}/param/{intent}/private/{namespace}/{node}/{path}")
    }

    pub fn from_scope(
        intent: ParamIntent,
        scope: Scope,
        namespace: &str,
        node: &str,
        path: &str,
    ) -> String {
        match scope {
            Scope::Global => Self::global(intent, path),
            Scope::Local => Self::local(intent, namespace, path),
            Scope::Private => Self::private(intent, namespace, node, path),
        }
    }
}

/// Builds graph plane key expressions for liveliness tokens and discovery.
pub(crate) struct GraphKey;

impl GraphKey {
    /// Session liveliness key
    pub fn session(namespace: &str, session_id: &str) -> String {
        format!("{ROOT}/graph/sessions/{namespace}/{session_id}")
    }

    /// Node liveliness key
    pub fn node(namespace: &str, node: &str) -> String {
        format!("{ROOT}/graph/nodes/{namespace}/{node}")
    }

    /// Publisher liveliness key
    pub fn publisher(namespace: &str, node: &str, scope: Scope, path: &str) -> String {
        format!("{ROOT}/graph/publishers/{namespace}/{node}/{scope}/{path}")
    }

    /// Parameter liveliness key
    pub fn parameter(namespace: &str, node: &str, scope: Scope, path: &str) -> String {
        format!("{ROOT}/graph/parameters/{namespace}/{node}/{scope}/{path}")
    }

    /// Pattern for sessions in namespace
    pub fn sessions_in(namespace: &str) -> String {
        format!("{ROOT}/graph/sessions/{namespace}/*")
    }

    /// Pattern for nodes in namespace
    pub fn nodes_in(namespace: &str) -> String {
        format!("{ROOT}/graph/nodes/{namespace}/*")
    }

    /// Pattern for publishers in namespace
    pub fn publishers_in(namespace: &str) -> String {
        format!("{ROOT}/graph/publishers/{namespace}/**")
    }

    /// Pattern for parameters in namespace
    pub fn parameters_in(namespace: &str) -> String {
        format!("{ROOT}/graph/parameters/{namespace}/**")
    }

    /// Pattern for all sessions
    pub fn all_sessions() -> String {
        format!("{ROOT}/graph/sessions/*/*")
    }

    /// Pattern for all nodes
    pub fn all_nodes() -> String {
        format!("{ROOT}/graph/nodes/*/*")
    }

    /// Pattern for all publishers
    pub fn all_publishers() -> String {
        format!("{ROOT}/graph/publishers/**")
    }

    /// Pattern for all parameters
    pub fn all_parameters() -> String {
        format!("{ROOT}/graph/parameters/**")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_key_global() {
        assert_eq!(
            DataKey::global("fleet_status"),
            "hulkz/data/global/fleet_status"
        );
    }

    #[test]
    fn data_key_local() {
        assert_eq!(
            DataKey::local("chappie", "camera/front"),
            "hulkz/data/local/chappie/camera/front"
        );
    }

    #[test]
    fn data_key_private() {
        assert_eq!(
            DataKey::private("chappie", "vision", "debug"),
            "hulkz/data/private/chappie/vision/debug"
        );
    }

    #[test]
    fn view_key_from_scope() {
        assert_eq!(
            ViewKey::from_scope(Scope::Private, "robot", "nav", "debug"),
            "hulkz/view/private/robot/nav/debug"
        );
    }

    #[test]
    fn param_key_read() {
        assert_eq!(
            ParamKey::private(ParamIntent::Read, "chappie", "motor", "max_speed"),
            "hulkz/param/read/private/chappie/motor/max_speed"
        );
    }

    #[test]
    fn param_key_write_local() {
        assert_eq!(
            ParamKey::local(ParamIntent::Write, "chappie", "wheel_radius"),
            "hulkz/param/write/local/chappie/wheel_radius"
        );
    }

    #[test]
    fn graph_session_key() {
        assert_eq!(
            GraphKey::session("chappie", "abc123@robot1"),
            "hulkz/graph/sessions/chappie/abc123@robot1"
        );
    }

    #[test]
    fn graph_node_key() {
        assert_eq!(
            GraphKey::node("chappie", "navigation"),
            "hulkz/graph/nodes/chappie/navigation"
        );
    }

    #[test]
    fn graph_publisher_key() {
        assert_eq!(
            GraphKey::publisher("chappie", "vision", Scope::Local, "camera/front"),
            "hulkz/graph/publishers/chappie/vision/local/camera/front"
        );
    }

    #[test]
    fn graph_sessions_pattern() {
        assert_eq!(
            GraphKey::sessions_in("chappie"),
            "hulkz/graph/sessions/chappie/*"
        );
    }

    #[test]
    fn graph_all_nodes_pattern() {
        assert_eq!(GraphKey::all_nodes(), "hulkz/graph/nodes/*/*");
    }

    #[test]
    fn graph_all_publishers_pattern() {
        assert_eq!(GraphKey::all_publishers(), "hulkz/graph/publishers/**");
    }
}
