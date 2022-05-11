use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use spl_network::{GamePhase, GameState, Penalty};

use super::Players;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameControllerState {
    pub game_state: GameState,
    pub game_phase: GamePhase,
    pub last_game_state_change: SystemTime,
    pub penalties: Players<Option<Penalty>>,
}
