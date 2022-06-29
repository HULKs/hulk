use macros::SerializeHierarchy;
use nalgebra::{Isometry2, Point2};
use serde::{Deserialize, Serialize};

use super::{FallState, Obstacle, PrimaryState, Role, Side};

#[derive(Clone, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct WorldState {
    pub ball: Option<BallState>,
    pub robot: RobotState,
    pub obstacles: Vec<Obstacle>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct BallState {
    pub position: Point2<f32>,
    #[leaf]
    pub field_side: Side,
}

impl Default for BallState {
    fn default() -> Self {
        Self {
            position: Point2::origin(),
            field_side: Side::Left,
        }
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct RobotState {
    pub robot_to_field: Option<Isometry2<f32>>,
    #[leaf]
    pub role: Role,
    #[leaf]
    pub primary_state: PrimaryState,
    #[leaf]
    pub fall_state: FallState,
    pub has_ground_contact: bool,
}
