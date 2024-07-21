use std::time::SystemTime;

use bevy::prelude::*;
use spl_network_messages::{GamePhase, GameState, PlayerNumber, Team, TeamColor, TeamState};
use types::{game_controller_state::GameControllerState, players::Players};

#[derive(Resource)]
pub struct GameController {
    pub state: GameControllerState,
}

impl Default for GameController {
    fn default() -> Self {
        Self {
            state: GameControllerState {
                game_state: GameState::Initial,
                game_phase: GamePhase::Normal,
                kicking_team: Team::Hulks,
                last_game_state_change: SystemTime::UNIX_EPOCH,
                penalties: Players::new(None),
                opponent_penalties: Players::new(None),
                remaining_amount_of_messages: 1200,
                sub_state: None,
                hulks_team_is_home_after_coin_toss: true,
                hulks_team: TeamState {
                    team_number: 24,
                    field_player_color: TeamColor::Green,
                    goal_keeper_color: TeamColor::Red,
                    goal_keeper_player_number: PlayerNumber::One,
                    score: 0,
                    penalty_shoot_index: 0,
                    penalty_shoots: Vec::new(),
                    remaining_amount_of_messages: 1200,
                    players: Vec::new(),
                },
                opponent_team: TeamState {
                    team_number: 1,
                    field_player_color: TeamColor::Black,
                    goal_keeper_color: TeamColor::Gray,
                    goal_keeper_player_number: PlayerNumber::One,
                    score: 0,
                    penalty_shoot_index: 0,
                    penalty_shoots: Vec::new(),
                    remaining_amount_of_messages: 1200,
                    players: Vec::new(),
                },
            },
        }
    }
}
