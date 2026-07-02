use std::collections::{BTreeMap, BTreeSet};

use ros_z::{entity::EndpointEntity, entity::EndpointKind, graph::GraphData};

use crate::model::{
    graph::{GraphSummary, NodeSummary, ServiceSummary, TopicSummary},
    watch::WatchEvent,
};

#[derive(Default)]
struct EndpointAggregate {
    type_names: BTreeSet<String>,
    publishers: usize,
    subscribers: usize,
    services: usize,
    clients: usize,
}

pub fn graph_summary(data: &GraphData) -> GraphSummary {
    GraphSummary {
        revision: data.revision(),
        topics: topic_summaries(data),
        nodes: node_summaries(data),
        services: service_summaries(data),
    }
}

pub fn topic_summaries(data: &GraphData) -> Vec<TopicSummary> {
    let mut by_topic = BTreeMap::<String, EndpointAggregate>::new();
    for endpoint in data.endpoints() {
        match endpoint.kind {
            EndpointKind::Publisher => {
                let aggregate = by_topic.entry(endpoint.topic.clone()).or_default();
                aggregate.type_names.insert(endpoint.type_info.name.clone());
                aggregate.publishers += 1;
            }
            EndpointKind::Subscription => {
                let aggregate = by_topic.entry(endpoint.topic.clone()).or_default();
                aggregate.type_names.insert(endpoint.type_info.name.clone());
                aggregate.subscribers += 1;
            }
            EndpointKind::Service | EndpointKind::Client => {}
        }
    }

    by_topic
        .into_iter()
        .map(|(name, aggregate)| {
            let type_name = aggregate.type_names.into_iter().next().unwrap_or_default();
            TopicSummary::new(name, type_name, aggregate.publishers, aggregate.subscribers)
        })
        .collect()
}

pub fn node_summaries(data: &GraphData) -> Vec<NodeSummary> {
    data.nodes()
        .map(|node| NodeSummary::new(node.name.clone(), normalized_namespace(&node.namespace)))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn service_summaries(data: &GraphData) -> Vec<ServiceSummary> {
    service_summaries_from_endpoints(data.endpoints())
}

fn service_summaries_from_endpoints<'a>(
    endpoints: impl IntoIterator<Item = &'a EndpointEntity>,
) -> Vec<ServiceSummary> {
    let mut by_service = BTreeMap::<String, EndpointAggregate>::new();
    let endpoints = endpoints.into_iter().collect::<Vec<_>>();

    for endpoint in &endpoints {
        match endpoint.kind {
            EndpointKind::Service => {
                let aggregate = by_service.entry(endpoint.topic.clone()).or_default();
                aggregate.type_names.insert(endpoint.type_info.name.clone());
                aggregate.services += 1;
            }
            EndpointKind::Client | EndpointKind::Publisher | EndpointKind::Subscription => {}
        }
    }

    for endpoint in endpoints {
        match endpoint.kind {
            EndpointKind::Client => {
                if let Some(aggregate) = by_service.get_mut(&endpoint.topic) {
                    aggregate.clients += 1;
                }
            }
            EndpointKind::Service | EndpointKind::Publisher | EndpointKind::Subscription => {}
        }
    }

    by_service
        .into_iter()
        .map(|(name, aggregate)| {
            let type_name = aggregate.type_names.into_iter().next().unwrap_or_default();
            ServiceSummary::new(name, type_name, aggregate.services, aggregate.clients)
        })
        .collect()
}

