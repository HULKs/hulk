use nalgebra::{UnitComplex, UnitQuaternion, Vector3};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct Parameters {
    pub acceleration_threshold: f32,
    pub delta_angular_velocity_threshold: f32,
    pub angular_velocity_bias_weight: f32,
    pub acceleration_weight: f32,
    pub falling_threshold: f32,
    pub force_sensitive_resistor_threshold: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct State {
    pub previous_angular_velocity: Vector3<f32>,
    pub angular_velocity_bias: Vector3<f32>,
    pub orientation: UnitQuaternion<f32>,
    pub is_initialized: bool,
}

impl State {
    pub fn yaw(&self) -> UnitComplex<f32> {
        let (_, _, yaw) = self.orientation.inverse().euler_angles();
        UnitComplex::new(yaw)
    }
}
