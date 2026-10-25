use std::time::Duration;

use coordinate_systems::Field;
use hsl_network_messages::{PassIntent, PlayerNumber, SubState};
use linear_algebra::{Orientation2, Point2, Vector2};
use types::{behavior_tree::Status, motion_command::BodyMotion, motion_type::MotionType};

use crate::{
    actions::stand,
    conditions::hulks_is_kicking_team,
    kick::{
        kick_ball_in_ground,
        reception::{is_receiving_ball, receive_rolling_ball},
    },
    node::{Blackboard, ball_timeout},
    substates::{is_sub_state, restart_pass_is_open, restart_pass_receiver},
    switch_motion_type::is_allowed_to_switch,
    walk::walk_to,
};

// shortcut: set-play readiness is unconfirmed; tune after measuring delivery and turning.
const PREPARATION_TIME: Duration = Duration::from_millis(150);
// Wind-up and flight can outlast the freshness window of each broadcast.
pub(crate) const PASS_EXECUTION_TIMEOUT: Duration = Duration::from_secs(6);

pub(crate) fn matches_announced_pass(blackboard: &Blackboard, pass: PassIntent) -> bool {
    let now = blackboard.world_state.now;
    let parameters = &blackboard.parameters.kicking.strategy.passing;
    blackboard
        .last_announced_pass
        .as_ref()
        .is_some_and(|announced| {
            announced.time <= now
                && now.duration_since(announced.time) < PASS_EXECUTION_TIMEOUT
                && announced.inner.receiver == pass.receiver
                && (announced.inner.target - pass.target).norm() <= parameters.target_half_width
        })
        && blackboard
            .last_sent_hsl_message_time
            .is_some_and(|sent| sent <= now && now.duration_since(sent) < PassIntent::TIMEOUT)
}

pub(super) fn pass_is_announced(
    blackboard: &Blackboard,
    receiver: PlayerNumber,
    target: Point2<Field>,
) -> bool {
    matches_announced_pass(blackboard, PassIntent { receiver, target })
        && blackboard.last_announced_pass.as_ref().is_some_and(|pass| {
            blackboard.world_state.now.duration_since(pass.time) >= PREPARATION_TIME
        })
}

pub(crate) fn is_executing_announced_pass(blackboard: &Blackboard, receiver: PlayerNumber) -> bool {
    if blackboard.last_motion_type != Some(MotionType::Kick)
        || blackboard.ball_action.or(blackboard.last_ball_action)
            != Some(super::strategy::BallAction::Pass { receiver })
        || !blackboard.last_announced_pass.as_ref().is_some_and(|pass| {
            pass.inner.receiver == receiver && matches_announced_pass(blackboard, pass.inner)
        })
    {
        return false;
    }
    if outgoing_pass_in_flight(blackboard) {
        return true;
    }
    let Some(transform) = blackboard.world_state.robot.ground_to_field else {
        return false;
    };
    let Some((ball, _)) = kick_ball_in_ground(blackboard, &transform) else {
        return false;
    };
    let Some(receiver) = blackboard.world_state.player_states[receiver].as_ref() else {
        return false;
    };
    let parameters = &blackboard.parameters.kicking.strategy.passing;
    let travel =
        (receiver.inner.pose.position() - transform * ball).norm() - parameters.receiver_standoff;
    // Allow contact drift, not an arbitrarily short kick when the receiver moves in.
    travel.is_finite()
        && travel >= (parameters.minimum_distance - parameters.target_half_width).max(0.0)
}

