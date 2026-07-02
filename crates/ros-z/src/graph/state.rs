use std::collections::{HashMap, hash_map::Entry};
use std::sync::Arc;

use parking_lot::{Mutex, MutexGuard};
use tokio::sync::watch;

use super::GraphRevision;
use crate::entity::{Entity, LivelinessKE};

#[derive(Debug, Clone)]
pub struct GraphData {
    revision: GraphRevision,
    entities: HashMap<LivelinessKE, Entity>,
}

pub(super) struct GraphInner {
    data: Mutex<GraphData>,
    revision_tx: watch::Sender<GraphRevision>,
}

impl GraphData {
    pub(super) fn new() -> Self {
        Self {
            revision: GraphRevision::INITIAL,
            entities: HashMap::new(),
        }
    }

    pub fn revision(&self) -> GraphRevision {
        self.revision
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

    pub(super) fn entities_raw(&self) -> impl Iterator<Item = &Entity> + '_ {
        self.entities.values()
    }
}

impl GraphInner {
    pub(super) fn new() -> Arc<Self> {
        let (revision_tx, _) = watch::channel(GraphRevision::INITIAL);
        Arc::new(Self {
            data: Mutex::new(GraphData::new()),
            revision_tx,
        })
    }

    pub(super) fn lock(&self) -> MutexGuard<'_, GraphData> {
        self.data.lock()
    }

    pub(super) fn revision(&self) -> GraphRevision {
        self.data.lock().revision()
    }

    pub(super) fn watch_revisions(&self) -> watch::Receiver<GraphRevision> {
        self.revision_tx.subscribe()
    }

    pub(super) fn insert(&self, key_expr: LivelinessKE, entity: Entity) -> bool {
        self.apply_effective_change(|data| data.insert(key_expr, entity))
    }

    pub(super) fn remove(&self, key_expr: &LivelinessKE) -> bool {
        self.apply_effective_change(|data| data.remove(key_expr))
    }

    fn apply_effective_change(&self, update: impl FnOnce(&mut GraphData) -> bool) -> bool {
        let revision = {
            let mut data = self.data.lock();
            if !update(&mut data) {
                return false;
            }
            data.revision = data.revision.next();
            data.revision
        };

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
        assert_eq!(data.entities_raw().collect::<Vec<_>>(), vec![&entity]);
    }

    #[test]
    fn duplicate_insert_reports_no_change() {
        let mut data = GraphData::new();
        let entity = Entity::Node(node("duplicate_node"));
        let key = key_for(&entity);

        assert!(data.insert(key.clone(), entity.clone()));
        assert!(!data.insert(key, entity));
        assert_eq!(data.entities_raw().count(), 1);
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
        assert_eq!(data.entities_raw().collect::<Vec<_>>(), vec![&new]);
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
        assert_eq!(data.entities_raw().collect::<Vec<_>>(), vec![&second]);
    }

    #[test]
    fn deleting_unknown_liveliness_key_is_unchanged() {
        let mut data = GraphData::new();
        let entity = Entity::Node(node("missing_node"));
        let key = key_for(&entity);

        assert!(!data.remove(&key));
        assert_eq!(data.entities_raw().count(), 0);
    }

    #[test]
    fn inner_insert_and_remove_advance_revision_only_for_effective_changes() {
        let inner = GraphInner::new();
        let entity = Entity::Node(node("store_revision_node"));
        let key = key_for(&entity);

        assert_eq!(inner.revision(), GraphRevision::INITIAL);
        assert!(inner.insert(key.clone(), entity.clone()));
        let insert_revision = inner.revision();
        assert!(insert_revision > GraphRevision::INITIAL);

        assert!(!inner.insert(key.clone(), entity.clone()));
        assert_eq!(inner.revision(), insert_revision);

        assert!(inner.remove(&key));
        let remove_revision = inner.revision();
        assert!(remove_revision > insert_revision);

        assert!(!inner.remove(&key));
        assert_eq!(inner.revision(), remove_revision);
    }
}
