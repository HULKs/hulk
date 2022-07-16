use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use spl_network_messages::{GamePhase, GameState, Penalty, SetPlay, Team};

use super::Players;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct GameControllerState {
    pub game_state: GameState,
    pub game_phase: GamePhase,
    pub kicking_team: Team,
    pub last_game_state_change: SystemTime,
    pub penalties: Players<Option<Penalty>>,
    pub remaining_amount_of_messages: u16,
    pub set_play: Option<SetPlay>,
}
