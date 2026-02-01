//! Scoped path parsing for topics, parameters, and other named resources.
//!
//! [`ScopedPath`] parses user-friendly path strings into scope + path pairs.
//! This is used throughout hulkz to resolve topic and parameter names to their
//! full Zenoh key expressions.
//!
//! # Prefix Syntax
//!
//! | Prefix | Scope | Visibility |
//! |--------|-------|------------|
//! | `/` | Global | Fleet-wide |
//! | (none) | Local | Robot-wide (default) |
//! | `~/` | Private | Node-internal |
//!
//! # Example
//!
//! ```rust
//! use hulkz::ScopedPath;
//!
//! let global: ScopedPath = "/fleet_status".try_into().unwrap();
//! let local: ScopedPath = "camera/front".try_into().unwrap();
//! let private: ScopedPath = "~/debug".try_into().unwrap();
//! ```

use crate::error::ScopedPathError;
use crate::key::{ParamIntent, Plane, Scope, ROOT};

/// A parsed path with scope information.
///
/// Used for topics, parameters, and other named resources that follow the
/// hulkz scoping convention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedPath {
    scope: Scope,
    path: String,
}

impl ScopedPath {
    /// Creates a new scoped path with explicit scope.
    pub fn new(scope: Scope, path: impl Into<String>) -> Self {
        Self {
            scope,
            path: path.into(),
        }
    }

    /// Parses a path string using prefix syntax.
    ///
    /// - `/path` → Global scope
    /// - `path` → Local scope (default)
    /// - `~/path` → Private scope
    ///
    /// This is infallible - any string is accepted. Use [`ScopedPath::parse_validated`]
    /// for stricter parsing.
    pub fn parse(input: &str) -> Self {
        if let Some(path) = input.strip_prefix('/') {
            Self {
                scope: Scope::Global,
                path: path.to_string(),
            }
        } else if let Some(path) = input.strip_prefix("~/") {
            Self {
                scope: Scope::Private,
                path: path.to_string(),
            }
        } else {
            Self {
                scope: Scope::Local,
                path: input.to_string(),
            }
        }
    }

    /// Parses a path string with validation.
    ///
    /// Rejects:
    /// - Empty paths
    /// - Paths with double slashes (`//`)
    /// - Paths ending with a slash
    pub fn parse_validated(input: &str) -> Result<Self, ScopedPathError> {
        let parsed = Self::parse(input);

        if parsed.path.is_empty() {
            return Err(ScopedPathError::Empty);
        }

        if parsed.path.starts_with('/') || parsed.path.ends_with('/') || parsed.path.contains("//")
        {
            return Err(ScopedPathError::Invalid(input.to_string()));
        }

        Ok(parsed)
    }

    /// Returns the scope of this path.
    pub fn scope(&self) -> Scope {
        self.scope
    }

    /// Returns the path component (without scope prefix).
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Generates a key expression for this path on the Data plane.
    pub fn to_data_key(&self, namespace: &str, node: &str) -> String {
        self.to_plane_key(Plane::Data, namespace, node)
    }

    /// Generates a key expression for this path on the View plane.
    pub fn to_view_key(&self, namespace: &str, node: &str) -> String {
        self.to_plane_key(Plane::View, namespace, node)
    }

    /// Generates a key expression for any plane.
    fn to_plane_key(&self, plane: Plane, namespace: &str, node: &str) -> String {
        match self.scope {
            Scope::Global => format!("{}/{}/{}/{}", ROOT, plane, self.scope, self.path),
            Scope::Local => format!(
                "{}/{}/{}/{}/{}",
                ROOT, plane, self.scope, namespace, self.path
            ),
            Scope::Private => format!(
                "{}/{}/{}/{}/{}/{}",
                ROOT, plane, self.scope, namespace, node, self.path
            ),
        }
    }

