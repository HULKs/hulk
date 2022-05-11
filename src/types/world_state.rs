use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

use super::{BallPosition, GameControllerState, RobotState, TeamState};

#[derive(Clone, Default, Debug, Serialize, Deserialize, SerializeHierarchy)]

pub struct WorldState {
    pub ball: BallPosition,
    #[leaf]
    pub game_controller: Option<GameControllerState>,
    pub robot: RobotState,
    pub team: Option<TeamState>,
}

impl WorldState {
    pub fn new() -> Self {
        Self {
            ball: BallPosition::default(),
            game_controller: None,
            robot: RobotState::default(),
            team: None,
        }
    }
}
