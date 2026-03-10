//! Key expression building for the hulkz key space.
//!
//! Provides construction of Zenoh key expressions following the hulkz plane architecture: Data,
//! View, Param, and Graph.

use std::fmt;

use crate::topic::encode_topic_segment;

/// Root prefix for data/view/param key expressions.
pub(crate) const ROOT: &str = "hulkz";

/// Builds data plane key expressions.
pub(crate) struct DataKey;

impl DataKey {
    pub fn topic(domain_id: u32, topic: &str) -> String {
        format!("{ROOT}/data/{domain_id}/{topic}")
    }
}

/// Builds view plane key expressions (JSON debug mirror).
pub(crate) struct ViewKey;

impl ViewKey {
    pub fn topic(domain_id: u32, topic: &str) -> String {
        format!("{ROOT}/view/{domain_id}/{topic}")
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
    pub fn topic(intent: ParamIntent, domain_id: u32, topic: &str) -> String {
        format!("{ROOT}/param/{intent}/{domain_id}/{topic}")
    }
}

/// Builds graph plane key expressions for liveliness tokens and discovery.
pub(crate) struct GraphKey;

impl GraphKey {
    pub fn session(domain_id: u32, zenoh_id: &str, namespace: &str, session_id: &str) -> String {
        format!("{ROOT}/graph/{domain_id}/{zenoh_id}/sessions/{namespace}/{session_id}")
    }

    pub fn node(domain_id: u32, zenoh_id: &str, namespace: &str, node: &str) -> String {
        format!("{ROOT}/graph/{domain_id}/{zenoh_id}/nodes/{namespace}/{node}")
    }

    pub fn publisher(
        domain_id: u32,
        zenoh_id: &str,
        namespace: &str,
        node: &str,
        topic: &str,
    ) -> String {
        let encoded_topic = encode_topic_segment(topic);
        format!(
            "{ROOT}/graph/{domain_id}/{zenoh_id}/publishers/{namespace}/{node}/{encoded_topic}"
        )
    }

    pub fn parameter(
        domain_id: u32,
        zenoh_id: &str,
        namespace: &str,
        node: &str,
        topic: &str,
    ) -> String {
        let encoded_topic = encode_topic_segment(topic);
        format!(
            "{ROOT}/graph/{domain_id}/{zenoh_id}/parameters/{namespace}/{node}/{encoded_topic}"
        )
    }

    /// Pattern for sessions in namespace.
    pub fn sessions_in(namespace: &str) -> String {
        format!("{ROOT}/graph/*/*/sessions/{namespace}/*")
    }

    /// Pattern for nodes in namespace.
    pub fn nodes_in(namespace: &str) -> String {
        format!("{ROOT}/graph/*/*/nodes/{namespace}/*")
    }

    /// Pattern for publishers in namespace.
    pub fn publishers_in(namespace: &str) -> String {
        format!("{ROOT}/graph/*/*/publishers/{namespace}/**")
    }

    /// Pattern for parameters in namespace.
    pub fn parameters_in(namespace: &str) -> String {
        format!("{ROOT}/graph/*/*/parameters/{namespace}/**")
    }

    /// Pattern for all sessions.
    pub fn all_sessions() -> String {
        format!("{ROOT}/graph/*/*/sessions/*/*")
    }

    /// Pattern for all nodes.
    pub fn all_nodes() -> String {
        format!("{ROOT}/graph/*/*/nodes/*/*")
    }

    /// Pattern for all publishers.
    pub fn all_publishers() -> String {
        format!("{ROOT}/graph/*/*/publishers/**")
    }

    /// Pattern for all parameters.
    pub fn all_parameters() -> String {
        format!("{ROOT}/graph/*/*/parameters/**")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_key_topic() {
        assert_eq!(
            DataKey::topic(0, "robot/camera/front"),
            "hulkz/data/0/robot/camera/front"
        );
    }

    #[test]
    fn view_key_topic() {
        assert_eq!(
            ViewKey::topic(0, "robot/camera/front"),
            "hulkz/view/0/robot/camera/front"
        );
    }

    #[test]
    fn param_key_read_topic() {
        assert_eq!(
            ParamKey::topic(ParamIntent::Read, 0, "robot/motor/max_speed"),
            "hulkz/param/read/0/robot/motor/max_speed"
        );
    }

    #[test]
    fn graph_session_key() {
        assert_eq!(
            GraphKey::session(0, "zid-1", "chappie", "abc123@robot1"),
            "hulkz/graph/0/zid-1/sessions/chappie/abc123@robot1"
        );
    }

    #[test]
    fn graph_node_key() {
        assert_eq!(
            GraphKey::node(0, "zid-1", "chappie", "navigation"),
            "hulkz/graph/0/zid-1/nodes/chappie/navigation"
        );
    }

    #[test]
    fn graph_publisher_key_encodes_topic() {
        assert_eq!(
            GraphKey::publisher(0, "zid-1", "chappie", "vision", "chappie/vision/camera/front"),
            "hulkz/graph/0/zid-1/publishers/chappie/vision/chappie%2Fvision%2Fcamera%2Ffront"
        );
    }

    #[test]
    fn graph_sessions_pattern() {
        assert_eq!(
            GraphKey::sessions_in("chappie"),
            "hulkz/graph/*/*/sessions/chappie/*"
        );
    }

    #[test]
    fn graph_all_nodes_pattern() {
        assert_eq!(GraphKey::all_nodes(), "hulkz/graph/*/*/nodes/*/*");
    }

    #[test]
    fn graph_all_publishers_pattern() {
        assert_eq!(GraphKey::all_publishers(), "hulkz/graph/*/*/publishers/**");
    }
}
