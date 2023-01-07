use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct OrientationFilterParameters {
    pub acceleration_threshold: f32,
    pub delta_angular_velocity_threshold: f32,
    pub angular_velocity_bias_weight: f32,
    pub acceleration_weight: f32,
    pub falling_threshold: f32,
    pub force_sensitive_resistor_threshold: f32,
}
