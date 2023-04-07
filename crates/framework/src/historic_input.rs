use std::{collections::BTreeMap, time::SystemTime};

#[derive(Debug)]
pub struct HistoricInput<DataType> {
    historic: BTreeMap<SystemTime, DataType>,
}

impl<DataType> From<BTreeMap<SystemTime, DataType>> for HistoricInput<DataType> {
    fn from(historic: BTreeMap<SystemTime, DataType>) -> Self {
        Self { historic }
    }
}

impl<DataType> HistoricInput<DataType>
where
    DataType: Copy,
{
    pub fn get(&self, system_time: &SystemTime) -> DataType {
        *self
            .historic
            .get(system_time)
            .expect("Failed to get historic input value at given timestamp")
    }
}
