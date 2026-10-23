use std::time::Duration;

use coordinate_systems::Field;
use hsl_network_messages::{GamePhase, PlayerNumber, SubState};
use linear_algebra::{Orientation2, Point2, Pose2, point};
use ros_z::time::Time;
use serde::{Deserialize, Serialize};
use types::{
    behavior_tree::Status, fall_detection::Posture, filtered_game_state::FilteredGameState,
    motion_command::OrientationMode, motion_type::MotionType, obstacles::Obstacle,
    players::Players, primary_state::PrimaryState,
};

use crate::{
    action,
    actions::stand,
    behavior_tree::Node,
    condition,
    conditions::is_finishing_striker_kick,
    goalkeeper::{goalkeeper_shot_block_position_subtree, walk_to_goalkeeper_default_position},
    head::look_at_ball_subtree,
    kick::passing::is_pass_receiver,
    node::{Blackboard, ball_timeout},
    selection, sequence, subtree,
    switch_motion_type::switch_motion_type,
    walk::walk_to,
};

// Based on Sonja's replacement_keeper claim election; retain its timing and rank backoff.
const ELECTION_WINDOW: Duration = Duration::from_millis(500);
const NO_CLAIM_TIMEOUT: Duration = Duration::from_secs(2);
const BACKOFF_STEP: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, ros_z::Message)]
enum ReplacementGoalkeeperState {
    #[default]
    Inactive,
    Candidate,
    Active,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ros_z::Message)]
pub struct ReplacementGoalkeeperCoordinator {
    state: ReplacementGoalkeeperState,
    known_replacement_goalkeeper: Option<PlayerNumber>,
    election_started_at: Option<Time>,
    replacement_needed_since: Option<Time>,
}

struct ElectionInput {
    now: Time,
    own_number: PlayerNumber,
    needed: bool,
    available: Players<bool>,
    eligible: bool,
    preferred: Option<PlayerNumber>,
    other_claim: Option<PlayerNumber>,
}

impl ReplacementGoalkeeperCoordinator {
    pub fn keeper(&self) -> Option<PlayerNumber> {
        self.known_replacement_goalkeeper
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state, ReplacementGoalkeeperState::Active)
    }

    pub fn claims_role(&self) -> bool {
        matches!(
            self.state,
            ReplacementGoalkeeperState::Candidate | ReplacementGoalkeeperState::Active
        )
    }

    pub fn is_electing(&self) -> bool {
        self.election_started_at.is_some() && self.keeper().is_none()
    }

    fn remember(&mut self, winner: PlayerNumber, own_number: PlayerNumber) {
        self.known_replacement_goalkeeper = Some(winner);
        self.state = if winner == own_number {
            ReplacementGoalkeeperState::Active
        } else {
            ReplacementGoalkeeperState::Inactive
        };
        self.election_started_at = None;
    }

    fn update(&mut self, input: ElectionInput) {
        if !input.needed
            || input
                .available
                .iter()
                .filter(|(_, available)| **available)
                .count()
                < 2
        {
            *self = Self::default();
            return;
        }
        if self
            .replacement_needed_since
            .is_some_and(|time| time > input.now)
            || self
                .election_started_at
                .is_some_and(|time| time > input.now)
        {
            *self = Self::default();
        }
        let needed_since = *self.replacement_needed_since.get_or_insert(input.now);
        if self.keeper().is_some_and(|winner| {
            !input.available[winner]
                || (winner != input.own_number && input.other_claim != Some(winner))
        }) {
            self.known_replacement_goalkeeper = None;
            self.state = ReplacementGoalkeeperState::Inactive;
            self.election_started_at = None;
        }
        if let Some(winner) = self.keeper() {
            // Resolve simultaneous claims, but never launch a new challenge to a fresh winner.
            self.remember(
                input.other_claim.map_or(winner, |other| other.min(winner)),
                input.own_number,
            );
            return;
        }
        if matches!(self.state, ReplacementGoalkeeperState::Candidate) && !input.eligible {
            self.state = ReplacementGoalkeeperState::Inactive;
        }
        let rank = input
            .available
            .iter()
            .filter(|(number, available)| **available && *number < input.own_number)
            .count() as u32;
        let fallback_due =
            input.now.duration_since(needed_since) >= NO_CLAIM_TIMEOUT + BACKOFF_STEP * rank;
        if input.eligible
            && input.other_claim.is_none()
            && (input.preferred == Some(input.own_number) || fallback_due)
        {
            self.state = ReplacementGoalkeeperState::Candidate;
        }
        let own_claim = self.claims_role().then_some(input.own_number);
        let Some(winner) = input.other_claim.into_iter().chain(own_claim).min() else {
            self.election_started_at = None;
            return;
        };
        let election_start = *self.election_started_at.get_or_insert(input.now);
        if input.now.duration_since(election_start) >= ELECTION_WINDOW {
            self.remember(winner, input.own_number);
        }
    }
}

