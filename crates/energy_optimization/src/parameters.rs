use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use types::joints::Joints;

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct CurrentMinimizerParameters {
    pub reset_threshold: f32,
    pub reset_speed: f32,
    pub reset_base_offset: f32,
    pub optimization_speed: f32,
    pub allowed_current: f32,
    pub optimization_sign: Joints<f32>,
    pub position_difference_threshold: f32,
    pub allowed_current_upper_threshold: f32,
}