pub(crate) fn outgoing_pass_in_flight(blackboard: &Blackboard) -> bool {
    let world = &blackboard.world_state;
    let (Some(announced), Some(ball), Some(transform)) = (
        blackboard.last_announced_pass.as_ref(),
        blackboard.ball.as_ref(),
        world.robot.ground_to_field,
    ) else {
        return false;
    };
    let pass = announced.inner;
    let parameters = &blackboard.parameters.kicking.strategy.passing;
    let Some(receiver) = world.player_states[pass.receiver].as_ref() else {
        return false;
    };
    let field = &blackboard.field_dimensions;
    if !field.length.is_finite()
        || field.length <= 0.0
        || !field.width.is_finite()
        || field.width <= 0.0
        || ball.position.x().abs()
            > field.length / 2.0 + super::geometry::BALL_FIELD_BOUNDARY_TOLERANCE
        || ball.position.y().abs()
            > field.width / 2.0 + super::geometry::BALL_FIELD_BOUNDARY_TOLERANCE
        || parameters.validate().is_err()
        || parameters.maximum_distance <= 0.0
        || world.robot.primary_state != types::primary_state::PrimaryState::Playing
        || pass.receiver == blackboard.player_number
        || pass.receiver == blackboard.parameters.goalkeeper.player_number
        || blackboard.ball_action.is_some_and(|action| {
            action
                != super::strategy::BallAction::Pass {
                    receiver: pass.receiver,
                }
        })
        || announced.time > world.now
        || world.now.duration_since(announced.time) >= PASS_EXECUTION_TIMEOUT
        || !matches_announced_pass(blackboard, pass)
        || ball.age > world.now
        || world.now.duration_since(ball.age) >= ball_timeout(&blackboard.parameters, ball.source)
        || receiver.time > world.now
        || world.now.duration_since(receiver.time) >= parameters.maximum_receiver_age
        || world
            .filtered_game_controller_state
            .as_ref()
            .is_some_and(|game| game.penalties[pass.receiver].is_some())
        || !ball
            .position
            .inner
            .iter()
            .chain(ball.velocity.inner.iter())
            .chain(pass.target.inner.iter())
            .chain(receiver.inner.pose.position().inner.iter())
            .chain(transform.translation().inner.iter())
            .all(|value| value.is_finite())
        || pass.target.x().abs() >= blackboard.field_dimensions.length / 2.0
        || pass.target.y().abs() >= blackboard.field_dimensions.width / 2.0
        || receiver.inner.pose.position().x().abs() >= blackboard.field_dimensions.length / 2.0
        || receiver.inner.pose.position().y().abs() >= blackboard.field_dimensions.width / 2.0
    {
        return false;
    }
    let position = transform.as_pose().position();
    let to_ball = ball.position - position;
    let to_target = pass.target - position;
    let Some(direction) = to_target.try_normalize(f32::EPSILON) else {
        return false;
    };
    let contact = blackboard.parameters.kicking.approach_ball_standoff
        + blackboard.field_dimensions.ball_radius;
    let corridor = parameters.target_half_width
        + blackboard.parameters.walking.path_planning.robot_radius
        + blackboard.field_dimensions.ball_radius;
    // The announced recipient owns this launch; recover if it stops, reverses, or misses its lane.
    to_target.norm() <= parameters.maximum_distance + parameters.receiver_standoff
        && (receiver.inner.pose.position() - pass.target).norm()
            <= parameters.receiver_standoff + blackboard.parameters.kicking.kick_activation_distance
        && to_ball.norm() > contact
        && ((receiver.inner.pose.position() - ball.position).norm() > contact
            || ball.velocity.norm()
                > 2.0
                    * blackboard
                        .parameters
                        .ball
                        .interception
                        .minimum_ball_velocity)
        && ball.velocity.norm()
            > blackboard
                .parameters
                .ball
                .interception
                .minimum_ball_velocity
        && to_ball.dot(&ball.velocity) > 0.0
        && (receiver.inner.pose.position() - ball.position).dot(&ball.velocity) > 0.0
        && (to_ball - direction * to_ball.dot(&direction)).norm() <= corridor
}

