use ros_z::topic_name::qualify_topic_name;

use crate::{Error, Result};

const TARGET_NODE_PLACEHOLDER: &str = "debug_target";

/// Validated topic reference used by debug subscriptions.
///
/// Relative references resolve against a target namespace. Absolute references
/// are used as-is through `ros-z` topic qualification. Private `~` references
/// resolve against the target namespace and target node name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TopicReference(String);

/// Identity of the ROS node namespace and optional node name being inspected.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TargetIdentity {
    namespace: String,
    node_name: Option<String>,
}

/// Topic name projected for display in a UI or debug tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedTopic {
    /// Display label for the topic under the active namespace context.
    pub display_name: String,
    /// Absolute topic name used for subscription.
    pub resolved_topic: String,
    /// Relationship between the topic and the active namespace.
    pub scope: ProjectedTopicScope,
}

/// Scope used when projecting a topic for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProjectedTopicScope {
    /// Topic is inside the active namespace and can be displayed relatively.
    RelativeToActiveNamespace,
    /// Topic is outside the active namespace and should be treated as fully qualified.
    FullyQualified,
}

/// Projects absolute or relative topic names for display under an active namespace.
pub struct TopicProjection;

impl TargetIdentity {
    /// Create a target identity from a ROS namespace.
    pub fn new(namespace: impl Into<String>) -> Result<Self> {
        let namespace = namespace.into();
        Ok(Self {
            namespace: normalize_target_namespace(&namespace)?,
            node_name: None,
        })
    }

    /// Create an updated identity with a validated target node name.
    pub fn with_node_name(mut self, node_name: impl Into<String>) -> Result<Self> {
        self.set_node_name(node_name)?;
        Ok(self)
    }

    /// Namespace of the target node.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Name of the target node, when known.
    pub fn node_name(&self) -> Option<&str> {
        self.node_name.as_deref()
    }

    /// Update the target namespace after validation.
    pub fn set_namespace(&mut self, namespace: impl Into<String>) -> Result<&mut Self> {
        let namespace = namespace.into();
        let namespace = normalize_target_namespace(&namespace)?;
        if let Some(node_name) = self.node_name.as_deref() {
            validate_node_name(&namespace, node_name)?;
        }
        self.namespace = namespace;
        Ok(self)
    }

    /// Update the target node name after validation.
    pub fn set_node_name(&mut self, node_name: impl Into<String>) -> Result<&mut Self> {
        let node_name = node_name.into();
        validate_node_name(&self.namespace, &node_name)?;
        self.node_name = Some(node_name);
        Ok(self)
    }
}

impl TopicReference {
    /// Validate and store a topic reference.
    pub fn new(topic: impl Into<String>) -> Result<Self> {
        let topic = topic.into();
        validate_topic_reference(&topic)?;

        Ok(Self(topic))
    }

    /// Return the original topic reference string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Return whether this reference is absolute.
    pub fn is_absolute(&self) -> bool {
        self.0.starts_with('/')
    }

    /// Return whether this reference is private to a target node.
    pub fn is_private(&self) -> bool {
        self.0.starts_with('~')
    }

    /// Resolve this reference against `target`.
    pub fn resolve(&self, target: &TargetIdentity) -> Result<String> {
        resolve_topic_name(&self.0, target)
    }
}

impl TopicProjection {
    /// Project topic names for display under `active_namespace`.
    ///
    /// Topics inside the active namespace display without that namespace prefix.
    /// Other absolute topics keep their leading `/` so callers can distinguish
    /// global topics from active-namespace topics.
    pub fn project<Topic>(
        active_namespace: &str,
        topics: impl IntoIterator<Item = Topic>,
    ) -> Result<Vec<ProjectedTopic>>
    where
        Topic: AsRef<str>,
    {
        let active_identity = TargetIdentity::new(active_namespace)?;
        topics
            .into_iter()
            .map(|topic| {
                let topic = TopicReference::new(topic.as_ref())?;
                let resolved_topic = topic.resolve(&active_identity)?;
                let (display_name, scope) =
                    display_name(active_identity.namespace(), &resolved_topic);

                Ok(ProjectedTopic {
                    display_name,
                    resolved_topic,
                    scope,
                })
            })
            .collect::<Result<Vec<_>>>()
    }
}

