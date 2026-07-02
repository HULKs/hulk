use std::time::{Duration, SystemTime};

use bevy::prelude::*;
use hsl_network_messages::{GamePhase, GameState, Half, PlayerNumber, Team, TeamColor, TeamState};
use types::{
    field_dimensions::GlobalFieldSide, filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState, game_controller_state::GameControllerState,
    players::Players, primary_state::PrimaryState,
};

use crate::behavior_tree_simulator::SimulatorPrimaryState;

const HULKS_TEAM_NUMBER: u8 = 24;
const OPPONENT_TEAM_NUMBER: u8 = 1;

#[derive(Resource, Clone, Debug)]
pub struct SimulatorGameState {
    pub game_controller_state: GameControllerState,
    pub filtered_game_controller_state: Option<FilteredGameControllerState>,
}

impl Default for SimulatorGameState {
    fn default() -> Self {
        let game_controller_state = default_game_controller_state();
        Self {
            filtered_game_controller_state: Some(filtered_game_controller_state_from(
                &game_controller_state,
            )),
            game_controller_state,
        }
    }
}

impl SimulatorGameState {
    pub fn set_game_state(&mut self, game_state: GameState, now: SystemTime) {
        self.game_controller_state.game_state = game_state;
        self.game_controller_state.last_game_state_change = now;
        self.sync_filtered_game_controller_state();
    }

    pub fn set_kicking_team(&mut self, kicking_team: Option<Team>) {
        self.game_controller_state.kicking_team = kicking_team;
        self.sync_filtered_game_controller_state();
    }

    pub fn set_sub_state(&mut self, sub_state: Option<hsl_network_messages::SubState>) {
        self.game_controller_state.sub_state = sub_state;
        self.sync_filtered_game_controller_state();
    }

    pub fn set_stopped(&mut self, stopped: bool) {
        self.game_controller_state.stopped = stopped;
        self.sync_filtered_game_controller_state();
    }

    pub fn set_game_phase(&mut self, game_phase: GamePhase) {
        self.game_controller_state.game_phase = game_phase;
        self.sync_filtered_game_controller_state();
    }

    pub fn sync_filtered_game_controller_state(&mut self) {
        self.filtered_game_controller_state = Some(filtered_game_controller_state_from(
            &self.game_controller_state,
        ));
    }
}

pub fn global_field_side_for_team(
    game_controller_state: &GameControllerState,
    team: Team,
) -> GlobalFieldSide {
    match team {
        Team::Hulks => game_controller_state.global_field_side,
        Team::Opponent => game_controller_state.global_field_side.mirror(),
    }
}

pub fn default_game_controller_state() -> GameControllerState {
    GameControllerState {
        game_state: GameState::Playing,
        stopped: false,
        game_phase: GamePhase::Normal,
        half: Half::First,
        remaining_time_in_half: Duration::ZERO,
        kicking_team: Some(Team::Hulks),
        last_game_state_change: SystemTime::UNIX_EPOCH,
        penalties: Players::new(None),
        opponent_penalties: Players::new(None),
        sub_state: None,
        global_field_side: GlobalFieldSide::Home,
        hulks_team: TeamState {
            team_number: HULKS_TEAM_NUMBER,
            field_player_color: TeamColor::Green,
            goal_keeper_color: TeamColor::Red,
            goal_keeper_player_number: Some(PlayerNumber::One),
            score: 0,
            penalty_shoot_index: 0,
            penalty_shoots: Vec::new(),
            remaining_amount_of_messages: 1200,
            players: Vec::new(),
        },
        opponent_team: TeamState {
            team_number: OPPONENT_TEAM_NUMBER,
            field_player_color: TeamColor::Black,
            goal_keeper_color: TeamColor::Gray,
            goal_keeper_player_number: Some(PlayerNumber::One),
            score: 0,
            penalty_shoot_index: 0,
            penalty_shoots: Vec::new(),
            remaining_amount_of_messages: 1200,
            players: Vec::new(),
        },
    }
}

