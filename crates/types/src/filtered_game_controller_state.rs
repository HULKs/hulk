use std::collections::HashMap;

use path_serde::{PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use spl_network_messages::{GamePhase, Penalty, PlayerNumber, SubState, Team};

use crate::{filtered_game_state::FilteredGameState, players::Players};

#[derive(
    Default, Clone, Debug, Serialize, Deserialize, PathSerialize, PathIntrospect, PartialEq,
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

    pub new_own_penalties_last_cycle: HashMap<PlayerNumber, Penalty>,
    pub new_opponent_penalties_last_cycle: HashMap<PlayerNumber, Penalty>,
}
