use std::{
    collections::{btree_map::Range, BTreeMap},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{audio, framework::future_queue::Data, spl_network, vision};

#[derive(Default)]
pub struct Databases {
    pub audio: Vec<audio::MainOutputs>,
    pub spl_network: Vec<spl_network::MainOutputs>,
    pub vision_top: Vec<vision::MainOutputs>,
    pub vision_bottom: Vec<vision::MainOutputs>,
}

#[derive(Default)]
pub struct PerceptionDatabases {
    databases: BTreeMap<SystemTime, Databases>,
    first_timestamp_of_temporary_databases: Option<SystemTime>,
}

impl PerceptionDatabases {
    pub fn update(
        &mut self,
        now: SystemTime,
        audio_update: (Vec<Data<audio::MainOutputs>>, Option<SystemTime>),
        spl_network_update: (Vec<Data<spl_network::MainOutputs>>, Option<SystemTime>),
        vision_top_update: (Vec<Data<vision::MainOutputs>>, Option<SystemTime>),
        vision_bottom_update: (Vec<Data<vision::MainOutputs>>, Option<SystemTime>),
    ) {
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

        let (audio_databases, first_timestamp_of_non_finalized_audio_database) = audio_update;
        let (spl_network_databases, first_timestamp_of_non_finalized_spl_network_database) =
            spl_network_update;
        let (vision_top_databases, first_timestamp_of_non_finalized_vision_top_database) =
            vision_top_update;
        let (vision_bottom_databases, first_timestamp_of_non_finalized_vision_bottom_database) =
            vision_bottom_update;

        self.first_timestamp_of_temporary_databases = [
            first_timestamp_of_non_finalized_audio_database,
            first_timestamp_of_non_finalized_spl_network_database,
            first_timestamp_of_non_finalized_vision_top_database,
            first_timestamp_of_non_finalized_vision_bottom_database,
        ]
        .iter()
        .copied()
        .flatten()
        .min();

        for timestamped_database in audio_databases {
            self.databases
                .get_mut(&timestamped_database.timestamp)
                .unwrap()
                .audio
                .push(timestamped_database.data);
        }

        for timestamped_database in spl_network_databases {
            self.databases
                .get_mut(&timestamped_database.timestamp)
                .unwrap()
                .spl_network
                .push(timestamped_database.data);
        }

        for timestamped_database in vision_top_databases {
            self.databases
                .get_mut(&timestamped_database.timestamp)
                .unwrap()
                .vision_top
                .push(timestamped_database.data);
        }

        for timestamped_database in vision_bottom_databases {
            self.databases
                .get_mut(&timestamped_database.timestamp)
                .unwrap()
                .vision_bottom
                .push(timestamped_database.data);
        }
    }

    pub fn get_first_timestamp_of_temporary_databases(&self) -> Option<SystemTime> {
        self.first_timestamp_of_temporary_databases
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

#[derive(Debug)]
pub struct PerceptionDataType<'a, DataType> {
    pub persistent: BTreeMap<SystemTime, Vec<&'a DataType>>,
    pub temporary: BTreeMap<SystemTime, Vec<&'a DataType>>,
}

#[allow(dead_code)]
type Entry<'a> = (&'a SystemTime, &'a Databases);
#[allow(dead_code)]
type ExtractedDataTypes<'a, DataType> = (SystemTime, Vec<&'a DataType>);

impl<'a, DataType> PerceptionDataType<'a, DataType> {
    #[allow(dead_code)]
    pub fn new(
        perception_databases: &'a PerceptionDatabases,
        datatype_extraction: fn(Entry) -> ExtractedDataTypes<DataType>,
    ) -> Self {
        let persistent = perception_databases
            .persistent()
            .map(datatype_extraction)
            .collect();
        let temporary = perception_databases
            .temporary()
            .map(datatype_extraction)
            .collect();
        Self {
            persistent,
            temporary,
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
            (vec![], None),
            (vec![], None),
            (vec![], None),
            (vec![], None),
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
                vec![Data::<vision::MainOutputs> {
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
