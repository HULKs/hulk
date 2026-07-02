use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub(crate) struct Arguments {
    /// Robot name used as ROS-Z namespace, e.g. robot-01. Omit for root namespace.
    #[arg(long, conflicts_with = "namespace")]
    pub(crate) robot: Option<String>,

    /// Explicit ROS-Z namespace. Omit for root namespace.
    #[arg(long)]
    pub(crate) namespace: Option<String>,

    /// Zenoh router endpoint, e.g. tcp/10.0.24.1:7447. Defaults to localhost router.
    #[arg(long)]
    pub(crate) router: Option<String>,
}

impl Arguments {
    pub(crate) fn namespace(&self) -> String {
        match (&self.namespace, &self.robot) {
            (Some(namespace), _) => normalize_namespace(namespace),
            (None, Some(robot)) => normalize_namespace(robot),
            (None, None) => "/".to_string(),
        }
    }

    pub(crate) fn router_display(&self) -> String {
        self.router
            .clone()
            .unwrap_or_else(|| "tcp/localhost:7447".to_string())
    }
}

fn normalize_namespace(namespace: &str) -> String {
    let namespace = namespace.trim_matches('/');
    if namespace.is_empty() {
        "/".to_string()
    } else {
        format!("/{namespace}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn robot_namespace_preserves_ros_z_graph_name_characters() {
        assert_eq!(normalize_namespace("42"), "/42");
        assert_eq!(normalize_namespace("robot-01"), "/robot-01");
        assert_eq!(normalize_namespace("robot_01"), "/robot_01");
    }

    #[test]
    fn explicit_namespace_is_normalized() {
        assert_eq!(normalize_namespace("/foo/bar-baz/"), "/foo/bar-baz");
    }

    #[test]
    fn root_namespace_is_normalized() {
        assert_eq!(normalize_namespace(""), "/");
        assert_eq!(normalize_namespace("/"), "/");
    }
}