fn resolve_topic_name(topic: &str, target: &TargetIdentity) -> Result<String> {
    if let Some(private_topic) = topic.strip_prefix('~') {
        let node_name = target
            .node_name()
            .ok_or_else(|| Error::MissingTargetNodeName {
                topic: topic.to_string(),
            })?;
        let private_topic = private_topic.trim_start_matches('/');
        let topic = if private_topic.is_empty() {
            node_name.to_string()
        } else {
            format!("{node_name}/{private_topic}")
        };
        return qualify_topic_name(&topic, target.namespace(), TARGET_NODE_PLACEHOLDER).map_err(
            |source| Error::InvalidTopicReference {
                topic: topic.to_string(),
                source,
            },
        );
    }

    let target = if topic.starts_with('/') {
        TargetIdentity::new("/")?
    } else {
        target.clone()
    };

    qualify_topic_name(topic, target.namespace(), TARGET_NODE_PLACEHOLDER).map_err(|source| {
        Error::InvalidTopicReference {
            topic: topic.to_string(),
            source,
        }
    })
}

fn validate_topic_reference(topic: &str) -> Result<()> {
    qualify_topic_name(topic, "/", TARGET_NODE_PLACEHOLDER)
        .map(|_| ())
        .map_err(|source| Error::InvalidTopicReference {
            topic: topic.to_string(),
            source,
        })
}

pub(crate) fn normalize_target_namespace(target_namespace: &str) -> Result<String> {
    let normalized_namespace = normalize_namespace(target_namespace);
    validate_target_namespace(target_namespace, &normalized_namespace)?;
    Ok(normalized_namespace)
}

fn validate_target_namespace(target_namespace: &str, normalized_namespace: &str) -> Result<()> {
    qualify_topic_name(
        "ros_z_debug_namespace_probe",
        normalized_namespace,
        TARGET_NODE_PLACEHOLDER,
    )
    .map(|_| ())
    .map_err(|error| Error::InvalidTargetNamespace {
        target_namespace: target_namespace.to_string(),
        source: error,
    })
}

fn validate_node_name(namespace: &str, node_name: &str) -> Result<()> {
    qualify_topic_name("ros_z_debug_node_probe", namespace, node_name)
        .map(|_| ())
        .map_err(|source| Error::InvalidTargetNodeName {
            target_node_name: node_name.to_string(),
            source,
        })
}

fn normalize_namespace(namespace: &str) -> String {
    let namespace = namespace.trim_matches('/');
    if namespace.is_empty() {
        "/".to_string()
    } else {
        format!("/{namespace}")
    }
}

