use std::time::Duration;

use coordinate_systems::{Field, Ground};
use filtering::hysteresis::less_than_with_hysteresis;
use hsl_network_messages::Team;
use linear_algebra::{Point2, Vector2, point, vector};
use types::{
    controller_input::Button,
    fall_detection::Posture,
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState,
    motion_type::MotionType,
    obstacles::{Obstacle, ObstacleKind},
    primary_state::PrimaryState,
    tracked_objects::{HumanoidRole, HumanoidTrack, TrackStatus},
};
use voronoi::Ownership;

use crate::{
    goalkeeper::goalkeeper_clearance_target_for_pose,
    kick::{
        kick_ball_position_in_ground,
        passing::{is_pass_receiver, outgoing_pass_in_flight},
    },
    node::{Blackboard, ball_timeout},
    substates::is_in_sub_state,
};

pub fn should_yield_goalkeeper_clearance(blackboard: &mut Blackboard) -> bool {
    let keeper_number = blackboard.parameters.goalkeeper.player_number;
    if blackboard.player_number == keeper_number
        || blackboard.last_motion_type == Some(MotionType::Kick)
        || is_in_sub_state(blackboard)
        || is_pass_receiver(blackboard)
    {
        return false;
    }
    goalkeeper_clearance_direction(blackboard).is_some()
}

pub(crate) fn goalkeeper_clearance_direction(blackboard: &Blackboard) -> Option<Vector2<Field>> {
    let world = &blackboard.world_state;
    let keeper_number = blackboard.parameters.goalkeeper.player_number;
    let ball = blackboard.ball.as_ref()?;
    let transform = world.robot.ground_to_field?;
    let keeper = world.player_states[keeper_number].as_ref()?;
    let field = &blackboard.field_dimensions;
    let passing = &blackboard.parameters.kicking.strategy.passing;
    let keeper_position = keeper.inner.pose.position();
    if ball.age > world.now
        || world.now.duration_since(ball.age) >= ball_timeout(&blackboard.parameters, ball.source)
        || keeper.time > world.now
        || world.now.duration_since(keeper.time) > passing.maximum_receiver_age
        || world
            .filtered_game_controller_state
            .as_ref()
            .is_some_and(|game| game.penalties[keeper_number].is_some())
        || !ball
            .position
            .inner
            .iter()
            .chain(ball.velocity.inner.iter())
            .chain(keeper_position.inner.iter())
            .chain(transform.translation().inner.iter())
            .all(|value| value.is_finite())
        || ball.position.x() < -field.length / 2.0
        || ball.position.x() >= 0.0
        || ball.position.y().abs() >= field.width / 2.0
        || keeper_position.x() < -field.length / 2.0
        || keeper_position.x() >= -field.length / 2.0 + field.penalty_area_length
        || keeper_position.y().abs() >= field.penalty_area_width / 2.0
    {
        return None;
    }
    // An addressed pass has an explicit source; do not infer a keeper clearance instead.
    if world.player_states.iter().any(|(number, state)| {
        let Some(sender) = state.as_ref() else {
            return false;
        };
        let sender_to_ball = ball.position - sender.inner.pose.position();
        let follows_field_pass =
            ball.velocity
                .try_normalize(f32::EPSILON)
                .is_some_and(|direction| {
                    sender_to_ball.dot(&direction)
                        >= -blackboard.parameters.kicking.kick_activation_distance
                });
        number != blackboard.player_number
            && (number == keeper_number || follows_field_pass)
            && sender.time <= world.now
            && world.now.duration_since(sender.time) < hsl_network_messages::PassIntent::TIMEOUT
            && !world
                .filtered_game_controller_state
                .as_ref()
                .is_some_and(|game| game.penalties[number].is_some())
            && sender
                .inner
                .pose
                .position()
                .inner
                .iter()
                .all(|value| value.is_finite())
            && sender.inner.pass.is_some_and(|pass| {
                pass.receiver == blackboard.player_number
                    && (number == keeper_number
                        || ball
                            .velocity
                            .dot(&(pass.target - sender.inner.pose.position()))
                            > 0.0)
                    && pass.target.inner.iter().all(|value| value.is_finite())
                    && pass.target.x().abs() < field.length / 2.0
                    && pass.target.y().abs() < field.width / 2.0
                    && (pass.target - sender.inner.pose.position()).norm()
                        <= passing.maximum_distance + passing.receiver_standoff
            })
    }) {
        return None;
    }
    let interception = blackboard.parameters.ball.interception;
    let to_keeper = keeper_position - ball.position;
    let to_robot = transform.as_pose().position() - ball.position;
    let corridor = blackboard.parameters.walking.path_planning.robot_radius
        + field.ball_radius
        + blackboard.parameters.kicking.strategy.obstacle_clearance;
    let moving = ball.velocity.norm() > interception.minimum_ball_velocity;
    if moving && ball.velocity.x() <= 0.0 {
        return None;
    }
    if moving {
        let direction = ball.velocity.normalize();
        // shortcut: infer origin from keeper proximity, replace when clearance intent is shared.
        return (to_keeper.dot(&direction) < 0.0
            && (to_keeper - direction * to_keeper.dot(&direction)).norm() <= corridor
            && to_robot.dot(&direction) > 0.0)
            .then_some(direction);
    }
    let ball_at_keeper = keeper.inner.pose.as_transform::<Ground>().inverse() * ball.position;
    if ball.position.x() >= -field.length / 2.0 + field.penalty_area_length
        || ball.position.y().abs() >= field.penalty_area_width / 2.0
        || ball_at_keeper.x() <= 0.0
        || ball_at_keeper.coords().norm()
            >= blackboard
                .parameters
                .goalkeeper
                .kick_away_ball_maximum_robot_distance
    {
        return None;
    }
    let direction = (goalkeeper_clearance_target_for_pose(blackboard, keeper.inner.pose)?
        - ball.position)
        .normalize();
    (to_robot.dot(&direction) > 0.0).then_some(direction)
}

