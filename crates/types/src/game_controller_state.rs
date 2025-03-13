use std::time::{Duration, SystemTime};

use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use spl_network_messages::{GamePhase, GameState, Penalty, SubState, Team, TeamState};

use crate::{field_dimensions::GlobalFieldSide, players::Players};

#[derive(Clone, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct GameControllerState {
    pub game_state: GameState,
    pub game_phase: GamePhase,
    pub remaining_time_in_half: Duration,
    pub kicking_team: Team,
    pub last_game_state_change: SystemTime,
    pub penalties: Players<Option<Penalty>>,
    pub opponent_penalties: Players<Option<Penalty>>,
    pub sub_state: Option<SubState>,
    pub global_field_side: GlobalFieldSide,
    pub hulks_team: TeamState,
    pub opponent_team: TeamState,
}
