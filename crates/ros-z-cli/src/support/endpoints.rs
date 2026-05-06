use std::{collections::BTreeSet, sync::Arc};

use ros_z::entity::{Entity, entity_get_endpoint};

use crate::{
    model::info::{EndpointSummary, NamedType},
    support::nodes::fully_qualified_node_name,
};

pub fn summarize_endpoints(entities: Vec<Arc<Entity>>) -> Vec<EndpointSummary> {
    let mut endpoints = BTreeSet::new();

    for entity in entities {
        if let Some(endpoint) = entity_get_endpoint(&entity) {
            let node = endpoint
                .node
                .as_ref()
                .map(|node| fully_qualified_node_name(&node.namespace, &node.name));
            let schema_hash = endpoint
                .type_info
                .as_ref()
                .and_then(|type_info| type_info.hash.as_ref().map(|hash| hash.to_string()));
            endpoints.insert((node, schema_hash));
        }
    }

    endpoints
        .into_iter()
        .map(|(node, schema_hash)| EndpointSummary { node, schema_hash })
        .collect()
}

pub fn named_types(entries: Vec<(String, String)>) -> Vec<NamedType> {
    let unique: BTreeSet<_> = entries.into_iter().collect();
    unique
        .into_iter()
        .map(|(name, type_name)| NamedType::new(name, type_name))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ros_z::entity::{EndpointEntity, EndpointKind, Entity, SchemaHash, TypeInfo};

    use super::summarize_endpoints;

    #[test]
    fn summarize_endpoints_keeps_missing_schema_hash() {
        let entities = vec![Arc::new(Entity::Endpoint(EndpointEntity {
            id: 7,
            node: None,
            kind: EndpointKind::Publisher,
            topic: "/demo".to_string(),
            type_info: Some(TypeInfo::new("std_msgs::String", None)),
            qos: Default::default(),
        }))];

        let summaries = summarize_endpoints(entities);

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].schema_hash, None);
    }

    #[test]
    fn summarize_endpoints_formats_present_schema_hash() {
        let hash = SchemaHash([0xcd; 32]);
        let expected = hash.to_string();
        let entities = vec![Arc::new(Entity::Endpoint(EndpointEntity {
            id: 7,
            node: None,
            kind: EndpointKind::Publisher,
            topic: "/demo".to_string(),
            type_info: Some(TypeInfo::with_hash("std_msgs::String", hash)),
            qos: Default::default(),
        }))];

        let summaries = summarize_endpoints(entities);

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].schema_hash.as_deref(), Some(expected.as_str()));
    }
}
