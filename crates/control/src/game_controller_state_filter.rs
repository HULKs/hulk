use std::time::{Duration, SystemTime};

use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::MainOutput;
use linear_algebra::{distance, Isometry2, Point2, Vector2};
use serde::{Deserialize, Serialize};
use spl_network_messages::{GamePhase, GameState, Team};
use types::{
    ball_position::BallPosition, cycle_time::CycleTime, field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState, filtered_whistle::FilteredWhistle,
    game_controller_state::GameControllerState, parameters::GameStateFilterParameters,
};
#[derive(Deserialize, Serialize)]
pub struct GameControllerStateFilter {
    state: State,
    opponent_state: State,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    ball_position: Input<Option<BallPosition<Ground>>, "ball_position?">,
    cycle_time: Input<CycleTime, "cycle_time">,
    filtered_whistle: Input<FilteredWhistle, "filtered_whistle">,
    ready_to_initial_trigger: Input<bool, "ready_to_initial_trigger">,
    game_controller_state: RequiredInput<Option<GameControllerState>, "game_controller_state?">,
    config: Parameter<GameStateFilterParameters, "game_state_filter">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,

    ground_to_field: CyclerState<Isometry2<Ground, Field>, "ground_to_field">,
}

#[context]
pub struct MainOutputs {
    pub filtered_game_controller_state: MainOutput<Option<FilteredGameControllerState>>,
}

impl GameControllerStateFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            state: State::Initial,
            opponent_state: State::Initial,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let game_states = filter_game_states(
            *context.ground_to_field,
            context.ball_position,
            context.field_dimensions,
            context.config,
            context.game_controller_state,
            context.filtered_whistle,
            context.cycle_time,
            &mut self.state,
            &mut self.opponent_state,
            *context.ready_to_initial_trigger,
        );
        let filtered_game_controller_state = FilteredGameControllerState {
            game_state: game_states.own,
            opponent_game_state: game_states.opponent,
            game_phase: context.game_controller_state.game_phase,
            kicking_team: context.game_controller_state.kicking_team,
            penalties: context.game_controller_state.penalties,
            remaining_number_of_messages: context
                .game_controller_state
                .remaining_amount_of_messages,
            sub_state: context.game_controller_state.sub_state,
            own_team_is_home_after_coin_toss: context
                .game_controller_state
                .hulks_team_is_home_after_coin_toss,
        };

        Ok(MainOutputs {
            filtered_game_controller_state: Some(filtered_game_controller_state).into(),
        })
    }
}

struct FilteredGameStates {
    own: FilteredGameState,
    opponent: FilteredGameState,
}