pub fn refresh_replacement_goalkeeper_role(blackboard: &mut Blackboard) -> Status {
    let input = election_input(blackboard);
    match input {
        Some(input) => blackboard.replacement_goalkeeper_coordinator.update(input),
        None => blackboard.replacement_goalkeeper_coordinator = Default::default(),
    }
    if blackboard.replacement_goalkeeper_coordinator.is_active() {
        blackboard.ball_action = None;
        blackboard.kick_target = None;
        blackboard.last_closest_to_ball = false;
        blackboard.closest_to_ball_entered_area_since = None;
        blackboard.closest_to_ball_left_area_since = None;
    }
    Status::Success
}

pub(crate) fn has_goal_cover(blackboard: &Blackboard) -> bool {
    let Some(number) = blackboard.replacement_goalkeeper_coordinator.keeper() else {
        return false;
    };
    let world = &blackboard.world_state;
    let field = &blackboard.field_dimensions;
    number != blackboard.parameters.goalkeeper.player_number
        && world.player_states[number]
            .as_ref()
            .is_some_and(|replacement| {
                replacement.time <= world.now
                    && world.now.duration_since(replacement.time)
                        < blackboard
                            .parameters
                            .network
                            .hsl_striker_message_receive_timeout
                    && replacement.inner.replacement_goalkeeper_claim
                    && world
                        .filtered_game_controller_state
                        .as_ref()
                        .is_none_or(|game| game.penalties[number].is_none())
                    && replacement.inner.pose.position().x() >= -field.length / 2.0
                    && (replacement.inner.pose.position() - point![-field.length / 2.0, 0.0]).norm()
                        <= field.goal_inner_width / 2.0 + blackboard.parameters.goalkeeper.x_offset
            })
}

fn election_input(blackboard: &mut Blackboard) -> Option<ElectionInput> {
    let protected_ball_action =
        is_finishing_striker_kick(blackboard) || is_pass_receiver(blackboard);
    let world = &blackboard.world_state;
    let game = world.filtered_game_controller_state.as_ref()?;
    let transform = world.robot.ground_to_field?;
    if world.robot.primary_state != PrimaryState::Playing
        || !matches!(game.game_phase, GamePhase::Normal | GamePhase::Extratime)
        || !matches!(
            game.game_state,
            FilteredGameState::Playing {
                ball_is_free: true,
                kick_off: false
            }
        )
        || game.sub_state == Some(SubState::PenaltyKick)
        || blackboard.remote_control_enabled
        || blackboard
            .parameters
            .control
            .injected_motion_command
            .is_some()
        || blackboard.is_injected_motion_command
        || !world
            .fall_detection
            .is_some_and(|fall| fall.is_valid(world.now) && fall.posture == Posture::Upright)
    {
        return None;
    }
    let own_number = blackboard.player_number;
    let regular_keeper = blackboard.parameters.goalkeeper.player_number;
    let timeout = blackboard
        .parameters
        .network
        .hsl_striker_message_receive_timeout;
    let mut positions = Players::new(None);
    for (number, state) in world.player_states.iter() {
        if game.penalties[number].is_some() {
            continue;
        }
        let pose = if number == own_number {
            Some(transform.as_pose())
        } else {
            state
                .as_ref()
                .filter(|state| {
                    state.time <= world.now && world.now.duration_since(state.time) < timeout
                })
                .map(|state| state.inner.pose)
        };
        positions[number] = pose
            .filter(|pose| {
                pose.position().inner.iter().all(|value| value.is_finite())
                    && pose.orientation().angle().is_finite()
            })
            .map(|pose| pose.position());
    }
    positions[own_number]?;
    let field = blackboard.field_dimensions;
    let offset = blackboard.parameters.goalkeeper.x_offset;
    if ![field.length, field.goal_inner_width, offset]
        .into_iter()
        .all(f32::is_finite)
        || field.length <= 0.0
        || field.goal_inner_width <= 0.0
        || offset < 0.0
    {
        return None;
    }
    let goal = point![-field.length / 2.0, 0.0];
    let return_radius = field.goal_inner_width / 2.0;
    let keeper_radius = return_radius
        + if blackboard
            .replacement_goalkeeper_coordinator
            .replacement_needed_since
            .is_some()
        {
            0.0
        } else {
            offset
        };
    let needed = positions[regular_keeper]
        .is_none_or(|position| position.x() < goal.x() || (position - goal).norm() > keeper_radius);
    let available = positions.map(|position| position.is_some());
    let mut fielders = available;
    fielders[regular_keeper] = false;
    let ball_carrier = blackboard
        .ball
        .as_ref()
        .filter(|ball| {
            ball.age <= world.now
                && world.now.duration_since(ball.age)
                    < ball_timeout(&blackboard.parameters, ball.source)
                && ball.position.inner.iter().all(|value| value.is_finite())
        })
        .and_then(|ball| nearest_player(&positions, ball.position, |_, _| true));
    let preferred = nearest_player(&positions, goal, |number, _| {
        number != regular_keeper && Some(number) != ball_carrier
    });
    let other_claim = world
        .player_states
        .iter()
        .filter_map(|(number, state)| {
            (number != own_number
                && fielders[number]
                && state
                    .as_ref()
                    .is_some_and(|state| state.inner.replacement_goalkeeper_claim))
            .then_some(number)
        })
        .min();
    Some(ElectionInput {
        now: world.now,
        own_number,
        needed,
        available: fielders,
        eligible: fielders[own_number]
            && Some(own_number) != ball_carrier
            && !protected_ball_action,
        preferred,
        other_claim,
    })
}

