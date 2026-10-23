use std::time::Duration;

use coordinate_systems::Field;
use geometry::line_segment::LineSegment;
use hsl_network_messages::{PassIntent, PlayerNumber, SubState};
use linear_algebra::{Orientation2, Point2, Vector2, vector};
use path_planner::path_planner::PathPlanner;
use types::{
    behavior_tree::Status, motion_command::BodyMotion, motion_type::MotionType, obstacles::Obstacle,
};

use crate::{
    actions::stand,
    conditions::{goalkeeper_clearance_direction, hulks_is_kicking_team},
    kick::{
        kick_ball_position_in_ground,
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
const PASS_EXECUTION_TIMEOUT: Duration = Duration::from_secs(6);

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
    let set_play = super::kickoff::own_kickoff(blackboard)
        || blackboard
            .world_state
            .filtered_game_controller_state
            .as_ref()
            .is_some_and(|game| game.sub_state.is_some());
    matches_announced_pass(blackboard, PassIntent { receiver, target })
        && (!set_play
            || blackboard.last_announced_pass.as_ref().is_some_and(|pass| {
                blackboard.world_state.now.duration_since(pass.time) >= PREPARATION_TIME
            }))
}

pub(super) fn is_executing_announced_pass(blackboard: &Blackboard, receiver: PlayerNumber) -> bool {
    if blackboard.last_motion_type != Some(MotionType::Kick)
        || blackboard.ball_action != Some(super::strategy::BallAction::Pass { receiver })
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
    let Some(ball) = kick_ball_position_in_ground(blackboard, &transform) else {
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
    if parameters.validate().is_err()
        || parameters.maximum_distance <= 0.0
        || world.robot.primary_state != types::primary_state::PrimaryState::Playing
        || pass.receiver == blackboard.player_number
        || pass.receiver == blackboard.parameters.goalkeeper.player_number
        || Some(pass.receiver) == blackboard.replacement_goalkeeper_coordinator.keeper()
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

fn incoming_pass(blackboard: &Blackboard) -> Option<(PassIntent, Point2<Field>)> {
    if goalkeeper_clearance_direction(blackboard).is_some() || outgoing_pass_in_flight(blackboard) {
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
            let projected = blackboard.projected_ball_position(ball.position, ball.velocity);
            ((projected - pass.target).norm()
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
                && (projected - own_position).norm() < (projected - sender).norm()
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
    let ball = blackboard.ball.as_ref().map_or(sender, |ball| {
        blackboard.projected_ball_position(ball.position, ball.velocity)
    });
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

pub fn yield_goalkeeper_clearance(blackboard: &mut Blackboard) -> Status {
    let transform = blackboard.world_state.robot.ground_to_field.unwrap();
    let ball = blackboard.ball.as_ref().unwrap();
    let position = ball.position;
    let Some(direction) = goalkeeper_clearance_direction(blackboard) else {
        return Status::Failure;
    };
    let lateral = vector![-direction.y(), direction.x()];
    let own_position = transform.as_pose().position();
    let offset = (own_position - position).dot(&lateral);
    let radius = blackboard.parameters.walking.path_planning.robot_radius;
    let clearance = radius
        + blackboard.field_dimensions.ball_radius
        + blackboard.parameters.kicking.strategy.obstacle_clearance;
    blackboard.ball_action = None;
    blackboard.kick_target = None;
    blackboard.last_closest_to_ball = false;
    blackboard.closest_to_ball_entered_area_since = None;
    blackboard.closest_to_ball_left_area_since = None;
    if offset.abs() >= clearance {
        return hold_and_face_ball(blackboard, position);
    }

    let mut planner = PathPlanner::default();
    planner.with_obstacles(&blackboard.world_state.obstacles, radius);
    planner.with_rule_obstacles(
        transform.inverse(),
        &blackboard.world_state.rule_obstacles,
        radius,
    );
    planner.with_ball(
        transform.inverse() * position,
        blackboard.field_dimensions.ball_radius,
        radius,
    );
    planner.with_goal_support_structures(transform.inverse(), &blackboard.field_dimensions);
    for (number, teammate) in blackboard.world_state.player_states.iter() {
        if number != blackboard.player_number
            && let Some(teammate) = teammate
            && teammate.time <= blackboard.world_state.now
            && blackboard.world_state.now.duration_since(teammate.time)
                <= blackboard
                    .parameters
                    .kicking
                    .strategy
                    .passing
                    .maximum_receiver_age
        {
            planner.with_obstacles(
                &[Obstacle::robot(
                    transform.inverse() * teammate.inner.pose.position(),
                    radius,
                    radius,
                )],
                radius,
            );
        }
    }
    let preferred_side = if offset < 0.0 { -1.0 } else { 1.0 };
    for side in [preferred_side, -preferred_side] {
        // Never cross the rolling ball's line to reach the other side.
        if offset * side < -f32::EPSILON {
            continue;
        }
        let target = own_position + lateral * (side * (clearance + radius) - offset);
        let target = transform.inverse() * target;
        let step = LineSegment(Point2::origin(), target);
        let field_target = transform * target;
        if field_target.x().abs() >= blackboard.field_dimensions.length / 2.0 - radius
            || field_target.y().abs() >= blackboard.field_dimensions.width / 2.0 - radius
            || planner
                .obstacles
                .iter()
                .any(|obstacle| obstacle.shape.intersects_line_segment(step))
        {
            continue;
        }
        let ball = transform.inverse() * position;
        let limits = blackboard.walking_velocity_limits;
        let velocity = target.coords().normalize() * blackboard.parameters.walking.speed.kicking;
        let forward_limit = if velocity.x() < 0.0 {
            -limits.forward_velocity_limits[0]
        } else {
            limits.forward_velocity_limits[1]
        };
        // Scale uniformly to stay on the obstacle-checked segment at any body orientation.
        // Retain the proven clearance-yield gait while interception uses the faster envelope.
        let lateral_limit = limits.lateral_velocity_limit.min(1.0);
        let scale = (velocity.x().abs() / forward_limit.max(f32::EPSILON))
            .max(velocity.y().abs() / lateral_limit.max(f32::EPSILON))
            .max(1.0);
        blackboard.path_obstacles_output = planner.obstacles;
        blackboard.body_motion = Some(BodyMotion::WalkWithVelocity {
            velocity: velocity / scale,
            angular_velocity: limits
                .scale_angular_axis(Orientation2::from_vector(ball.coords()).angle()),
        });
        return Status::Success;
    }
    hold_and_face_ball(blackboard, position)
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

#[cfg(test)]
mod tests {
    use super::*;
    use linear_algebra::{Isometry2, point};
    use types::{
        field_dimensions::{FieldDimensions, Side},
        time_wrapper::TimeWrapper,
        walking_velocity_limits::WalkingVelocityLimits,
        world_state::{BallSource, PlayerState},
    };

    use crate::{kick::reception::is_ready_to_receive_kick, node::LastBall};

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
        assert!(!crate::conditions::is_finishing_striker_kick(&mut sender));
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

    #[test]
    fn receiver_follows_measured_pass_after_leaving_announced_target() {
        let mut receiver = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            last_motion_type: Some(MotionType::Walk),
            ball: Some(LastBall {
                position: point![2.8, -0.7],
                velocity: vector![-0.2, 1.0],
                age: Default::default(),
                field_side: Side::Right,
                source: BallSource::Own,
            }),
            walking_velocity_limits: WalkingVelocityLimits {
                forward_velocity_limits: [-0.8, 1.8],
                lateral_velocity_limit: 1.0,
                angular_velocity_limit: 2.0,
            },
            ..Default::default()
        };
        receiver.player_number = PlayerNumber::Two;
        receiver.world_state.robot.ground_to_field =
            Some(Isometry2::from_parts(vector![2.0, 0.0], 0.0));
        receiver.parameters.goalkeeper.player_number = PlayerNumber::One;
        receiver.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        receiver.parameters.ball.interception.minimum_ball_velocity = 0.3;
        receiver.parameters.kicking.kick_activation_distance = 1.3;
        receiver.parameters.kicking.approach_ball_standoff = 0.3;
        receiver.parameters.kicking.strategy.maximum_shot_distance = 6.0;
        receiver.parameters.kicking.target_speed = 3.4;
        receiver.parameters.kicking.allow_strong_kicks = true;
        receiver.parameters.walking.path_planning.robot_radius = 0.4;
        let passing = &mut receiver.parameters.kicking.strategy.passing;
        passing.maximum_distance = 4.0;
        passing.receiver_standoff = 0.5;
        passing.target_half_width = 0.2;
        receiver.world_state.player_states.three = Some(TimeWrapper {
            time: receiver.world_state.now,
            inner: PlayerState {
                pose: point![0.0, -1.0].into(),
                pass: Some(PassIntent {
                    receiver: PlayerNumber::Two,
                    target: point![1.5, -1.0],
                }),
                ..crate::test_support::player_state()
            },
        });

        let mut waiting = receiver.clone();
        waiting.ball.as_mut().unwrap().position = point![2.8, 0.0];
        waiting.ball.as_mut().unwrap().velocity = Vector2::zeros();
        assert!(is_pass_receiver(&mut waiting));
        assert_eq!(prepare_to_receive_pass(&mut waiting), Status::Success);
        assert!(matches!(waiting.body_motion,
            Some(BodyMotion::WalkWithVelocity { velocity, angular_velocity })
            if velocity == Vector2::zeros() && angular_velocity == 0.0));
        // Clearance yielding and other ordinary holds keep their existing Stand behavior.
        assert_eq!(
            hold_and_face_ball(&mut waiting, point![2.8, 0.0]),
            Status::Success
        );
        assert!(matches!(waiting.body_motion, Some(BodyMotion::Stand)));

        let mut sideline = waiting.clone();
        sideline.world_state.robot.ground_to_field = Some(Isometry2::from_parts(
            vector![2.0, 2.9],
            std::f32::consts::PI,
        ));
        sideline.ball.as_mut().unwrap().position = point![0.2, 2.8];
        let sender = sideline.world_state.player_states.three.as_mut().unwrap();
        sender.inner.pose = point![0.0, 2.8].into();
        sender.inner.pass.as_mut().unwrap().target = point![1.5, 2.35];
        assert!(is_pass_receiver(&mut sideline));
        assert_eq!(prepare_to_receive_pass(&mut sideline), Status::Success);
        let Some(BodyMotion::Walk { ref path, .. }) = sideline.body_motion else {
            panic!("receiver must join the announced inward lane before launch")
        };
        let types::path::PathSegment::LineSegment(last) = path.last_segment() else {
            panic!("expected final approach to receiving stance")
        };
        let target = point![1.5, 2.35];
        let transform = sideline.world_state.robot.ground_to_field.unwrap();
        let direction = (target - sideline.ball.as_ref().unwrap().position).normalize();
        let displacement = transform * last.1 - transform.as_pose().position();
        assert!(displacement.dot(&direction).abs() < 1e-5);
        let expected = target + direction * 0.5;
        assert!(
            (transform * last.1 - expected)
                .dot(&vector![-direction.y(), direction.x()])
                .abs()
                < 1e-5
        );

        for speed in [-0.31, -0.29, 0.0, 0.29, 1.0] {
            let mut holding = waiting.clone();
            holding.ball.as_mut().unwrap().position = point![2.8, 0.17];
            holding.ball.as_mut().unwrap().velocity = vector![speed, 0.0];
            assert_eq!(prepare_to_receive_pass(&mut holding), Status::Success);
            assert!(matches!(holding.body_motion,
                Some(BodyMotion::WalkWithVelocity { velocity, .. })
                if velocity == Vector2::zeros()));
        }

        assert!(is_pass_receiver(&mut receiver));
        assert!(is_pass_ball_arriving(&mut receiver));
        assert!(!is_ready_to_receive_kick(&receiver));
        assert_eq!(prepare_to_receive_pass(&mut receiver), Status::Success);
        assert!(matches!(
            receiver.body_motion,
            Some(BodyMotion::WalkWithVelocity { .. })
        ));

        // Retain possession after leaving the advertised region to meet and settle the ball.
        receiver.ball.as_mut().unwrap().position = point![2.3, -0.2];
        assert!(is_pass_receiver(&mut receiver));
        assert!(is_pass_ball_arriving(&mut receiver));
        assert!(!is_ready_to_receive_kick(&receiver));
        receiver.ball.as_mut().unwrap().velocity = Vector2::zeros();
        assert!(is_pass_receiver(&mut receiver));
        assert!(is_pass_ball_arriving(&mut receiver));
        assert_eq!(receive_rolling_ball(&mut receiver), Status::Failure);
        receiver.world_state.now = receiver.world_state.now + Duration::from_millis(1);
        crate::tree::striker_subtree().tick_with_trace(&mut receiver);
        assert!(matches!(
            receiver.body_motion,
            Some(BodyMotion::Kick {
                strong: true,
                target_speed: 3.4,
                ..
            })
        ));
        assert!(is_pass_receiver(&mut receiver));

        // Intent cannot pull a receiver toward an unrelated distant or stale pass.
        receiver
            .world_state
            .player_states
            .three
            .as_mut()
            .unwrap()
            .inner
            .pass
            .as_mut()
            .unwrap()
            .target = point![-4.0, -1.0];
        assert!(!is_pass_receiver(&mut receiver));
        receiver
            .world_state
            .player_states
            .three
            .as_mut()
            .unwrap()
            .inner
            .pass
            .as_mut()
            .unwrap()
            .target = point![1.5, -1.0];
        receiver.world_state.now = receiver.world_state.now + PassIntent::TIMEOUT;
        assert!(!is_pass_receiver(&mut receiver));
    }

    #[test]
    fn old_addressed_pass_does_not_capture_a_keeper_clearance() {
        let mut receiver = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            last_motion_type: Some(MotionType::Walk),
            ball: Some(LastBall {
                position: point![-3.5, 0.0],
                velocity: vector![2.0, 0.0],
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            }),
            walking_velocity_limits: WalkingVelocityLimits {
                forward_velocity_limits: [-0.5, 1.8],
                lateral_velocity_limit: 1.0,
                angular_velocity_limit: 2.0,
            },
            ..Default::default()
        };
        receiver.player_number = PlayerNumber::Two;
        receiver.world_state.robot.ground_to_field =
            Some(Isometry2::from_parts(vector![-2.0, 0.0], 0.0));
        receiver.parameters.goalkeeper.player_number = PlayerNumber::One;
        receiver.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        receiver.parameters.walking.path_planning.robot_radius = 0.4;
        receiver.parameters.kicking.kick_activation_distance = 1.3;
        receiver
            .parameters
            .kicking
            .strategy
            .passing
            .maximum_distance = 8.0;
        receiver
            .parameters
            .kicking
            .strategy
            .passing
            .maximum_receiver_age = Duration::from_secs(3);
        receiver.world_state.player_states.one = Some(TimeWrapper {
            time: receiver.world_state.now,
            inner: PlayerState {
                pose: point![-4.2, 0.0].into(),
                ..crate::test_support::player_state()
            },
        });
        receiver.world_state.player_states.three = Some(TimeWrapper {
            time: receiver.world_state.now,
            inner: PlayerState {
                pose: point![-0.5, -1.0].into(),
                pass: Some(PassIntent {
                    receiver: PlayerNumber::Two,
                    target: point![-2.0, 0.0],
                }),
                ..crate::test_support::player_state()
            },
        });

        assert!(goalkeeper_clearance_direction(&receiver).is_some());
        assert!(!is_pass_receiver(&mut receiver));
        assert!(!is_pass_ball_arriving(&mut receiver));
        assert!(!is_receiving_ball(&receiver));
        assert_eq!(receive_rolling_ball(&mut receiver), Status::Failure);
        receiver.ball_action = Some(super::super::strategy::BallAction::Dribble);
        assert_eq!(
            super::super::execution::execute_ball_action(&mut receiver),
            Status::Success
        );
        assert!(matches!(
            receiver.body_motion,
            Some(BodyMotion::WalkWithVelocity { velocity, .. }) if velocity.y() > 0.0
        ));
        assert!(receiver.ball_action.is_none());

        // Cancel an already active kick without entering Walk before its switch dwell.
        receiver.last_motion_type = Some(MotionType::Kick);
        receiver.parameters.control.allow_switch.kick = Duration::from_secs(1);
        assert_eq!(
            super::super::execution::execute_ball_action(&mut receiver),
            Status::Success
        );
        assert!(matches!(receiver.body_motion, Some(BodyMotion::Stand)));
    }
}