#[allow(clippy::too_many_arguments)]
fn filter_game_states(
    ground_to_field: Isometry2<Ground, Field>,
    ball_position: Option<&BallPosition<Ground>>,
    field_dimensions: &FieldDimensions,
    config: &GameStateFilterParameters,
    game_controller_state: &GameControllerState,
    filtered_whistle: &FilteredWhistle,
    cycle_time: &CycleTime,
    state: &mut State,
    opponent_state: &mut State,
    ready_to_initial_trigger: bool,
) -> FilteredGameStates {
    let ball_detected_far_from_any_goal = ball_detected_far_from_any_goal(
        ground_to_field,
        ball_position,
        field_dimensions,
        config.whistle_acceptance_goal_distance,
    );
    *state = next_filtered_state(
        *state,
        game_controller_state,
        filtered_whistle.is_detected,
        cycle_time.start_time,
        config,
        ball_detected_far_from_any_goal,
        ready_to_initial_trigger,
    );
    *opponent_state = next_filtered_state(
        *opponent_state,
        game_controller_state,
        filtered_whistle.is_detected,
        cycle_time.start_time,
        config,
        ball_detected_far_from_any_goal,
        ready_to_initial_trigger,
    );
    let ball_detected_far_from_kick_off_point = ball_position
        .map(|ball| {
            let absolute_ball_position = ground_to_field * ball.position;
            distance(absolute_ball_position, Point2::origin())
                > config.distance_to_consider_ball_moved_in_kick_off
        })
        .unwrap_or(false);

    let filtered_game_state = state.construct_hulks_filtered_game_state(
        game_controller_state,
        cycle_time.start_time,
        ball_detected_far_from_kick_off_point,
        config,
    );

    let filtered_opponent_game_state = opponent_state.construct_opponent_filtered_game_state(
        game_controller_state,
        cycle_time.start_time,
        ball_detected_far_from_kick_off_point,
        config,
    );

    FilteredGameStates {
        own: filtered_game_state,
        opponent: filtered_opponent_game_state,
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
    ready_to_initial_trigger: bool,
) -> State {
    match (current_state, game_controller_state.game_state) {
        (State::Finished, GameState::Initial) => State::Initial,
        (State::Finished, _) => match game_controller_state.game_phase {
            GamePhase::PenaltyShootout { .. } => State::Set,
            _ => State::Finished,
        },
        (
            State::TentativeFinished {
                time_when_finished_clicked,
            },
            GameState::Finished,
        ) if cycle_start_time
            .duration_since(time_when_finished_clicked)
            .unwrap()
            >= config.tentative_finish_duration =>
        {
            State::Finished
        }
        (
            State::TentativeFinished {
                time_when_finished_clicked,
            },
            GameState::Finished,
        ) => State::TentativeFinished {
            time_when_finished_clicked,
        },
        (State::TentativeFinished { .. }, game_state) => State::from_game_state(game_state),
        (_, GameState::Finished) => State::TentativeFinished {
            time_when_finished_clicked: cycle_start_time,
        },
        (State::Initial, GameState::Initial) => {
            if ready_to_initial_trigger {
                State::OverArmInReady
            } else {
                State::Initial
            }
        }
        (State::Ready, GameState::Initial) => State::Ready,

        (State::Initial | State::Ready, _)
        | (State::Set, GameState::Initial | GameState::Ready | GameState::Playing)
        | (
            State::WhistleInSet { .. },
            GameState::Initial | GameState::Ready | GameState::Playing,
        )
        | (State::Playing, GameState::Initial | GameState::Ready | GameState::Set)
        | (
            State::WhistleInPlaying { .. },
            GameState::Initial | GameState::Ready | GameState::Set,
        ) => State::from_game_state(game_controller_state.game_state),
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
        (State::OverArmInReady { .. }, GameState::Initial) => State::Ready,
        (State::OverArmInReady { .. }, _) => {
            State::from_game_state(game_controller_state.game_state)
        }
    }
}

fn ball_detected_far_from_any_goal(
    ground_to_field: Isometry2<Ground, Field>,
    ball: Option<&BallPosition<Ground>>,
    field_dimensions: &FieldDimensions,
    whistle_acceptance_goal_distance: Vector2<Field>,
) -> bool {
    match ball {
        Some(ball) => {
            let ball_on_field = ground_to_field * ball.position;
            ball_on_field.x().abs()
                < field_dimensions.length / 2.0 - whistle_acceptance_goal_distance.x()
                || ball_on_field.y().abs()
                    > field_dimensions.goal_inner_width / 2.0 + whistle_acceptance_goal_distance.y()
        }
        None => false,
    }
}

fn in_grace_period(
    cycle_start_time: SystemTime,
    start_time: SystemTime,
    grace_period: Duration,
) -> bool {
    cycle_start_time
        .duration_since(start_time)
        .expect("Time ran backwards")
        < grace_period
}

#[derive(Clone, Copy, Deserialize, Serialize)]
enum State {
    Initial,
    Ready,
    OverArmInReady,
    Set,
    WhistleInSet {
        time_when_whistle_was_detected: SystemTime,
    },
    Playing,
    WhistleInPlaying {
        time_when_whistle_was_detected: SystemTime,
    },
    TentativeFinished {
        time_when_finished_clicked: SystemTime,
    },
    Finished,
}

impl State {
    fn from_game_state(game_state: GameState) -> Self {
        match game_state {
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
            State::OverArmInReady => FilteredGameState::Ready {
                kicking_team: game_controller_state.kicking_team,
            },
            State::Ready => FilteredGameState::Ready {
                kicking_team: game_controller_state.kicking_team,
            },
            State::Set => FilteredGameState::Set,
            State::WhistleInSet {
                time_when_whistle_was_detected,
            } => {
                let kick_off_grace_period = in_grace_period(
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
                    kick_off: !is_in_sub_state,
                }
            }
            State::Playing => FilteredGameState::Playing {
                ball_is_free: !(is_in_sub_state && opponent_is_kicking_team),
                kick_off: false,
            },
            State::WhistleInPlaying { .. } => FilteredGameState::Ready {
                kicking_team: Team::Uncertain,
            },
            State::Finished => match game_controller_state.game_phase {
                GamePhase::PenaltyShootout { .. } => FilteredGameState::Set,
                _ => FilteredGameState::Finished,
            },
            // is hack @schluis
            State::TentativeFinished { .. } => FilteredGameState::Set,
        }
    }
}