fn nearest_player(
    positions: &Players<Option<Point2<Field>>>,
    target: Point2<Field>,
    eligible: impl Fn(PlayerNumber, Point2<Field>) -> bool,
) -> Option<PlayerNumber> {
    positions
        .iter()
        .filter_map(|(number, position)| {
            let position = (*position)?;
            eligible(number, position).then_some((number, (position - target).norm_squared()))
        })
        .min_by(
            |(left_number, left_distance), (right_number, right_distance)| {
                left_distance
                    .total_cmp(right_distance)
                    .then(left_number.cmp(right_number))
            },
        )
        .map(|(number, _)| number)
}

pub fn is_active_replacement_goalkeeper(blackboard: &mut Blackboard) -> bool {
    blackboard.replacement_goalkeeper_coordinator.is_active()
}

pub fn replacement_goalkeeper_subtree() -> Node<Blackboard> {
    sequence!(
        subtree!(look_at_ball_subtree),
        selection!(
            sequence!(
                condition!(is_shot_uncovered_by_regular_keeper),
                subtree!(goalkeeper_shot_block_position_subtree),
            ),
            switch_motion_type(
                MotionType::Walk,
                selection!(
                    action!(walk_to_complementary_goal_cover),
                    action!(walk_to_goalkeeper_default_position),
                ),
                action!(stand),
            ),
            action!(stand),
        ),
    )
}

fn regular_keeper_position(blackboard: &Blackboard) -> Option<Point2<Field>> {
    let world = &blackboard.world_state;
    let number = blackboard.parameters.goalkeeper.player_number;
    if world.filtered_game_controller_state.as_ref()?.penalties[number].is_some() {
        return None;
    }
    let keeper = world.player_states[number].as_ref()?;
    let position = keeper.inner.pose.position();
    (keeper.time <= world.now
        && world.now.duration_since(keeper.time)
            < blackboard
                .parameters
                .network
                .hsl_striker_message_receive_timeout
        && position.inner.iter().all(|value| value.is_finite())
        && keeper.inner.pose.angle().is_finite()
        && position.x() >= -blackboard.field_dimensions.length / 2.0)
        .then_some(position)
}

fn is_shot_uncovered_by_regular_keeper(blackboard: &mut Blackboard) -> bool {
    let (Some(keeper), Some(ball)) = (regular_keeper_position(blackboard), &blackboard.ball) else {
        return true;
    };
    if ball.velocity.x() >= 0.0 || keeper.x() >= ball.position.x() {
        return true;
    }
    let shot_y = ball.position.y()
        + ball.velocity.y() * (keeper.x() - ball.position.x()) / ball.velocity.x();
    let body_half_width = blackboard.parameters.walking.path_planning.robot_radius / 2.0;
    (shot_y - keeper.y()).abs() > body_half_width + blackboard.field_dimensions.ball_radius
}

