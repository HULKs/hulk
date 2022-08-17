use std::{collections::BTreeMap, time::SystemTime};

#[derive(Default)]
pub struct HistoricDatabases<MainOutputs> {
    pub databases: BTreeMap<SystemTime, MainOutputs>,
}

impl<MainOutputs> HistoricDatabases<MainOutputs>
where
    MainOutputs: Clone,
{
    pub fn update(
        &mut self,
        now: SystemTime,
        first_timestamp_of_temporary_databases: Option<SystemTime>,
        main_outputs: &MainOutputs,
    ) {
        if let Some(first_timestamp_of_temporary_databases) = first_timestamp_of_temporary_databases
        {
            self.databases = self
                .databases
                .split_off(&first_timestamp_of_temporary_databases);
            self.databases.insert(now, main_outputs.clone());
        } else {
            self.databases.clear();
        }
    }
}
