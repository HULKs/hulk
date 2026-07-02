use std::collections::BTreeSet;

use ros_z::entity::{EndpointEntity, EndpointKind};

pub fn publisher_topic_completions<'a>(
    publishers: impl Iterator<Item = &'a EndpointEntity>,
    active_namespace: &str,
    input: &str,
) -> Vec<String> {
    let namespace_prefix = completion_namespace_prefix(active_namespace);
    let absolute = input.starts_with('/');

    publishers
        .filter(|endpoint| endpoint.kind == EndpointKind::Publisher)
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

    use super::publisher_topic_completions;

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

        let suggestions = publisher_topic_completions(endpoints.iter(), "/42", "sta");

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

        let suggestions = publisher_topic_completions(endpoints.iter(), "/42", "/");

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

        let suggestions = publisher_topic_completions(endpoints.iter(), "/42", "");

        assert_eq!(suggestions, vec!["status".to_string()]);
    }
}
