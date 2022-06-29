use std::time::{Duration, SystemTime};

use macros::{module, require_some};
use nalgebra::{Isometry2, Vector2};
use spl_network::{GamePhase, GameState, Penalty, PlayerNumber};

use crate::types::{
    BallPosition, Buttons, FieldDimensions, FilteredGameState, FilteredWhistle,
    GameControllerState, SensorData,
};

pub struct GameStateFilter {
    game_state: FilteredGameState,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = filtered_whistle, data_type = FilteredWhistle)]
#[input(path = buttons, data_type = Buttons)]
#[input(path = ball_position, data_type = BallPosition)]
#[persistent_state(path = robot_to_field, data_type = Isometry2<f32>)]
#[parameter(path = player_number, data_type = PlayerNumber)]
#[parameter(path = field_dimensions, data_type = FieldDimensions)]
#[parameter(path = control.game_state_filter.max_wait_for_ready_message, data_type = f32)]
#[parameter(path = control.game_state_filter.whistle_acceptance_goal_distance, data_type = Vector2<f32>)]
#[input(path = game_controller_state, data_type = GameControllerState)]
#[main_output(data_type = FilteredGameState)]
impl GameStateFilter {}

impl GameStateFilter {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            game_state: FilteredGameState::Initial,
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let game_controller_state = match context.game_controller_state {
            Some(game_controller_state) => game_controller_state,
            None => {
                return Ok(MainOutputs {
                    filtered_game_state: None,
                })
            }
        };

        let cycle_start_time = require_some!(context.sensor_data).cycle_info.start_time;
        let whistle_started_this_cycle = require_some!(context.filtered_whistle).started_this_cycle;
        let illegal_motion_in_set = matches!(
            game_controller_state.penalties[*context.player_number],
            Some(Penalty::IllegalMotionInSet { .. })
        );
        self.game_state = next_game_state(
            self.game_state,
            whistle_started_this_cycle,
            game_controller_state,
            illegal_motion_in_set,
            cycle_start_time,
            Duration::from_secs_f32(*context.max_wait_for_ready_message),
            ball_is_near_goal(
                *context.robot_to_field,
                *context.ball_position,
                context.field_dimensions,
                *context.whistle_acceptance_goal_distance,
            ),
        );

        Ok(MainOutputs {
            filtered_game_state: Some(self.game_state),
        })
    }
}

pub fn from_game_state(game_state: &GameState, changed_time: SystemTime) -> FilteredGameState {
    match game_state {
        GameState::Initial => FilteredGameState::Initial,
        GameState::Ready => FilteredGameState::Ready { changed_time },
        GameState::Set => FilteredGameState::Set,
        GameState::Playing => FilteredGameState::Playing { changed_time },
        GameState::Finished => FilteredGameState::Finished,
    }
}

fn next_game_state(
    previous_game_state: FilteredGameState,
    whistle_started_this_cycle: bool,
    game_controller_state: &GameControllerState,
    illegal_motion_in_set: bool,
    cycle_start_time: SystemTime,
    max_wait_for_ready_message: Duration,
    ball_is_near_goal: Option<bool>,
) -> FilteredGameState {
    let previous_game_state = if illegal_motion_in_set {
        FilteredGameState::Set
    } else {
        previous_game_state
    };
    match (
        previous_game_state,
        whistle_started_this_cycle,
        game_controller_state.game_state,
        game_controller_state.game_phase,
    ) {
        (FilteredGameState::Set, true, GameState::Set, GamePhase::Normal)
        | (FilteredGameState::Playing { .. }, true, GameState::Set, GamePhase::Normal) => {
            FilteredGameState::Playing {
                changed_time: cycle_start_time,
            }
        }
        (FilteredGameState::Playing { .. }, true, GameState::Playing, GamePhase::Normal) => {
            if let Some(false) = ball_is_near_goal {
                FilteredGameState::Playing {
                    changed_time: cycle_start_time,
                }
            } else {
                FilteredGameState::Ready {
                    changed_time: cycle_start_time,
                }
            }
        }
        (FilteredGameState::Ready { .. }, true, GameState::Playing, GamePhase::Normal) => {
            FilteredGameState::Ready {
                changed_time: cycle_start_time,
            }
        }
        (FilteredGameState::Ready { changed_time }, _, GameState::Playing, GamePhase::Normal)
            if cycle_start_time
                .duration_since(changed_time)
                .expect("Last state change was after cycle start time")
                < max_wait_for_ready_message =>
        {
            previous_game_state
        }
        (FilteredGameState::Playing { changed_time }, _, GameState::Set, GamePhase::Normal)
            if cycle_start_time
                .duration_since(changed_time)
                .expect("Last state change was after cycle start time")
                < max_wait_for_ready_message =>
        {
            previous_game_state
        }
        _ => from_game_state(
            &game_controller_state.game_state,
            game_controller_state.last_game_state_change,
        ),
    }
}

fn ball_is_near_goal(
    robot_to_field: Isometry2<f32>,
    ball_position: Option<BallPosition>,
    field_dimensions: &FieldDimensions,
    whistle_acceptance_goal_distance: Vector2<f32>,
) -> Option<bool> {
    let ball_on_field = robot_to_field * ball_position?.position;
    Some(
        ball_on_field.x.abs() > field_dimensions.length / 2.0 - whistle_acceptance_goal_distance.x
            && ball_on_field.y.abs()
                < field_dimensions.goal_inner_width / 2.0 + whistle_acceptance_goal_distance.y,
    )
}
