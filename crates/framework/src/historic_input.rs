use std::{collections::BTreeMap, time::SystemTime};

pub struct HistoricInput<'context, DataType> {
    historic: BTreeMap<SystemTime, &'context DataType>,
}

impl<'context, DataType> From<BTreeMap<SystemTime, &'context DataType>>
    for HistoricInput<'context, DataType>
{
    fn from(historic: BTreeMap<SystemTime, &'context DataType>) -> Self {
        Self { historic }
    }
}

impl<'context, DataType> HistoricInput<'context, DataType> {
    pub fn get(&self, system_time: SystemTime) -> &'context DataType {
        return *self
            .historic
            .get(&system_time)
            .expect("Failed to get historic input value at given timestamp");
    }
}