pub fn filtered_game_controller_state_from(
    game_controller_state: &GameControllerState,
) -> FilteredGameControllerState {
    filtered_game_controller_state_for_team(game_controller_state, Team::Hulks)
}

pub fn filtered_game_controller_state_for_team(
    game_controller_state: &GameControllerState,
    team: Team,
) -> FilteredGameControllerState {
    let (penalties, remaining_number_of_messages) = match team {
        Team::Hulks => (
            game_controller_state.penalties,
            game_controller_state
                .hulks_team
                .remaining_amount_of_messages,
        ),
        Team::Opponent => (
            game_controller_state.opponent_penalties,
            game_controller_state
                .opponent_team
                .remaining_amount_of_messages,
        ),
    };

    FilteredGameControllerState {
        game_state: filtered_game_state_from(game_controller_state),
        opponent_game_state: filtered_game_state_from(game_controller_state),
        half: game_controller_state.half,
        remaining_time_in_half: game_controller_state.remaining_time_in_half,
        game_phase: game_controller_state.game_phase,
        kicking_team: game_controller_state
            .kicking_team
            .map(|kicking_team| local_team_for(team, kicking_team)),
        penalties,
        remaining_number_of_messages,
        sub_state: game_controller_state.sub_state,
        global_field_side: global_field_side_for_team(game_controller_state, team),
        new_own_penalties_last_cycle: Default::default(),
        new_opponent_penalties_last_cycle: Default::default(),
    }
}

fn local_team_for(own_team: Team, canonical_team: Team) -> Team {
    if own_team == canonical_team {
        Team::Hulks
    } else {
        Team::Opponent
    }
}

pub fn filtered_game_state_from(game_controller_state: &GameControllerState) -> FilteredGameState {
    if game_controller_state.stopped {
        return FilteredGameState::Stop;
    }

    match game_controller_state.game_state {
        GameState::Initial => FilteredGameState::Initial,
        GameState::Ready => FilteredGameState::Ready,
        GameState::Set => FilteredGameState::Set,
        GameState::Playing => FilteredGameState::Playing {
            ball_is_free: true,
            kick_off: false,
        },
        GameState::Finished => FilteredGameState::Finished,
    }
}

pub fn primary_state_from_game_controller_state(
    game_controller_state: &GameControllerState,
) -> PrimaryState {
    match filtered_game_state_from(game_controller_state) {
        FilteredGameState::Initial => PrimaryState::Initial,
        FilteredGameState::Ready => PrimaryState::Ready,
        FilteredGameState::Set => PrimaryState::Set,
        FilteredGameState::Playing { .. } => PrimaryState::Playing,
        FilteredGameState::Finished => PrimaryState::Finished,
        FilteredGameState::Stop => PrimaryState::Stop,
    }
}

pub fn sync_primary_states_from_game_state(
    game_state: Res<SimulatorGameState>,
    mut robots: Query<&mut SimulatorPrimaryState>,
) {
    let primary_state = primary_state_from_game_controller_state(&game_state.game_controller_state);
    for mut robot_primary_state in &mut robots {
        robot_primary_state.primary_state = primary_state;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use hsl_network_messages::Penalty;

    #[test]
    fn opponent_filtered_state_uses_local_team_perspective() {
        let mut game_controller_state = default_game_controller_state();
        game_controller_state.global_field_side = GlobalFieldSide::Away;
        game_controller_state.kicking_team = Some(Team::Opponent);
        game_controller_state
            .opponent_team
            .remaining_amount_of_messages = 7;
        game_controller_state.opponent_penalties.three = Some(Penalty::Pushing {
            remaining: Duration::from_secs(5),
        });

        let filtered_state =
            filtered_game_controller_state_for_team(&game_controller_state, Team::Opponent);

        assert_eq!(filtered_state.global_field_side, GlobalFieldSide::Home);
        assert_eq!(filtered_state.kicking_team, Some(Team::Hulks));
        assert_eq!(filtered_state.remaining_number_of_messages, 7);
        assert!(matches!(
            filtered_state.penalties.three,
            Some(Penalty::Pushing { .. })
        ));
    }
}
