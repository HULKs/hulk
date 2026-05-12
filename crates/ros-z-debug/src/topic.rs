use std::collections::HashMap;

use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TopicSelector(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedTopic {
    pub display_name: String,
    pub resolved_topic: String,
    pub ambiguous: bool,
}

pub struct TopicProjection;

impl TopicSelector {
    pub fn new(selector: impl Into<String>) -> Result<Self> {
        let selector = selector.into();

        if selector.is_empty() {
            return Err(Error::InvalidTopicSelector {
                selector,
                reason: "topic selector must not be empty".to_string(),
            });
        }

        if selector.starts_with('~') {
            return Err(Error::InvalidTopicSelector {
                selector,
                reason: "private topics are not supported in milestone 1".to_string(),
            });
        }

        Ok(Self(selector))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_absolute(&self) -> bool {
        self.0.starts_with('/')
    }

    pub fn resolve(&self, namespace: &str) -> Result<String> {
        if self.0.starts_with('~') {
            return Err(Error::InvalidTopicSelector {
                selector: self.0.clone(),
                reason: "private topics are not supported in milestone 1".to_string(),
            });
        }

        if self.is_absolute() {
            return Ok(normalize_absolute_topic(&self.0));
        }

        let namespace = normalize_namespace(namespace);
        if namespace == "/" {
            Ok(format!("/{}", self.0.trim_start_matches('/')))
        } else {
            Ok(format!("{}/{}", namespace, self.0.trim_start_matches('/')))
        }
    }
}

impl TopicProjection {
    pub fn project(
        active_namespace: &str,
        topics: impl IntoIterator<Item = String>,
    ) -> Vec<ProjectedTopic> {
        let active_namespace = normalize_namespace(active_namespace);
        let mut projected = topics
            .into_iter()
            .map(|topic| {
                let resolved_topic = if topic.starts_with('/') {
                    normalize_absolute_topic(&topic)
                } else if active_namespace == "/" {
                    format!("/{}", topic.trim_start_matches('/'))
                } else {
                    format!("{}/{}", active_namespace, topic.trim_start_matches('/'))
                };
                let display_name = display_name(&active_namespace, &resolved_topic);

                ProjectedTopic {
                    display_name,
                    resolved_topic,
                    ambiguous: false,
                }
            })
            .collect::<Vec<_>>();

        let mut display_name_counts = HashMap::new();
        for topic in &projected {
            *display_name_counts
                .entry(topic.display_name.clone())
                .or_insert(0usize) += 1;
        }

        for topic in &mut projected {
            topic.ambiguous = display_name_counts[&topic.display_name] > 1;
        }

        projected
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

fn normalize_absolute_topic(topic: &str) -> String {
    let topic = topic.trim_end_matches('/');
    if topic.is_empty() {
        "/".to_string()
    } else if topic.starts_with('/') {
        topic.to_string()
    } else {
        format!("/{topic}")
    }
}

fn display_name(active_namespace: &str, resolved_topic: &str) -> String {
    if active_namespace == "/" {
        return resolved_topic.trim_start_matches('/').to_string();
    }

    let namespace_prefix = format!("{active_namespace}/");
    resolved_topic
        .strip_prefix(&namespace_prefix)
        .unwrap_or(resolved_topic)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{TopicProjection, TopicSelector};

    #[test]
    fn relative_selector_resolves_under_namespace() {
        let selector = TopicSelector::new("my_data/foo").unwrap();

        assert_eq!(selector.resolve("alpha").unwrap(), "/alpha/my_data/foo");
    }

    #[test]
    fn absolute_selector_ignores_namespace() {
        let selector = TopicSelector::new("/diagnostics").unwrap();

        assert_eq!(selector.resolve("alpha").unwrap(), "/diagnostics");
    }

    #[test]
    fn private_selector_is_rejected_in_milestone_one() {
        let error = TopicSelector::new("~private").unwrap_err();

        assert!(
            error
                .to_string()
                .contains("private topics are not supported")
        );
    }

    #[test]
    fn projection_displays_active_namespace_topics_as_relative() {
        let projected =
            TopicProjection::project("alpha", ["/alpha/foo".to_string(), "/beta/foo".to_string()]);

        assert!(
            projected
                .iter()
                .any(|topic| topic.display_name == "foo" && topic.resolved_topic == "/alpha/foo")
        );
        assert!(
            projected
                .iter()
                .any(|topic| topic.display_name == "/beta/foo"
                    && topic.resolved_topic == "/beta/foo")
        );
    }

    #[test]
    fn projection_marks_duplicate_display_names_ambiguous() {
        let projected = TopicProjection::project(
            "alpha",
            ["/alpha/foo".to_string(), "/alpha/foo".to_string()],
        );

        let ambiguous = projected
            .iter()
            .filter(|topic| topic.display_name == "foo")
            .collect::<Vec<_>>();
        assert_eq!(ambiguous.len(), 2);
        assert!(ambiguous.iter().all(|topic| topic.ambiguous));
    }
}
