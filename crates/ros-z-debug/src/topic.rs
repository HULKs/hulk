use ros_z::topic_name::qualify_topic_name;

use crate::{Error, Result};

const TARGET_NODE_PLACEHOLDER: &str = "debug_target";

/// Validated topic selector used by debug subscriptions.
///
/// Relative selectors resolve against a target namespace. Absolute selectors
/// are used as-is through `ros-z` topic qualification. Private `~` selectors are
/// rejected because the debug manager has target namespace context, not target
/// node context.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TopicSelector(String);

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

impl TopicSelector {
    /// Validate and store a topic selector.
    pub fn new(selector: impl Into<String>) -> Result<Self> {
        let selector = selector.into();

        if selector.starts_with('~') {
            return Err(Error::UnsupportedPrivateTopicSelector { selector });
        }

        resolve_topic_name(&selector, "/")?;

        Ok(Self(selector))
    }

    /// Return the original selector string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Return whether this selector is absolute.
    pub fn is_absolute(&self) -> bool {
        self.0.starts_with('/')
    }

    /// Resolve this selector against `target_namespace`.
    pub fn resolve(&self, target_namespace: &str) -> Result<String> {
        resolve_topic_name(&self.0, target_namespace)
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
        let active_namespace = normalize_target_namespace(active_namespace)?;
        topics
            .into_iter()
            .map(|topic| {
                let resolved_topic = resolve_topic_name(topic.as_ref(), &active_namespace)?;
                let (display_name, scope) = display_name(&active_namespace, &resolved_topic);

                Ok(ProjectedTopic {
                    display_name,
                    resolved_topic,
                    scope,
                })
            })
            .collect::<Result<Vec<_>>>()
    }
}

fn resolve_topic_name(selector: &str, target_namespace: &str) -> Result<String> {
    if selector.starts_with('~') {
        return Err(Error::UnsupportedPrivateTopicSelector {
            selector: selector.to_string(),
        });
    }

    let namespace = if selector.starts_with('/') {
        "/".to_string()
    } else {
        normalize_target_namespace(target_namespace)?
    };

    qualify_topic_name(selector, &namespace, TARGET_NODE_PLACEHOLDER).map_err(|error| {
        Error::InvalidTopicSelector {
            selector: selector.to_string(),
            source: error,
        }
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
    use std::error::Error as _;

    use super::{ProjectedTopicScope, TopicProjection, TopicSelector};
    use crate::Error;

    #[test]
    fn relative_selector_resolves_under_target_namespace() {
        let selector = TopicSelector::new("my_data/foo").unwrap();

        assert_eq!(selector.resolve("alpha").unwrap(), "/alpha/my_data/foo");
    }

    #[test]
    fn relative_selector_trims_trailing_slash() {
        let selector = TopicSelector::new("foo/").unwrap();

        assert_eq!(selector.resolve("alpha").unwrap(), "/alpha/foo");
    }

    #[test]
    fn target_namespace_trims_trailing_slash() {
        let selector = TopicSelector::new("foo").unwrap();

        assert_eq!(selector.resolve("/alpha/").unwrap(), "/alpha/foo");
    }

    #[test]
    fn absolute_selector_ignores_namespace() {
        let selector = TopicSelector::new("/diagnostics").unwrap();

        assert_eq!(selector.resolve("alpha").unwrap(), "/diagnostics");
    }

    #[test]
    fn absolute_selector_does_not_validate_target_namespace() {
        let selector = TopicSelector::new("/diagnostics").unwrap();

        assert_eq!(selector.resolve("123invalid/").unwrap(), "/diagnostics");
    }

    #[test]
    fn relative_selector_reports_invalid_target_namespace() {
        let selector = TopicSelector::new("diagnostics").unwrap();

        let error = selector.resolve("alpha%bad").unwrap_err();

        assert!(
            error
                .to_string()
                .contains("invalid target namespace 'alpha%bad'")
        );
    }

    #[test]
    fn private_selector_rejection_explains_missing_target_node() {
        let error = TopicSelector::new("~private").unwrap_err();

        assert!(matches!(
            error,
            Error::UnsupportedPrivateTopicSelector { ref selector } if selector == "~private"
        ));
    }

    #[test]
    fn invalid_relative_selector_is_rejected_during_construction() {
        let error = TopicSelector::new("foo%bar").unwrap_err();

        assert!(error.source().is_some());
        assert!(error.to_string().contains("invalid component 'foo%bar'"));
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

        assert!(error.to_string().contains("invalid component 'foo%bar'"));
    }
}