fn complementary_goal_cover_position(blackboard: &Blackboard) -> Option<Point2<Field>> {
    let keeper = regular_keeper_position(blackboard)?;
    let field = blackboard.field_dimensions;
    let goal_x = -field.length / 2.0;
    if keeper.x() > goal_x + field.penalty_area_length
        || keeper.y().abs() > field.penalty_area_width / 2.0
    {
        return None;
    }
    let ball = blackboard.ball.as_ref()?;
    if ball.age > blackboard.world_state.now
        || blackboard.world_state.now.duration_since(ball.age)
            >= ball_timeout(&blackboard.parameters, ball.source)
        || !ball.position.inner.iter().all(|value| value.is_finite())
    {
        return None;
    }
    let radius = blackboard.parameters.walking.path_planning.robot_radius;
    let half_width = field.goal_inner_width / 2.0;
    let defense_x = goal_x + blackboard.parameters.goalkeeper.x_offset;
    let maximum_y = (half_width
        - blackboard
            .parameters
            .goalkeeper
            .distance_to_goalpost
            .max(radius))
    .max(0.0);
    let own_y = blackboard
        .world_state
        .robot
        .ground_to_field?
        .translation()
        .y();
    let preferred_side = if own_y.abs() > radius / 2.0 {
        own_y.signum()
    } else {
        -1.0
    };

    // Project the keeper's body onto the goal and take the larger remaining opening.
    let projection = if ball.position.x() > keeper.x() + radius / 2.0 {
        (ball.position.x() - goal_x) / (ball.position.x() - keeper.x())
    } else {
        1.0
    };
    let shadow_y = ball.position.y() + (keeper.y() - ball.position.y()) * projection;
    let shadow_half_width = radius / 2.0 * projection;
    let lower_end = (shadow_y - shadow_half_width).clamp(-half_width, half_width);
    let upper_start = (shadow_y + shadow_half_width).clamp(-half_width, half_width);
    let lower_width = lower_end + half_width;
    let upper_width = half_width - upper_start;
    let side = if (upper_width - lower_width).abs() <= radius / 2.0 {
        preferred_side
    } else if upper_width > lower_width {
        1.0
    } else {
        -1.0
    };
    let goal_y = if lower_width.max(upper_width) <= f32::EPSILON {
        side * half_width / 2.0
    } else if side < 0.0 {
        (-half_width + lower_end) / 2.0
    } else {
        (upper_start + half_width) / 2.0
    };
    let mut target_y = if ball.position.x() > defense_x {
        ball.position.y()
            + (goal_y - ball.position.y()) * (ball.position.x() - defense_x)
                / (ball.position.x() - goal_x)
    } else {
        goal_y
    };
    let clearance = radius * 1.5;
    let longitudinal_gap = (defense_x - keeper.x()).abs();
    if longitudinal_gap < clearance {
        let lateral_gap = (clearance.powi(2) - longitudinal_gap.powi(2)).sqrt();
        if (target_y - keeper.y()).abs() < lateral_gap {
            target_y = keeper.y() + side * lateral_gap;
        }
    }
    Some(point![defense_x, target_y.clamp(-maximum_y, maximum_y)])
}

fn walk_to_complementary_goal_cover(blackboard: &mut Blackboard) -> Status {
    let (Some(position), Some(keeper), Some(transform), Some(ball)) = (
        complementary_goal_cover_position(blackboard),
        regular_keeper_position(blackboard),
        blackboard.world_state.robot.ground_to_field,
        blackboard.ball.as_ref(),
    ) else {
        return Status::Failure;
    };
    let target = transform.inverse() * position;
    let ball = transform.inverse() * ball.position;
    let pose = Pose2::from_parts(target, Orientation2::from_vector(ball - target));
    let parameters = blackboard.parameters.walking.walk_and_stand;
    let blocking_speed = blackboard.parameters.walking.speed.blocking;
    let radius = blackboard.parameters.walking.path_planning.robot_radius / 2.0;
    let original_obstacles = blackboard.world_state.obstacles.len();
    blackboard.world_state.obstacles.push(Obstacle::robot(
        transform.inverse() * keeper,
        radius,
        radius,
    ));
    blackboard.walk_position = Some(target);
    let status = walk_to(
        blackboard,
        pose,
        blocking_speed,
        OrientationMode::LookAt {
            target: ball,
            tolerance: parameters.orientation_tolerance,
        },
        parameters.normal_distance_to_be_aligned,
        parameters.goalkeeper_hysteresis,
    );
    blackboard
        .world_state
        .obstacles
        .truncate(original_obstacles);
    status
}

#[cfg(test)]
mod tests {
    use hsl_network_messages::Penalty;
    use linear_algebra::{Pose2, vector};
    use types::{
        fall_detection::FallDetection,
        field_dimensions::{FieldDimensions, Side},
        filtered_game_controller_state::FilteredGameControllerState,
        motion_command::BodyMotion,
        path::traits::EndPoints,
        time_wrapper::TimeWrapper,
        world_state::{BallSource, PlayerState},
    };

    use super::*;
    use crate::node::LastBall;