fn display_name(active_namespace: &str, resolved_topic: &str) -> (String, ProjectedTopicScope) {
    if active_namespace == "/" {
        return (
            resolved_topic.trim_start_matches('/').to_string(),
            ProjectedTopicScope::RelativeToActiveNamespace,
        );
    }

    let namespace_prefix = format!("{active_namespace}/");
    if let Some(relative_name) = resolved_topic.strip_prefix(&namespace_prefix) {
        (
            relative_name.to_string(),
            ProjectedTopicScope::RelativeToActiveNamespace,
        )
    } else {
        (
            resolved_topic.to_string(),
            ProjectedTopicScope::FullyQualified,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{ProjectedTopicScope, TargetIdentity, TopicProjection, TopicReference};
    use crate::Error;

    #[test]
    fn relative_topic_reference_resolves_nested_path_under_target_namespace() {
        let topic = TopicReference::new("my_data/foo").unwrap();
        let identity = TargetIdentity::new("alpha").unwrap();

        assert_eq!(topic.resolve(&identity).unwrap(), "/alpha/my_data/foo");
    }

    #[test]
    fn relative_topic_reference_trims_trailing_slash() {
        let topic = TopicReference::new("foo/").unwrap();
        let identity = TargetIdentity::new("alpha").unwrap();

        assert_eq!(topic.resolve(&identity).unwrap(), "/alpha/foo");
    }

    #[test]
    fn target_namespace_trims_trailing_slash() {
        let topic = TopicReference::new("foo").unwrap();
        let identity = TargetIdentity::new("/alpha/").unwrap();

        assert_eq!(topic.resolve(&identity).unwrap(), "/alpha/foo");
    }

    #[test]
    fn absolute_topic_reference_ignores_namespace() {
        let topic = TopicReference::new("/diagnostics").unwrap();
        let identity = TargetIdentity::new("alpha").unwrap();

        assert_eq!(topic.resolve(&identity).unwrap(), "/diagnostics");
    }

    #[test]
    fn target_identity_rejects_invalid_namespace() {
        let error = TargetIdentity::new("alpha%bad").unwrap_err();

        assert!(matches!(
            error,
            Error::InvalidTargetNamespace { ref target_namespace, .. }
                if target_namespace == "alpha%bad"
        ));
    }

    #[test]
    fn relative_topic_reference_resolves_under_target_namespace() {
        let topic = TopicReference::new("ball_position").unwrap();
        let identity = TargetIdentity::new("/42").unwrap();

        assert_eq!(topic.resolve(&identity).unwrap(), "/42/ball_position");
    }

    #[test]
    fn absolute_topic_reference_ignores_target_identity() {
        let topic = TopicReference::new("/diagnostics").unwrap();
        let identity = TargetIdentity::new("/42").unwrap();

        assert_eq!(topic.resolve(&identity).unwrap(), "/diagnostics");
    }

    #[test]
    fn private_topic_reference_resolves_with_target_node_name() {
        let topic = TopicReference::new("~trace").unwrap();
        let identity = TargetIdentity::new("/42")
            .unwrap()
            .with_node_name("behavior_node")
            .unwrap();

        assert_eq!(topic.resolve(&identity).unwrap(), "/42/behavior_node/trace");
    }

    #[test]
    fn private_topic_reference_requires_target_node_name() {
        let topic = TopicReference::new("~trace").unwrap();
        let identity = TargetIdentity::new("/42").unwrap();

        let error = topic.resolve(&identity).unwrap_err();

        assert!(matches!(error, Error::MissingTargetNodeName { .. }));
    }

    #[test]
    fn target_identity_set_namespace_leaves_previous_state_unchanged_on_error() {
        let mut identity = TargetIdentity::new("/alpha").unwrap();

        let error = identity.set_namespace("alpha%bad").unwrap_err();

        assert!(matches!(
            error,
            Error::InvalidTargetNamespace { ref target_namespace, .. }
                if target_namespace == "alpha%bad"
        ));
        assert_eq!(identity.namespace(), "/alpha");
    }

    #[test]
    fn target_identity_set_node_name_leaves_previous_state_unchanged_on_error() {
        let mut identity = TargetIdentity::new("/42")
            .unwrap()
            .with_node_name("behavior_node")
            .unwrap();

        let error = identity.set_node_name("bad%node").unwrap_err();

        assert!(matches!(
            error,
            Error::InvalidTargetNodeName { ref target_node_name, .. }
                if target_node_name == "bad%node"
        ));
        assert_eq!(identity.node_name(), Some("behavior_node"));
    }

    #[test]
    fn private_topic_reference_is_accepted_during_construction() {
        let topic = TopicReference::new("~private").unwrap();

        assert_eq!(topic.as_str(), "~private");
        assert!(topic.is_private());
    }

    #[test]
    fn invalid_relative_topic_reference_is_rejected_during_construction() {
        let error = TopicReference::new("foo%bar").unwrap_err();

        match error {
            Error::InvalidTopicReference { topic, source } => {
                assert_eq!(topic, "foo%bar");
                assert!(source.to_string().contains("invalid component 'foo%bar'"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn projection_displays_active_namespace_topics_as_relative() {
        let projected = TopicProjection::project("alpha", ["/alpha/foo", "/beta/foo"]).unwrap();

        assert!(projected.iter().any(|topic| topic.display_name == "foo"
            && topic.resolved_topic == "/alpha/foo"
            && topic.scope == ProjectedTopicScope::RelativeToActiveNamespace));
        assert!(
            projected
                .iter()
                .any(|topic| topic.display_name == "/beta/foo"
                    && topic.resolved_topic == "/beta/foo"
                    && topic.scope == ProjectedTopicScope::FullyQualified)
        );
    }

    #[test]
    fn projection_canonicalizes_relative_topics_with_trailing_slash() {
        let projected = TopicProjection::project("alpha", ["foo/"]).unwrap();

        assert_eq!(projected[0].display_name, "foo");
        assert_eq!(projected[0].resolved_topic, "/alpha/foo");
    }

    #[test]
    fn projection_preserves_duplicate_resolved_topics() {
        let projected = TopicProjection::project("alpha", ["/alpha/foo", "/alpha/foo"]).unwrap();

        let duplicated = projected
            .iter()
            .filter(|topic| topic.display_name == "foo")
            .collect::<Vec<_>>();
        assert_eq!(duplicated.len(), 2);
    }

    #[test]
    fn projection_rejects_invalid_relative_topics_using_ros_z_validation() {
        let error = TopicProjection::project("alpha", ["foo%bar"]).unwrap_err();

        match error {
            Error::InvalidTopicReference { topic, source } => {
                assert_eq!(topic, "foo%bar");
                assert!(source.to_string().contains("invalid component 'foo%bar'"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
