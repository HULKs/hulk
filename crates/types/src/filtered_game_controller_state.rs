use path_serde::{PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use spl_network_messages::{GamePhase, Penalty, SubState, Team};

use crate::{
    filtered_game_state::FilteredGameState,
    last_filtered_game_controller_state_change::LastFilteredGameControllerStateChanges,
    players::Players,
};

#[derive(
    Default, Clone, Copy, Debug, Serialize, Deserialize, PathSerialize, PathIntrospect, PartialEq,
)]

pub struct FilteredGameControllerState {
    pub game_state: FilteredGameState,
    pub opponent_game_state: FilteredGameState,
    pub game_phase: GamePhase,
    pub kicking_team: Team,
    pub penalties: Players<Option<Penalty>>,
    pub remaining_number_of_messages: u16,
    pub sub_state: Option<SubState>,
    pub own_team_is_home_after_coin_toss: bool,
    pub last_filtered_game_controller_state_change: LastFilteredGameControllerStateChanges,
}
