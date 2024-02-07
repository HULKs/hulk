use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::joints::Joints;

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct CurrentMinimizerParameters {
    pub reset_threshold: f32,
    pub reset_speed: f32,
    pub reset_base_offset: f32,
    pub optimization_speed: f32,
    pub allowed_current: f32,
    pub optimization_sign: Joints<f32>,
    pub position_difference_threshold: f32,
}