pub(crate) fn opponent_near_ball(blackboard: &Blackboard, distance: f32) -> bool {
    let Some(ball) = &blackboard.ball else {
        return false;
    };
    opponent_near_position(blackboard, ball.position, distance)
}

pub(crate) fn opponent_ball_holder(blackboard: &Blackboard) -> Option<Point2<Field>> {
    let world = &blackboard.world_state;
    let ball = blackboard.ball.as_ref()?;
    let transform = world.robot.ground_to_field?;
    let tracks = world.humanoid_tracks.as_ref()?;
    if ball.age > world.now
        || world.now.duration_since(ball.age) > Duration::from_millis(250)
        || world.now.duration_since(ball.age) >= ball_timeout(&blackboard.parameters, ball.source)
        || tracks.time > world.now
        || world.now.duration_since(tracks.time) > Duration::from_millis(250)
        || !ball.velocity.norm().is_finite()
        || ball.velocity.norm()
            > (2.0
                * blackboard
                    .parameters
                    .ball
                    .interception
                    .minimum_ball_velocity)
                .max(0.5)
    {
        return None;
    }
    world
        .obstacles
        .iter()
        .filter(|obstacle| matches!(obstacle.kind, ObstacleKind::Robot | ObstacleKind::Unknown))
        .filter_map(|obstacle| {
            let track = tracked_humanoid(blackboard, obstacle)?;
            if track.role != HumanoidRole::Opponent
                || track.remote_only
                || !track.robot_probability.is_finite()
                || track.robot_probability < 0.5
                || !track.time_since_detection.is_some_and(|age| {
                    age + world.now.duration_since(tracks.time) <= Duration::from_millis(500)
                })
            {
                return None;
            }
            let position = transform * track.position_in_ground;
            let distance = (ball.position - position).norm();
            (distance
                <= blackboard.parameters.kicking.approach_ball_standoff
                    + obstacle.radius_at_foot_height
                    + blackboard.field_dimensions.ball_radius)
                .then_some((distance, position))
        })
        .min_by(|(a, _), (b, _)| a.total_cmp(b))
        .map(|(_, position)| position)
}

pub(crate) fn opponent_shot_direction(blackboard: &Blackboard) -> Option<Vector2<Field>> {
    let holder = opponent_ball_holder(blackboard)?;
    let direction = (blackboard.ball.as_ref()?.position - holder)
        .try_normalize(blackboard.field_dimensions.ball_radius)?;
    // Tracks have no yaw: holder-to-ball geometry is only a forward-shot hint.
    (direction.x() < -0.5).then_some(direction)
}

pub(crate) fn opponent_near_position(
    blackboard: &Blackboard,
    target: Point2<Field>,
    distance: f32,
) -> bool {
    let Some(transform) = blackboard.world_state.robot.ground_to_field else {
        return false;
    };
    enemy_obstacles(blackboard).any(|obstacle| {
        let position = transform * obstacle.position;
        let predicted = tracked_humanoid(blackboard, obstacle)
            .filter(|track| track.velocity_in_ground.inner.iter().all(|v| v.is_finite()))
            .map_or(position, |track| {
                position + transform * track.velocity_in_ground * 0.3
            });
        (position - target).norm().min((predicted - target).norm()) - obstacle.radius_at_foot_height
            <= distance
    })
}

pub(crate) fn enemy_obstacles(blackboard: &Blackboard) -> impl Iterator<Item = &Obstacle> {
    blackboard
        .world_state
        .obstacles
        .iter()
        .filter(move |obstacle| {
            if !matches!(
                obstacle.kind,
                ObstacleKind::Robot | ObstacleKind::Person | ObstacleKind::Unknown
            ) {
                return false;
            }
            if let Some(track) = tracked_humanoid(blackboard, obstacle) {
                return track.role == HumanoidRole::Opponent;
            }
            let Some(transform) = blackboard.world_state.robot.ground_to_field else {
                return false;
            };
            let position = transform * obstacle.position;
            !blackboard
                .world_state
                .player_states
                .iter()
                .any(|(_, teammate)| {
                    teammate.as_ref().is_some_and(|teammate| {
                        teammate.time <= blackboard.world_state.now
                            && blackboard.world_state.now.duration_since(teammate.time)
                                <= blackboard
                                    .parameters
                                    .kicking
                                    .strategy
                                    .passing
                                    .maximum_receiver_age
                            && (position - teammate.inner.pose.position()).norm()
                                <= blackboard.parameters.walking.path_planning.robot_radius
                                    + obstacle.radius_at_foot_height
                    })
                })
        })
}

fn tracked_humanoid<'a>(
    blackboard: &'a Blackboard,
    obstacle: &Obstacle,
) -> Option<&'a HumanoidTrack> {
    let world = &blackboard.world_state;
    let tracks = world.humanoid_tracks.as_ref()?;
    if tracks.time > world.now || world.now.duration_since(tracks.time) > Duration::from_millis(500)
    {
        return None;
    }
    tracks
        .tracks
        .iter()
        .filter(|track| {
            track.status == TrackStatus::Confirmed
                && track.existence >= 0.5
                && track
                    .time_since_detection
                    .is_some_and(|age| age <= Duration::from_secs(1))
                && track.position_in_ground.inner.iter().all(|v| v.is_finite())
        })
        .min_by(|a, b| {
            (a.position_in_ground - obstacle.position)
                .norm_squared()
                .total_cmp(&(b.position_in_ground - obstacle.position).norm_squared())
        })
        .filter(|track| {
            (track.position_in_ground - obstacle.position).norm()
                <= blackboard
                    .parameters
                    .walking
                    .path_planning
                    .robot_radius
                    .max(obstacle.radius_at_foot_height)
        })
}

