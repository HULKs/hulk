use std::{collections::BTreeMap, time::SystemTime};

pub struct PerceptionInput<VectorType> {
    pub persistent: BTreeMap<SystemTime, VectorType>,
    pub temporary: BTreeMap<SystemTime, VectorType>,
}