    fn board(number: PlayerNumber) -> Blackboard {
        let now = Time::zero() + Duration::from_secs(10);
        let mut board = Blackboard {
            field_dimensions: FieldDimensions {
                length: 14.0,
                goal_inner_width: 2.4,
                ..FieldDimensions::SPL_2025
            },
            last_motion_type: Some(MotionType::Walk),
            ..Default::default()
        };
        board.parameters.goalkeeper.player_number = PlayerNumber::One;
        board.parameters.goalkeeper.x_offset = 0.3;
        board.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        board.parameters.network.hsl_striker_message_receive_timeout = Duration::from_secs(3);
        board.world_state.now = now;
        board.player_number = number;
        board.world_state.robot.primary_state = PrimaryState::Playing;
        board.world_state.filtered_game_controller_state = Some(FilteredGameControllerState {
            game_state: FilteredGameState::Playing {
                ball_is_free: true,
                kick_off: false,
            },
            ..crate::test_support::filtered_game_controller_state()
        });
        board.world_state.fall_detection = Some(FallDetection {
            time: now,
            valid_until: now + Duration::from_secs(30),
            posture: Posture::Upright,
        });
        for (player, position) in [
            (PlayerNumber::One, point![-4.0, 0.0]),
            (PlayerNumber::Two, point![-5.0, 2.0]),
            (PlayerNumber::Three, point![-2.0, 0.0]),
        ] {
            board.world_state.player_states[player] = Some(TimeWrapper {
                time: now,
                inner: PlayerState {
                    pose: position.into(),
                    ..crate::test_support::player_state()
                },
            });
        }
        board.world_state.robot.ground_to_field = Some(
            board.world_state.player_states[number]
                .as_ref()
                .unwrap()
                .inner
                .pose
                .as_transform(),
        );
        board.ball = Some(LastBall {
            position: point![-2.0, 0.0],
            velocity: vector![0.0, 0.0],
            age: now,
            field_side: Side::Left,
            source: BallSource::Own,
        });
        board
    }

    fn advance(board: &mut Blackboard, duration: Duration) {
        board.world_state.now = board.world_state.now + duration;
        board.ball.as_mut().unwrap().age = board.world_state.now;
        for number in [
            PlayerNumber::One,
            PlayerNumber::Two,
            PlayerNumber::Three,
            PlayerNumber::Four,
            PlayerNumber::Five,
        ] {
            if let Some(state) = &mut board.world_state.player_states[number] {
                state.time = board.world_state.now;
            }
        }
    }

    fn elect(board: &mut Blackboard) {
        refresh_replacement_goalkeeper_role(board);
        advance(board, ELECTION_WINDOW);
        refresh_replacement_goalkeeper_role(board);
    }

    fn cover_board(keeper: Point2<Field>) -> Blackboard {
        let mut board = board(PlayerNumber::Two);
        board.parameters.walking.path_planning.robot_radius = 0.4;
        board.parameters.goalkeeper.distance_to_goalpost = 0.15;
        board.parameters.walking.speed.blocking = 0.8;
        board.parameters.walking.speed.minimum_speed = 0.2;
        board.parameters.walking.speed.velocity_fade_distance = 3.0;
        board.walking_velocity_limits.forward_velocity_limits = [-0.5, 1.0];
        board.walking_velocity_limits.lateral_velocity_limit = 1.0;
        board.walking_velocity_limits.angular_velocity_limit = 2.0;
        board.world_state.robot.ground_to_field =
            Some(Pose2::new(point![-6.7, -0.7], 0.0).as_transform());
        board
            .world_state
            .player_states
            .one
            .as_mut()
            .unwrap()
            .inner
            .pose = keeper.into();
        board.ball.as_mut().unwrap().position = point![0.0, 0.0];
        board.last_ball = board.ball.clone();
        board
    }

    #[test]
    fn near_keeper_cover_mirrors_onto_opposite_goal_opening() {
        let mut targets = Vec::new();
        for side in [-1.0, 1.0] {
            let mut board = cover_board(point![-5.6, side * 0.5]);
            let target = complementary_goal_cover_position(&board).unwrap();
            assert!(target.y() * side < -0.2);
            assert!((target.x() + 6.7).abs() < 1e-5);
            assert!(target.y().abs() <= 0.8);
            let original_obstacles = board.world_state.obstacles.len();
            assert_eq!(
                walk_to_complementary_goal_cover(&mut board),
                Status::Success
            );
            assert_eq!(board.world_state.obstacles.len(), original_obstacles);
            assert!(matches!(board.body_motion, Some(BodyMotion::Walk { .. })));
            targets.push(target);
        }
        assert!((targets[0].y() + targets[1].y()).abs() < 1e-5);
    }

    #[test]
    fn central_keeper_ties_keep_current_side_and_body_clearance() {
        for side in [-1.0, 1.0] {
            let mut board = cover_board(point![-6.7, 0.0]);
            board.world_state.robot.ground_to_field =
                Some(Pose2::new(point![-6.7, side * 0.7], 0.0).as_transform());
            board.ball.as_mut().unwrap().position = point![-6.0, 0.0];
            for noise in [-0.001, 0.0, 0.001] {
                board
                    .world_state
                    .player_states
                    .one
                    .as_mut()
                    .unwrap()
                    .inner
                    .pose = point![-6.7, noise].into();
                let target = complementary_goal_cover_position(&board).unwrap();
                assert!(target.y() * side >= 0.59);
                assert!((target.y() - noise).abs() >= 0.6 - 1e-5);
                assert!(target.y().abs() <= 0.8);
            }
        }
    }

