use std::collections::{BTreeMap, BTreeSet};

use ros_z::graph::GraphSnapshot;

use crate::model::watch::WatchEvent;

pub fn diff_snapshots(previous: &GraphSnapshot, current: &GraphSnapshot) -> Vec<WatchEvent> {
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
