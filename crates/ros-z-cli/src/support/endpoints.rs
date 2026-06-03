use std::{collections::BTreeSet, sync::Arc};

use ros_z::entity::Entity;

use crate::model::info::{EndpointSummary, NamedType};

pub fn summarize_endpoints(entities: Vec<Arc<Entity>>) -> Vec<EndpointSummary> {
    let endpoints = entities.iter().filter_map(|entity| match &**entity {
        Entity::Endpoint(endpoint) => Some(endpoint),
        Entity::Node(_) => None,
    });
    summarize_endpoint_entities(endpoints)
}

pub fn summarize_endpoint_entities<'a>(
    endpoints: impl IntoIterator<Item = &'a ros_z::entity::EndpointEntity>,
) -> Vec<EndpointSummary> {
    let mut summaries = BTreeSet::new();

    for endpoint in endpoints {
        let node = endpoint.node.fully_qualified_name();
        let schema_hash = endpoint.type_info.hash.to_string();
        summaries.insert((node, schema_hash));
    }

    summaries
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
    fn summarize_endpoints_formats_present_schema_hash() {
        let hash = SchemaHash([0xcd; 32]);
        let expected = hash.to_string();
        let entities = vec![Arc::new(Entity::Endpoint(EndpointEntity {
            id: 7,
            node: ros_z::entity::NodeEntity {
                z_id: Default::default(),
                id: 1,
                name: "node".to_string(),
                namespace: "/".to_string(),
            },
            kind: EndpointKind::Publisher,
            topic: "/demo".to_string(),
            type_info: TypeInfo::new("std_msgs::String", hash),
            qos: Default::default(),
        }))];

        let summaries = summarize_endpoints(entities);

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].schema_hash, expected.as_str());
    }
}
