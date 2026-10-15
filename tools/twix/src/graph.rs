use std::collections::BTreeSet;

use ros_z_debug::query::TopicInfo;

pub struct TopicCompletionQuery<'a> {
    active_namespace: &'a str,
    input: &'a str,
    type_name: Option<String>,
}

impl<'a> TopicCompletionQuery<'a> {
    pub fn new(active_namespace: &'a str, input: &'a str) -> Self {
        Self {
            active_namespace,
            input,
            type_name: None,
        }
    }

    pub fn type_name(mut self, type_name: impl Into<String>) -> Self {
        self.type_name = Some(type_name.into());
        self
    }

    pub fn complete_topics(self, topics: &[TopicInfo]) -> Vec<String> {
        let namespace = self.active_namespace.trim_matches('/');
        let prefix = if namespace.is_empty() {
            "/".into()
        } else {
            format!("/{namespace}/")
        };
        topics
            .iter()
            .filter(|topic| {
                self.type_name
                    .as_ref()
                    .is_none_or(|name| topic.type_info.name == *name)
            })
            .filter_map(|topic| {
                if self.input.starts_with('/') {
                    Some(topic.name.clone())
                } else {
                    topic.name.strip_prefix(&prefix).map(str::to_owned)
                }
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_completion_respects_namespace_type_and_duplicates() {
        let topic = |name: &str, kind| TopicInfo {
            name: name.into(),
            type_info: ros_z::TypeInfo::new(kind, ros_z::SchemaHash::zero()),
            schema: None,
            error: None,
        };
        let topics = [
            topic("/42/image", "image"),
            topic("/42/image", "image"),
            topic("/43/image", "image"),
            topic("/42/status", "status"),
        ];
        assert_eq!(
            TopicCompletionQuery::new("/42/", "")
                .type_name("image")
                .complete_topics(&topics),
            ["image"]
        );
        assert_eq!(
            TopicCompletionQuery::new("/42", "/")
                .type_name("image")
                .complete_topics(&topics),
            ["/42/image", "/43/image"]
        );
    }
}
