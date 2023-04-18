use filtering::LowPassFilter;
use nalgebra::Vector3;
use serde::{Serialize, Deserialize};
use types::{Joints, SensorData};
use crate::Condition;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilizedCondition {
    tolerance: f32,
    filtered_velocity: LowPassFilter<Vector3<f32>>,
}

impl Condition for StabilizedCondition {
    fn is_finished(&self) -> bool {
        self.filtered_velocity.state().norm() < self.tolerance
    }

    fn update(&mut self, sensor_data: &SensorData) {
        self.filtered_velocity
            .update(sensor_data.inertial_measurement_unit.angular_velocity);
    }

    fn value(&self) -> Option<Joints> {
        None
    }

    fn reset(&mut self) {}
}