    /// Generates a parameter key expression with the given intent.
    pub fn to_param_key(&self, intent: ParamIntent, namespace: &str, node: &str) -> String {
        match self.scope {
            Scope::Global => {
                format!("{}/param/{}/{}/{}", ROOT, intent, self.scope, self.path)
            }
            Scope::Local => {
                format!(
                    "{}/param/{}/{}/{}/{}",
                    ROOT, intent, self.scope, namespace, self.path
                )
            }
            Scope::Private => {
                format!(
                    "{}/param/{}/{}/{}/{}/{}",
                    ROOT, intent, self.scope, namespace, node, self.path
                )
            }
        }
    }

    /// Generates a graph publisher liveliness key.
    ///
    /// Schema: `hulkz/graph/publishers/{namespace}/{node}/{scope}/{path}`
    pub fn to_graph_publisher_key(&self, namespace: &str, node: &str) -> String {
        format!(
            "{}/graph/publishers/{}/{}/{}/{}",
            ROOT, namespace, node, self.scope, self.path
        )
    }
}

impl TryFrom<&str> for ScopedPath {
    type Error = ScopedPathError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse_validated(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_global() {
        let path = ScopedPath::parse("/imu");
        assert_eq!(path.scope(), Scope::Global);
        assert_eq!(path.path(), "imu");
    }

    #[test]
    fn parse_local() {
        let path = ScopedPath::parse("camera/front");
        assert_eq!(path.scope(), Scope::Local);
        assert_eq!(path.path(), "camera/front");
    }

    #[test]
    fn parse_private() {
        let path = ScopedPath::parse("~/debug/state");
        assert_eq!(path.scope(), Scope::Private);
        assert_eq!(path.path(), "debug/state");
    }

    #[test]
    fn validated_rejects_empty() {
        assert!(ScopedPath::parse_validated("/").is_err());
        assert!(ScopedPath::parse_validated("~/").is_err());
        assert!(ScopedPath::parse_validated("").is_err());
    }

    #[test]
    fn validated_rejects_invalid_slashes() {
        assert!(ScopedPath::parse_validated("foo//bar").is_err());
        assert!(ScopedPath::parse_validated("foo/bar/").is_err());
    }

    #[test]
    fn to_data_key_local() {
        let path = ScopedPath::parse("camera/front");
        assert_eq!(
            path.to_data_key("chappie", "vision"),
            "hulkz/data/local/chappie/camera/front"
        );
    }

    #[test]
    fn to_data_key_global() {
        let path = ScopedPath::parse("/imu");
        assert_eq!(
            path.to_data_key("chappie", "vision"),
            "hulkz/data/global/imu"
        );
    }

    #[test]
    fn to_view_key_private() {
        let path = ScopedPath::parse("~/debug");
        assert_eq!(
            path.to_view_key("robot1", "nav"),
            "hulkz/view/private/robot1/nav/debug"
        );
    }

    #[test]
    fn to_param_key_read() {
        let path = ScopedPath::parse("~/max_speed");
        assert_eq!(
            path.to_param_key(ParamIntent::Read, "chappie", "motor"),
            "hulkz/param/read/private/chappie/motor/max_speed"
        );
    }

    #[test]
    fn to_param_key_write_local() {
        let path = ScopedPath::parse("wheel_radius");
        assert_eq!(
            path.to_param_key(ParamIntent::Write, "chappie", "motor"),
            "hulkz/param/write/local/chappie/wheel_radius"
        );
    }

    #[test]
    fn to_graph_publisher_key_local() {
        let path = ScopedPath::parse("camera/front");
        assert_eq!(
            path.to_graph_publisher_key("chappie", "vision"),
            "hulkz/graph/publishers/chappie/vision/local/camera/front"
        );
    }

    #[test]
    fn to_graph_publisher_key_private() {
        let path = ScopedPath::parse("~/debug/state");
        assert_eq!(
            path.to_graph_publisher_key("chappie", "nav"),
            "hulkz/graph/publishers/chappie/nav/private/debug/state"
        );
    }

    #[test]
    fn to_graph_publisher_key_global() {
        let path = ScopedPath::parse("/fleet_status");
        assert_eq!(
            path.to_graph_publisher_key("chappie", "coordinator"),
            "hulkz/graph/publishers/chappie/coordinator/global/fleet_status"
        );
    }
}