pub fn diff_graph_summaries(previous: &GraphSummary, current: &GraphSummary) -> Vec<WatchEvent> {
    let mut events = Vec::new();
    let revision = current.revision;

    let previous_topics: BTreeMap<_, _> = previous
        .topics
        .iter()
        .map(|topic| (topic.name.clone(), topic.type_name.clone()))
        .collect();
    let current_topics: BTreeMap<_, _> = current
        .topics
        .iter()
        .map(|topic| (topic.name.clone(), topic.type_name.clone()))
        .collect();
    for (name, type_name) in &current_topics {
        if !previous_topics.contains_key(name) {
            events.push(WatchEvent::TopicDiscovered {
                revision,
                name: name.clone(),
                type_name: type_name.clone(),
            });
        }
    }
    for name in previous_topics.keys() {
        if !current_topics.contains_key(name) {
            events.push(WatchEvent::TopicRemoved {
                revision,
                name: name.clone(),
            });
        }
    }

    let previous_nodes: BTreeSet<_> = previous
        .nodes
        .iter()
        .map(|node| (node.namespace.clone(), node.name.clone()))
        .collect();
    let current_nodes: BTreeSet<_> = current
        .nodes
        .iter()
        .map(|node| (node.namespace.clone(), node.name.clone()))
        .collect();
    for (namespace, name) in &current_nodes {
        if !previous_nodes.contains(&(namespace.clone(), name.clone())) {
            events.push(WatchEvent::NodeDiscovered {
                revision,
                namespace: namespace.clone(),
                name: name.clone(),
            });
        }
    }
    for (namespace, name) in &previous_nodes {
        if !current_nodes.contains(&(namespace.clone(), name.clone())) {
            events.push(WatchEvent::NodeRemoved {
                revision,
                namespace: namespace.clone(),
                name: name.clone(),
            });
        }
    }

    let previous_services: BTreeMap<_, _> = previous
        .services
        .iter()
        .map(|service| (service.name.clone(), service.type_name.clone()))
        .collect();
    let current_services: BTreeMap<_, _> = current
        .services
        .iter()
        .map(|service| (service.name.clone(), service.type_name.clone()))
        .collect();
    for (name, type_name) in &current_services {
        if !previous_services.contains_key(name) {
            events.push(WatchEvent::ServiceDiscovered {
                revision,
                name: name.clone(),
                type_name: type_name.clone(),
            });
        }
    }
    for name in previous_services.keys() {
        if !current_services.contains_key(name) {
            events.push(WatchEvent::ServiceRemoved {
                revision,
                name: name.clone(),
            });
        }
    }

    events
}

fn normalized_namespace(namespace: &str) -> String {
    if namespace.is_empty() {
        "/".to_string()
    } else if namespace.starts_with('/') {
        namespace.to_string()
    } else {
        format!("/{namespace}")
    }
}

#[cfg(test)]
mod tests {
    use ros_z::entity::{EndpointEntity, EndpointKind, NodeEntity, SchemaHash, TypeInfo};

    use super::service_summaries_from_endpoints;

    fn endpoint(id: usize, kind: EndpointKind, service: &str, type_name: &str) -> EndpointEntity {
        EndpointEntity {
            id,
            node: NodeEntity {
                z_id: Default::default(),
                id,
                name: format!("node_{id}"),
                namespace: "/graph_test".to_string(),
            },
            kind,
            topic: service.to_string(),
            type_info: TypeInfo::new(type_name, SchemaHash::zero()),
            qos: Default::default(),
        }
    }

    #[test]
    fn service_summaries_ignore_client_only_services() {
        let summaries = service_summaries_from_endpoints(
            [endpoint(
                1,
                EndpointKind::Client,
                "/client_only",
                "test_msgs::ClientOnly",
            )]
            .iter(),
        );

        assert!(summaries.is_empty());
    }

    #[test]
    fn service_summaries_count_clients_only_for_existing_services() {
        let endpoints = [
            endpoint(1, EndpointKind::Service, "/served", "test_msgs::Served"),
            endpoint(2, EndpointKind::Client, "/served", "test_msgs::Served"),
            endpoint(
                3,
                EndpointKind::Client,
                "/client_only",
                "test_msgs::ClientOnly",
            ),
        ];

        let summaries = service_summaries_from_endpoints(endpoints.iter());

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].name, "/served");
        assert_eq!(summaries[0].servers, 1);
        assert_eq!(summaries[0].clients, 1);
    }
}
