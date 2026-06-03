use std::collections::HashMap;

use crate::entity::{Entity, LivelinessKE};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum GraphMutation {
    Inserted(Entity),
    Removed(Entity),
    Replaced { old: Entity, new: Entity },
    Unchanged,
}

pub(super) struct GraphData {
    entities: HashMap<LivelinessKE, Entity>,
}

impl GraphData {
    pub(super) fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    pub(super) fn insert(&mut self, key_expr: LivelinessKE, entity: Entity) -> GraphMutation {
        match self.entities.get_mut(&key_expr) {
            None => {
                self.entities.insert(key_expr, entity.clone());
                GraphMutation::Inserted(entity)
            }
            Some(current) if current == &entity => GraphMutation::Unchanged,
            Some(current) => {
                let old = std::mem::replace(current, entity.clone());
                GraphMutation::Replaced { old, new: entity }
            }
        }
    }

    pub(super) fn remove(&mut self, key_expr: &LivelinessKE) -> GraphMutation {
        self.entities
            .remove(key_expr)
            .map(GraphMutation::Removed)
            .unwrap_or(GraphMutation::Unchanged)
    }

    pub(super) fn entities(&self) -> impl Iterator<Item = &Entity> + '_ {
        self.entities.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{
        EndpointEntity, EndpointKind, NodeEntity, SchemaHash, TypeInfo,
        entity_to_liveliness_key_expr,
    };
    use zenoh::session::ZenohId;

    fn node(name: &str) -> NodeEntity {
        NodeEntity::new(ZenohId::default(), 1, name.to_string(), String::new())
    }

    fn publisher(node: &NodeEntity, id: usize, topic: &str) -> EndpointEntity {
        EndpointEntity {
            id,
            node: node.clone(),
            kind: EndpointKind::Publisher,
            topic: topic.to_string(),
            type_info: TypeInfo::new("std_msgs::String", SchemaHash::zero()),
            qos: Default::default(),
        }
    }

    fn key_for(entity: &Entity) -> LivelinessKE {
        entity_to_liveliness_key_expr(entity).expect("test entity should format as liveliness key")
    }

    #[test]
    fn insert_stores_one_entity_by_liveliness_key() {
        let mut data = GraphData::new();
        let entity = Entity::Node(node("inserted_node"));
        let key = key_for(&entity);

        assert_eq!(
            data.insert(key, entity.clone()),
            GraphMutation::Inserted(entity.clone())
        );
        assert_eq!(data.entities().collect::<Vec<_>>(), vec![&entity]);
    }

    #[test]
    fn duplicate_insert_is_unchanged_and_does_not_duplicate() {
        let mut data = GraphData::new();
        let entity = Entity::Node(node("duplicate_node"));
        let key = key_for(&entity);

        assert_eq!(
            data.insert(key.clone(), entity.clone()),
            GraphMutation::Inserted(entity.clone())
        );
        assert_eq!(data.insert(key, entity), GraphMutation::Unchanged);
        assert_eq!(data.entities().count(), 1);
    }

    #[test]
    fn replacing_same_key_reports_old_and_new_entities() {
        let mut data = GraphData::new();
        let node = node("replace_node");
        let old = Entity::Endpoint(publisher(&node, 2, "/old_topic"));
        let new = Entity::Endpoint(publisher(&node, 3, "/new_topic"));
        let key = key_for(&old);

        assert_eq!(
            data.insert(key.clone(), old.clone()),
            GraphMutation::Inserted(old.clone())
        );
        assert_eq!(
            data.insert(key, new.clone()),
            GraphMutation::Replaced {
                old,
                new: new.clone()
            }
        );
        assert_eq!(data.entities().collect::<Vec<_>>(), vec![&new]);
    }

    #[test]
    fn delete_removes_only_matching_liveliness_key() {
        let mut data = GraphData::new();
        let first = Entity::Node(node("first_node"));
        let second = Entity::Node(NodeEntity::new(
            ZenohId::default(),
            2,
            "second_node".to_string(),
            String::new(),
        ));
        let first_key = key_for(&first);
        let second_key = key_for(&second);

        data.insert(first_key.clone(), first.clone());
        data.insert(second_key, second.clone());

        assert_eq!(data.remove(&first_key), GraphMutation::Removed(first));
        assert_eq!(data.entities().collect::<Vec<_>>(), vec![&second]);
    }

    #[test]
    fn deleting_unknown_liveliness_key_is_unchanged() {
        let mut data = GraphData::new();
        let entity = Entity::Node(node("missing_node"));
        let key = key_for(&entity);

        assert_eq!(data.remove(&key), GraphMutation::Unchanged);
        assert_eq!(data.entities().count(), 0);
    }
}
