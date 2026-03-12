use std::{collections::BTreeMap, time::SystemTime};

#[derive(Debug)]
pub struct PerceptionInput<VectorType> {
    pub persistent: BTreeMap<SystemTime, VectorType>,
    pub temporary: BTreeMap<SystemTime, VectorType>,
}
