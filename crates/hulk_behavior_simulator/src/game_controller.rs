use std::time::SystemTime;

use bevy::prelude::*;

use spl_network_messages::{
    GamePhase, GameState, Penalty, PlayerNumber, Team, TeamColor, TeamState,
};
use types::{game_controller_state::GameControllerState, players::Players};

#[derive(Resource, Default)]
struct GameControllerControllerState {
    last_state_change: Time,
}

#[derive(Clone, Copy, Event)]
pub enum GameControllerCommand {
    SetGameState(GameState),
    Goal(Team),
    Penalize(PlayerNumber, Penalty),
    Unpenalize(PlayerNumber),
}

fn game_controller_controller(
    mut commands: EventReader<GameControllerCommand>,
    mut state: ResMut<GameControllerControllerState>,
    mut game_controller: ResMut<GameController>,
    time: ResMut<Time>,
) {
    for command in commands.read() {
        match *command {
            GameControllerCommand::SetGameState(game_state) => {
                game_controller.state.game_state = game_state;
                state.last_state_change = time.as_generic();
            }
            GameControllerCommand::Goal(team) => {
                match team {
                    Team::Hulks => &mut game_controller.state.hulks_team,
                    Team::Opponent => &mut game_controller.state.opponent_team,
                    Team::Uncertain => unimplemented!("Can't score goal for unknown team"),
                }
                .score += 1;
                game_controller.state.game_state = GameState::Ready;
                state.last_state_change = time.as_generic();
            }
            GameControllerCommand::Penalize(player_number, penalty) => {
                game_controller.state.penalties[player_number] = Some(penalty);
            }
            GameControllerCommand::Unpenalize(player_number) => {
                game_controller.state.penalties[player_number] = None;
            }
        }
    }

    match game_controller.state.game_state {
        GameState::Initial => {
            game_controller.state.game_state = GameState::Standby;
            state.last_state_change = time.as_generic();
        }
        GameState::Standby => {
            if time.elapsed_seconds() - state.last_state_change.elapsed_seconds() > 5.0 {
                game_controller.state.game_state = GameState::Ready;
                state.last_state_change = time.as_generic();
            }
        }
        GameState::Ready => {
            if time.elapsed_seconds() - state.last_state_change.elapsed_seconds() > 30.0 {
                game_controller.state.game_state = GameState::Set;
                state.last_state_change = time.as_generic();
            }
        }
        GameState::Set => {
            if time.elapsed_seconds() - state.last_state_change.elapsed_seconds() > 3.0 {
                game_controller.state.game_state = GameState::Playing;
                state.last_state_change = time.as_generic();
            }
        }
        GameState::Playing => {}
        GameState::Finished => {}
    }
}

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

pub fn game_controller_plugin(app: &mut App) {
    app.add_systems(Update, game_controller_controller);
    app.init_resource::<GameControllerControllerState>();
    app.init_resource::<Events<GameControllerCommand>>();
}
