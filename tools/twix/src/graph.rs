use std::collections::BTreeSet;

use ros_z::entity::{EndpointEntity, EndpointKind};

pub struct TopicCompletionQuery<'a> {
    active_namespace: &'a str,
    input: &'a str,
    endpoint_kind: Option<EndpointKind>,
    type_name: Option<String>,
}

impl<'a> TopicCompletionQuery<'a> {
    pub fn new(active_namespace: &'a str, input: &'a str) -> Self {
        Self {
            active_namespace,
            input,
            endpoint_kind: None,
            type_name: None,
        }
    }

    pub fn endpoint_kind(mut self, kind: EndpointKind) -> Self {
        self.endpoint_kind = Some(kind);
        self
    }

    pub fn type_name(mut self, type_name: impl Into<String>) -> Self {
        self.type_name = Some(type_name.into());
        self
    }

    pub fn complete<'b>(self, endpoints: impl Iterator<Item = &'b EndpointEntity>) -> Vec<String> {
        let namespace_prefix = completion_namespace_prefix(self.active_namespace);
        let absolute = self.input.starts_with('/');

        endpoints
            .filter(|endpoint| self.endpoint_kind.is_none_or(|kind| endpoint.kind == kind))
            .filter(|endpoint| {
                self.type_name
                    .as_ref()
                    .is_none_or(|type_name| endpoint.type_info.name == *type_name)
            })
            .filter_map(|endpoint| {
                if absolute {
                    return Some(endpoint.topic.clone());
                }

                endpoint
                    .topic
                    .strip_prefix(&namespace_prefix)
                    .map(ToString::to_string)
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

fn completion_namespace_prefix(namespace: &str) -> String {
    let namespace = namespace.trim_matches('/');
    if namespace.is_empty() {
        "/".to_string()
    } else {
        format!("/{namespace}/")
    }
}

#[cfg(test)]
mod tests {
    use ros_z::entity::{EndpointEntity, EndpointKind, NodeEntity, SchemaHash, TypeInfo};

    use super::TopicCompletionQuery;

    fn endpoint(kind: EndpointKind, topic: &str) -> EndpointEntity {
        EndpointEntity {
            id: 1,
            node: NodeEntity::new(Default::default(), 1, "node".to_string(), "/42".to_string()),
            kind,
            topic: topic.to_string(),
            type_info: TypeInfo::new("std_msgs::String", SchemaHash::zero()),
            qos: Default::default(),
        }
    }

    #[test]
    fn relative_input_suggests_publishers_under_active_namespace() {
        let endpoints = [
            endpoint(EndpointKind::Publisher, "/42/status"),
            endpoint(EndpointKind::Publisher, "/42/camera/image"),
            endpoint(EndpointKind::Publisher, "/43/status"),
        ];

        let suggestions = TopicCompletionQuery::new("/42", "sta")
            .endpoint_kind(EndpointKind::Publisher)
            .complete(endpoints.iter());

        assert_eq!(
            suggestions,
            vec!["camera/image".to_string(), "status".to_string()]
        );
    }

    #[test]
    fn absolute_input_suggests_all_publishers_as_absolute_topics() {
        let endpoints = [
            endpoint(EndpointKind::Publisher, "/42/status"),
            endpoint(EndpointKind::Publisher, "/43/status"),
        ];

        let suggestions = TopicCompletionQuery::new("/42", "/")
            .endpoint_kind(EndpointKind::Publisher)
            .complete(endpoints.iter());

        assert_eq!(
            suggestions,
            vec!["/42/status".to_string(), "/43/status".to_string()]
        );
    }

    #[test]
    fn completion_ignores_subscriber_only_topics_and_deduplicates_publishers() {
        let endpoints = [
            endpoint(EndpointKind::Publisher, "/42/status"),
            endpoint(EndpointKind::Publisher, "/42/status"),
            endpoint(EndpointKind::Subscription, "/42/command"),
        ];

        let suggestions = TopicCompletionQuery::new("/42", "")
            .endpoint_kind(EndpointKind::Publisher)
            .complete(endpoints.iter());

        assert_eq!(suggestions, vec!["status".to_string()]);
    }

    #[test]
    fn query_can_filter_by_endpoint_kind_and_type_name() {
        let endpoints = [
            EndpointEntity {
                type_info: TypeInfo::new("types::Image", SchemaHash::zero()),
                ..endpoint(EndpointKind::Publisher, "/42/camera/image")
            },
            EndpointEntity {
                type_info: TypeInfo::new("std_msgs::String", SchemaHash::zero()),
                ..endpoint(EndpointKind::Publisher, "/42/status")
            },
            EndpointEntity {
                type_info: TypeInfo::new("types::Image", SchemaHash::zero()),
                ..endpoint(EndpointKind::Subscription, "/42/camera/command")
            },
        ];

        let suggestions = TopicCompletionQuery::new("/42", "")
            .endpoint_kind(EndpointKind::Publisher)
            .type_name("types::Image")
            .complete(endpoints.iter());

        assert_eq!(suggestions, vec!["camera/image".to_string()]);
    }
}
