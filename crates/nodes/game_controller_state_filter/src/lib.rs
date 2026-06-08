use std::{boxed::Box, future::Future, pin::Pin};
use std::{collections::HashMap, sync::Arc};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use coordinate_systems::Field;
use hsl_network_messages::{GamePhase, GameState, Penalty, PlayerNumber, SubState, Team};
use linear_algebra::{Point2, Vector2, distance};
use ros_z::{prelude::*, qos::QosDurability, time::Time};
use types::{
    field_dimensions::FieldDimensions, filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState, filtered_whistle::FilteredWhistle,
    game_controller_state::GameControllerState, parameters::GameStateFilterParameters,
    players::Players, world_state::BallState,
};

use crate::state::State;

pub mod state;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("game_controller_state_filter")
        .build()
        .await?;

    let parameters =
        node.bind_parameter_as::<GameStateFilterParameters>("game_controller_state_filter")?;
    let field_dimensions_cache = node
        .create_cache::<FieldDimensions>("field_dimensions", 1)?
        .with_qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let player_number_cache = node
        .create_cache::<PlayerNumber>("player_number", 1)?
        .with_qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;

    let game_controller_state_sub = node
        .subscriber::<Option<GameControllerState>>("game_controller_state")?
        .build()
        .await?;
    let filtered_whistle_cache = node
        .create_cache::<FilteredWhistle>("filtered_whistle", 1)?
        .build()
        .await?;
    let ball_state_cache = node
        .create_cache::<Option<BallState>>("ball_state", 1)?
        .build()
        .await?;

    let whistle_in_set_ball_position_pub = node
        .publisher::<Option<Point2<Field>>>("whistle_in_set_ball_position")?
        .build()
        .await?;
    let filtered_game_controller_state_pub = node
        .publisher::<FilteredGameControllerState>("filtered_game_controller_state")?
        .build()
        .await?;

    let mut latest_known_ball_state = None;
    let mut game_controller_state_filter = GameControllerStateFilter::default();

    loop {
        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();

        let Some(game_controller_state) = game_controller_state_sub.recv().await? else {
            continue;
        };

        let (Some(field_dimensions), Some(player_number)) = (
            field_dimensions_cache.get_latest(),
            player_number_cache.get_latest(),
        ) else {
            continue;
        };

        let filtered_whistle = filtered_whistle_cache.get_latest().unwrap_or_default();

        let current_ball_state_time = ball_state_cache.latest_stamp();
        let current_ball_state = ball_state_cache.get_latest().and_then(|maybe| *maybe);
        if let Some((ball_state, time)) = current_ball_state_time.zip(current_ball_state) {
            latest_known_ball_state = Some((ball_state, time))
        };

        let filtered_game_controller_state = game_controller_state_filter
            .compute_filtered_game_controller_state(
                node.clock().now(),
                parameters,
                &field_dimensions,
                &player_number,
                &game_controller_state,
                &latest_known_ball_state,
                &filtered_whistle,
                &current_ball_state,
            );

        whistle_in_set_ball_position_pub
            .publish(&game_controller_state_filter.whistle_in_set_ball_position)
            .await?;
        filtered_game_controller_state_pub
            .publish(&filtered_game_controller_state)
            .await?;
    }
}

struct FilteredGameStates {
    own: FilteredGameState,
    opponent: FilteredGameState,
}

#[derive(Default, Deserialize, Serialize)]
pub struct GameControllerStateFilter {
    state: State,
    opponent_state: State,
    last_game_controller_state: Option<GameControllerState>,
    whistle_in_set_ball_position: Option<Point2<Field>>,
    last_time_hulk_was_penalized: Option<Time>,
    last_time_opponent_was_penalized: Option<Time>,
}