    #[test]
    fn far_or_unavailable_keeper_leaves_full_goal_cover() {
        for case in 0..6 {
            let mut board = cover_board(point![-5.6, 0.5]);
            match case {
                0 => {
                    board
                        .world_state
                        .player_states
                        .one
                        .as_mut()
                        .unwrap()
                        .inner
                        .pose =
                        point![-7.0 + board.field_dimensions.penalty_area_length + 0.1, 0.5].into()
                }
                1 => board.world_state.player_states.one = None,
                2 => board.world_state.player_states.one.as_mut().unwrap().time = Time::zero(),
                3 => {
                    board
                        .world_state
                        .filtered_game_controller_state
                        .as_mut()
                        .unwrap()
                        .penalties
                        .one = Some(Penalty::PickUp {
                        remaining: Duration::from_secs(30),
                    })
                }
                4 => {
                    board
                        .world_state
                        .player_states
                        .one
                        .as_mut()
                        .unwrap()
                        .inner
                        .pose = point![-7.1, 0.0].into()
                }
                _ => {
                    board.world_state.player_states.one.as_mut().unwrap().time =
                        board.world_state.now + Duration::from_secs(1)
                }
            }
            assert!(
                complementary_goal_cover_position(&board).is_none(),
                "case {case}"
            );
            replacement_goalkeeper_subtree().tick_with_trace(&mut board);
            let Some(BodyMotion::Walk { path, .. }) = board.body_motion else {
                panic!("missing or distant keeper must leave normal full-goal coverage");
            };
            let target = board.world_state.robot.ground_to_field.unwrap() * path.end_point();
            assert!((target - point![-6.7, 0.0]).norm() < 1e-5, "case {case}");
        }
    }

    #[test]
    fn fast_shot_prediction_only_yields_when_real_keeper_already_blocks_trajectory() {
        for side in [-1.0, 1.0] {
            let mut board = cover_board(point![-5.6, side * 0.5]);
            board.ball.as_mut().unwrap().position = point![-2.0, side * 0.5];
            board.ball.as_mut().unwrap().velocity = vector![-3.0, 0.0];
            board.last_ball = board.ball.clone();
            assert!(!is_shot_uncovered_by_regular_keeper(&mut board));
            replacement_goalkeeper_subtree().tick_with_trace(&mut board);
            assert!(matches!(board.body_motion, Some(BodyMotion::Walk { .. })));
            let target =
                board.world_state.robot.ground_to_field.unwrap() * board.walk_position.unwrap();
            assert!(target.y() * side < 0.0);

            board.ball.as_mut().unwrap().position = point![-2.0, -side * 0.5];
            board.ball.as_mut().unwrap().velocity = vector![-3.0, side * 0.2];
            board.last_ball = board.ball.clone();
            assert!(is_shot_uncovered_by_regular_keeper(&mut board));
            replacement_goalkeeper_subtree().tick_with_trace(&mut board);
            let target =
                board.world_state.robot.ground_to_field.unwrap() * board.walk_position.unwrap();
            assert!((target.y() - side * (-0.5 + 0.2 * 4.7 / 3.0)).abs() < 1e-5);
            assert!(matches!(
                board.body_motion,
                Some(BodyMotion::WalkWithVelocity { .. })
            ));
            assert!(board.ball_action.is_none());
        }
    }

