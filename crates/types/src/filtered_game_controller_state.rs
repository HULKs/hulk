use std::collections::HashMap;

use path_serde::{PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use spl_network_messages::{GamePhase, Penalty, SubState, Team};

use crate::filtered_game_state::FilteredGameState;

#[derive(
    Default, Clone, Debug, Serialize, Deserialize, PathSerialize, PathIntrospect, PartialEq,
)]

pub struct FilteredGameControllerState {
    pub game_state: FilteredGameState,
    pub opponent_game_state: FilteredGameState,
    pub game_phase: GamePhase,
    pub kicking_team: Team,
    pub penalties: HashMap<usize, Option<Penalty>>,
    pub opponent_penalties: HashMap<usize, Option<Penalty>>,
    pub goal_keeper_number: usize,
    pub opponent_goal_keeper_number: usize,
    pub remaining_number_of_messages: u16,
    pub sub_state: Option<SubState>,
    pub own_team_is_home_after_coin_toss: bool,

    pub new_own_penalties_last_cycle: HashMap<usize, Penalty>,
    pub new_opponent_penalties_last_cycle: HashMap<usize, Penalty>,
}
