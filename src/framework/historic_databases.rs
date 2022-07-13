use std::{collections::BTreeMap, iter::FromIterator, time::SystemTime};

use crate::control::Database;

#[derive(Default)]
pub struct HistoricDatabases {
    databases: BTreeMap<SystemTime, Database>,
}

impl HistoricDatabases {
    pub fn update(
        &mut self,
        now: SystemTime,
        first_timestamp_of_temporary_databases: Option<SystemTime>,
        database: &Database,
    ) {
        if let Some(first_timestamp_of_temporary_databases) = first_timestamp_of_temporary_databases
        {
            self.databases = self
                .databases
                .split_off(&first_timestamp_of_temporary_databases);
            self.databases.insert(now, database.clone());
        } else {
            self.databases.clear();
        }
    }
}

#[derive(Debug)]
pub struct HistoricDataType<'a, DataType> {
    historic: BTreeMap<SystemTime, &'a DataType>,
}

impl<'a, DataType> HistoricDataType<'a, DataType> {
    pub fn new(
        cycle_start_time: SystemTime,
        historic_databases: &'a HistoricDatabases,
        datatype_in_this_cycle: &'a DataType,
        historic_datatype_extraction: fn(
            (&'a SystemTime, &'a Database),
        ) -> (SystemTime, &'a DataType),
    ) -> Self {
        let mut historic =
            BTreeMap::from_iter([(cycle_start_time, datatype_in_this_cycle)].iter().copied());
        historic.extend(
            historic_databases
                .databases
                .iter()
                .map(historic_datatype_extraction),
        );
        Self { historic }
    }

    pub fn get(&self, system_time: SystemTime) -> &'a DataType {
        return *self
            .historic
            .get(&system_time)
            .expect("Failed to get historic at given time stamp");
    }
}