pub(crate) fn incoming_pass(blackboard: &Blackboard) -> Option<(PassIntent, Point2<Field>)> {
    if outgoing_pass_in_flight(blackboard) {
        return None;
    }
    let world = &blackboard.world_state;
    let transform = world.robot.ground_to_field?;
    let own_position = transform.as_pose().position();
    let parameters = &blackboard.parameters.kicking.strategy.passing;
    parameters.validate().ok()?;
    let following_rolling_pass = blackboard.ball.as_ref().is_some_and(|ball| {
        ball.age <= world.now
            && world.now.duration_since(ball.age)
                < ball_timeout(&blackboard.parameters, ball.source)
            && ((is_receiving_ball(blackboard)
                && (own_position - ball.position).dot(&ball.velocity) > 0.0)
                || (own_position - ball.position).norm()
                    <= blackboard.parameters.kicking.kick_activation_distance)
    });
    if parameters.maximum_distance <= 0.0
        || !own_position.x().is_finite()
        || !own_position.y().is_finite()
    {
        return None;
    }
    world
        .player_states
        .iter()
        .filter_map(|(sender, state)| {
            let state = state.as_ref()?;
            let pass = state.inner.pass?;
            if sender == blackboard.player_number
                || pass.receiver != blackboard.player_number
                || state.time > world.now
                || world.now.duration_since(state.time) >= PassIntent::TIMEOUT
                || world
                    .filtered_game_controller_state
                    .as_ref()
                    .is_some_and(|game| game.penalties[sender].is_some())
                || !pass.target.x().is_finite()
                || !pass.target.y().is_finite()
                || pass.target.x().abs() >= blackboard.field_dimensions.length / 2.0
                || pass.target.y().abs() >= blackboard.field_dimensions.width / 2.0
                || (pass.target - own_position).norm()
                    > if following_rolling_pass {
                        parameters.maximum_distance
                    } else {
                        parameters.receiver_standoff + 2.0 * parameters.target_half_width
                    }
                || (pass.target - state.inner.pose.position()).norm()
                    > parameters.maximum_distance + parameters.receiver_standoff
                || !state.inner.pose.position().x().is_finite()
                || !state.inner.pose.position().y().is_finite()
            {
                return None;
            }
            Some((state.time, pass, state.inner.pose.position()))
        })
        .max_by_key(|(time, ..)| *time)
        .map(|(_, pass, sender)| (pass, sender))
}

pub fn is_pass_receiver(blackboard: &mut Blackboard) -> bool {
    incoming_pass(blackboard).is_some()
}

pub fn is_pass_ball_arriving(blackboard: &mut Blackboard) -> bool {
    let parameters = &blackboard.parameters.kicking.strategy.passing;
    let arriving = incoming_pass(blackboard).is_some_and(|(pass, sender)| {
        let own_position = blackboard
            .world_state
            .robot
            .ground_to_field
            .unwrap()
            .as_pose()
            .position();
        blackboard.ball.as_ref().is_some_and(|ball| {
            let observed = ball.position;
            ((observed - pass.target).norm()
                <= parameters.receiver_standoff + parameters.target_half_width
                || (is_receiving_ball(blackboard)
                    && (own_position - ball.position).dot(&ball.velocity) > 0.0)
                || ((own_position - ball.position).norm()
                    <= blackboard.parameters.kicking.kick_activation_distance
                    && ball.velocity.norm()
                        <= blackboard
                            .parameters
                            .ball
                            .interception
                            .minimum_ball_velocity))
                && (observed - own_position).norm() < (observed - sender).norm()
        })
    });
    if arriving {
        // The addressed receiver claims possession without the normal entry delay.
        blackboard.last_closest_to_ball = true;
        blackboard.closest_to_ball_entered_area_since = None;
        blackboard.closest_to_ball_left_area_since = None;
    }
    arriving
}

