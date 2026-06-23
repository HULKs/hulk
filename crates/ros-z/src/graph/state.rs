use std::collections::{HashMap, hash_map::Entry};
use std::sync::Arc;

use parking_lot::{Mutex, MutexGuard};
use tokio::sync::watch;

use super::GraphRevision;
use crate::entity::{Entity, LivelinessKE};

pub(super) struct GraphData {
    entities: HashMap<LivelinessKE, Entity>,
}

pub(super) struct GraphState {
    data: GraphData,
    revision: GraphRevision,
}

impl GraphState {
    pub(super) fn revision(&self) -> GraphRevision {
        self.revision
    }

    pub(super) fn entities(&self) -> impl Iterator<Item = &Entity> + '_ {
        self.data.entities()
    }
}

#[derive(Clone)]
pub(crate) struct GraphStore {
    state: Arc<Mutex<GraphState>>,
    revision_tx: watch::Sender<GraphRevision>,
}

impl GraphStore {
    pub(super) fn new() -> Self {
        let (revision_tx, _) = watch::channel(GraphRevision::INITIAL);
        Self {
            state: Arc::new(Mutex::new(GraphState {
                data: GraphData::new(),
                revision: GraphRevision::INITIAL,
            })),
            revision_tx,
        }
    }

    pub(super) fn revision(&self) -> GraphRevision {
        *self.revision_tx.borrow()
    }

    pub(super) fn subscribe_changes(&self) -> watch::Receiver<GraphRevision> {
        self.revision_tx.subscribe()
    }

    pub(super) fn state(&self) -> MutexGuard<'_, GraphState> {
        self.state.lock()
    }

    pub(super) fn insert(&self, key_expr: LivelinessKE, entity: Entity) -> bool {
        self.apply_effective_change(|data| data.insert(key_expr, entity))
    }

    pub(super) fn remove(&self, key_expr: &LivelinessKE) -> bool {
        self.apply_effective_change(|data| data.remove(key_expr))
    }

    fn apply_effective_change(&self, update: impl FnOnce(&mut GraphData) -> bool) -> bool {
        let mut state = self.state.lock();
        if !update(&mut state.data) {
            return false;
        }

        let revision = state.revision.next();
        state.revision = revision;

        self.revision_tx.send_if_modified(|current| {
            if *current >= revision {
                return false;
            }
            *current = revision;
            true
        });
        true
    }
}

impl GraphData {
    pub(super) fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    pub(super) fn insert(&mut self, key_expr: LivelinessKE, entity: Entity) -> bool {
        match self.entities.entry(key_expr) {
            Entry::Vacant(entry) => {
                entry.insert(entity);
                true
            }
            Entry::Occupied(mut entry) if entry.get() != &entity => {
                entry.insert(entity);
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    pub(super) fn remove(&mut self, key_expr: &LivelinessKE) -> bool {
        self.entities.remove(key_expr).is_some()
    }

    pub(super) fn entities(&self) -> impl Iterator<Item = &Entity> + '_ {
        self.entities.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EndpointEntity, EndpointKind, NodeEntity, SchemaHash, TypeInfo};
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
        entity
            .liveliness_key_expr()
            .expect("test entity should format as liveliness key")
    }

    #[test]
    fn insert_reports_change_for_new_entity() {
        let mut data = GraphData::new();
        let entity = Entity::Node(node("inserted_node"));
        let key = key_for(&entity);

        assert!(data.insert(key, entity.clone()));
        assert_eq!(data.entities().collect::<Vec<_>>(), vec![&entity]);
    }

    #[test]
    fn duplicate_insert_reports_no_change() {
        let mut data = GraphData::new();
        let entity = Entity::Node(node("duplicate_node"));
        let key = key_for(&entity);

        assert!(data.insert(key.clone(), entity.clone()));
        assert!(!data.insert(key, entity));
        assert_eq!(data.entities().count(), 1);
    }

    #[test]
    fn replacing_same_key_reports_change() {
        let mut data = GraphData::new();
        let node = node("replace_node");
        let old = Entity::Endpoint(publisher(&node, 2, "/old_topic"));
        let new = Entity::Endpoint(publisher(&node, 3, "/new_topic"));
        let key = key_for(&old);

        assert!(data.insert(key.clone(), old));
        assert!(data.insert(key, new.clone()));
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

        assert!(data.remove(&first_key));
        assert_eq!(data.entities().collect::<Vec<_>>(), vec![&second]);
    }

    #[test]
    fn deleting_unknown_liveliness_key_is_unchanged() {
        let mut data = GraphData::new();
        let entity = Entity::Node(node("missing_node"));
        let key = key_for(&entity);

        assert!(!data.remove(&key));
        assert_eq!(data.entities().count(), 0);
    }

    #[test]
    fn store_insert_and_remove_advance_revision_only_for_effective_changes() {
        let store = GraphStore::new();
        let entity = Entity::Node(node("store_revision_node"));
        let key = key_for(&entity);

        assert_eq!(store.revision(), GraphRevision::INITIAL);
        assert!(store.insert(key.clone(), entity.clone()));
        let insert_revision = store.revision();
        assert!(insert_revision > GraphRevision::INITIAL);

        assert!(!store.insert(key.clone(), entity.clone()));
        assert_eq!(store.revision(), insert_revision);

        assert!(store.remove(&key));
        let remove_revision = store.revision();
        assert!(remove_revision > insert_revision);

        assert!(!store.remove(&key));
        assert_eq!(store.revision(), remove_revision);
    }
}
