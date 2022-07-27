use module_derive::{module, require_some};
use nalgebra::{distance, Isometry2, Point2, Vector2};
use spl_network::{GameState, PlayerNumber, Team};
use std::time::{Duration, SystemTime};

use types::{
    BallPosition, Buttons, FieldDimensions, FilteredGameState, FilteredWhistle,
    GameControllerState, SensorData,
};

use crate::framework::configuration;

#[derive(Clone, Copy)]
enum State {
    Initial,
    Ready,
    Set,
    WhistleInSet {
        time_when_whistle_was_detected: SystemTime,
    },
    Playing,
    WhistleInPlaying {
        time_when_whistle_was_detected: SystemTime,
    },
    Finished,
}

impl State {
    fn from_game_controller(game_controller_state: &GameControllerState) -> Self {
        match game_controller_state.game_state {
            GameState::Initial => State::Initial,
            GameState::Ready => State::Ready,
            GameState::Set => State::Set,
            GameState::Playing => State::Playing,
            GameState::Finished => State::Finished,
        }
    }

    fn construct_filtered_game_state(
        &self,
        game_controller_state: &GameControllerState,
        cycle_start_time: SystemTime,
        ball_detected_far_from_kick_off_point: bool,
        config: &configuration::GameStateFilter,
    ) -> FilteredGameState {
        let is_in_set_play = matches!(game_controller_state.set_play, Some(_));
        let opponent_is_kicking_team = matches!(
            game_controller_state.kicking_team,
            Team::Opponent | Team::Uncertain
        );

        match self {
            State::Initial => FilteredGameState::Initial,
            State::Ready => FilteredGameState::Ready {
                kicking_team: game_controller_state.kicking_team,
            },
            State::Set => FilteredGameState::Set,
            State::WhistleInSet {
                time_when_whistle_was_detected,
            } => {
                let kick_off_grace_period = in_kick_off_grace_period(
                    cycle_start_time,
                    *time_when_whistle_was_detected,
                    config.kick_off_grace_period + config.game_controller_controller_delay,
                );
                let opponent_kick_off = opponent_is_kicking_team
                    && kick_off_grace_period
                    && !ball_detected_far_from_kick_off_point;
                let opponent_set_play = opponent_is_kicking_team && is_in_set_play;
                FilteredGameState::Playing {
                    ball_is_free: !opponent_kick_off && !opponent_set_play,
                }
            }
            State::Playing => FilteredGameState::Playing {
                ball_is_free: !(is_in_set_play && opponent_is_kicking_team),
            },
            State::WhistleInPlaying { .. } => FilteredGameState::Ready {
                kicking_team: Team::Uncertain,
            },
            State::Finished => match game_controller_state.game_phase {
                spl_network::GamePhase::PenaltyShootout { .. } => FilteredGameState::Set,
                _ => FilteredGameState::Finished,
            },
        }
    }
}

pub struct GameStateFilter {
    state: State,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = filtered_whistle, data_type = FilteredWhistle)]
#[input(path = buttons, data_type = Buttons)]
#[input(path = ball_position, data_type = BallPosition)]
#[persistent_state(path = robot_to_field, data_type = Isometry2<f32>)]
#[parameter(path = player_number, data_type = PlayerNumber)]
#[parameter(path = field_dimensions, data_type = FieldDimensions)]
#[parameter(path = control.game_state_filter, data_type = configuration::GameStateFilter, name = config)]
#[input(path = game_controller_state, data_type = GameControllerState)]
#[main_output(data_type = FilteredGameState)]
impl GameStateFilter {}

impl GameStateFilter {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            state: State::Initial,
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let game_controller_state = require_some!(context.game_controller_state);
        let cycle_start_time = require_some!(context.sensor_data).cycle_info.start_time;
        let is_whistle_detected = require_some!(context.filtered_whistle).started_this_cycle;

        let ball_detected_far_from_any_goal = ball_detected_far_from_any_goal(
            *context.robot_to_field,
            *context.ball_position,
            context.field_dimensions,
            context.config.whistle_acceptance_goal_distance,
        );

