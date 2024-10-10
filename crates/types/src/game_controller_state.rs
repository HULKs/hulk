use std::time::SystemTime;

use crate::players::Players;

use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use spl_network_messages::{GamePhase, GameState, Penalty, SubState, Team, TeamState};

#[derive(Clone, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct GameControllerState {
    pub game_state: GameState,
    pub game_phase: GamePhase,
    pub kicking_team: Team,
    pub last_game_state_change: SystemTime,
    pub penalties: Players<Option<Penalty>>,
    pub opponent_penalties: Players<Option<Penalty>>,
    pub sub_state: Option<SubState>,
    pub hulks_team_is_home_after_coin_toss: bool,
    pub hulks_team: TeamState,
    pub opponent_team: TeamState,
}
