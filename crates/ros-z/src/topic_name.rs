// Copyright 2025 ZettaScale Technology
//
// Topic name qualification and expansion for native ros-z APIs.

/// Errors that can occur during topic name qualification
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TopicNameError {
    /// Topic name is empty
    #[error("topic name is empty")]
    Empty,
    /// Topic name ends with a forward slash
    #[error("topic name ends with forward slash")]
    EndsWithSlash,
    /// Topic name contains invalid characters
    #[error("topic name contains invalid characters: {0}")]
    InvalidCharacters(String),
    /// Namespace is invalid
    #[error("invalid namespace: {0}")]
    InvalidNamespace(String),
    /// Node name is invalid
    #[error("invalid node name: {0}")]
    InvalidNodeName(String),
}

/// Validate one concrete ros-z graph-name component.
///
/// Components follow Zenoh key-expression chunk rules, with extra exclusions
/// for ros-z concrete endpoints and current liveliness identity escaping.
#[cfg(test)]
fn is_valid_graph_component(component: &str) -> bool {
    invalid_graph_component_reason(component).is_none()
}

fn invalid_graph_component_reason(component: &str) -> Option<&'static str> {
    if component.is_empty() {
        return Some("component is empty");
    }

    for character in component.chars() {
        match character {
            '/' => return Some("component contains '/'"),
            '%' => return Some("component contains '%'"),
            '#' => return Some("component contains '#'"),
            '$' => return Some("component contains '$'"),
            '?' => return Some("component contains '?'"),
            '*' => return Some("component contains '*'"),
            _ => {}
        }
    }

    None
}

fn validate_graph_component(component: &str) -> Result<(), String> {
    match invalid_graph_component_reason(component) {
        Some(reason) => Err(format!("invalid component '{component}': {reason}")),
        None => Ok(()),
    }
}

fn validate_graph_path(path: &str) -> Result<(), String> {
    for component in path.split('/') {
        validate_graph_component(component)?;
    }

    Ok(())
}

/// Validate a namespace string.
/// Namespaces can be empty, "/", or concrete components separated by "/".
pub(crate) fn validate_namespace(namespace: &str) -> Result<(), TopicNameError> {
    if namespace.is_empty() || namespace == "/" {
        return Ok(());
    }

    if namespace.ends_with('/') {
        return Err(TopicNameError::InvalidNamespace(
            "namespace cannot end with '/'".to_string(),
        ));
    }

    let namespace_path = namespace.strip_prefix('/').unwrap_or(namespace);
    validate_graph_path(namespace_path).map_err(TopicNameError::InvalidNamespace)
}

/// Validate a node name.
pub(crate) fn validate_node_name(node_name: &str) -> Result<(), TopicNameError> {
    validate_graph_component(node_name).map_err(TopicNameError::InvalidNodeName)
}