        self.state = next_filtered_state(
            self.state,
            game_controller_state,
            is_whistle_detected,
            cycle_start_time,
            context.config,
            ball_detected_far_from_any_goal,
        );

        let ball_detected_far_from_kick_off_point = context
            .ball_position
            .map(|ball| {
                let absolute_ball_position = *context.robot_to_field * ball.position;
                distance(&absolute_ball_position, &Point2::origin())
                    > context.config.distance_to_consider_ball_moved_in_kick_off
            })
            .unwrap_or(false);

        let filtered_game_state = self.state.construct_filtered_game_state(
            game_controller_state,
            cycle_start_time,
            ball_detected_far_from_kick_off_point,
            context.config,
        );

        Ok(MainOutputs {
            filtered_game_state: Some(filtered_game_state),
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn next_filtered_state(
    current_state: State,
    game_controller_state: &GameControllerState,
    is_whistle_detected: bool,
    cycle_start_time: SystemTime,
    config: &configuration::GameStateFilter,
    ball_detected_far_from_any_goal: bool,
) -> State {
    match (current_state, game_controller_state.game_state) {
        (State::Initial | State::Ready | State::Finished, _)
        | (
            State::Set,
            GameState::Initial | GameState::Ready | GameState::Playing | GameState::Finished,
        )
        | (
            State::WhistleInSet { .. },
            GameState::Initial | GameState::Ready | GameState::Playing | GameState::Finished,
        )
        | (
            State::Playing,
            GameState::Initial | GameState::Ready | GameState::Set | GameState::Finished,
        )
        | (
            State::WhistleInPlaying { .. },
            GameState::Initial | GameState::Ready | GameState::Set | GameState::Finished,
        ) => State::from_game_controller(game_controller_state),
        (State::Set, GameState::Set) => {
            if is_whistle_detected {
                State::WhistleInSet {
                    time_when_whistle_was_detected: cycle_start_time,
                }
            } else {
                State::Set
            }
        }
        (
            State::WhistleInSet {
                time_when_whistle_was_detected,
            },
            GameState::Set,
        ) => {
            if cycle_start_time
                .duration_since(time_when_whistle_was_detected)
                .unwrap()
                < config.playing_message_delay + config.game_controller_controller_delay
            {
                State::WhistleInSet {
                    time_when_whistle_was_detected,
                }
            } else {
                State::Set
            }
        }
        (State::Playing, GameState::Playing) => {
            if is_whistle_detected && !ball_detected_far_from_any_goal {
                State::WhistleInPlaying {
                    time_when_whistle_was_detected: cycle_start_time,
                }
            } else {
                State::Playing
            }
        }
        (
            State::WhistleInPlaying {
                time_when_whistle_was_detected,
            },
            GameState::Playing,
        ) => {
            if cycle_start_time
                .duration_since(time_when_whistle_was_detected)
                .unwrap()
                < config.ready_message_delay + config.game_controller_controller_delay
            {
                State::WhistleInPlaying {
                    time_when_whistle_was_detected,
                }
            } else {
                State::Playing
            }
        }
    }
}

fn ball_detected_far_from_any_goal(
    robot_to_field: Isometry2<f32>,
    ball: Option<BallPosition>,
    field_dimensions: &FieldDimensions,
    whistle_acceptance_goal_distance: Vector2<f32>,
) -> bool {
    match ball {
        Some(ball) => {
            let ball_on_field = robot_to_field * ball.position;
            ball_on_field.x.abs()
                < field_dimensions.length / 2.0 - whistle_acceptance_goal_distance.x
                || ball_on_field.y.abs()
                    > field_dimensions.goal_inner_width / 2.0 + whistle_acceptance_goal_distance.y
        }
        None => false,
    }
}

fn in_kick_off_grace_period(
    cycle_start_time: SystemTime,
    time_entered_playing: SystemTime,
    kick_off_grace_period: Duration,
) -> bool {
    cycle_start_time
        .duration_since(time_entered_playing)
        .expect("Time ran backwards")
        < kick_off_grace_period
}
