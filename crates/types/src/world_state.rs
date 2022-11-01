use nalgebra::{Isometry2, Point2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use spl_network_messages::PlayerNumber;

use crate::GameControllerState;

use crate::PenaltyShotDirection;

use super::{FallState, FilteredGameState, Obstacle, PrimaryState, Role, Side};
use spl_network_messages::GamePhase;

#[derive(Clone, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct WorldState {
    pub ball: Option<BallState>,
    #[leaf]
    pub filtered_game_state: Option<FilteredGameState>,
    #[leaf]
    pub game_controller_state: Option<GameControllerState>,
    #[leaf]
    pub game_phase: GamePhase,
    pub obstacles: Vec<Obstacle>,
    pub robot: RobotState,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct BallState {
    pub position: Point2<f32>,
    #[leaf]
    pub penalty_shot_direction: Option<PenaltyShotDirection>,
    #[leaf]
    pub field_side: Side,
}

impl Default for BallState {
    fn default() -> Self {
        Self {
            position: Point2::origin(),
            penalty_shot_direction: Default::default(),
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
    #[leaf]
    pub player_number: PlayerNumber,
}
