//! Scoped path parsing for topics, parameters, and other named resources.
//!
//! [`ScopedPath`] parses user-friendly path strings into scope + path pairs. This is used
//! throughout hulkz to resolve topic and parameter names to their full Zenoh key expressions.
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

use std::fmt;

use crate::error::ScopedPathError;

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

/// A parsed path with scope information.
///
/// Used for topics, parameters, and other named resources that follow the hulkz scoping
/// convention.
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
    /// This is infallible - any string is accepted. Use [`ScopedPath::parse_validated`] for
    /// stricter parsing.
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
    fn new_with_explicit_scope() {
        let path = ScopedPath::new(Scope::Private, "my/path");
        assert_eq!(path.scope(), Scope::Private);
        assert_eq!(path.path(), "my/path");
    }
}
