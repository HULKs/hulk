//! Integration tests for the Hulkz key space structure.

use hulkz::{graph_node_key, KeyError, KeyExpr, ParamIntent, Plane, Scope, ScopedPath};

mod data_plane {
    use super::*;

    #[test]
    fn global_path() {
        let path: ScopedPath = "/imu".try_into().unwrap();
        assert_eq!(path.to_data_key("ns", "node"), "hulkz/data/global/imu");
    }

    #[test]
    fn local_path() {
        let path: ScopedPath = "camera/front".try_into().unwrap();
        assert_eq!(
            path.to_data_key("chappie", "vision"),
            "hulkz/data/local/chappie/camera/front"
        );
    }

    #[test]
    fn private_path() {
        let path: ScopedPath = "~/debug/state".try_into().unwrap();
        assert_eq!(
            path.to_data_key("chappie", "nav"),
            "hulkz/data/private/chappie/nav/debug/state"
        );
    }
}

mod view_plane {
    use super::*;

    #[test]
    fn mirrors_data_structure() {
        let path: ScopedPath = "odometry".try_into().unwrap();
        assert_eq!(
            path.to_view_key("chappie", "odom"),
            "hulkz/view/local/chappie/odometry"
        );
    }
}

mod param_plane {
    use super::*;

    #[test]
    fn private_read() {
        let path = ScopedPath::parse("~/max_speed");
        let key = path.to_param_key(ParamIntent::Read, "chappie", "motor");
        assert_eq!(key, "hulkz/param/read/private/chappie/motor/max_speed");
    }

    #[test]
    fn local_write() {
        let path = ScopedPath::parse("wheel_radius");
        let key = path.to_param_key(ParamIntent::Write, "chappie", "motor");
        assert_eq!(key, "hulkz/param/write/local/chappie/wheel_radius");
    }
}

mod graph_plane {
    use super::*;

    #[test]
    fn node_liveliness() {
        assert_eq!(
            graph_node_key("chappie", "nav"),
            "hulkz/graph/nodes/chappie/nav"
        );
    }
}

mod key_expr_builder {
    use super::*;

    #[test]
    fn build_returns_result() {
        let result = KeyExpr::new(Plane::Data, Scope::Local, "path")
            .namespace("ns")
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn missing_namespace_is_error() {
        let result = KeyExpr::new(Plane::Data, Scope::Local, "path").build();
        assert!(matches!(result, Err(KeyError::MissingNamespace)));
    }

    #[test]
    fn missing_node_is_error() {
        let result = KeyExpr::new(Plane::Data, Scope::Private, "path")
            .namespace("ns")
            .build();
        assert!(matches!(result, Err(KeyError::MissingNode)));
    }
}

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
}
