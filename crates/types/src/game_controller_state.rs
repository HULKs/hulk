use std::time::{Duration, SystemTime};

use hsl_network_messages::{GamePhase, GameState, Half, Penalty, SubState, Team, TeamState};
use ros_z::Message;
use serde::{Deserialize, Serialize};

use crate::{field_dimensions::GlobalFieldSide, players::Players};

#[derive(Clone, Debug, Serialize, Deserialize, Message)]
pub struct GameControllerState {
    pub game_state: GameState,
    pub stopped: bool,
    pub game_phase: GamePhase,
    pub half: Half,
    pub remaining_time_in_half: Duration,
    #[serde(default)]
    pub secondary_time: Duration,
    pub kicking_team: Option<Team>,
    pub last_game_state_change: SystemTime,
    pub penalties: Players<Option<Penalty>>,
    pub opponent_penalties: Players<Option<Penalty>>,
    pub sub_state: Option<SubState>,
    pub global_field_side: GlobalFieldSide,
    pub hulks_team: TeamState,
    pub opponent_team: TeamState,
}

impl GameControllerState {
    /// HSL encodes kickoff as no sub-state, with a kicking team and a running secondary timer.
    pub fn is_kick_off(&self) -> bool {
        self.game_state == GameState::Playing
            && matches!(self.game_phase, GamePhase::Normal | GamePhase::Extratime)
            && self.sub_state.is_none()
            && self.kicking_team.is_some()
            && !self.secondary_time.is_zero()
    }
}
