use std::time::SystemTime;

use bevy::prelude::*;

use spl_network_messages::{
    GamePhase, GameState, Penalty, PlayerNumber, SubState, Team, TeamColor, TeamState,
};
use types::{game_controller_state::GameControllerState, players::Players};

use crate::{autoref::autoref, whistle::WhistleResource};

#[derive(Resource, Default)]
struct GameControllerControllerState {
    last_state_change: Time,
}

#[derive(Clone, Copy, Event)]
pub enum GameControllerCommand {
    SetGameState(GameState),
    SetGamePhase(GamePhase),
    SetSubState(Option<SubState>, Team),
    SetKickingTeam(Team),
    Goal(Team),
    Penalize(PlayerNumber, Penalty),
    Unpenalize(PlayerNumber),
    BallIsFree,
}

fn game_controller_controller(
    mut commands: EventReader<GameControllerCommand>,
    mut state: ResMut<GameControllerControllerState>,
    mut game_controller: ResMut<GameController>,
    whistle: ResMut<WhistleResource>,
    time: ResMut<Time>,
) {
    for command in commands.read() {
        match *command {
            GameControllerCommand::SetGameState(game_state) => {
                game_controller.state.game_state = game_state;
                state.last_state_change = time.as_generic();
            }
            GameControllerCommand::SetGamePhase(game_phase) => {
                game_controller.state.game_phase = game_phase;
                state.last_state_change = time.as_generic();
            }
            GameControllerCommand::SetKickingTeam(team) => {
                game_controller.state.kicking_team = Some(team);
                state.last_state_change = time.as_generic();
            }
            GameControllerCommand::Goal(team) => {
                match team {
                    Team::Hulks => {
                        game_controller.state.kicking_team = Some(Team::Opponent);
                        &mut game_controller.state.hulks_team
                    }
                    Team::Opponent => {
                        game_controller.state.kicking_team = Some(Team::Hulks);
                        &mut game_controller.state.opponent_team
                    }
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
            GameControllerCommand::SetSubState(sub_state, team) => {
                game_controller.state.sub_state = sub_state;
                game_controller.state.kicking_team = Some(team);
                if sub_state == Some(SubState::PenaltyKick) {
                    game_controller.state.game_state = GameState::Ready;
                }
                state.last_state_change = time.as_generic();
            }
            GameControllerCommand::BallIsFree => {
                game_controller.state.sub_state = None;
                state.last_state_change = time.as_generic();
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
            if Some(time.elapsed()) == whistle.last_whistle {
                game_controller.state.game_state = GameState::Playing;
                state.last_state_change = time.as_generic();
            }
        }
        GameState::Playing => {}
        GameState::Finished => {}
    }

    if game_controller.state.sub_state.is_some()
        && time.elapsed_seconds() - state.last_state_change.elapsed_seconds() > 30.0
    {
        game_controller.state.sub_state = None;
        state.last_state_change = time.as_generic();
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
                kicking_team: Some(Team::Hulks),
                last_game_state_change: SystemTime::UNIX_EPOCH,
                penalties: Players::new(None),
                opponent_penalties: Players::new(None),
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
    app.add_systems(Update, game_controller_controller.after(autoref));
    app.init_resource::<GameControllerControllerState>();
    app.init_resource::<Events<GameControllerCommand>>();
}