pub fn is_finishing_striker_kick(blackboard: &mut Blackboard) -> bool {
    let now = blackboard.world_state.now;
    blackboard.last_closest_to_ball
        && !outgoing_pass_in_flight(blackboard)
        && blackboard.last_motion_type == Some(MotionType::Kick)
        && blackboard.last_motion_switch_time <= now
        && now.duration_since(blackboard.last_motion_switch_time) < Duration::from_millis(1500)
        && blackboard.ball.as_ref().is_some_and(|ball| {
            ball.age <= now
                && now.duration_since(ball.age) < ball_timeout(&blackboard.parameters, ball.source)
        })
}

pub fn is_ball_interception_candidate(blackboard: &mut Blackboard) -> bool {
    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.ball,
        &blackboard.world_state.robot.ground_to_field,
    ) {
        let parameters = &blackboard.parameters.ball.interception;

        let ball_in_ground = ground_to_field.inverse() * ball.position;
        let ball_is_in_front_of_robot = ball_in_ground.coords().norm()
            < parameters.maximum_ball_distance
            && ball_in_ground.x() > 0.0;
        let ball_is_moving_towards_robot = (ground_to_field.inverse() * ball.velocity).x()
            < -parameters.minimum_ball_velocity_towards_robot;

        let ball_in_field_velocity = ball.velocity;
        let ball_is_moving = ball_in_field_velocity.norm() > parameters.minimum_ball_velocity;
        let ball_is_moving_towards_own_half =
            ball_in_field_velocity.x() < -parameters.minimum_ball_velocity_towards_own_half;

        ball_is_in_front_of_robot
            && ball_is_moving
            && ball_is_moving_towards_robot
            && ball_is_moving_towards_own_half
    } else {
        false
    }
}

pub fn is_close_to_ball(blackboard: &mut Blackboard) -> bool {
    let mut is_close = false;
    if let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field
        && let Some(ball) = kick_ball_position_in_ground(blackboard, &ground_to_field)
    {
        let distance_to_ball = ball.coords().norm();
        let parameters = &blackboard.parameters.kicking;
        is_close = less_than_with_hysteresis(
            blackboard.last_close_enough_to_kick,
            distance_to_ball,
            parameters.kick_activation_distance,
            parameters.kick_activation_hysteresis,
        );
        blackboard.last_close_enough_to_kick = is_close;
    }

    is_close
}

pub fn is_close_to_ball_aligned(blackboard: &mut Blackboard) -> bool {
    let mut is_close_and_aligned = false;
    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.ball,
        &blackboard.world_state.robot.ground_to_field,
    ) {
        let ball_in_ground = ground_to_field.inverse() * ball.position;
        let goal_position =
            ground_to_field.inverse() * point!(blackboard.field_dimensions.length / 2.0, 0.0);
        let target_kick_position = ball_in_ground
            - (goal_position - ball_in_ground).normalize()
                * blackboard.parameters.kicking.approach_ball_standoff;

        let parameters = &blackboard.parameters.substates;
        let distance_to_ball = target_kick_position.coords().norm();
        let is_close = less_than_with_hysteresis(
            blackboard.last_close_enough_to_kick,
            distance_to_ball,
            parameters.distance_for_kick,
            parameters.distance_for_kick_hysteresis,
        );

        let direction = (goal_position - target_kick_position).normalize();
        let robot_facing_direction = vector!(1.0, 0.0);
        let is_aligned =
            direction.angle(&robot_facing_direction) < parameters.alignment_angle_threshold;

        blackboard.direction_difference = direction.angle(&robot_facing_direction);

        is_close_and_aligned = is_close && (is_aligned || blackboard.last_close_enough_to_kick);
        blackboard.last_close_enough_to_kick = is_close_and_aligned;
    }

    is_close_and_aligned
}

pub fn is_closest_to_ball(blackboard: &mut Blackboard) -> bool {
    if outgoing_pass_in_flight(blackboard)
        || (!is_finishing_striker_kick(blackboard)
            && blackboard.player_number != blackboard.parameters.goalkeeper.player_number
            && teammate_controls_ball(blackboard))
    {
        blackboard.last_closest_to_ball = false;
        blackboard.closest_to_ball_entered_area_since = None;
        blackboard.closest_to_ball_left_area_since = None;
        return false;
    }
    let own_player_number = blackboard.player_number;
    let mut predictive_entry = false;
    let elected_owner =
        if let (Some(ball), Some(voronoi_map)) = (&blackboard.ball, &blackboard.voronoi_map) {
            // Ownership must not depend on each robot's close-range pursuit cutoff.
            let projected = blackboard.ball_pursuit_position(ball.position, ball.velocity);
            predictive_entry = ball.age <= blackboard.world_state.now
                && blackboard.world_state.now.duration_since(ball.age)
                    < ball_timeout(&blackboard.parameters, ball.source)
                && ball.velocity.norm()
                    > blackboard
                        .parameters
                        .ball
                        .interception
                        .minimum_ball_velocity
                && projected != ball.position;
            match voronoi_map.ownership_at(projected) {
                Some(Ownership::Robot(player_number)) => Some(player_number),
                Some(Ownership::Blocked) => {
                    match voronoi_map.nearest_non_blocked_ownership(projected) {
                        Some(Ownership::Robot(player_number)) => Some(player_number),
                        _ => None,
                    }
                }
                _ => None,
            }
        } else {
            None
        };
    let raw_is_closest =
        is_finishing_striker_kick(blackboard) || elected_owner == Some(own_player_number);

    let now = blackboard.world_state.now;
    if raw_is_closest {
        blackboard.closest_to_ball_entered_area_since =
            blackboard.closest_to_ball_entered_area_since.or(Some(now));
        blackboard.closest_to_ball_left_area_since = None;
    } else {
        blackboard.closest_to_ball_left_area_since =
            blackboard.closest_to_ball_left_area_since.or(Some(now));
        blackboard.closest_to_ball_entered_area_since = None;
    }

    let parameters = blackboard.parameters.ball.closest_to_ball;
    let is_closest = if blackboard.last_closest_to_ball {
        raw_is_closest
            // A new elected owner cancels the old forecast without overlapping exit grace.
            || (elected_owner.is_none()
                && blackboard
                    .closest_to_ball_left_area_since
                    .is_some_and(|since| now.duration_since(since) < parameters.exit_duration))
    } else {
        raw_is_closest
            && (predictive_entry
                || is_ball_interception_candidate(blackboard)
                || blackboard.ball.as_ref().is_some_and(|ball| {
                    ball.age <= now
                        && now.duration_since(ball.age)
                            < ball_timeout(&blackboard.parameters, ball.source)
                        && opponent_near_ball(
                            blackboard,
                            blackboard.parameters.kicking.strategy.pressure_distance,
                        )
                })
                || blackboard
                    .closest_to_ball_entered_area_since
                    .is_some_and(|since| now.duration_since(since) >= parameters.enter_duration))
    };

    blackboard.last_closest_to_ball = is_closest;
    is_closest
}

