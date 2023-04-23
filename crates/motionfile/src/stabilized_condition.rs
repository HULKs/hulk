use std::fmt::Debug;

use crate::Condition;

use filtering::low_pass_filter::LowPassFilter;
use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use types::SensorData;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilizedCondition {
    tolerance: f32,
    #[serde(with = "serialize")]
    #[serde(rename = "alpha")]
    filtered_velocity: LowPassFilter<Vector3<f32>>,
}

mod serialize {
    use filtering::low_pass_filter::LowPassFilter;
    use nalgebra::Vector3;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        filter: &LowPassFilter<Vector3<f32>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_f32(filter.alpha())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<LowPassFilter<Vector3<f32>>, D::Error> {
        let alpha = f32::deserialize(deserializer)?;
        Ok(LowPassFilter::with_alpha(Vector3::zeros(), alpha))
    }
}

impl Condition for StabilizedCondition {
    fn is_finished(&self) -> bool {
        self.filtered_velocity.state().norm() < self.tolerance
    }

    fn update(&mut self, sensor_data: &SensorData) {
        self.filtered_velocity
            .update(sensor_data.inertial_measurement_unit.angular_velocity);
    }

    fn reset(&mut self) {}
}