pub fn prepare_to_receive_pass(blackboard: &mut Blackboard) -> Status {
    let Some((pass, sender)) = incoming_pass(blackboard) else {
        return Status::Failure;
    };
    // Move only from a fresh lane estimate; stationary launches start at the ball, not the body.
    let launch_origin = blackboard
        .ball
        .as_ref()
        .filter(|ball| {
            ball.age <= blackboard.world_state.now
                && blackboard.world_state.now.duration_since(ball.age)
                    < ball_timeout(&blackboard.parameters, ball.source)
        })
        .and_then(|ball| {
            let speed = ball.velocity.norm();
            (speed
                <= blackboard
                    .parameters
                    .ball
                    .interception
                    .minimum_ball_velocity
                && (ball.position - sender).norm()
                    <= blackboard.parameters.kicking.kick_activation_distance
                        + blackboard.parameters.kicking.kick_activation_hysteresis)
                .then_some(ball.position)
        });
    if launch_origin.is_none() && receive_rolling_ball(blackboard) == Status::Success {
        return Status::Success;
    }
    let ball = blackboard
        .ball
        .as_ref()
        .map_or(sender, |ball| ball.position);
    let transform = blackboard.world_state.robot.ground_to_field.unwrap();
    if !super::kickoff::own_kickoff(blackboard)
        && let Some(direction) =
            launch_origin.and_then(|origin| (pass.target - origin).try_normalize(f32::EPSILON))
    {
        let offset = pass.target - transform.as_pose().position();
        let lateral_offset = offset - direction * offset.dot(&direction);
        // Join the lane without walking longitudinally toward the pending pass.
        let target = transform.as_pose().position() + lateral_offset;
        let walking = blackboard.parameters.walking.walk_and_stand;
        // Join an inward-shifted announced lane before arrival; measured reception owns it after.
        if (ball - pass.target).dot(&direction) < 0.0
            && target.x().abs()
                < blackboard.field_dimensions.length / 2.0 - blackboard.field_dimensions.ball_radius
            && target.y().abs()
                < blackboard.field_dimensions.width / 2.0 - blackboard.field_dimensions.ball_radius
            && lateral_offset.norm()
                > walking
                    .target_reached_thresholds
                    .x
                    .max(blackboard.field_dimensions.ball_radius)
        {
            if blackboard.last_motion_type != Some(MotionType::Walk)
                && !is_allowed_to_switch(blackboard)
            {
                return stand(blackboard);
            }
            let target = transform.inverse() * target;
            let ball = transform.inverse() * ball;
            return walk_to(
                blackboard,
                linear_algebra::Pose2::from_parts(target, Orientation2::from_vector(ball - target)),
                blackboard.parameters.walking.speed.kicking,
                types::motion_command::OrientationMode::LookAt {
                    target: ball,
                    tolerance: walking.orientation_tolerance,
                },
                walking.normal_distance_to_be_aligned,
                walking.hysteresis,
            );
        }
    }
    if blackboard.last_motion_type != Some(MotionType::Walk) && !is_allowed_to_switch(blackboard) {
        return stand(blackboard);
    }
    let status = hold_and_face_ball(blackboard, ball);
    if matches!(blackboard.body_motion, Some(BodyMotion::Stand)) {
        // Keep an addressed receiver ready without introducing the Stand-to-Walk dwell.
        blackboard.body_motion = Some(BodyMotion::WalkWithVelocity {
            velocity: Vector2::zeros(),
            angular_velocity: 0.0,
        });
    }
    status
}

pub fn prepare_for_restart_pass(blackboard: &mut Blackboard) -> Status {
    if !hulks_is_kicking_team(blackboard)
        || !(is_sub_state(blackboard, SubState::CornerKick)
            || is_sub_state(blackboard, SubState::ThrowIn))
    {
        return Status::Failure;
    }
    let (Some(ball), Some(transform)) = (
        blackboard.ball.as_ref(),
        blackboard.world_state.robot.ground_to_field,
    ) else {
        return Status::Failure;
    };
    let ball = ball.position;
    let position = transform.as_pose().position();
    if restart_pass_is_open(blackboard, position) {
        if is_sub_state(blackboard, SubState::CornerKick)
            && restart_pass_receiver(blackboard)
                .is_some_and(|(receiver, _)| receiver == blackboard.player_number)
        {
            blackboard.ball_action = None;
            blackboard.kick_target = None;
            return hold_and_face_ball(blackboard, ball);
        }
        // Until a pass is announced, open-lane supporters keep their weighted Voronoi position.
        return Status::Failure;
    }
    let Some((receiver, target)) = restart_pass_receiver(blackboard) else {
        return Status::Failure;
    };
    if receiver != blackboard.player_number {
        return Status::Failure;
    }
    blackboard.ball_action = None;
    blackboard.kick_target = None;
    if blackboard.last_motion_type != Some(MotionType::Walk) && !is_allowed_to_switch(blackboard) {
        return stand(blackboard);
    }
    let walking = blackboard.parameters.walking.walk_and_stand;
    let target = transform.inverse() * target;
    let ball = transform.inverse() * ball;
    walk_to(
        blackboard,
        linear_algebra::Pose2::from_parts(target, Orientation2::from_vector(ball - target)),
        blackboard.parameters.walking.speed.kicking,
        types::motion_command::OrientationMode::LookAt {
            target: ball,
            tolerance: walking.orientation_tolerance,
        },
        walking.normal_distance_to_be_aligned,
        walking.hysteresis,
    )
}

