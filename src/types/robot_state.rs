use macros::SerializeHierarchy;
use nalgebra::Isometry2;
use serde::{Deserialize, Serialize};

use super::{FallState, PrimaryState, Role};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct RobotState {
    pub pose: Isometry2<f32>,
    #[leaf]
    pub role: Role,
    #[leaf]
    pub primary_state: PrimaryState,
    #[leaf]
    pub fall_state: FallState,
    pub walk_target_pose: Isometry2<f32>,
}

impl Default for RobotState {
    fn default() -> Self {
        Self {
            pose: Isometry2::identity(),
            role: Role::Striker,
            primary_state: PrimaryState::Unstiff,
            fall_state: FallState::Upright,
            walk_target_pose: Isometry2::identity(),
        }
    }
}
