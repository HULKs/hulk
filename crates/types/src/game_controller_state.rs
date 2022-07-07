use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use spl_network::{GamePhase, GameState, Penalty, SetPlay, Team};

use super::Players;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct GameControllerState {
    pub game_state: GameState,
    pub game_phase: GamePhase,
    pub kicking_team: Team,
    pub last_game_state_change: SystemTime,
    pub penalties: Players<Option<Penalty>>,
    pub remaining_amount_of_messages: u16,
    pub set_play: Option<SetPlay>,
}