    #[test]
    fn one_defender_wins_and_conflicting_claims_converge_to_lower_number() {
        let mut defender = board(PlayerNumber::Two);
        let mut striker = board(PlayerNumber::Three);
        refresh_replacement_goalkeeper_role(&mut defender);
        refresh_replacement_goalkeeper_role(&mut striker);
        assert!(defender.replacement_goalkeeper_coordinator.claims_role());
        assert!(!striker.replacement_goalkeeper_coordinator.claims_role());
        striker
            .world_state
            .player_states
            .two
            .as_mut()
            .unwrap()
            .inner
            .replacement_goalkeeper_claim = true;
        advance(&mut defender, ELECTION_WINDOW);
        advance(&mut striker, ELECTION_WINDOW);
        refresh_replacement_goalkeeper_role(&mut defender);
        refresh_replacement_goalkeeper_role(&mut striker);
        advance(&mut striker, ELECTION_WINDOW);
        refresh_replacement_goalkeeper_role(&mut striker);
        assert!(defender.replacement_goalkeeper_coordinator.is_active());
        assert_eq!(
            striker.replacement_goalkeeper_coordinator.keeper(),
            Some(PlayerNumber::Two)
        );
        assert!(!striker.replacement_goalkeeper_coordinator.is_active());

        // A disconnected election can have two winners until the first claims arrive.
        striker
            .replacement_goalkeeper_coordinator
            .remember(PlayerNumber::Three, PlayerNumber::Three);
        defender
            .world_state
            .player_states
            .three
            .as_mut()
            .unwrap()
            .inner
            .replacement_goalkeeper_claim = true;
        refresh_replacement_goalkeeper_role(&mut defender);
        refresh_replacement_goalkeeper_role(&mut striker);
        assert!(defender.replacement_goalkeeper_coordinator.is_active());
        assert!(!striker.replacement_goalkeeper_coordinator.claims_role());
        assert_eq!(
            striker.replacement_goalkeeper_coordinator.keeper(),
            Some(PlayerNumber::Two)
        );

        defender.ball.as_mut().unwrap().position = point![-5.0, 2.0];
        defender.ball_action = Some(crate::kick::strategy::BallAction::Dribble);
        defender.kick_target = Some(point![1.0, 0.0]);
        defender.last_closest_to_ball = true;
        refresh_replacement_goalkeeper_role(&mut defender);
        assert!(defender.replacement_goalkeeper_coordinator.is_active());
        assert!(defender.ball_action.is_none());
        assert!(defender.kick_target.is_none());
        assert!(!defender.last_closest_to_ball);
    }

    #[test]
    fn fresh_existing_claim_is_stable_and_invalid_winners_are_replaced() {
        for invalidation in 0..3 {
            let mut board = board(PlayerNumber::Two);
            board
                .world_state
                .player_states
                .three
                .as_mut()
                .unwrap()
                .inner
                .replacement_goalkeeper_claim = true;
            elect(&mut board);
            assert_eq!(
                board.replacement_goalkeeper_coordinator.keeper(),
                Some(PlayerNumber::Three)
            );
            assert!(!board.replacement_goalkeeper_coordinator.claims_role());
            refresh_replacement_goalkeeper_role(&mut board);
            assert_eq!(
                board.replacement_goalkeeper_coordinator.keeper(),
                Some(PlayerNumber::Three)
            );

            board.world_state.player_states.four = Some(TimeWrapper {
                time: board.world_state.now,
                inner: PlayerState {
                    pose: point![1.0, 1.0].into(),
                    ..crate::test_support::player_state()
                },
            });
            match invalidation {
                0 => board.world_state.player_states.three.as_mut().unwrap().time = Time::zero(),
                1 => {
                    board
                        .world_state
                        .filtered_game_controller_state
                        .as_mut()
                        .unwrap()
                        .penalties
                        .three = Some(Penalty::PickUp {
                        remaining: Duration::from_secs(30),
                    })
                }
                _ => {
                    board
                        .world_state
                        .player_states
                        .three
                        .as_mut()
                        .unwrap()
                        .inner
                        .replacement_goalkeeper_claim = false
                }
            }
            refresh_replacement_goalkeeper_role(&mut board);
            assert!(
                board.replacement_goalkeeper_coordinator.keeper().is_none(),
                "case {invalidation}"
            );
            assert!(
                board.replacement_goalkeeper_coordinator.claims_role(),
                "case {invalidation}"
            );
            board.world_state.now = board.world_state.now + ELECTION_WINDOW;
            board.ball.as_mut().unwrap().age = board.world_state.now;
            refresh_replacement_goalkeeper_role(&mut board);
            assert!(
                board.replacement_goalkeeper_coordinator.is_active(),
                "case {invalidation}"
            );
        }
    }

    #[test]
    fn keeper_excursion_and_return_have_hysteresis_and_use_configured_number() {
        let mut board = board(PlayerNumber::Two);
        board.parameters.goalkeeper.player_number = PlayerNumber::Three;
        board.ball.as_mut().unwrap().position = point![-4.0, 0.0];
        let set_keeper = |board: &mut Blackboard, x| {
            board
                .world_state
                .player_states
                .three
                .as_mut()
                .unwrap()
                .inner
                .pose = point![x, 0.0].into();
        };
        set_keeper(&mut board, -5.6);
        refresh_replacement_goalkeeper_role(&mut board);
        assert!(!board.replacement_goalkeeper_coordinator.claims_role());
        set_keeper(&mut board, -5.4);
        elect(&mut board);
        assert!(board.replacement_goalkeeper_coordinator.is_active());
        set_keeper(&mut board, -5.6);
        refresh_replacement_goalkeeper_role(&mut board);
        assert!(board.replacement_goalkeeper_coordinator.is_active());
        set_keeper(&mut board, -5.9);
        refresh_replacement_goalkeeper_role(&mut board);
        assert!(board.replacement_goalkeeper_coordinator.keeper().is_none());
        set_keeper(&mut board, -7.1);
        elect(&mut board);
        assert!(board.replacement_goalkeeper_coordinator.is_active());
    }

