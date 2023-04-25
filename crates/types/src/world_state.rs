use nalgebra::{Isometry2, Point2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use spl_network_messages::PlayerNumber;

use crate::GameControllerState;

use crate::PenaltyShotDirection;

use super::{FallState, FilteredGameState, Obstacle, PrimaryState, Role, Side};

#[derive(Clone, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct WorldState {
    pub ball: Option<BallState>,
    pub filtered_game_state: Option<FilteredGameState>,
    pub game_controller_state: Option<GameControllerState>,
    pub obstacles: Vec<Obstacle>,
    pub position_of_interest: Point2<f32>,
    pub robot: RobotState,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct BallState {
    pub ball_in_ground: Point2<f32>,
    pub ball_in_field: Point2<f32>,
    pub penalty_shot_direction: Option<PenaltyShotDirection>,
    pub field_side: Side,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct RobotState {
    pub robot_to_field: Option<Isometry2<f32>>,
    pub role: Role,
    pub primary_state: PrimaryState,
    pub fall_state: FallState,
    pub has_ground_contact: bool,
    pub player_number: PlayerNumber,
}