/// Qualify a topic name according to ros-z naming rules.
///
/// This function takes a topic name and qualifies it based on the node's namespace and name.
///
/// Rules:
/// - Absolute topics (starting with '/') are returned as-is (with trailing slash removed if present)
/// - Private topics (starting with '~') are expanded to `/<namespace>/<node_name>/<topic>`
/// - Relative topics are expanded to `/<namespace>/<topic>`
/// - Empty namespace is treated as "/"
///
/// # Arguments
/// * `topic` - The input topic name (can be absolute, relative, or private)
/// * `namespace` - The node's namespace (can be "" or "/")
/// * `node_name` - The node's name
///
/// # Returns
/// * `Ok(String)` - The fully qualified topic name
/// * `Err(TopicNameError)` - If validation fails
///
/// # Examples
/// ```
/// use ros_z::topic_name::qualify_topic_name;
///
/// // Absolute topic
/// assert_eq!(qualify_topic_name("/chatter", "/ns", "node").unwrap(), "/chatter");
///
/// // Relative topic in root namespace
/// assert_eq!(qualify_topic_name("chatter", "/", "node").unwrap(), "/chatter");
///
/// // Relative topic in named namespace
/// assert_eq!(qualify_topic_name("chatter", "/ns", "node").unwrap(), "/ns/chatter");
///
/// // Private topic
/// assert_eq!(qualify_topic_name("~my_topic", "/ns", "node").unwrap(), "/ns/node/my_topic");
/// ```
pub fn qualify_topic_name(
    topic: &str,
    namespace: &str,
    node_name: &str,
) -> Result<String, TopicNameError> {
    // Validate inputs
    if topic.is_empty() {
        return Err(TopicNameError::Empty);
    }

    validate_namespace(namespace)?;
    validate_node_name(node_name)?;

    // Normalize namespace: ensure it starts with "/" if not empty
    let namespace = if namespace.is_empty() {
        "".to_string()
    } else if namespace.starts_with('/') {
        namespace.to_string()
    } else {
        format!("/{}", namespace)
    };

    // Handle different topic name types
    let qualified = if topic.starts_with('/') {
        // Absolute topic - use as-is, but remove trailing slash if present.
        let topic = topic.strip_suffix('/').unwrap_or(topic);
        let topic_path = topic.strip_prefix('/').unwrap_or(topic);
        if topic_path.is_empty() {
            return Err(TopicNameError::InvalidCharacters(
                "topic cannot be just '/'".to_string(),
            ));
        }
        validate_graph_path(topic_path).map_err(TopicNameError::InvalidCharacters)?;
        topic.to_string()
    } else if topic.starts_with('~') {
        // Private topic - expand with namespace and node name
        let topic_suffix = topic.strip_prefix('~').unwrap();
        let topic_suffix = topic_suffix.strip_prefix('/').unwrap_or(topic_suffix);

        let topic_suffix = topic_suffix.strip_suffix('/').unwrap_or(topic_suffix);

        if !topic_suffix.is_empty() {
            validate_graph_path(topic_suffix).map_err(|source| {
                TopicNameError::InvalidCharacters(format!("invalid private topic suffix: {source}"))
            })?;
        }

        if namespace.is_empty() || namespace == "/" {
            if topic_suffix.is_empty() {
                format!("/{}", node_name)
            } else {
                format!("/{}/{}", node_name, topic_suffix)
            }
        } else if topic_suffix.is_empty() {
            format!("{}/{}", namespace, node_name)
        } else {
            format!("{}/{}/{}", namespace, node_name, topic_suffix)
        }
    } else {
        // Relative topic - expand with namespace only
        let topic = topic.strip_suffix('/').unwrap_or(topic);

        validate_graph_path(topic).map_err(TopicNameError::InvalidCharacters)?;

        if namespace.is_empty() || namespace == "/" {
            format!("/{}", topic)
        } else {
            format!("{}/{}", namespace, topic)
        }
    };

    Ok(qualified)
}

/// Qualify a service name according to ros-z naming rules.
///
/// Service names follow the same rules as topic names
pub fn qualify_service_name(
    service: &str,
    namespace: &str,
    node_name: &str,
) -> Result<String, TopicNameError> {
    qualify_topic_name(service, namespace, node_name)
}

