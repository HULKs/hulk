//! Integration tests for the hulkz key space structure.
//!
//! These tests verify the public API for scoped paths and scope parsing.
//! Internal key-building logic is tested via unit tests in the respective modules.

use hulkz::{Scope, ScopedPath};

mod scoped_path_syntax {
    use super::*;

    #[test]
    fn slash_prefix_is_global() {
        let path: ScopedPath = "/fleet".try_into().unwrap();
        assert!(matches!(path.scope(), Scope::Global));
    }

    #[test]
    fn tilde_prefix_is_private() {
        let path: ScopedPath = "~/debug".try_into().unwrap();
        assert!(matches!(path.scope(), Scope::Private));
    }

    #[test]
    fn no_prefix_is_local() {
        let path: ScopedPath = "sensor".try_into().unwrap();
        assert!(matches!(path.scope(), Scope::Local));
    }

    #[test]
    fn empty_rejected() {
        assert!(ScopedPath::try_from("").is_err());
    }

    #[test]
    fn double_slash_rejected() {
        assert!(ScopedPath::try_from("foo//bar").is_err());
    }

    #[test]
    fn path_accessor() {
        let path: ScopedPath = "/fleet/status".try_into().unwrap();
        assert_eq!(path.path(), "fleet/status");

        let path: ScopedPath = "~/debug/level".try_into().unwrap();
        assert_eq!(path.path(), "debug/level");

        let path: ScopedPath = "camera/front".try_into().unwrap();
        assert_eq!(path.path(), "camera/front");
    }

    #[test]
    fn parse_infallible() {
        // ScopedPath::parse is infallible, accepts any string
        let path = ScopedPath::parse("");
        assert_eq!(path.path(), "");

        let path = ScopedPath::parse("//weird//path//");
        assert_eq!(path.scope(), Scope::Global);
        assert_eq!(path.path(), "/weird//path//");
    }

    #[test]
    fn new_with_explicit_scope() {
        let path = ScopedPath::new(Scope::Private, "my/path");
        assert_eq!(path.scope(), Scope::Private);
        assert_eq!(path.path(), "my/path");
    }
}