fn teammate_controls_ball(blackboard: &Blackboard) -> bool {
    let world = &blackboard.world_state;
    let (Some(ball), Some(transform)) = (&blackboard.ball, world.robot.ground_to_field) else {
        return false;
    };
    if ball.age > world.now
        || world.now.duration_since(ball.age) >= ball_timeout(&blackboard.parameters, ball.source)
        || !ball
            .position
            .inner
            .iter()
            .chain(transform.translation().inner.iter())
            .all(|value| value.is_finite())
    {
        return false;
    }
    let own_distance = (transform.as_pose().position() - ball.position).norm();
    world.player_states.iter().any(|(number, state)| {
        number != blackboard.player_number
            && number != blackboard.parameters.goalkeeper.player_number
            && Some(number) != blackboard.replacement_goalkeeper_coordinator.keeper()
            && world
                .filtered_game_controller_state
                .as_ref()
                .is_none_or(|game| game.penalties[number].is_none())
            && state.as_ref().is_some_and(|state| {
                let distance = (state.inner.pose.position() - ball.position).norm();
                state.time <= world.now
                    && world.now.duration_since(state.time)
                        < blackboard
                            .parameters
                            .kicking
                            .strategy
                            .passing
                            .maximum_receiver_age
                    && state
                        .inner
                        .pose
                        .position()
                        .inner
                        .iter()
                        .all(|value| value.is_finite())
                    && distance <= blackboard.parameters.kicking.kick_activation_distance
                    && distance + blackboard.parameters.kicking.approach_ball_standoff
                        < own_distance
            })
    })
}

fn posture(blackboard: &Blackboard) -> Option<Posture> {
    blackboard
        .world_state
        .fall_detection
        .filter(|fall_detection| fall_detection.is_valid(blackboard.world_state.now))
        .map(|fall_detection| fall_detection.posture)
}

pub fn should_do_fall_protection(blackboard: &mut Blackboard) -> bool {
    matches!(posture(blackboard), None | Some(Posture::Falling))
}

pub fn is_fallen(blackboard: &mut Blackboard) -> bool {
    matches!(posture(blackboard), Some(Posture::Fallen { .. }))
}

pub fn is_ready_for_stand_up(blackboard: &mut Blackboard) -> bool {
    matches!(
        posture(blackboard),
        Some(
            Posture::Fallen {
                is_ready_for_standup: true
            } | Posture::StandingUp
        )
    )
}

pub fn is_goalkeeper(blackboard: &mut Blackboard) -> bool {
    blackboard.player_number == blackboard.parameters.goalkeeper.player_number
}

pub fn is_primary_state(blackboard: &mut Blackboard, primary_state: PrimaryState) -> bool {
    blackboard.world_state.robot.primary_state == primary_state
}

pub fn is_remote_control_enabled(blackboard: &mut Blackboard) -> bool {
    blackboard.remote_control_enabled
}

pub fn is_controller_connected(blackboard: &mut Blackboard) -> bool {
    blackboard
        .controller_input
        .as_ref()
        .is_some_and(|controller| controller.connected)
}

pub fn is_remote_kick_mode(blackboard: &mut Blackboard, button: Button) -> bool {
    blackboard
        .controller_input
        .as_ref()
        .is_some_and(|controller| controller.is_pressed(button))
}

pub fn has_ball_position(blackboard: &mut Blackboard) -> bool {
    blackboard.ball.is_some()
}

pub fn has_hypothetical_ball_position(blackboard: &mut Blackboard) -> bool {
    !blackboard
        .world_state
        .hypothetical_ball_positions
        .is_empty()
}

pub fn hulks_is_kicking_team(blackboard: &mut Blackboard) -> bool {
    matches!(
        blackboard.world_state.filtered_game_controller_state,
        Some(FilteredGameControllerState {
            kicking_team: Some(Team::Hulks),
            ..
        })
    )
}

pub fn is_ball_free(blackboard: &mut Blackboard) -> bool {
    matches!(
        blackboard.world_state.filtered_game_controller_state,
        Some(FilteredGameControllerState {
            game_state: FilteredGameState::Playing {
                ball_is_free: true,
                ..
            },
            ..
        })
    )
}

pub fn is_kickoff(blackboard: &mut Blackboard) -> bool {
    matches!(
        blackboard.world_state.filtered_game_controller_state,
        Some(FilteredGameControllerState {
            game_state: FilteredGameState::Playing { kick_off: true, .. },
            ..
        })
    )
}

pub fn is_last_hulk_standing(blackboard: &mut Blackboard) -> bool {
    let own_player_number = blackboard.player_number;

    blackboard
        .world_state
        .player_states
        .iter()
        .all(|(player_number, player_state)| {
            player_number == own_player_number || player_state.is_none()
        })
        && blackboard
            .world_state
            .filtered_game_controller_state
            .as_ref()
            .is_none_or(|game| {
                game.penalties
                    .iter()
                    .all(|(number, penalty)| number == own_player_number || penalty.is_some())
            })
}

