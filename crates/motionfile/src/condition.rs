use enum_dispatch::enum_dispatch;
use serde::{Serialize, Deserialize};
use types::{SensorData, Joints};
use crate::StabilizedCondition;

#[enum_dispatch(ConditionEnum)]
pub trait Condition: Clone {
    fn is_finished(&self) -> bool;
    fn update(&mut self, sensor_data: &SensorData);
    fn value(&self) -> Option<Joints<f32>>;
    fn reset(&mut self);
}

#[enum_dispatch]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionEnum {
    StabilizedCondition
}