    #[test]
    fn chasing_regular_keeper_counts_as_ball_carrier_and_own_pose_needs_no_echo() {
        let mut board = board(PlayerNumber::Two);
        board.ball.as_mut().unwrap().position = point![-4.0, 0.0];
        board.world_state.player_states.two = None;
        elect(&mut board);
        assert!(board.replacement_goalkeeper_coordinator.is_active());
        assert_eq!(
            board.replacement_goalkeeper_coordinator.keeper(),
            Some(PlayerNumber::Two)
        );

        board.replacement_goalkeeper_coordinator = Default::default();
        board.ball.as_mut().unwrap().position = point![-5.0, 2.0];
        refresh_replacement_goalkeeper_role(&mut board);
        assert!(!board.replacement_goalkeeper_coordinator.claims_role());
    }

    #[test]
    fn no_claim_fallback_preserves_rank_backoff_and_election_window() {
        for (number, rank) in [(PlayerNumber::Two, 0), (PlayerNumber::Three, 1)] {
            let mut coordinator = ReplacementGoalkeeperCoordinator::default();
            let input = |elapsed| ElectionInput {
                now: Time::zero() + elapsed,
                own_number: number,
                needed: true,
                available: Players {
                    two: true,
                    three: true,
                    ..Players::new(false)
                },
                eligible: true,
                preferred: None,
                other_claim: None,
            };
            let delay = NO_CLAIM_TIMEOUT + BACKOFF_STEP * rank;
            coordinator.update(input(Duration::ZERO));
            coordinator.update(input(delay - Duration::from_millis(1)));
            assert!(!coordinator.claims_role());
            coordinator.update(input(delay));
            assert!(coordinator.claims_role());
            assert!(coordinator.is_electing());
            assert!(!coordinator.is_active());
            coordinator.update(input(delay + ELECTION_WINDOW));
            assert!(coordinator.is_active());
        }
    }

    #[test]
    fn last_field_player_and_unsafe_or_setup_states_drop_local_role() {
        for case in 0..10 {
            let mut board = board(PlayerNumber::Two);
            elect(&mut board);
            assert!(board.replacement_goalkeeper_coordinator.is_active());
            match case {
                0 => board.world_state.player_states.three = None,
                1 => board.remote_control_enabled = true,
                2 => board.world_state.fall_detection.as_mut().unwrap().posture = Posture::Falling,
                3 => board.world_state.filtered_game_controller_state = None,
                4 => board.world_state.robot.ground_to_field = None,
                5 => board.world_state.robot.primary_state = PrimaryState::Penalized,
                6 => {
                    board
                        .world_state
                        .filtered_game_controller_state
                        .as_mut()
                        .unwrap()
                        .sub_state = Some(SubState::PenaltyKick)
                }
                7 => {
                    board
                        .world_state
                        .filtered_game_controller_state
                        .as_mut()
                        .unwrap()
                        .game_state = FilteredGameState::Ready
                }
                8 => {
                    board
                        .world_state
                        .filtered_game_controller_state
                        .as_mut()
                        .unwrap()
                        .game_state = FilteredGameState::Playing {
                        ball_is_free: true,
                        kick_off: true,
                    }
                }
                _ => {
                    board.parameters.control.injected_motion_command =
                        Some(types::motion_command::MotionCommand::Damping)
                }
            }
            refresh_replacement_goalkeeper_role(&mut board);
            assert!(
                !board.replacement_goalkeeper_coordinator.claims_role(),
                "case {case}"
            );
            assert!(
                board.replacement_goalkeeper_coordinator.keeper().is_none(),
                "case {case}"
            );
        }
    }

    #[test]
    fn cover_position_never_chases_or_kicks_a_nearby_ball() {
        let mut board = board(PlayerNumber::Two);
        let transform = Pose2::new(point![-5.0, 2.0], 0.0).as_transform();
        board.world_state.robot.ground_to_field = Some(transform);
        board.ball.as_mut().unwrap().position = point![-4.7, 2.0];
        let (status, _) = replacement_goalkeeper_subtree().tick_with_trace(&mut board);
        assert_eq!(status, Status::Success);
        let Some(BodyMotion::Walk { path, .. }) = board.body_motion else {
            panic!("cover must walk toward its goal instead of taking the nearby ball");
        };
        assert!(board.ball_action.is_none());
        let target = transform * path.end_point();
        assert!((target.x() + 6.7).abs() < 1e-5);
        assert!(target.y().abs() < board.field_dimensions.goal_inner_width / 2.0);
    }
}
