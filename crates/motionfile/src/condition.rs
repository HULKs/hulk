use std::fmt::Debug;

use crate::StabilizedCondition;

use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use types::SensorData;

#[enum_dispatch(ConditionEnum)]
pub trait Condition: Clone {
    fn is_finished(&self) -> bool;
    fn update(&mut self, sensor_data: &SensorData);
    fn reset(&mut self);
}

#[enum_dispatch]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionEnum {
    StabilizedCondition,
}
