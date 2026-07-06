use nalgebra::Vector3;
use serde::{Deserialize, Serialize};

use crate::fall_state::FallState;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ConditionInput {
    pub filtered_angular_velocity: Vector3<f32>,
    pub fall_state: FallState,
    pub ground_contact: bool,
}
