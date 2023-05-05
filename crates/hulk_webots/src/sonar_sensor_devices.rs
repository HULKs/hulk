use types::SonarSensors;
use webots::{DistanceSensor, Robot};

use super::hardware_interface::SIMULATION_TIME_STEP;

pub struct SonarSensorDevices {
    left: DistanceSensor,
    right: DistanceSensor,
}

impl Default for SonarSensorDevices {
    fn default() -> Self {
        let left = Robot::get_distance_sensor("Sonar/Left");
        left.enable(SIMULATION_TIME_STEP);

        let right = Robot::get_distance_sensor("Sonar/Right");
        right.enable(SIMULATION_TIME_STEP);

        Self { left, right }
    }
}

impl SonarSensorDevices {
    pub fn get_values(&self) -> SonarSensors {
        SonarSensors {
            left: self.left.get_value() as f32,
            right: self.right.get_value() as f32,
        }
    }
}
