//! Integration tests for the hulkz key space structure.
//!
//! These tests verify the public API for scoped paths and scope parsing.
//! Internal key-building logic is tested via unit tests in the respective modules.

use hulkz::{Scope, ScopedPath};

mod scoped_path_syntax {
    use super::*;

    #[test]
    fn slash_prefix_is_global() {
        let path: ScopedPath = "/fleet".into();
        assert!(matches!(path.scope(), Scope::Global));
    }

    #[test]
    fn tilde_prefix_is_private() {
        let path: ScopedPath = "~/debug".into();
        assert!(matches!(path.scope(), Scope::Private));
    }

    #[test]
    fn no_prefix_is_local() {
        let path: ScopedPath = "sensor".into();
        assert!(matches!(path.scope(), Scope::Local));
    }

    #[test]
    fn path_accessor() {
        let path: ScopedPath = "/fleet/status".into();
        assert_eq!(path.path(), "fleet/status");

        let path: ScopedPath = "~/debug/level".into();
        assert_eq!(path.path(), "debug/level");

        let path: ScopedPath = "camera/front".into();
        assert_eq!(path.path(), "camera/front");
    }

    #[test]
    fn new_with_explicit_scope() {
        let path = ScopedPath::new(Scope::Private, "my/path");
        assert_eq!(path.scope(), Scope::Private);
        assert_eq!(path.path(), "my/path");
    }
}