pub(crate) fn qualify_remote_private_service_name(
    service_basename: &str,
    namespace: &str,
    node_name: &str,
) -> Result<String, TopicNameError> {
    let service_basename = service_basename
        .strip_prefix('~')
        .unwrap_or(service_basename);
    let service_basename = service_basename
        .strip_prefix('/')
        .unwrap_or(service_basename);
    let private_service_name = if service_basename.is_empty() {
        "~".to_string()
    } else {
        format!("~{service_basename}")
    };

    qualify_service_name(&private_service_name, namespace, node_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_absolute_topics() {
        assert_eq!(
            qualify_topic_name("/chatter", "/", "node").unwrap(),
            "/chatter"
        );
        assert_eq!(
            qualify_topic_name("/chatter", "/ns", "node").unwrap(),
            "/chatter"
        );
        assert_eq!(
            qualify_topic_name("/foo/bar", "/ns", "node").unwrap(),
            "/foo/bar"
        );
    }

    #[test]
    fn test_absolute_topics_trailing_slash() {
        assert_eq!(
            qualify_topic_name("/chatter/", "/ns", "node").unwrap(),
            "/chatter"
        );
    }

    #[test]
    fn test_relative_topics_root_namespace() {
        assert_eq!(
            qualify_topic_name("chatter", "/", "node").unwrap(),
            "/chatter"
        );
        assert_eq!(
            qualify_topic_name("chatter", "", "node").unwrap(),
            "/chatter"
        );
    }

    #[test]
    fn test_relative_topics_named_namespace() {
        assert_eq!(
            qualify_topic_name("chatter", "/ns", "node").unwrap(),
            "/ns/chatter"
        );
        assert_eq!(
            qualify_topic_name("foo/bar", "/ns", "node").unwrap(),
            "/ns/foo/bar"
        );
        assert_eq!(
            qualify_topic_name("chatter", "/my/nested/ns", "node").unwrap(),
            "/my/nested/ns/chatter"
        );
    }

    #[test]
    fn test_private_topics() {
        assert_eq!(
            qualify_topic_name("~my_topic", "/", "node").unwrap(),
            "/node/my_topic"
        );
        assert_eq!(
            qualify_topic_name("~my_topic", "/ns", "node").unwrap(),
            "/ns/node/my_topic"
        );
        assert_eq!(qualify_topic_name("~", "/ns", "node").unwrap(), "/ns/node");
        assert_eq!(
            qualify_topic_name("~/my_topic", "/ns", "node").unwrap(),
            "/ns/node/my_topic"
        );
    }

    #[test]
    fn test_private_topics_nested() {
        assert_eq!(
            qualify_topic_name("~foo/bar", "/ns", "node").unwrap(),
            "/ns/node/foo/bar"
        );
    }

    #[test]
    fn accepts_zenoh_native_digit_and_hyphen_components() {
        assert_eq!(
            qualify_topic_name("chatter", "/42", "node").unwrap(),
            "/42/chatter"
        );
        assert_eq!(
            qualify_topic_name("~status", "/robot", "123node").unwrap(),
            "/robot/123node/status"
        );
        assert_eq!(
            qualify_topic_name("42/status", "/robot", "node").unwrap(),
            "/robot/42/status"
        );
        assert_eq!(
            qualify_topic_name("camera-left/image_raw", "/robot-01", "node").unwrap(),
            "/robot-01/camera-left/image_raw"
        );
        assert_eq!(
            qualify_service_name("42/service", "/7", "123node").unwrap(),
            "/7/42/service"
        );
    }

    #[test]
    fn rejects_non_concrete_or_protocol_reserved_components() {
        for topic in [
            "foo//bar",
            "/foo//bar",
            "/foo%bar",
            "/foo#bar",
            "/foo$bar",
            "/foo?bar",
            "/foo*bar",
        ] {
            assert!(
                matches!(
                    qualify_topic_name(topic, "/", "node"),
                    Err(TopicNameError::InvalidCharacters(_))
                ),
                "topic {topic:?} should be rejected"
            );
        }

        assert!(matches!(
            qualify_topic_name("status", "/robot//ns", "node"),
            Err(TopicNameError::InvalidNamespace(_))
        ));
        assert!(matches!(
            qualify_topic_name("~status", "/robot", "node%name"),
            Err(TopicNameError::InvalidNodeName(_))
        ));
    }

    #[test]
    fn test_empty_topic() {
        assert!(matches!(
            qualify_topic_name("", "/", "node"),
            Err(TopicNameError::Empty)
        ));
    }

    #[test]
    fn test_invalid_namespace() {
        assert!(matches!(
            qualify_topic_name("chatter", "/ns/", "node"),
            Err(TopicNameError::InvalidNamespace(_))
        ));
    }

    #[test]
    fn test_invalid_node_name() {
        assert!(matches!(
            qualify_topic_name("chatter", "/ns", ""),
            Err(TopicNameError::InvalidNodeName(_))
        ));
        assert!(matches!(
            qualify_topic_name("chatter", "/ns", "node%name"),
            Err(TopicNameError::InvalidNodeName(_))
        ));
    }

    #[test]
    fn test_valid_graph_components() {
        assert!(is_valid_graph_component("foo"));
        assert!(is_valid_graph_component("_foo"));
        assert!(is_valid_graph_component("123"));
        assert!(is_valid_graph_component("foo123"));
        assert!(is_valid_graph_component("foo_bar"));
        assert!(is_valid_graph_component("foo-bar"));
        assert!(is_valid_graph_component("FooBar"));

        assert!(!is_valid_graph_component(""));
        assert!(!is_valid_graph_component("foo/bar"));
        assert!(!is_valid_graph_component("foo%bar"));
        assert!(!is_valid_graph_component("foo#bar"));
        assert!(!is_valid_graph_component("foo$bar"));
        assert!(!is_valid_graph_component("foo?bar"));
        assert!(!is_valid_graph_component("foo*bar"));
    }

    #[test]
    fn test_service_names() {
        assert_eq!(
            qualify_service_name("/add_two_ints", "/", "node").unwrap(),
            "/add_two_ints"
        );
        assert_eq!(
            qualify_service_name("add_two_ints", "/ns", "node").unwrap(),
            "/ns/add_two_ints"
        );
        assert_eq!(
            qualify_service_name("~my_service", "/ns", "node").unwrap(),
            "/ns/node/my_service"
        );
    }

    #[test]
    fn test_remote_private_service_names_are_absolute() {
        assert_eq!(
            qualify_remote_private_service_name("get_state", "", "node").unwrap(),
            "/node/get_state"
        );
        assert_eq!(
            qualify_remote_private_service_name("get_state", "/", "node").unwrap(),
            "/node/get_state"
        );
        assert_eq!(
            qualify_remote_private_service_name("get_state", "tools", "node").unwrap(),
            "/tools/node/get_state"
        );
        assert_eq!(
            qualify_remote_private_service_name("get_state", "/tools", "node").unwrap(),
            "/tools/node/get_state"
        );
    }

    #[test]
    fn test_remote_private_service_helper_accepts_empty_basename() {
        assert_eq!(
            qualify_remote_private_service_name("", "/tools", "node").unwrap(),
            "/tools/node"
        );
    }
}