impl GameControllerStateFilter {
    #[allow(clippy::too_many_arguments)]
    fn compute_filtered_game_controller_state(
        &mut self,
        now: Time,
        parameters: &GameStateFilterParameters,
        field_dimensions: &FieldDimensions,
        player_number: &PlayerNumber,
        game_controller_state: &GameControllerState,
        latest_known_ball_state: &Option<(Time, BallState)>,
        filtered_whistle: &FilteredWhistle,
        current_ball_state: &Option<BallState>,
    ) -> FilteredGameControllerState {
        let (new_own_penalties_last_cycle, new_opponent_penalties_last_cycle) = self
            .last_game_controller_state
            .as_ref()
            .map(|last| {
                (
                    penalty_diff(last.penalties, game_controller_state.penalties),
                    penalty_diff(
                        last.opponent_penalties,
                        game_controller_state.opponent_penalties,
                    ),
                )
            })
            .unwrap_or_default();

        let did_receive_motion_in_set_penalty = new_own_penalties_last_cycle
            .iter()
            .chain(new_opponent_penalties_last_cycle.iter())
            .any(|(_, penalty)| matches!(penalty, Penalty::MotionInSet { .. }));

        let fake_detected_free_kick_kicking_team = None;

        let latest_ball_state = latest_known_ball_state.filter(|(ball_state_time, _)| {
            let is_not_in_penalty_kick =
                game_controller_state.sub_state != Some(SubState::PenaltyKick);

            if is_not_in_penalty_kick
                && now.duration_since(*ball_state_time) > parameters.duration_to_keep_observed_ball
            {
                return false;
            }
            true
        });

        let kicking_team = self.find_kicking_team(
            &now,
            parameters,
            game_controller_state,
            &latest_ball_state,
            &new_own_penalties_last_cycle,
            &new_opponent_penalties_last_cycle,
            // detected_free_kick_kicking_team,
            fake_detected_free_kick_kicking_team,
            filtered_whistle,
        );

        let game_states = self.filter_game_states(
            &now,
            parameters,
            field_dimensions,
            player_number,
            game_controller_state,
            current_ball_state,
            filtered_whistle,
            // visual_referee_proceed_to_ready,
            did_receive_motion_in_set_penalty,
            kicking_team,
        );

        FilteredGameControllerState {
            game_state: game_states.own,
            opponent_game_state: game_states.opponent,
            remaining_time_in_half: game_controller_state.remaining_time_in_half,
            game_phase: game_controller_state.game_phase,
            kicking_team,
            penalties: game_controller_state.penalties,
            remaining_number_of_messages: game_controller_state
                .hulks_team
                .remaining_amount_of_messages,
            sub_state: game_controller_state.sub_state,
            global_field_side: game_controller_state.global_field_side,
            new_own_penalties_last_cycle,
            new_opponent_penalties_last_cycle,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn filter_game_states(
        &mut self,
        now: &Time,
        parameters: &GameStateFilterParameters,
        field_dimensions: &FieldDimensions,
        player_number: &PlayerNumber,
        game_controller_state: &GameControllerState,
        current_ball_state: &Option<BallState>,
        filtered_whistle: &FilteredWhistle,
        did_receive_motion_in_set_penalty: bool,
        filtered_kicking_team: Option<Team>,
    ) -> FilteredGameStates {
        let ball_detected_far_from_any_goal = ball_detected_far_from_any_goal(
            current_ball_state,
            field_dimensions,
            parameters.whistle_acceptance_goal_distance,
        );
        self.state = next_filtered_state(
            now,
            self.state,
            game_controller_state,
            filtered_whistle.is_detected,
            parameters,
            ball_detected_far_from_any_goal,
            did_receive_motion_in_set_penalty,
        );
        self.opponent_state = next_filtered_state(
            now,
            self.opponent_state,
            game_controller_state,
            filtered_whistle.is_detected,
            parameters,
            ball_detected_far_from_any_goal,
            did_receive_motion_in_set_penalty,
        );

        if let State::WhistleInSet { .. } = self.state
            && self.whistle_in_set_ball_position.is_none()
            && let Some(ball_state) = current_ball_state.as_ref()
        {
            self.whistle_in_set_ball_position = Some(ball_state.ball_in_field);
        };

        let motion_in_set = matches!(
            game_controller_state.penalties[*player_number],
            Some(Penalty::MotionInSet { .. })
        );
        if matches!(self.state, State::Playing) || motion_in_set {
            self.whistle_in_set_ball_position = None;
        }

        let ball_detected_far_from_kick_off_point = current_ball_state
            .map(|ball| {
                let current_ball_position = ball.ball_in_field;
                let reference_position = self.whistle_in_set_ball_position.unwrap_or_default();
                distance(reference_position, current_ball_position)
                    > parameters.distance_to_consider_ball_moved_in_kick_off
            })
            .unwrap_or(false);

        let filtered_game_state = self.state.construct_filtered_game_state_for_team(
            now,
            game_controller_state,
            Team::Hulks,
            ball_detected_far_from_kick_off_point,
            parameters,
            filtered_kicking_team,
        );

        let filtered_opponent_game_state =
            self.opponent_state.construct_filtered_game_state_for_team(
                now,
                game_controller_state,
                Team::Opponent,
                ball_detected_far_from_kick_off_point,
                parameters,
                filtered_kicking_team,
            );

        FilteredGameStates {
            own: filtered_game_state,
            opponent: filtered_opponent_game_state,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn find_kicking_team(
        &mut self,
        now: &Time,
        parameters: &GameStateFilterParameters,
        game_controller_state: &GameControllerState,
        latest_ball_state: &Option<(Time, BallState)>,
        new_own_penalties_last_cycle: &HashMap<PlayerNumber, Penalty>,
        new_opponent_penalties_last_cycle: &HashMap<PlayerNumber, Penalty>,
        detected_free_kick_kicking_team: Option<Team>,
        filtered_whistle: &FilteredWhistle,
    ) -> Option<Team> {
        if let Some(kicking_team) = game_controller_state.kicking_team {
            return Some(kicking_team);
        }

        let ball_is_in_opponent_half = latest_ball_state
            .map(|(_, ball_state)| ball_state.ball_in_field.x().is_sign_positive());

        if !new_own_penalties_last_cycle.is_empty() {
            self.last_time_hulk_was_penalized = Some(*now);
        }

        if self
            .last_time_hulk_was_penalized
            .is_some_and(|last_time_hulk_was_penalized| {
                now.duration_since(last_time_hulk_was_penalized)
                    > parameters.duration_to_keep_new_penalties
            })
        {
            self.last_time_hulk_was_penalized = None;
        }

        if !new_opponent_penalties_last_cycle.is_empty() {
            self.last_time_opponent_was_penalized = Some(*now);
        }

        if self
            .last_time_opponent_was_penalized
            .is_some_and(|last_time_opponent_was_penalized| {
                now.duration_since(last_time_opponent_was_penalized)
                    > parameters.duration_to_keep_new_penalties
            })
        {
            self.last_time_opponent_was_penalized = None;
        }

        match game_controller_state {
            GameControllerState {
                sub_state: Some(SubState::CornerKick),
                ..
            } if ball_is_in_opponent_half? => Some(Team::Hulks),
            GameControllerState {
                sub_state: Some(SubState::CornerKick),
                ..
            } if !ball_is_in_opponent_half? => Some(Team::Opponent),
            GameControllerState {
                sub_state: Some(SubState::GoalKick),
                ..
            } if ball_is_in_opponent_half? => Some(Team::Opponent),
            GameControllerState {
                sub_state: Some(SubState::GoalKick),
                ..
            } if !ball_is_in_opponent_half? => Some(Team::Hulks),
            GameControllerState {
                sub_state: Some(SubState::PenaltyKick),
                ..
            } if ball_is_in_opponent_half? => Some(Team::Hulks),
            GameControllerState {
                sub_state: Some(SubState::PenaltyKick),
                ..
            } if !ball_is_in_opponent_half? => Some(Team::Opponent),
            GameControllerState {
                sub_state: Some(SubState::DirectFreeKick),
                ..
            } if self.last_time_hulk_was_penalized.is_some() => Some(Team::Opponent),
            GameControllerState {
                sub_state: Some(SubState::DirectFreeKick), //TODO: CHeck if direct free kick is right an no other substate
                ..
            } if self.last_time_opponent_was_penalized.is_some() => Some(Team::Hulks),
            GameControllerState {
                sub_state: Some(SubState::ThrowIn),
                ..
            } if detected_free_kick_kicking_team.is_some() => detected_free_kick_kicking_team,
            GameControllerState {
                game_state: GameState::Playing,
                sub_state: None,
                ..
            } => match (filtered_whistle.is_detected, ball_is_in_opponent_half?) {
                (true, false) => Some(Team::Opponent),
                (true, true) => Some(Team::Hulks),
                _ => None,
            },
            _ => None,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn next_filtered_state(
    now: &Time,
    current_state: State,
    game_controller_state: &GameControllerState,
    is_whistle_detected: bool,
    parameters: &GameStateFilterParameters,
    ball_detected_far_from_any_goal: bool,
    did_receive_motion_in_set_penalty: bool,
) -> State {
    match (
        current_state,
        game_controller_state.game_state,
        game_controller_state.stopped,
    ) {
        (_, _, true) => State::Stop,
        (State::Stop, game_state, false) => State::from_game_state(game_state),
        (State::Finished, GameState::Initial, _) => State::Initial,
        (State::Finished, _, _) => match game_controller_state.game_phase {
            GamePhase::PenaltyShootout { .. } => State::Set,
            _ => State::Finished,
        },
        (
            State::TentativeFinished {
                time_when_finished_clicked,
            },
            GameState::Finished,
            _,
        ) if now.duration_since(time_when_finished_clicked)
            >= parameters.tentative_finish_duration =>
        {
            State::Finished
        }
        (
            State::TentativeFinished {
                time_when_finished_clicked,
            },
            GameState::Finished,
            _,
        ) => State::TentativeFinished {
            time_when_finished_clicked,
        },
        (State::TentativeFinished { .. }, game_state, _) => State::from_game_state(game_state),
        (_, GameState::Finished, _) => State::TentativeFinished {
            time_when_finished_clicked: *now,
        },
        (State::Initial | State::Ready, _, _)
        | (State::Set, GameState::Initial | GameState::Ready | GameState::Playing, _)
        | (
            State::WhistleInSet { .. },
            GameState::Initial | GameState::Ready | GameState::Playing,
            _,
        )
        | (State::Playing, GameState::Initial | GameState::Ready | GameState::Set, _)
        | (
            State::WhistleInPlaying { .. },
            GameState::Initial | GameState::Ready | GameState::Set,
            _,
        ) => State::from_game_state(game_controller_state.game_state),
        (State::Set, GameState::Set, _) => {
            if is_whistle_detected {
                State::WhistleInSet {
                    time_when_whistle_was_detected: *now,
                }
            } else {
                State::Set
            }
        }
        (State::WhistleInSet { .. }, GameState::Set, _) if did_receive_motion_in_set_penalty => {
            State::Set
        }
        (
            State::WhistleInSet {
                time_when_whistle_was_detected,
            },
            GameState::Set,
            _,
        ) => {
            if now.duration_since(time_when_whistle_was_detected)
                < parameters.playing_message_delay + parameters.game_controller_controller_delay
            {
                State::WhistleInSet {
                    time_when_whistle_was_detected,
                }
            } else {
                State::Playing
            }
        }
        (State::Playing, GameState::Playing, _) => {
            if is_whistle_detected && !ball_detected_far_from_any_goal {
                State::WhistleInPlaying {
                    time_when_whistle_was_detected: *now,
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
            _,
        ) => {
            if now.duration_since(time_when_whistle_was_detected)
                < parameters.ready_message_delay + parameters.game_controller_controller_delay
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
    maybe_ball_state: &Option<BallState>,
    field_dimensions: &FieldDimensions,
    whistle_acceptance_goal_distance: Vector2<Field>,
) -> bool {
    match maybe_ball_state {
        Some(ball_state) => {
            ball_state.ball_in_field.x().abs()
                < field_dimensions.length / 2.0 - whistle_acceptance_goal_distance.x()
                || ball_state.ball_in_field.y().abs()
                    > field_dimensions.goal_inner_width / 2.0 + whistle_acceptance_goal_distance.y()
        }
        None => false,
    }
}

fn penalty_diff(
    last: Players<Option<Penalty>>,
    current: Players<Option<Penalty>>,
) -> HashMap<PlayerNumber, Penalty> {
    let current_penalties = current
        .iter()
        .fold(HashMap::new(), |mut map, (player, penalty)| {
            if let Some(penalty) = penalty {
                map.insert(player, *penalty);
            }
            map
        });
    last.iter()
        .fold(current_penalties, |mut map, (player, penalty)| {
            if penalty.is_some() {
                map.remove(&player);
            }
            map
        })
}
