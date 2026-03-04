//! Integration tests for topic expression parsing and resolution.

use hulkz::TopicExpression;

mod topic_expression_syntax {
    use super::*;

    #[test]
    fn absolute_expression_resolves_without_namespace_prefix() {
        let expr: TopicExpression = "/fleet".into();
        let resolved = expr.resolve("robot", Some("nav")).unwrap();
        assert_eq!(resolved, "fleet");
    }

    #[test]
    fn private_current_expression_uses_default_node() {
        let expr: TopicExpression = "~/debug".into();
        let resolved = expr.resolve("robot", Some("planner")).unwrap();
        assert_eq!(resolved, "robot/planner/debug");
    }

    #[test]
    fn relative_expression_uses_namespace_prefix() {
        let expr: TopicExpression = "sensor".into();
        let resolved = expr.resolve("robot", Some("planner")).unwrap();
        assert_eq!(resolved, "robot/sensor");
    }

    #[test]
    fn explicit_private_node_expression_uses_embedded_node() {
        let expr: TopicExpression = "~vision/debug/level".into();
        let resolved = expr.resolve("robot", Some("planner")).unwrap();
        assert_eq!(resolved, "robot/vision/debug/level");
    }

    #[test]
    fn private_current_expression_requires_node() {
        let expr: TopicExpression = "~/debug".into();
        assert!(expr.resolve("robot", None).is_err());
    }
}