fn hold_and_face_ball(blackboard: &mut Blackboard, ball: Point2<Field>) -> Status {
    let transform = blackboard.world_state.robot.ground_to_field.unwrap();
    let direction = transform.inverse() * ball;
    let Some(direction) = direction.coords().try_normalize(f32::EPSILON) else {
        return stand(blackboard);
    };
    let angle = Orientation2::from_vector(direction).angle();
    if angle.abs()
        <= blackboard
            .parameters
            .walking
            .walk_and_stand
            .orientation_tolerance
    {
        return stand(blackboard);
    }
    // Keep the reception position while turning toward the ball.
    blackboard.body_motion = Some(BodyMotion::WalkWithVelocity {
        velocity: Vector2::zeros(),
        angular_velocity: blackboard.walking_velocity_limits.scale_angular_axis(angle),
    });
    Status::Success
}

/// Finish only a pass that this robot already announced while taking a restart.
pub fn is_finishing_restart_pass(blackboard: &mut Blackboard) -> bool {
    let Some(announced) = blackboard.last_announced_pass.as_ref() else {
        return false;
    };
    let pass = announced.inner;
    if !is_executing_announced_pass(blackboard, pass.receiver) {
        return false;
    }
    blackboard.ball_action = Some(super::strategy::BallAction::Pass {
        receiver: pass.receiver,
    });
    true
}

pub(crate) const RESTART_RECEIVER_EXTRA_DISTANCE: f32 = 0.5;

