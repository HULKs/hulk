use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Default, Debug, Clone, Serialize, Deserialize, SerializeHierarchy)]
pub struct ConditionInput {
    pub filtered_angular_velocity: Vector3<f32>,
}
