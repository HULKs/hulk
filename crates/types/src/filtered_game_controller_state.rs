use std::{collections::HashMap, time::Duration};

use path_serde::{PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use spl_network_messages::{GamePhase, Penalty, PlayerNumber, SubState, Team};

use crate::{
    field_dimensions::GlobalFieldSide, filtered_game_state::FilteredGameState, players::Players,
};

#[derive(Clone, Debug, Serialize, Deserialize, PathSerialize, PathIntrospect, PartialEq)]
pub struct FilteredGameControllerState {
    pub game_state: FilteredGameState,
    pub opponent_game_state: FilteredGameState,
    pub previous_own_game_state: Option<FilteredGameState>,
    pub remaining_time_in_half: Duration,
    pub game_phase: GamePhase,
    pub kicking_team: Option<Team>,
    pub penalties: Players<Option<Penalty>>,
    pub remaining_number_of_messages: u16,
    pub sub_state: Option<SubState>,
    pub global_field_side: GlobalFieldSide,

    pub new_own_penalties_last_cycle: HashMap<PlayerNumber, Penalty>,
    pub new_opponent_penalties_last_cycle: HashMap<PlayerNumber, Penalty>,
}

impl Default for FilteredGameControllerState {
    fn default() -> Self {
        Self {
            game_state: Default::default(),
            opponent_game_state: Default::default(),
            previous_own_game_state: None,
            remaining_time_in_half: Duration::ZERO,
            game_phase: Default::default(),
            kicking_team: Default::default(),
            penalties: Default::default(),
            remaining_number_of_messages: Default::default(),
            sub_state: Default::default(),
            global_field_side: GlobalFieldSide::Away,
            new_own_penalties_last_cycle: Default::default(),
            new_opponent_penalties_last_cycle: Default::default(),
        }
    }
}
