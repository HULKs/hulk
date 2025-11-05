use std::{
    collections::{btree_map::Range, BTreeMap},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::future_queue::Updates;

#[derive(Default)]
pub struct PerceptionDatabases<Databases> {
    databases: BTreeMap<SystemTime, Databases>,
    first_timestamp_of_temporary_databases: Option<SystemTime>,
}

impl<Databases> PerceptionDatabases<Databases>
where
    Databases: Default,
{
    pub fn update(&mut self, now: SystemTime, updates: impl Updates<Databases>) {
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

    pub fn persistent(&'_ self) -> Range<'_, SystemTime, Databases> {
        if let Some(first_timestamp_of_temporary_databases) =
            self.first_timestamp_of_temporary_databases
        {
            self.databases
                .range(..first_timestamp_of_temporary_databases)
        } else {
            self.databases.range(..)
        }
    }

    pub fn temporary(&'_ self) -> Range<'_, SystemTime, Databases> {
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
    use crate::{Item, Update};

    use super::*;

    #[derive(Default)]
    struct MainOutputs {}

    struct Updates {
        audio: Update<MainOutputs>,
        spl_network: Update<MainOutputs>,
        vision: Update<MainOutputs>,
    }

    #[derive(Default)]
    struct Databases {
        pub vision: Vec<MainOutputs>,
        pub spl_network: Vec<MainOutputs>,
        pub audio: Vec<MainOutputs>,
    }

    impl crate::Updates<Databases> for Updates {
        fn first_timestamp_of_temporary_databases(&self) -> Option<std::time::SystemTime> {
            [
                self.vision.first_timestamp_of_non_finalized_database,
                self.spl_network.first_timestamp_of_non_finalized_database,
                self.audio.first_timestamp_of_non_finalized_database,
            ]
            .iter()
            .copied()
            .flatten()
            .min()
        }
        fn push_to_databases(
            self,
            databases: &mut std::collections::BTreeMap<std::time::SystemTime, Databases>,
        ) {
            for timestamped_database in self.vision.items {
                databases
                    .get_mut(&timestamped_database.timestamp)
                    .unwrap()
                    .vision
                    .push(timestamped_database.data);
            }
            for timestamped_database in self.spl_network.items {
                databases
                    .get_mut(&timestamped_database.timestamp)
                    .unwrap()
                    .spl_network
                    .push(timestamped_database.data);
            }
            for timestamped_database in self.audio.items {
                databases
                    .get_mut(&timestamped_database.timestamp)
                    .unwrap()
                    .audio
                    .push(timestamped_database.data);
            }
        }
    }

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
                audio: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: None,
                },
                spl_network: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: None,
                },
                vision: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: None,
                },
            },
        );

        assert_eq!(databases.databases.len(), 1);
        assert!(databases.databases.contains_key(&instant));
        assert!(databases.databases[&instant].vision.is_empty());
        assert_eq!(databases.first_timestamp_of_temporary_databases, None);

        let persistent_item = databases.persistent().next();
        assert!(persistent_item.is_some());
        if let Some((persistent_item_instant, persistent_item_databases)) = persistent_item {
            assert_eq!(persistent_item_instant, &instant);
            assert!(persistent_item_databases.vision.is_empty());
        }
        assert!(databases.temporary().next().is_none());
    }

    #[test]
    fn vision_updates_creates_single_persistent_item() {
        let mut databases = PerceptionDatabases::default();
        assert!(databases.databases.is_empty());
        assert_eq!(databases.first_timestamp_of_temporary_databases, None);

        assert!(databases.persistent().next().is_none());
        assert!(databases.temporary().next().is_none());

        let instant = SystemTime::now();
        databases.update(
            instant,
            Updates {
                audio: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: None,
                },
                spl_network: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: None,
                },
                vision: Update {
                    items: vec![Item::<MainOutputs> {
                        timestamp: instant,
                        data: Default::default(),
                    }],
                    first_timestamp_of_non_finalized_database: None,
                },
            },
        );

        assert_eq!(databases.databases.len(), 1);
        assert!(databases.databases.contains_key(&instant));
        assert_eq!(databases.databases[&instant].vision.len(), 1);
        assert_eq!(databases.first_timestamp_of_temporary_databases, None);

        let persistent_item = databases.persistent().next();
        assert!(persistent_item.is_some());
        if let Some((persistent_item_instant, persistent_item_databases)) = persistent_item {
            assert_eq!(persistent_item_instant, &instant);
            assert_eq!(persistent_item_databases.vision.len(), 1);
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
            Updates {
                audio: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: None,
                },
                spl_network: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: None,
                },
                vision: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: Some(instant_a),
                },
            },
        );

        assert_eq!(databases.databases.len(), 1);
        assert!(databases.databases.contains_key(&instant_a));
        assert!(databases.databases[&instant_a].vision.is_empty());
        assert_eq!(
            databases.first_timestamp_of_temporary_databases,
            Some(instant_a)
        );

        assert!(databases.persistent().next().is_none());
        let temporary_item = databases.temporary().next();
        assert!(temporary_item.is_some());
        if let Some((temporary_item_instant, temporary_item_databases)) = temporary_item {
            assert_eq!(temporary_item_instant, &instant_a);
            assert!(temporary_item_databases.vision.is_empty());
        }

        let instant_b = SystemTime::now();
        databases.update(
            instant_b,
            Updates {
                audio: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: None,
                },
                spl_network: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: Some(instant_a),
                },
                vision: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: Some(instant_b),
                },
            },
        );

        assert_eq!(databases.databases.len(), 2);
        assert!(databases.databases.contains_key(&instant_a));
        assert!(databases.databases.contains_key(&instant_b));
        assert!(databases.databases[&instant_a].vision.is_empty());
        assert!(databases.databases[&instant_b].vision.is_empty());
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
            assert!(temporary_item_databases.vision.is_empty());
        }
        let temporary_item = temporary_iterator.next();
        assert!(temporary_item.is_some());
        if let Some((temporary_item_instant, temporary_item_databases)) = temporary_item {
            assert_eq!(temporary_item_instant, &instant_b);
            assert!(temporary_item_databases.vision.is_empty());
        }

        let instant_c = SystemTime::now();
        databases.update(
            instant_c,
            Updates {
                audio: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: None,
                },
                spl_network: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: None,
                },
                vision: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: None,
                },
            },
        );

        assert_eq!(databases.databases.len(), 3);
        assert!(databases.databases.contains_key(&instant_a));
        assert!(databases.databases.contains_key(&instant_b));
        assert!(databases.databases.contains_key(&instant_c));
        assert!(databases.databases[&instant_a].vision.is_empty());
        assert!(databases.databases[&instant_b].vision.is_empty());
        assert!(databases.databases[&instant_c].vision.is_empty());
        assert_eq!(
            databases.first_timestamp_of_temporary_databases,
            Some(instant_b)
        );

        let persistent_item = databases.persistent().next();
        assert!(persistent_item.is_some());
        if let Some((persistent_item_instant, persistent_item_databases)) = persistent_item {
            assert_eq!(persistent_item_instant, &instant_a);
            assert!(persistent_item_databases.vision.is_empty());
        }
        let mut temporary_iterator = databases.temporary();
        let temporary_item = temporary_iterator.next();
        assert!(temporary_item.is_some());
        if let Some((temporary_item_instant, temporary_item_databases)) = temporary_item {
            assert_eq!(temporary_item_instant, &instant_b);
            assert!(temporary_item_databases.vision.is_empty());
        }
        let temporary_item = temporary_iterator.next();
        assert!(temporary_item.is_some());
        if let Some((temporary_item_instant, temporary_item_databases)) = temporary_item {
            assert_eq!(temporary_item_instant, &instant_c);
            assert!(temporary_item_databases.vision.is_empty());
        }

        let instant_d = SystemTime::now();
        databases.update(
            instant_d,
            Updates {
                audio: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: None,
                },
                spl_network: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: None,
                },
                vision: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: None,
                },
            },
        );

        assert_eq!(databases.databases.len(), 3);
        assert!(databases.databases.contains_key(&instant_b));
        assert!(databases.databases.contains_key(&instant_c));
        assert!(databases.databases.contains_key(&instant_d));
        assert!(databases.databases[&instant_b].vision.is_empty());
        assert!(databases.databases[&instant_c].vision.is_empty());
        assert!(databases.databases[&instant_d].vision.is_empty());
        assert_eq!(databases.first_timestamp_of_temporary_databases, None);

        let mut persistent_iterator = databases.persistent();
        let persistent_item = persistent_iterator.next();
        assert!(persistent_item.is_some());
        if let Some((persistent_item_instant, persistent_item_databases)) = persistent_item {
            assert_eq!(persistent_item_instant, &instant_b);
            assert!(persistent_item_databases.vision.is_empty());
        }
        let persistent_item = persistent_iterator.next();
        assert!(persistent_item.is_some());
        if let Some((persistent_item_instant, persistent_item_databases)) = persistent_item {
            assert_eq!(persistent_item_instant, &instant_c);
            assert!(persistent_item_databases.vision.is_empty());
        }
        let persistent_item = persistent_iterator.next();
        assert!(persistent_item.is_some());
        if let Some((persistent_item_instant, persistent_item_databases)) = persistent_item {
            assert_eq!(persistent_item_instant, &instant_d);
            assert!(persistent_item_databases.vision.is_empty());
        }
        assert!(databases.temporary().next().is_none());

        let instant_e = SystemTime::now();
        databases.update(
            instant_e,
            Updates {
                audio: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: None,
                },
                spl_network: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: None,
                },
                vision: Update {
                    items: vec![],
                    first_timestamp_of_non_finalized_database: None,
                },
            },
        );

        assert_eq!(databases.databases.len(), 1);
        assert!(databases.databases.contains_key(&instant_e));
        assert!(databases.databases[&instant_e].vision.is_empty());
        assert_eq!(databases.first_timestamp_of_temporary_databases, None);

        let persistent_item = databases.persistent().next();
        assert!(persistent_item.is_some());
        if let Some((persistent_item_instant, persistent_item_databases)) = persistent_item {
            assert_eq!(persistent_item_instant, &instant_e);
            assert!(persistent_item_databases.vision.is_empty());
        }
        assert!(databases.temporary().next().is_none());
    }
}
