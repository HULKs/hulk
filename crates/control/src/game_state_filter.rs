use std::time::{Duration, SystemTime};

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use nalgebra::{distance, Isometry2, Point2, Vector2};
use spl_network_messages::{GamePhase, GameState, Team};
use types::{
    parameters::GameStateFilterParameters, BallPosition, CycleTime, FieldDimensions,
    FilteredGameState, FilteredWhistle, GameControllerState,
};

pub struct GameStateFilter {
    state: State,
    opponent_state: State,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    ball_position: Input<Option<BallPosition>, "ball_position?">,
    cycle_time: Input<CycleTime, "cycle_time">,
    filtered_whistle: Input<FilteredWhistle, "filtered_whistle">,
    game_controller_state: RequiredInput<Option<GameControllerState>, "game_controller_state?">,

    config: Parameter<GameStateFilterParameters, "game_state_filter">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,

    robot_to_field: PersistentState<Isometry2<f32>, "robot_to_field">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub filtered_game_state: MainOutput<Option<FilteredGameState>>,
    pub filtered_opponent_game_state: MainOutput<Option<FilteredGameState>>,
}

impl GameStateFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            state: State::Initial,
            opponent_state: State::Initial,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let ball_detected_far_from_any_goal = ball_detected_far_from_any_goal(
            *context.robot_to_field,
            context.ball_position,
            context.field_dimensions,
            context.config.whistle_acceptance_goal_distance,
        );

        self.state = next_filtered_state(
            self.state,
            context.game_controller_state,
            context.filtered_whistle.is_detected,
            context.cycle_time.start_time,
            context.config,
            ball_detected_far_from_any_goal,
        );

        self.opponent_state = next_filtered_state(
            self.opponent_state,
            context.game_controller_state,
            context.filtered_whistle.is_detected,
            context.cycle_time.start_time,
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

        let filtered_game_state = self.state.construct_hulks_filtered_game_state(
            context.game_controller_state,
            context.cycle_time.start_time,
            ball_detected_far_from_kick_off_point,
            context.config,
        );

        let filtered_opponent_game_state =
            self.opponent_state.construct_opponent_filtered_game_state(
                context.game_controller_state,
                context.cycle_time.start_time,
                ball_detected_far_from_kick_off_point,
                context.config,
            );

        Ok(MainOutputs {
            filtered_game_state: Some(filtered_game_state).into(),
            filtered_opponent_game_state: Some(filtered_opponent_game_state).into(),
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn next_filtered_state(
    current_state: State,
    game_controller_state: &GameControllerState,
    is_whistle_detected: bool,
    cycle_start_time: SystemTime,
    config: &GameStateFilterParameters,
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
    ball: Option<&BallPosition>,
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

    fn construct_hulks_filtered_game_state(
        &self,
        game_controller_state: &GameControllerState,
        cycle_start_time: SystemTime,
        ball_detected_far_from_kick_off_point: bool,
        config: &GameStateFilterParameters,
    ) -> FilteredGameState {
        let opponent_is_kicking_team = matches!(
            game_controller_state.kicking_team,
            Team::Opponent | Team::Uncertain
        );
        self.construct_filtered_game_state(
            game_controller_state,
            opponent_is_kicking_team,
            cycle_start_time,
            ball_detected_far_from_kick_off_point,
            config,
        )
    }

    fn construct_opponent_filtered_game_state(
        &self,
        game_controller_state: &GameControllerState,
        cycle_start_time: SystemTime,
        ball_detected_far_from_kick_off_point: bool,
        config: &GameStateFilterParameters,
    ) -> FilteredGameState {
        let hulks_is_kicking_team = matches!(game_controller_state.kicking_team, Team::Hulks);
        self.construct_filtered_game_state(
            game_controller_state,
            hulks_is_kicking_team,
            cycle_start_time,
            ball_detected_far_from_kick_off_point,
            config,
        )
    }

    fn construct_filtered_game_state(
        &self,
        game_controller_state: &GameControllerState,
        opponent_is_kicking_team: bool,
        cycle_start_time: SystemTime,
        ball_detected_far_from_kick_off_point: bool,
        config: &GameStateFilterParameters,
    ) -> FilteredGameState {
        let is_in_sub_state = game_controller_state.sub_state.is_some();

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
                let opponent_sub_state = opponent_is_kicking_team && is_in_sub_state;
                FilteredGameState::Playing {
                    ball_is_free: !opponent_kick_off && !opponent_sub_state,
                }
            }
            State::Playing => FilteredGameState::Playing {
                ball_is_free: !(is_in_sub_state && opponent_is_kicking_team),
            },
            State::WhistleInPlaying { .. } => FilteredGameState::Ready {
                kicking_team: Team::Uncertain,
            },
            State::Finished => match game_controller_state.game_phase {
                GamePhase::PenaltyShootout { .. } => FilteredGameState::Set,
                _ => FilteredGameState::Finished,
            },
        }
    }
}