pub fn is_simple(blackboard: &mut Blackboard) -> bool {
    blackboard.parameters.control.is_simple
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::LastBall;
    use hsl_network_messages::{Penalty, PlayerNumber};
    use types::{
        behavior_tree::{NodeTrace, Status},
        field_dimensions::{FieldDimensions, Side},
        multivariate_normal_distribution::MultivariateNormalDistribution,
        time_wrapper::TimeWrapper,
        tracked_objects::TrackedObjects,
        world_state::{BallSource, PlayerState},
    };

    fn succeeded(trace: &NodeTrace, name: &str) -> bool {
        (trace.name == name && trace.status == Status::Success)
            || trace.children.iter().any(|child| succeeded(child, name))
    }

    #[test]
    fn fresh_opponent_tracks_identify_pressure_and_predict_closing_robots() {
        let mut board = Blackboard::default();
        board.world_state.now = board.world_state.now + Duration::from_secs(10);
        board.world_state.robot.ground_to_field = Some(Default::default());
        board.parameters.walking.path_planning.robot_radius = 0.4;
        board
            .parameters
            .kicking
            .strategy
            .passing
            .maximum_receiver_age = Duration::from_secs(1);
        board.world_state.obstacles = vec![Obstacle::robot(point![1.6, 0.0], 0.2, 0.2)];
        board.world_state.humanoid_tracks = Some(TrackedObjects {
            time: board.world_state.now,
            ground_to_odometry: Default::default(),
            ground_to_field: Some(Default::default()),
            tracks: vec![HumanoidTrack {
                id: 1,
                existence: 1.0,
                status: TrackStatus::Confirmed,
                robot_probability: 1.0,
                human_probability: 0.0,
                role: HumanoidRole::Opponent,
                teammate_probabilities: vec![],
                remote_only: false,
                detection_probability: 1.0,
                time_since_detection: Some(Duration::ZERO),
                state: MultivariateNormalDistribution {
                    mean: nalgebra::SVector::zeros(),
                    covariance: nalgebra::SMatrix::identity(),
                },
                position_in_ground: point![1.6, 0.0],
                velocity_in_ground: vector![-2.1, 0.0],
                position_in_field: Some(point![1.6, 0.0]),
            }],
        });
        let target = Point2::origin();
        assert!(opponent_near_position(&board, target, 0.8));
        board.world_state.humanoid_tracks.as_mut().unwrap().tracks[0].velocity_in_ground =
            vector![2.0, 0.0];
        assert!(!opponent_near_position(&board, target, 0.8));

        // Identified opponents remain threats even beside a teammate's reported position.
        board.world_state.player_states.three = Some(TimeWrapper {
            time: board.world_state.now,
            inner: PlayerState {
                pose: point![1.6, 0.0].into(),
                ..crate::test_support::player_state()
            },
        });
        assert!(opponent_near_position(&board, target, 1.5));
        for role in [
            HumanoidRole::Teammate(PlayerNumber::Three),
            HumanoidRole::Human,
        ] {
            board.world_state.humanoid_tracks.as_mut().unwrap().tracks[0].role = role;
            assert!(!opponent_near_position(&board, target, 1.5));
        }
        board.world_state.player_states.three = None;
        for time in [
            board.world_state.now - Duration::from_secs(1),
            board.world_state.now + Duration::from_secs(1),
        ] {
            board.world_state.humanoid_tracks.as_mut().unwrap().time = time;
            assert!(opponent_near_position(&board, target, 1.5));
        }
        board.world_state.humanoid_tracks.as_mut().unwrap().time = board.world_state.now;
        board.world_state.humanoid_tracks = None;
        assert!(opponent_near_position(&board, target, 1.5));
    }

    #[test]
    fn shot_hint_requires_fresh_observed_opponent_and_forward_holder_geometry() {
        let mut board = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            ..Default::default()
        };
        board.world_state.now = board.world_state.now + Duration::from_secs(10);
        let transform = linear_algebra::Isometry2::from_parts(vector![0.5, -0.7], 0.8);
        board.world_state.robot.ground_to_field = Some(transform);
        board.parameters.walking.path_planning.robot_radius = 0.4;
        board.parameters.kicking.approach_ball_standoff = 0.3;
        board.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        board.parameters.ball.pursuit_lookahead = Duration::from_millis(500);
        board.parameters.ball.interception.minimum_ball_velocity = 0.3;
        board.ball = Some(LastBall {
            position: point![1.3, 0.1],
            velocity: vector![-0.4, 0.0],
            age: board.world_state.now,
            field_side: Side::Left,
            source: BallSource::Own,
        });
        let holder = point![1.6, 0.0];
        board.world_state.obstacles = vec![Obstacle::robot(transform.inverse() * holder, 0.2, 0.2)];
        board.world_state.humanoid_tracks = Some(TrackedObjects {
            time: board.world_state.now,
            ground_to_odometry: Default::default(),
            ground_to_field: Some(transform),
            tracks: vec![HumanoidTrack {
                id: 1,
                existence: 1.0,
                status: TrackStatus::Confirmed,
                robot_probability: 1.0,
                human_probability: 0.0,
                role: HumanoidRole::Opponent,
                teammate_probabilities: vec![],
                remote_only: false,
                detection_probability: 1.0,
                time_since_detection: Some(Duration::ZERO),
                state: MultivariateNormalDistribution {
                    mean: nalgebra::SVector::zeros(),
                    covariance: nalgebra::SMatrix::identity(),
                },
                position_in_ground: transform.inverse() * holder,
                velocity_in_ground: Vector2::zeros(),
                position_in_field: Some(holder),
            }],
        });
        let ball = board.ball.as_ref().unwrap();
        approx::assert_relative_eq!(
            opponent_shot_direction(&board).unwrap(),
            (ball.position - holder).normalize(),
            epsilon = 1e-5
        );
        assert_eq!(
            board.ball_pursuit_position(ball.position, ball.velocity),
            ball.position
        );

        let mut keeper = board.clone();
        keeper.field_dimensions.length = 22.0;
        keeper.field_dimensions.width = 14.0;
        keeper.field_dimensions.goal_inner_width = 2.4;
        keeper.field_dimensions.ball_radius = 0.105;
        keeper.parameters.goalkeeper.player_number = PlayerNumber::One;
        keeper.parameters.goalkeeper.x_offset = 0.3;
        keeper.parameters.goalkeeper.distance_to_goalpost = 0.15;
        keeper.player_number = PlayerNumber::One;
        keeper.last_motion_type = Some(MotionType::Walk);
        keeper.ball.as_mut().unwrap().position = point![1.3, -0.02];
        keeper.ball.as_mut().unwrap().velocity = Vector2::zeros();
        keeper.last_ball = keeper.ball.clone();
        let tree = crate::goalkeeper::goalkeeper_subtree();
        let (status, trace) = tree.tick_with_trace(&mut keeper);
        assert_eq!(status, Status::Success);
        assert!(succeeded(
            &trace,
            "walk_to_goalkeeper_pre_shot_block_position"
        ));
        assert!(!succeeded(&trace, "set_goalkeeper_shot_block_position"));
        let anticipated = transform * keeper.walk_position.unwrap();
        approx::assert_relative_eq!(anticipated, point![-10.7, -0.82], epsilon = 1e-4);

        keeper.ball.as_mut().unwrap().velocity = vector![-3.0, 0.1];
        keeper.last_ball = keeper.ball.clone();
        let (status, trace) = tree.tick_with_trace(&mut keeper);
        assert_eq!(status, Status::Success);
        assert!(succeeded(&trace, "set_goalkeeper_shot_block_position"));
        assert!(!succeeded(
            &trace,
            "walk_to_goalkeeper_pre_shot_block_position"
        ));
        let measured = transform * keeper.walk_position.unwrap();
        approx::assert_relative_eq!(measured, point![-10.7, 0.38], epsilon = 1e-4);

        for role in [
            HumanoidRole::Human,
            HumanoidRole::Teammate(PlayerNumber::Two),
        ] {
            let mut invalid = board.clone();
            invalid.world_state.humanoid_tracks.as_mut().unwrap().tracks[0].role = role;
            assert!(opponent_shot_direction(&invalid).is_none());
            assert!(opponent_ball_holder(&invalid).is_none());
        }
        for age in [Duration::from_millis(501), Duration::from_secs(1)] {
            let mut invalid = board.clone();
            invalid.world_state.humanoid_tracks.as_mut().unwrap().tracks[0].time_since_detection =
                Some(age);
            assert!(opponent_shot_direction(&invalid).is_none());
        }
        for time in [
            board.world_state.now - Duration::from_millis(251),
            board.world_state.now + Duration::from_millis(1),
        ] {
            let mut invalid = board.clone();
            invalid.world_state.humanoid_tracks.as_mut().unwrap().time = time;
            assert!(opponent_shot_direction(&invalid).is_none());
        }
        let mut invalid = board.clone();
        invalid.world_state.humanoid_tracks.as_mut().unwrap().tracks[0].remote_only = true;
        assert!(opponent_shot_direction(&invalid).is_none());
        invalid = board.clone();
        invalid.world_state.humanoid_tracks = None;
        assert!(opponent_shot_direction(&invalid).is_none());
        for age in [Duration::from_millis(251), Duration::from_secs(1)] {
            let mut invalid = board.clone();
            invalid.ball.as_mut().unwrap().age = invalid.world_state.now - age;
            assert!(opponent_shot_direction(&invalid).is_none());
        }
        board.ball.as_mut().unwrap().velocity = vector![-3.0, 0.0];
        assert!(opponent_ball_holder(&board).is_none());
        board.ball.as_mut().unwrap().velocity = Vector2::zeros();
        board.ball.as_mut().unwrap().position = point![1.9, 0.1];
        assert!(opponent_ball_holder(&board).is_some());
        assert!(opponent_shot_direction(&board).is_none());
    }

    #[test]
    fn near_and_distant_robots_elect_the_same_interceptor() {
        let mut board = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            ..Default::default()
        };
        let sites = [
            (
                linear_algebra::Pose2::new(point![0.0, 0.0], 0.0),
                PlayerNumber::Two,
            ),
            (
                linear_algebra::Pose2::new(point![0.9, 0.0], 0.0),
                PlayerNumber::Three,
            ),
        ];
        let mut map = voronoi::VoronoiGrid::new(point![-4.5, -3.0], point![4.5, 3.0], 0.1);
        map.multi_source_dijkstra(&sites);
        board.voronoi_map = Some(map);
        board.parameters.goalkeeper.player_number = PlayerNumber::One;
        board.parameters.ball.pursuit_lookahead = Duration::from_millis(500);
        board.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        board.ball = Some(LastBall {
            position: point![1.2, 0.0],
            velocity: vector![-2.0, 0.0],
            age: board.world_state.now,
            field_side: Side::Left,
            source: BallSource::Own,
        });
        let mut owners = Vec::new();
        for (pose, player) in sites {
            let mut peer = board.clone();
            peer.player_number = player;
            peer.world_state.robot.ground_to_field = Some(pose.as_transform());
            if is_closest_to_ball(&mut peer) {
                owners.push(player);
            }
        }
        assert_eq!(owners, vec![PlayerNumber::Two]);
    }

    #[test]
    fn future_region_handoff_cancels_previous_owner_and_invalidated_forecasts() {
        let sites = [
            (linear_algebra::Pose2::default(), PlayerNumber::Two),
            (point![2.0, 0.0].into(), PlayerNumber::Three),
        ];
        let mut peers = Vec::new();
        for (pose, player) in sites {
            let mut board = Blackboard {
                player_number: player,
                field_dimensions: FieldDimensions::SPL_2025,
                last_closest_to_ball: player == PlayerNumber::Two,
                ..Default::default()
            };
            board.world_state.robot.ground_to_field = Some(pose.as_transform());
            board.parameters.goalkeeper.player_number = PlayerNumber::One;
            board.parameters.ball.pursuit_lookahead = Duration::from_millis(500);
            board.parameters.ball.last_ball_timeout = Duration::from_secs(1);
            board.parameters.ball.interception.minimum_ball_velocity = 0.3;
            board.parameters.ball.closest_to_ball.enter_duration = Duration::from_millis(300);
            board.parameters.ball.closest_to_ball.exit_duration = Duration::from_millis(150);
            board.parameters.kicking.kick_activation_distance = 0.3;
            board.parameters.kicking.approach_ball_standoff = 0.3;
            board.parameters.voronoi.grid_resolution = 0.1;
            board.parameters.voronoi.padding = 0.5;
            board
                .parameters
                .kicking
                .strategy
                .passing
                .maximum_receiver_age = Duration::from_secs(1);
            board.ball = Some(LastBall {
                position: point![0.8, 0.0],
                velocity: vector![2.0, 0.0],
                age: board.world_state.now,
                field_side: Side::Left,
                source: BallSource::Own,
            });
            for (peer_pose, peer) in sites {
                if peer != player {
                    board.world_state.player_states[peer] = Some(TimeWrapper {
                        time: board.world_state.now,
                        inner: PlayerState {
                            pose: peer_pose,
                            ..crate::test_support::player_state()
                        },
                    });
                }
            }
            assert_eq!(
                crate::voronoi::calculate_voronoi_grid(&mut board),
                Status::Success
            );
            assert_eq!(
                board
                    .voronoi_map
                    .as_ref()
                    .unwrap()
                    .ownership_at(point![0.8, 0.0]),
                Some(Ownership::Robot(PlayerNumber::Two))
            );
            assert!(!is_ball_interception_candidate(&mut board));
            let elected = is_closest_to_ball(&mut board);
            assert_eq!(elected, player == PlayerNumber::Three);
            peers.push(board);
        }
        for velocity in [Vector2::zeros(), vector![-2.0, 0.0], vector![f32::NAN, 0.0]] {
            let mut invalidated = peers[1].clone();
            invalidated.ball.as_mut().unwrap().velocity = velocity;
            assert!(!is_closest_to_ball(&mut invalidated));
        }
        peers[0].ball.as_mut().unwrap().velocity = vector![-2.0, 0.0];
        assert!(is_closest_to_ball(&mut peers[0]));
    }

    #[test]
    fn incoming_ball_claims_its_region_without_waiting_for_entry_dwell() {
        let player = hsl_network_messages::PlayerNumber::Two;
        let mut map = voronoi::VoronoiGrid::new(point![-4.5, -3.0], point![4.5, 3.0], 0.2);
        map.multi_source_dijkstra(&[(linear_algebra::Pose2::new(point![0.0, 0.0], 0.0), player)]);
        let mut board = Blackboard {
            voronoi_map: Some(map),
            ball: Some(LastBall {
                position: point![2.0, 0.6],
                velocity: Default::default(),
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            }),
            ..Default::default()
        };
        board.player_number = player;
        board.world_state.robot.ground_to_field = Some(Default::default());
        board.parameters.ball.closest_to_ball.enter_duration = Duration::from_millis(300);
        board.parameters.ball.closest_to_ball.exit_duration = Duration::from_millis(150);
        board.parameters.ball.interception.maximum_ball_distance = 4.0;
        board.parameters.ball.interception.minimum_ball_velocity = 0.3;
        board
            .parameters
            .ball
            .interception
            .minimum_ball_velocity_towards_robot = 0.7;
        board
            .parameters
            .ball
            .interception
            .minimum_ball_velocity_towards_own_half = 0.7;
        assert!(!is_closest_to_ball(&mut board));
        board.ball.as_mut().unwrap().velocity = vector![-2.0, 0.0];
        assert!(is_closest_to_ball(&mut board));
        board.voronoi_map = None;
        assert!(is_closest_to_ball(&mut board));
        board.world_state.now = board.world_state.now + Duration::from_millis(150);
        assert!(!is_closest_to_ball(&mut board));
    }

    #[test]
    fn fresh_opponent_possession_skips_entry_dwell_only_for_elected_owner() {
        let poses = [
            (PlayerNumber::Two, linear_algebra::Pose2::default()),
            (PlayerNumber::Three, point![-2.0, 2.0].into()),
        ];
        let mut owners = Vec::new();
        for (player, pose) in poses {
            let mut board = Blackboard {
                player_number: player,
                field_dimensions: types::field_dimensions::FieldDimensions::SPL_2025,
                ..Default::default()
            };
            board.world_state.now = board.world_state.now + Duration::from_secs(10);
            board.world_state.robot.ground_to_field = Some(pose.as_transform());
            board.parameters.goalkeeper.player_number = PlayerNumber::One;
            board.parameters.ball.last_ball_timeout = Duration::from_millis(500);
            board.parameters.ball.team_ball_timeout = Duration::from_secs(1);
            board.parameters.ball.closest_to_ball.enter_duration = Duration::from_millis(300);
            board.parameters.kicking.strategy.pressure_distance = 0.8;
            board.parameters.voronoi.grid_resolution = 0.2;
            board.parameters.voronoi.padding = 0.5;
            board
                .parameters
                .kicking
                .strategy
                .passing
                .maximum_receiver_age = Duration::from_secs(3);
            board.ball = Some(LastBall {
                position: point![1.2, 0.0],
                velocity: Default::default(),
                age: board.world_state.now,
                field_side: Side::Left,
                source: BallSource::Own,
            });
            for (peer, peer_pose) in poses {
                if peer != player {
                    board.world_state.player_states[peer] = Some(TimeWrapper {
                        time: board.world_state.now,
                        inner: PlayerState {
                            pose: peer_pose,
                            ..crate::test_support::player_state()
                        },
                    });
                }
            }
            board.world_state.obstacles = vec![Obstacle::robot(
                pose.as_transform::<Ground>().inverse() * point![1.3, 0.0],
                0.2,
                0.2,
            )];
            crate::voronoi::calculate_voronoi_grid(&mut board);
            assert_eq!(
                board
                    .voronoi_map
                    .as_ref()
                    .unwrap()
                    .ownership_at(point![1.2, 0.0]),
                Some(Ownership::Blocked)
            );
            for source in [BallSource::Own, BallSource::Team] {
                for future in [false, true] {
                    let mut invalid = board.clone();
                    let ball = invalid.ball.as_mut().unwrap();
                    ball.source = source;
                    ball.age = if future {
                        invalid.world_state.now + Duration::from_millis(1)
                    } else {
                        invalid.world_state.now - ball_timeout(&invalid.parameters, source)
                    };
                    assert!(!is_closest_to_ball(&mut invalid));
                }
            }
            let mut uncontested = board.clone();
            uncontested.world_state.obstacles.clear();
            assert!(!is_closest_to_ball(&mut uncontested));
            if is_closest_to_ball(&mut board) {
                owners.push(player);
            }
        }
        assert_eq!(owners, vec![PlayerNumber::Two]);
    }

    #[test]
    fn striker_defers_to_teammate_controlling_ball_unless_finishing_kick() {
        let mut map = voronoi::VoronoiGrid::new(point![-4.5, -3.0], point![4.5, 3.0], 0.2);
        map.multi_source_dijkstra(&[(linear_algebra::Pose2::default(), PlayerNumber::Two)]);
        let mut board = Blackboard {
            voronoi_map: Some(map),
            ..Default::default()
        };
        board.world_state.now = board.world_state.now + Duration::from_secs(1);
        board.player_number = PlayerNumber::Two;
        board.world_state.robot.ground_to_field = Some(Default::default());
        board.parameters.goalkeeper.player_number = PlayerNumber::One;
        board.parameters.ball.last_ball_timeout = Duration::from_secs(5);
        board.parameters.kicking.kick_activation_distance = 1.3;
        board.parameters.kicking.approach_ball_standoff = 0.3;
        board
            .parameters
            .kicking
            .strategy
            .passing
            .maximum_receiver_age = Duration::from_secs(1);
        board.ball = Some(LastBall {
            position: point![1.2, 0.0],
            velocity: Default::default(),
            age: board.world_state.now,
            field_side: Side::Left,
            source: BallSource::Own,
        });
        board.world_state.player_states.three = Some(TimeWrapper {
            time: board.world_state.now,
            inner: PlayerState {
                pose: point![1.0, 0.0].into(),
                ..crate::test_support::player_state()
            },
        });
        assert!(!is_closest_to_ball(&mut board));
        assert!(board.closest_to_ball_entered_area_since.is_none());
        board.last_closest_to_ball = true;
        assert!(!is_closest_to_ball(&mut board));
        assert!(board.closest_to_ball_left_area_since.is_none());
        for case in 0..6 {
            let mut available = board.clone();
            let peer = available.world_state.player_states.three.as_mut().unwrap();
            match case {
                0 => peer.time = Default::default(),
                1 => peer.inner.pose = point![-2.0, 0.0].into(),
                2 => {
                    available.world_state.filtered_game_controller_state =
                        Some(FilteredGameControllerState {
                            penalties: types::players::Players {
                                three: Some(Penalty::PickUp {
                                    remaining: Duration::from_secs(1),
                                }),
                                ..Default::default()
                            },
                            ..crate::test_support::filtered_game_controller_state()
                        });
                }
                3 => peer.time = available.world_state.now + Duration::from_secs(1),
                4 => peer.inner.pose = point![f32::NAN, 0.0].into(),
                _ => {
                    available.world_state.robot.ground_to_field = Some(
                        linear_algebra::Isometry2::from_parts(vector![1.1, 0.0], 0.0),
                    )
                }
            }
            assert!(is_closest_to_ball(&mut available), "eligibility {case}");
        }
        board.last_closest_to_ball = true;
        board.last_motion_type = Some(MotionType::Kick);
        assert!(is_finishing_striker_kick(&mut board));
        assert!(is_closest_to_ball(&mut board));
        board.world_state.now = board.world_state.now + Duration::from_millis(500);
        assert!(!is_finishing_striker_kick(&mut board));
        assert!(!is_closest_to_ball(&mut board));
    }

    #[test]
    fn keeps_striker_while_finishing_kick_then_releases_or_loses_ball() {
        let mut board = Blackboard {
            last_closest_to_ball: true,
            last_motion_type: Some(MotionType::Kick),
            ball: Some(LastBall {
                position: point![0.5, 0.0],
                velocity: Default::default(),
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            }),
            ..Default::default()
        };
        board.parameters.ball.last_ball_timeout = Duration::from_secs(5);
        board.world_state.now = board.world_state.now + Duration::from_secs(1);
        assert!(is_closest_to_ball(&mut board));
        board.world_state.now = board.world_state.now + Duration::from_millis(500);
        assert!(!is_closest_to_ball(&mut board));
        board.world_state.now = Default::default();
        board.last_closest_to_ball = true;
        board.ball = None;
        assert!(!is_finishing_striker_kick(&mut board));
        board.last_motion_type = Some(MotionType::Walk);
        assert!(!is_finishing_striker_kick(&mut board));
    }
}