pub(super) fn pass_receiver_position(
    blackboard: &Blackboard,
    _ball: Point2<Field>,
    receiver: PlayerNumber,
) -> Option<Point2<Field>> {
    let world = &blackboard.world_state;
    let parameters = &blackboard.parameters.kicking.strategy.passing;
    if receiver == blackboard.player_number
        || receiver == blackboard.parameters.goalkeeper.player_number
        || world
            .filtered_game_controller_state
            .as_ref()
            .is_some_and(|state| state.penalties[receiver].is_some())
    {
        return None;
    }
    let state = world.player_states[receiver].as_ref()?;
    if state.time > world.now
        || world.now.duration_since(state.time) >= parameters.maximum_receiver_age
    {
        return None;
    }
    let position = state.inner.pose.position();
    let field = &blackboard.field_dimensions;
    (position
        .inner
        .iter()
        .all(|coordinate| coordinate.is_finite())
        && position.x().abs() < field.length / 2.0
        && position.y().abs() < field.width / 2.0
        && (super::strategy::own_non_penalty_restart(blackboard)
            || is_executing_announced_pass(blackboard, receiver)))
    .then_some(position)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::LastBall;
    use linear_algebra::{Isometry2, point, vector};
    use types::{
        field_dimensions::{FieldDimensions, Side},
        time_wrapper::TimeWrapper,
        world_state::{BallSource, PlayerState},
    };

    fn launched_pass_sender() -> Blackboard {
        let mut sender = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            last_motion_type: Some(MotionType::Kick),
            last_closest_to_ball: true,
            ..Default::default()
        };
        sender.world_state.now = sender.world_state.now + Duration::from_secs(1);
        sender.player_number = PlayerNumber::Three;
        sender.world_state.robot.primary_state = types::primary_state::PrimaryState::Playing;
        sender.world_state.robot.ground_to_field = Some(Default::default());
        sender.world_state.filtered_game_controller_state = Some(
            types::filtered_game_controller_state::FilteredGameControllerState {
                remaining_time_in_half: Duration::from_secs(100),
                remaining_number_of_messages: 10_000,
                ..crate::test_support::filtered_game_controller_state()
            },
        );
        sender.parameters.goalkeeper.player_number = PlayerNumber::One;
        sender.parameters.ball.last_ball_timeout = Duration::from_secs(5);
        sender.parameters.ball.interception.minimum_ball_velocity = 0.3;
        sender.parameters.ball.closest_to_ball.exit_duration = Duration::from_millis(150);
        sender.parameters.kicking.approach_ball_standoff = 0.3;
        sender.parameters.kicking.kick_activation_distance = 1.3;
        sender.parameters.walking.path_planning.robot_radius = 0.4;
        let parameters = &mut sender.parameters.kicking.strategy.passing;
        parameters.maximum_distance = 8.0;
        parameters.receiver_standoff = 0.5;
        parameters.target_half_width = 0.2;
        parameters.maximum_receiver_age = Duration::from_secs(5);
        sender.ball = Some(LastBall {
            position: point![0.5, 0.0],
            velocity: vector![2.0, 0.0],
            age: sender.world_state.now,
            field_side: Side::Left,
            source: BallSource::Own,
        });
        sender.world_state.player_states.two = Some(TimeWrapper {
            time: sender.world_state.now,
            inner: PlayerState {
                pose: point![3.5, 0.0].into(),
                ..crate::test_support::player_state()
            },
        });
        sender.last_announced_pass = Some(TimeWrapper {
            time: sender.world_state.now - Duration::from_millis(200),
            inner: PassIntent {
                receiver: PlayerNumber::Two,
                target: point![3.0, 0.0],
            },
        });
        sender.last_sent_hsl_message_time = Some(sender.last_announced_pass.as_ref().unwrap().time);
        let mut map = voronoi::VoronoiGrid::new(point![-4.5, -3.0], point![4.5, 3.0], 0.2);
        map.multi_source_dijkstra(&[(linear_algebra::Pose2::default(), PlayerNumber::Three)]);
        sender.voronoi_map = Some(map);
        sender
    }

    #[test]
    fn launched_pass_releases_sender_until_received_missed_or_expired() {
        let mut sender = launched_pass_sender();
        assert!(outgoing_pass_in_flight(&sender));
        let mut finishing = sender.clone();
        finishing
            .parameters
            .kicking
            .strategy
            .passing
            .minimum_distance = 3.0;
        finishing.ball_action = Some(super::super::strategy::BallAction::Pass {
            receiver: PlayerNumber::Two,
        });
        assert!(is_executing_announced_pass(&finishing, PlayerNumber::Two));
        assert!(!crate::conditions::is_closest_to_ball(&mut sender));
        // Keep the recipient informed during flight without extending the local handoff.
        let announced_at = sender.last_announced_pass.as_ref().unwrap().time;
        let Some(types::messages::OutgoingMessage::Hsl(hsl_network_messages::HulkMessage::State(
            message,
        ))) = sender.try_sending_state_message()
        else {
            panic!("expected state broadcast")
        };
        assert_eq!(
            message.pass,
            Some(sender.last_announced_pass.as_ref().unwrap().inner)
        );
        assert_eq!(
            sender.last_announced_pass.as_ref().unwrap().time,
            announced_at
        );
        sender.world_state.now = sender.world_state.now + Duration::from_millis(200);
        sender.ball_action = Some(super::super::strategy::BallAction::Pass {
            receiver: PlayerNumber::Two,
        });
        for motion in [MotionType::Walk, MotionType::Stand] {
            sender.last_motion_type = Some(motion);
            assert!(outgoing_pass_in_flight(&sender));
            assert!(!crate::conditions::is_closest_to_ball(&mut sender));
        }
        let mut arriving = sender.clone();
        arriving.ball.as_mut().unwrap().position = point![3.2, 0.0];
        assert!(
            outgoing_pass_in_flight(&arriving),
            "a fast arrival still needs its reception intent"
        );

        for case in 0..11 {
            let mut recovery = sender.clone();
            match case {
                0 => recovery.ball.as_mut().unwrap().velocity = Vector2::zeros(),
                1 => recovery.ball.as_mut().unwrap().velocity = vector![-2.0, 0.0],
                2 => recovery.ball.as_mut().unwrap().position = point![1.0, 2.0],
                3 => {
                    recovery
                        .world_state
                        .player_states
                        .two
                        .as_mut()
                        .unwrap()
                        .time = recovery.world_state.now + Duration::from_secs(1)
                }
                4 => recovery.world_state.now = announced_at + PASS_EXECUTION_TIMEOUT,
                5 => {
                    recovery
                        .world_state
                        .filtered_game_controller_state
                        .as_mut()
                        .unwrap()
                        .penalties
                        .two = Some(hsl_network_messages::Penalty::PickUp {
                        remaining: Duration::from_secs(1),
                    })
                }
                6 => {
                    recovery.ball.as_mut().unwrap().position = point![3.2, 0.0];
                    recovery.ball.as_mut().unwrap().velocity = vector![0.4, 0.0];
                }
                7 => recovery.ball.as_mut().unwrap().position = point![0.3, 0.0],
                8 => {
                    recovery.last_announced_pass.as_mut().unwrap().inner.target =
                        point![f32::NAN, 0.0]
                }
                9 => recovery.ball_action = Some(super::super::strategy::BallAction::Clear),
                _ => {
                    recovery.ball_action = Some(super::super::strategy::BallAction::Pass {
                        receiver: PlayerNumber::Four,
                    });
                }
            }
            assert!(!outgoing_pass_in_flight(&recovery), "recovery {case}");
        }
        sender.world_state.now = announced_at + PASS_EXECUTION_TIMEOUT;
        let Some(types::messages::OutgoingMessage::Hsl(hsl_network_messages::HulkMessage::State(
            message,
        ))) = sender.try_sending_state_message()
        else {
            panic!("expected expired pass cancellation")
        };
        assert!(message.pass.is_none());
        assert!(sender.last_announced_pass.is_none());
    }

    #[test]
    fn pass_execution_outlives_peer_freshness_without_extending_announcement() {
        let mut sender = launched_pass_sender();
        let announced_at = sender.last_announced_pass.as_ref().unwrap().time;
        let pass = sender.last_announced_pass.as_ref().unwrap().inner;
        sender.last_motion_type = Some(MotionType::Walk);
        sender.ball_action = Some(super::super::strategy::BallAction::Pass {
            receiver: pass.receiver,
        });
        sender.ball.as_mut().unwrap().position = point![2.0, 0.0];
        for age in [
            Duration::from_secs(3),
            Duration::from_secs(4),
            PASS_EXECUTION_TIMEOUT - Duration::from_millis(1),
        ] {
            sender.world_state.now = announced_at + age;
            sender.ball.as_mut().unwrap().age = sender.world_state.now;
            sender.world_state.player_states.two.as_mut().unwrap().time = sender.world_state.now;
            sender.last_sent_hsl_message_time =
                Some(sender.world_state.now - Duration::from_millis(200));
            assert!(matches_announced_pass(&sender, pass));
            assert!(outgoing_pass_in_flight(&sender));
            let Some(types::messages::OutgoingMessage::Hsl(
                hsl_network_messages::HulkMessage::State(message),
            )) = sender.try_sending_state_message()
            else {
                panic!("expected in-flight pass broadcast")
            };
            assert_eq!(message.pass, Some(pass));
            assert_eq!(
                sender.last_announced_pass.as_ref().unwrap().time,
                announced_at
            );
        }

        let mut stale_message = sender.clone();
        stale_message.last_sent_hsl_message_time =
            Some(stale_message.world_state.now - Duration::from_secs(3));
        assert!(!matches_announced_pass(&stale_message, pass));
        assert!(!outgoing_pass_in_flight(&stale_message));

        let mut receiver = sender.clone();
        receiver.player_number = PlayerNumber::Two;
        receiver.last_announced_pass = None;
        receiver.ball_action = None;
        receiver.world_state.robot.ground_to_field =
            Some(Isometry2::from_parts(vector![3.5, 0.0], 0.0));
        receiver.world_state.player_states.three = Some(TimeWrapper {
            time: receiver.world_state.now,
            inner: PlayerState {
                pass: Some(pass),
                ..crate::test_support::player_state()
            },
        });
        assert!(is_pass_receiver(&mut receiver));
        receiver
            .world_state
            .player_states
            .three
            .as_mut()
            .unwrap()
            .time = receiver.world_state.now - Duration::from_secs(3);
        assert!(!is_pass_receiver(&mut receiver));

        sender.world_state.now = announced_at + Duration::from_secs(6);
        sender.ball.as_mut().unwrap().age = sender.world_state.now;
        sender.world_state.player_states.two.as_mut().unwrap().time = sender.world_state.now;
        sender.last_sent_hsl_message_time =
            Some(sender.world_state.now - Duration::from_millis(200));
        assert!(!matches_announced_pass(&sender, pass));
        assert!(!outgoing_pass_in_flight(&sender));
        let Some(types::messages::OutgoingMessage::Hsl(hsl_network_messages::HulkMessage::State(
            message,
        ))) = sender.try_sending_state_message()
        else {
            panic!("expected expired pass cancellation")
        };
        assert!(message.pass.is_none());
        assert!(sender.last_announced_pass.is_none());
    }
}
