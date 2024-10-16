use coordinate_systems::Robot;
use linear_algebra::{Vector2, Vector3};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::joints::Joints;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct InertialMeasurementUnitData {
    // Linear acceleration is coming from a left handed coordinate system
    pub linear_acceleration: Vector3<Robot>,
    pub angular_velocity: Vector3<Robot>,
    pub roll_pitch: Vector2<Robot>,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct SonarSensors {
    pub left: f32,
    pub right: f32,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct Foot {
    pub front_left: f32,
    pub front_right: f32,
    pub rear_left: f32,
    pub rear_right: f32,
}

impl Foot {
    pub fn fill(value: f32) -> Self {
        Self {
            front_left: value,
            front_right: value,
            rear_left: value,
            rear_right: value,
        }
    }

    pub fn sum(&self) -> f32 {
        self.front_left + self.front_right + self.rear_left + self.rear_right
    }
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct ForceSensitiveResistors {
    pub left: Foot,
    pub right: Foot,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
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

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct SensorData {
    pub positions: Joints<f32>,
    pub inertial_measurement_unit: InertialMeasurementUnitData,
    pub sonar_sensors: SonarSensors,
    pub force_sensitive_resistors: ForceSensitiveResistors,
    pub touch_sensors: TouchSensors,
    pub temperature_sensors: Joints<f32>,
    pub currents: Joints<f32>,
}
