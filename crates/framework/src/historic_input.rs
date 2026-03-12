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
    // This is a hack. The previous implementation used a historic.get(). This sometimes failed during replay, because the
    // given SystemTime was not a key in the historic.
    pub fn get_nearest(&self, system_time: &SystemTime) -> DataType {
        let after = self.historic.range(system_time..).next();
        let before = self.historic.range(..system_time).next_back();

        match (before, after) {
            (Some((before_time, before_val)), Some((after_time, after_val))) => {
                let diff_before = system_time.duration_since(*before_time).unwrap_or_default();
                let diff_after = after_time.duration_since(*system_time).unwrap_or_default();

                if diff_before < diff_after {
                    *before_val
                } else {
                    *after_val
                }
            }
            (Some((_, before_val)), None) => *before_val,
            (None, Some((_, after_val))) => *after_val,
            (None, None) => panic!("HistoricInput is empty"),
        }
    }
}
