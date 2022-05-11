use macros::SerializeHierarchy;
use nalgebra::{Vector2, Vector3};
use serde::{Deserialize, Serialize};

use super::{CycleInfo, Joints};

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct InertialMeasurementUnitData {
    pub linear_acceleration: Vector3<f32>,
    pub angular_velocity: Vector3<f32>,
    pub roll_pitch: Vector2<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct SonarSensors {
    pub left: f32,
    pub right: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct Foot {
    pub front_left: f32,
    pub front_right: f32,
    pub rear_left: f32,
    pub rear_right: f32,
}

impl Foot {
    pub fn sum(&self) -> f32 {
        self.front_left + self.front_right + self.rear_left + self.rear_right
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct ForceSensitiveResistors {
    pub left: Foot,
    pub right: Foot,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct TouchSensors {
    pub chest_button: bool,
    pub head_front: bool,
    pub head_middle: bool,
    pub head_rear: bool,
    pub left_foot_left: bool,
    pub left_foot_right: bool,
    pub left_hand_back: bool,
    pub left_hand_left: bool,
    pub left_hand_right: bool,
    pub right_foot_left: bool,
    pub right_foot_right: bool,
    pub right_hand_back: bool,
    pub right_hand_left: bool,
    pub right_hand_right: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct SensorData {
    pub cycle_info: CycleInfo,
    pub positions: Joints,
    pub inertial_measurement_unit: InertialMeasurementUnitData,
    pub sonar_sensors: SonarSensors,
    pub force_sensitive_resistors: ForceSensitiveResistors,
    pub touch_sensors: TouchSensors,
}
