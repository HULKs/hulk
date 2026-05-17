use std::time::Duration;

use hsl_network_messages::{GamePhase, GameState, Team};
use ros_z::time::Time;
use serde::{Deserialize, Serialize};
use types::{
    filtered_game_state::FilteredGameState, game_controller_state::GameControllerState,
    parameters::GameStateFilterParameters,
};

#[derive(Clone, Copy, Deserialize, Serialize)]
pub enum State {
    Initial,
    Ready,
    Set,
    Stop,
    WhistleInSet {
        time_when_whistle_was_detected: Time,
    },
    Playing,
    WhistleInPlaying {
        time_when_whistle_was_detected: Time,
    },
    TentativeFinished {
        time_when_finished_clicked: Time,
    },
    Finished,
}

impl State {
    pub fn from_game_state(game_state: GameState) -> Self {
        match game_state {
            GameState::Initial => State::Initial,
            GameState::Ready => State::Ready,
            GameState::Set => State::Set,
            GameState::Playing => State::Playing,
            GameState::Finished => State::Finished,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn construct_filtered_game_state_for_team(
        &self,
        now: &Time,
        game_controller_state: &GameControllerState,
        team: Team,
        ball_detected_far_from_kick_off_point: bool,
        config: &GameStateFilterParameters,
        filtered_kicking_team: Option<Team>,
    ) -> FilteredGameState {
        let is_in_sub_state = game_controller_state.sub_state.is_some();
        let opponent_is_kicking_team = filtered_kicking_team != Some(team);

        match self {
            State::Initial => FilteredGameState::Initial,
            State::Ready => FilteredGameState::Ready,
            State::Set => FilteredGameState::Set,
            State::Stop => FilteredGameState::Stop,
            State::WhistleInSet {
                time_when_whistle_was_detected,
            } => {
                let kick_off_grace_period = is_in_grace_period(
                    now,
                    *time_when_whistle_was_detected,
                    config.kick_off_grace_period + config.game_controller_controller_delay,
                );
                let opponent_kick_off = opponent_is_kicking_team
                    && kick_off_grace_period
                    && !ball_detected_far_from_kick_off_point;
                let opponent_sub_state = opponent_is_kicking_team && is_in_sub_state;

                FilteredGameState::Playing {
                    ball_is_free: !opponent_kick_off && !opponent_sub_state,
                    kick_off: !is_in_sub_state,
                }
            }
            State::Playing => FilteredGameState::Playing {
                ball_is_free: !(is_in_sub_state && opponent_is_kicking_team),
                kick_off: false,
            },
            State::WhistleInPlaying { .. } => FilteredGameState::Ready,
            State::Finished => match game_controller_state.game_phase {
                GamePhase::PenaltyShootout { .. } => FilteredGameState::Set,
                _ => FilteredGameState::Finished,
            },
            // is hack @schluis
            State::TentativeFinished { .. } => FilteredGameState::Set,
        }
    }
}

fn is_in_grace_period(now: &Time, start_time: Time, grace_period: Duration) -> bool {
    now.duration_since(start_time) < grace_period
}
