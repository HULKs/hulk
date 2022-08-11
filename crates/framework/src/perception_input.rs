use std::{collections::BTreeMap, time::SystemTime};

pub struct PerceptionInput<'context, DataType> {
    pub persistent: BTreeMap<SystemTime, Vec<&'context DataType>>,
    pub temporary: BTreeMap<SystemTime, Vec<&'context DataType>>,
}
