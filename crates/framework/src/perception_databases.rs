use std::{
    collections::{btree_map::Range, BTreeMap},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::Item;

include!(concat!(env!("OUT_DIR"), "/perception_databases_structs.rs"));

#[derive(Default)]
pub struct PerceptionDatabases {
    databases: BTreeMap<SystemTime, Databases>,
    first_timestamp_of_temporary_databases: Option<SystemTime>,
}

impl PerceptionDatabases {
    pub fn update(&mut self, now: SystemTime, updates: Updates) {
        if let Some(first_timestamp_of_temporary_databases) =
            self.first_timestamp_of_temporary_databases
        {
            let databases_to_keep = self
                .databases
                .split_off(&first_timestamp_of_temporary_databases);
            self.databases = databases_to_keep;
        } else {
            self.databases.clear();
        }

        self.databases.insert(now, Default::default());

        self.first_timestamp_of_temporary_databases =
            updates.first_timestamp_of_temporary_databases();
        updates.push_to_databases(&mut self.databases);
    }

    pub fn get_first_timestamp_of_temporary_databases(&self) -> Option<SystemTime> {
        self.first_timestamp_of_temporary_databases
    }

    pub fn persistent(&self) -> Range<SystemTime, Databases> {
        if let Some(first_timestamp_of_temporary_databases) =
            self.first_timestamp_of_temporary_databases
        {
            self.databases
                .range(..first_timestamp_of_temporary_databases)
        } else {
            self.databases.range(..)
        }
    }

    pub fn temporary(&self) -> Range<SystemTime, Databases> {
        if let Some(first_timestamp_of_temporary_databases) =
            self.first_timestamp_of_temporary_databases
        {
            self.databases
                .range(first_timestamp_of_temporary_databases..)
        } else {
            self.databases.range(UNIX_EPOCH..UNIX_EPOCH)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_updates_creates_single_persistent_item() {
        let mut databases = PerceptionDatabases::default();
        assert!(databases.databases.is_empty());
        assert_eq!(databases.first_timestamp_of_temporary_databases, None);

        assert!(databases.persistent().next().is_none());
        assert!(databases.temporary().next().is_none());

        let instant = SystemTime::now();
        databases.update(
            instant,
            Updates {
                vision_top: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: None,
                },
                vision_bottom: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: None,
                },
            },
        );

        assert_eq!(databases.databases.len(), 1);
        assert!(databases.databases.contains_key(&instant));
        assert!(databases.databases[&instant].vision_top.is_empty());
        assert!(databases.databases[&instant].vision_bottom.is_empty());
        assert_eq!(databases.first_timestamp_of_temporary_databases, None);

        let persistent_item = databases.persistent().next();
        assert!(persistent_item.is_some());
        if let Some((persistent_item_instant, persistent_item_databases)) = persistent_item {
            assert_eq!(persistent_item_instant, &instant);
            assert!(persistent_item_databases.vision_top.is_empty());
            assert!(persistent_item_databases.vision_bottom.is_empty());
        }
        assert!(databases.temporary().next().is_none());
    }

    #[test]
    fn vision_top_updates_creates_single_persistent_item() {
        let mut databases = PerceptionDatabases::default();
        assert!(databases.databases.is_empty());
        assert_eq!(databases.first_timestamp_of_temporary_databases, None);

        assert!(databases.persistent().next().is_none());
        assert!(databases.temporary().next().is_none());

        let instant = SystemTime::now();
        databases.update(
            instant,
            (vec![], None),
            (vec![], None),
            (
                vec![Item::<vision::MainOutputs> {
                    timestamp: instant,
                    data: Default::default(),
                }],
                None,
            ),
            (vec![], None),
        );

        assert_eq!(databases.databases.len(), 1);
        assert!(databases.databases.contains_key(&instant));
        assert_eq!(databases.databases[&instant].vision_top.len(), 1);
        assert!(databases.databases[&instant].vision_bottom.is_empty());
        assert_eq!(databases.first_timestamp_of_temporary_databases, None);

        let persistent_item = databases.persistent().next();
        assert!(persistent_item.is_some());
        if let Some((persistent_item_instant, persistent_item_databases)) = persistent_item {
            assert_eq!(persistent_item_instant, &instant);
            assert_eq!(persistent_item_databases.vision_top.len(), 1);
            assert!(persistent_item_databases.vision_bottom.is_empty());
        }
        assert!(databases.temporary().next().is_none());
    }

    #[test]
    fn multiple_announcing_updates_keep_items() {
        let mut databases = PerceptionDatabases::default();
        assert!(databases.databases.is_empty());
        assert_eq!(databases.first_timestamp_of_temporary_databases, None);

        assert!(databases.persistent().next().is_none());
        assert!(databases.temporary().next().is_none());

        let instant_a = SystemTime::now();
        databases.update(
            instant_a,
            (vec![], None),
            (vec![], None),
            (vec![], Some(instant_a)),
            (vec![], None),
        );

        assert_eq!(databases.databases.len(), 1);
        assert!(databases.databases.contains_key(&instant_a));
        assert!(databases.databases[&instant_a].vision_top.is_empty());
        assert!(databases.databases[&instant_a].vision_bottom.is_empty());
        assert_eq!(
            databases.first_timestamp_of_temporary_databases,
            Some(instant_a)
        );

        assert!(databases.persistent().next().is_none());
        let temporary_item = databases.temporary().next();
        assert!(temporary_item.is_some());
        if let Some((temporary_item_instant, temporary_item_databases)) = temporary_item {
            assert_eq!(temporary_item_instant, &instant_a);
            assert!(temporary_item_databases.vision_top.is_empty());
            assert!(temporary_item_databases.vision_bottom.is_empty());
        }

        let instant_b = SystemTime::now();
        databases.update(
            instant_b,
            (vec![], None),
            (vec![], Some(instant_a)),
            (vec![], Some(instant_b)),
            (vec![], None),
        );

        assert_eq!(databases.databases.len(), 2);
        assert!(databases.databases.contains_key(&instant_a));
        assert!(databases.databases.contains_key(&instant_b));
        assert!(databases.databases[&instant_a].vision_top.is_empty());
        assert!(databases.databases[&instant_a].vision_bottom.is_empty());
        assert!(databases.databases[&instant_b].vision_top.is_empty());
        assert!(databases.databases[&instant_b].vision_bottom.is_empty());
        assert_eq!(
            databases.first_timestamp_of_temporary_databases,
            Some(instant_a)
        );

        assert!(databases.persistent().next().is_none());
        let mut temporary_iterator = databases.temporary();
        let temporary_item = temporary_iterator.next();
        assert!(temporary_item.is_some());
        if let Some((temporary_item_instant, temporary_item_databases)) = temporary_item {
            assert_eq!(temporary_item_instant, &instant_a);
            assert!(temporary_item_databases.vision_top.is_empty());
            assert!(temporary_item_databases.vision_bottom.is_empty());
        }
        let temporary_item = temporary_iterator.next();
        assert!(temporary_item.is_some());
        if let Some((temporary_item_instant, temporary_item_databases)) = temporary_item {
            assert_eq!(temporary_item_instant, &instant_b);
            assert!(temporary_item_databases.vision_top.is_empty());
            assert!(temporary_item_databases.vision_bottom.is_empty());
        }

        let instant_c = SystemTime::now();
        databases.update(
            instant_c,
            (vec![], None),
            (vec![], None),
            (vec![], None),
            (vec![], Some(instant_b)),
        );

        assert_eq!(databases.databases.len(), 3);
        assert!(databases.databases.contains_key(&instant_a));
        assert!(databases.databases.contains_key(&instant_b));
        assert!(databases.databases.contains_key(&instant_c));
        assert!(databases.databases[&instant_a].vision_top.is_empty());
        assert!(databases.databases[&instant_a].vision_bottom.is_empty());
        assert!(databases.databases[&instant_b].vision_top.is_empty());
        assert!(databases.databases[&instant_b].vision_bottom.is_empty());
        assert!(databases.databases[&instant_c].vision_top.is_empty());
        assert!(databases.databases[&instant_c].vision_bottom.is_empty());
        assert_eq!(
            databases.first_timestamp_of_temporary_databases,
            Some(instant_b)
        );

        let persistent_item = databases.persistent().next();
        assert!(persistent_item.is_some());
        if let Some((persistent_item_instant, persistent_item_databases)) = persistent_item {
            assert_eq!(persistent_item_instant, &instant_a);
            assert!(persistent_item_databases.vision_top.is_empty());
            assert!(persistent_item_databases.vision_bottom.is_empty());
        }
        let mut temporary_iterator = databases.temporary();
        let temporary_item = temporary_iterator.next();
        assert!(temporary_item.is_some());
        if let Some((temporary_item_instant, temporary_item_databases)) = temporary_item {
            assert_eq!(temporary_item_instant, &instant_b);
            assert!(temporary_item_databases.vision_top.is_empty());
            assert!(temporary_item_databases.vision_bottom.is_empty());
        }
        let temporary_item = temporary_iterator.next();
        assert!(temporary_item.is_some());
        if let Some((temporary_item_instant, temporary_item_databases)) = temporary_item {
            assert_eq!(temporary_item_instant, &instant_c);
            assert!(temporary_item_databases.vision_top.is_empty());
            assert!(temporary_item_databases.vision_bottom.is_empty());
        }

        let instant_d = SystemTime::now();
        databases.update(
            instant_d,
            (vec![], None),
            (vec![], None),
            (vec![], None),
            (vec![], None),
        );

        assert_eq!(databases.databases.len(), 3);
        assert!(databases.databases.contains_key(&instant_b));
        assert!(databases.databases.contains_key(&instant_c));
        assert!(databases.databases.contains_key(&instant_d));
        assert!(databases.databases[&instant_b].vision_top.is_empty());
        assert!(databases.databases[&instant_b].vision_bottom.is_empty());
        assert!(databases.databases[&instant_c].vision_top.is_empty());
        assert!(databases.databases[&instant_c].vision_bottom.is_empty());
        assert!(databases.databases[&instant_d].vision_top.is_empty());
        assert!(databases.databases[&instant_d].vision_bottom.is_empty());
        assert_eq!(databases.first_timestamp_of_temporary_databases, None);

        let mut persistent_iterator = databases.persistent();
        let persistent_item = persistent_iterator.next();
        assert!(persistent_item.is_some());
        if let Some((persistent_item_instant, persistent_item_databases)) = persistent_item {
            assert_eq!(persistent_item_instant, &instant_b);
            assert!(persistent_item_databases.vision_top.is_empty());
            assert!(persistent_item_databases.vision_bottom.is_empty());
        }
        let persistent_item = persistent_iterator.next();
        assert!(persistent_item.is_some());
        if let Some((persistent_item_instant, persistent_item_databases)) = persistent_item {
            assert_eq!(persistent_item_instant, &instant_c);
            assert!(persistent_item_databases.vision_top.is_empty());
            assert!(persistent_item_databases.vision_bottom.is_empty());
        }
        let persistent_item = persistent_iterator.next();
        assert!(persistent_item.is_some());
        if let Some((persistent_item_instant, persistent_item_databases)) = persistent_item {
            assert_eq!(persistent_item_instant, &instant_d);
            assert!(persistent_item_databases.vision_top.is_empty());
            assert!(persistent_item_databases.vision_bottom.is_empty());
        }
        assert!(databases.temporary().next().is_none());

        let instant_e = SystemTime::now();
        databases.update(
            instant_e,
            (vec![], None),
            (vec![], None),
            (vec![], None),
            (vec![], None),
        );

        assert_eq!(databases.databases.len(), 1);
        assert!(databases.databases.contains_key(&instant_e));
        assert!(databases.databases[&instant_e].vision_top.is_empty());
        assert!(databases.databases[&instant_e].vision_bottom.is_empty());
        assert_eq!(databases.first_timestamp_of_temporary_databases, None);

        let persistent_item = databases.persistent().next();
        assert!(persistent_item.is_some());
        if let Some((persistent_item_instant, persistent_item_databases)) = persistent_item {
            assert_eq!(persistent_item_instant, &instant_e);
            assert!(persistent_item_databases.vision_top.is_empty());
            assert!(persistent_item_databases.vision_bottom.is_empty());
        }
        assert!(databases.temporary().next().is_none());
    }
}
