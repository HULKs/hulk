use std::time::{Duration, SystemTime};

use macros::{module, require_some};
use spl_network::{GamePhase, GameState, Penalty};

use crate::types::{Buttons, FilteredGameState, FilteredWhistle, GameControllerState, SensorData};

pub struct GameStateFilter {
    game_state: FilteredGameState,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = filtered_whistle, data_type = FilteredWhistle)]
#[input(path = buttons, data_type = Buttons)]
#[parameter(path = player_number, data_type = usize)]
#[parameter(path = control.game_state_filter.max_wait_for_ready_message, data_type = f32)]
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
        (FilteredGameState::Playing { .. }, true, GameState::Playing, GamePhase::Normal)
        | (FilteredGameState::Ready { .. }, true, GameState::Playing, GamePhase::Normal) => {
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
