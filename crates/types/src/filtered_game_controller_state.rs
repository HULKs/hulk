
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use spl_network_messages::{GamePhase, Penalty, SubState, Team};

use crate::{players::Players, filtered_game_states::FilteredGameState};

#[derive(Default, Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct FilteredGameControllerState {
    pub game_state: FilteredGameState,
    pub opponent_game_state: FilteredGameState,
    pub game_phase: GamePhase,
    pub kicking_team: Team,
    pub penalties: Players<Option<Penalty>>,
    pub remaining_amount_of_messages: u16,
    pub sub_state: Option<SubState>,
    pub hulks_team_is_home_after_coin_toss: bool,
}
