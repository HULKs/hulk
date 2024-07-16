use nalgebra::Vector3;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::fall_state::FallState;

#[derive(
    Default, Debug, Clone, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct ConditionInput {
    pub filtered_angular_velocity: Vector3<f32>,
    pub fall_state: FallState,
    pub ground_contact: bool,
}
