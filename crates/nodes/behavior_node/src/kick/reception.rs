use coordinate_systems::Ground;
use kinematics::robot_dimensions::RobotDimensions;
use linear_algebra::{Orientation2, Point2, Vector2, point, vector};
use types::{
    behavior_tree::Status,
    motion_command::{BodyMotion, OrientationMode},
    motion_type::MotionType,
    path::traits::PathProgress,
    walking_velocity_limits::WalkingVelocityLimits,
};

use crate::{
    conditions::{goalkeeper_clearance_direction, opponent_ball_holder, opponent_near_position},
    kick::kick_ball_in_ground,
    node::{Blackboard, MAXIMUM_BALL_PREDICTION_TIME, ball_prediction_time, ball_timeout},
    switch_motion_type::is_allowed_to_switch,
    walk::plan_interception,
};

const INTERCEPTION_TIME_STEP: f32 = 0.1;

pub(crate) fn is_receiving_ball(blackboard: &Blackboard) -> bool {
    if goalkeeper_clearance_direction(blackboard).is_some() {
        return false;
    }
    rolling_ball_in_ground(blackboard).is_some_and(|(position, velocity)| {
        let own_goal = blackboard
            .world_state
            .robot
            .ground_to_field
            .unwrap()
            .inverse()
            * point![-blackboard.field_dimensions.length / 2.0, 0.0];
        should_block_ball(position, velocity, own_goal)
    })
}

pub(crate) fn is_ready_to_receive_kick(blackboard: &Blackboard) -> bool {
    let Some((position, velocity)) = rolling_ball_in_ground(blackboard) else {
        return false;
    };
    if blackboard.player_number != blackboard.parameters.goalkeeper.player_number {
        let controlled = controlled_ball_at_feet(blackboard, position, velocity)
            && super::execution::resolve_target(blackboard).is_some_and(|target| {
                let transform = blackboard.world_state.robot.ground_to_field.unwrap();
                let direction = transform.inverse() * target - position;
                direction.x() > 0.0 && direction.dot(&position.coords()) > 0.0
            });
        return position.x() > 0.0
            && position.coords().norm() <= blackboard.parameters.kicking.kick_activation_distance
            && (controlled
                || (is_receiver_under_pressure(blackboard)
                    && position.coords().dot(&velocity) < 0.0));
    }
    is_on_contact_path(blackboard, position, velocity)
}

fn is_receiver_under_pressure(blackboard: &Blackboard) -> bool {
    // An opponent near the distant passer must not make an unmarked receiver rush its trap.
    blackboard
        .world_state
        .robot
        .ground_to_field
        .is_some_and(|transform| {
            opponent_near_position(
                blackboard,
                transform * Point2::origin(),
                blackboard.parameters.kicking.receiving_opponent_distance,
            )
        })
}

pub(crate) fn is_settling_pass(blackboard: &mut Blackboard) -> bool {
    blackboard.player_number != blackboard.parameters.goalkeeper.player_number
        && super::passing::is_pass_receiver(blackboard)
        && !is_receiver_under_pressure(blackboard)
}

fn is_on_contact_path(
    blackboard: &Blackboard,
    position: Point2<Ground>,
    velocity: Vector2<Ground>,
) -> bool {
    let standoff = blackboard.parameters.kicking.approach_ball_standoff;
    let half_width = (blackboard.parameters.walking.path_planning.robot_radius / 2.0)
        .max(2.0 * blackboard.field_dimensions.ball_radius);
    let contact = point![standoff, 0.0];
    let contact_time = (contact - position).dot(&velocity) / velocity.norm_squared();
    if position.x() > 0.0
        && position.coords().norm() <= blackboard.parameters.kicking.kick_activation_distance
        && (0.0..=0.25).contains(&contact_time)
        && (position + velocity * contact_time - contact).norm() <= half_width
    {
        return true;
    }
    let closing_speed = -velocity.x();
    // Allow 250 ms for turn/swing preparation, within the policy's 2 m input limit.
    let handoff_distance = if closing_speed > 1.0 {
        (blackboard.parameters.kicking.kick_activation_distance + closing_speed * 0.25).min(2.0)
    } else {
        blackboard.parameters.kicking.kick_activation_distance
    };
    if position.x() <= 0.0
        || position.coords().norm() > handoff_distance
        || Orientation2::from_vector(position.coords()).angle().abs() > 0.45
        || (velocity.x() >= 0.0
            && position.x() > standoff + blackboard.field_dimensions.ball_radius)
    {
        return false;
    }
    let lateral_position = if velocity.x() < -f32::EPSILON && position.x() > standoff {
        position.y() + velocity.y() * (standoff - position.x()) / velocity.x()
    } else {
        position.y()
    };
    lateral_position.abs() <= half_width
}

fn controlled_ball_at_feet(
    blackboard: &Blackboard,
    position: Point2<Ground>,
    velocity: Vector2<Ground>,
) -> bool {
    position.coords().norm() <= 2.0 * blackboard.parameters.kicking.approach_ball_standoff
        && position.y().abs() <= blackboard.parameters.walking.path_planning.robot_radius / 2.0
        && velocity.norm()
            <= 2.0
                * blackboard
                    .parameters
                    .ball
                    .interception
                    .minimum_ball_velocity
}

fn rolling_ball_in_ground(blackboard: &Blackboard) -> Option<(Point2<Ground>, Vector2<Ground>)> {
    fresh_ball_in_ground(blackboard).filter(|(_, velocity)| {
        velocity.norm()
            > blackboard
                .parameters
                .ball
                .interception
                .minimum_ball_velocity
    })
}

fn fresh_ball_in_ground(blackboard: &Blackboard) -> Option<(Point2<Ground>, Vector2<Ground>)> {
    if opponent_ball_holder(blackboard).is_some() {
        return None;
    }
    let transform = blackboard.world_state.robot.ground_to_field?;
    let (position, velocity) = kick_ball_in_ground(blackboard, &transform)?;
    let now = blackboard.world_state.now;
    let fresh_ball = blackboard.ball.as_ref().is_some_and(|ball| {
        ball.age <= now
            && now.duration_since(ball.age) < ball_timeout(&blackboard.parameters, ball.source)
    });
    fresh_ball.then_some((position, velocity))
}

pub fn receive_rolling_ball(blackboard: &mut Blackboard) -> Status {
    if goalkeeper_clearance_direction(blackboard).is_some() {
        return Status::Failure;
    }
    let settle = is_settling_pass(blackboard);
    let Some((position, velocity)) = (if settle {
        fresh_ball_in_ground(blackboard)
    } else {
        rolling_ball_in_ground(blackboard)
    }) else {
        return Status::Failure;
    };
    let transform = blackboard.world_state.robot.ground_to_field.unwrap();
    let continuing_deflection = blackboard.last_motion_type == Some(MotionType::Kick)
        && blackboard.ball_action.is_none()
        && matches!(
            blackboard.last_motion_command,
            Some(types::motion_command::MotionCommand::Kick {
                quick: true,
                strong: false,
                soft: false,
                ..
            })
        );
    if blackboard.player_number != blackboard.parameters.goalkeeper.player_number
        && !super::passing::is_pass_receiver(blackboard)
        && !controlled_ball_at_feet(blackboard, position, velocity)
        && position.coords().dot(&velocity) < 0.0
        && (transform * vector![1.0, 0.0]).x() > 0.0
        && is_on_contact_path(blackboard, position, velocity)
        && (blackboard.last_motion_type == Some(MotionType::Walk)
            || continuing_deflection
            || is_allowed_to_switch(blackboard))
    {
        // Deflect loose balls promptly; keep addressed passes in the settling behavior.
        if super::kick(blackboard) == Status::Failure {
            return Status::Failure;
        }
        if let Some(BodyMotion::Kick {
            quick,
            strong,
            soft,
            kick_direction,
            ..
        }) = &mut blackboard.body_motion
        {
            *quick = true;
            *strong = false;
            *soft = false;
            *kick_direction = Orientation2::default();
        }
        return Status::Success;
    }
    if blackboard.last_motion_type != Some(MotionType::Walk) && !is_allowed_to_switch(blackboard) {
        return Status::Failure;
    }
    if blackboard.player_number != blackboard.parameters.goalkeeper.player_number
        && controlled_ball_at_feet(blackboard, position, velocity)
        && !is_receiver_under_pressure(blackboard)
    {
        super::strategy::select_ball_action(blackboard);
        if settle {
            // Controlled possession can turn toward an outgoing target behind the receiver.
            return Status::Failure;
        }
    }
    if is_ready_to_receive_kick(blackboard) {
        return Status::Failure;
    }
    let goal = transform.inverse() * point![-blackboard.field_dimensions.length / 2.0, 0.0];
    let pass_receiver = super::passing::is_pass_receiver(blackboard);
    let hold_pass = settle
        && (position.coords().dot(&velocity) >= 0.0
            || velocity.norm()
                <= blackboard
                    .parameters
                    .ball
                    .interception
                    .minimum_ball_velocity);
    if blackboard.player_number == blackboard.parameters.goalkeeper.player_number
        && !should_block_ball(position, velocity, goal)
    {
        return Status::Failure;
    }
    if !settle {
        super::strategy::select_ball_action(blackboard);
    }
    let kick_orientation = (!settle)
        .then(|| super::execution::resolve_target(blackboard))
        .flatten()
        .filter(|_| {
            position.x() > 0.0
                && position.coords().norm()
                    <= 2.0 * blackboard.parameters.kicking.approach_ball_standoff
        })
        .and_then(|target| {
            (transform.inverse() * target - position)
                .try_normalize(f32::EPSILON)
                .map(Orientation2::from_vector)
        });
    let orientation = kick_orientation.unwrap_or_else(|| {
        Orientation2::from_vector(if hold_pass {
            position.coords()
        } else {
            -velocity
        })
    });
    let standoff = blackboard.parameters.kicking.approach_ball_standoff;
    let limits = blackboard.walking_velocity_limits;
    let walk_speed = blackboard.parameters.walking.speed.kicking;
    let walking = blackboard.parameters.walking.walk_and_stand;
    let hold_distance = if settle {
        // Tolerate lane noise while still moving a foot into a centered arrival.
        0.8 * RobotDimensions::ROBOT_TO_LEFT_PELVIS.y()
    } else {
        walking.target_reached_thresholds.x
    };
    let target = if hold_pass {
        Some(Point2::origin())
    } else if settle {
        let ball_in_receiving_frame = orientation.as_transform::<Ground>().inverse() * position;
        let foot_offset =
            RobotDimensions::ROBOT_TO_LEFT_PELVIS.y() * ball_in_receiving_frame.y().signum();
        // Put a foot in the incoming line instead of centering the gap between the legs.
        let contact = orientation.as_transform::<Ground>() * vector![standoff, foot_offset];
        // Hold the incoming line; advancing into the arrival makes a calm trap bounce away.
        let offset = position.coords() - contact;
        let direction = velocity.normalize();
        Some((offset - direction * offset.dot(&direction)).as_point())
    } else {
        blocking_target(
            position,
            velocity,
            (!pass_receiver).then_some(goal),
            standoff,
            limits,
            walk_speed,
            orientation,
        )
    };
    let Some(target) = target else {
        return Status::Failure;
    };
    let target_in_field = transform * target;
    let field = &blackboard.field_dimensions;
    if target_in_field.x().abs() >= field.length / 2.0 - field.ball_radius
        || target_in_field.y().abs() >= field.width / 2.0 - field.ball_radius
    {
        return Status::Failure;
    }
    blackboard.ball_action = None;
    blackboard.kick_target = None;
    let ball_in_receiving_frame = orientation.as_transform::<Ground>().inverse() * position;
    let contact_imminent = ball_in_receiving_frame.y().abs()
        <= blackboard.parameters.walking.path_planning.robot_radius / 2.0
            + blackboard.field_dimensions.ball_radius
        && ball_in_receiving_frame.x() <= standoff + velocity.norm() * 0.35;
    if target.coords().norm() <= hold_distance
        || (settle
            && (orientation.angle().abs() > walking.orientation_tolerance || contact_imminent))
    {
        // Keep Walk active while holding the lane so an arriving ball needs no Stand dwell.
        blackboard.body_motion = Some(BodyMotion::WalkWithVelocity {
            velocity: Vector2::zeros(),
            angular_velocity: limits.scale_angular_axis(
                (if settle {
                    Some(orientation)
                } else {
                    kick_orientation
                })
                .unwrap_or_else(|| Orientation2::from_vector(position.coords()))
                .angle(),
            ),
        });
        return Status::Success;
    }
    let path = plan_interception(blackboard, target, transform);
    let forward = path.forward(Point2::origin());
    if settle {
        blackboard.body_motion = Some(BodyMotion::WalkWithVelocity {
            velocity: crate::walk::adaptive_blocking_velocity(
                blackboard,
                forward * walk_speed.min(target.coords().norm() / 0.35),
                position,
            ),
            angular_velocity: limits.scale_angular_axis(orientation.angle()),
        });
        return Status::Success;
    }
    if forward.x() < 0.0 {
        // Turning and translation are independent; do not back into a rolling ball.
        blackboard.body_motion = Some(BodyMotion::WalkWithVelocity {
            velocity: Vector2::zeros(),
            angular_velocity: limits
                .scale_angular_axis(Orientation2::from_vector(position.coords()).angle()),
        });
        return Status::Success;
    }
    let kick_orientation =
        kick_orientation.filter(|direction| direction.as_unit_vector().dot(&forward) >= 0.0);
    blackboard.body_motion = Some(BodyMotion::Walk {
        path,
        orientation_mode: match kick_orientation {
            Some(direction) => OrientationMode::LookTowards {
                direction,
                tolerance: walking.orientation_tolerance,
            },
            None => OrientationMode::LookAt {
                target: position,
                tolerance: walking.orientation_tolerance,
            },
        },
        target_orientation: kick_orientation
            .unwrap_or_else(|| Orientation2::from_vector(position.coords())),
        distance_to_be_aligned: walking.normal_distance_to_be_aligned,
        speed: walk_speed.min(target.coords().norm() / 0.35),
    });
    Status::Success
}

fn should_block_ball(
    position: Point2<Ground>,
    velocity: Vector2<Ground>,
    own_goal: Point2<Ground>,
) -> bool {
    position.coords().dot(&velocity) < 0.0 || velocity.dot(&(own_goal - position)) > 0.0
}

fn blocking_target(
    position: Point2<Ground>,
    velocity: Vector2<Ground>,
    own_goal: Option<Point2<Ground>>,
    standoff: f32,
    limits: WalkingVelocityLimits,
    walk_speed: f32,
    orientation: Orientation2<Ground>,
) -> Option<Point2<Ground>> {
    if !position
        .inner
        .iter()
        .chain(velocity.inner.iter())
        .all(|value| value.is_finite())
        || velocity.norm_squared() <= f32::EPSILON
    {
        return None;
    }
    // Arrive behind the contact point facing the outgoing kick, avoiding a late turn.
    let contact_offset = -orientation.as_unit_vector() * standoff;
    let horizon = ball_prediction_time(velocity.norm(), MAXIMUM_BALL_PREDICTION_TIME);
    for step in 1..=(horizon / INTERCEPTION_TIME_STEP).ceil() as u32 {
        let time = (step as f32 * INTERCEPTION_TIME_STEP).min(horizon);
        let target = position + velocity * time + contact_offset;
        if travel_time(target, orientation, limits, walk_speed) <= time {
            return Some(target);
        }
    }
    // A fast ball can be unreachable. Still occupy its goal lane instead of chasing it.
    if let Some(to_goal) = own_goal.map(|goal| goal - position)
        && velocity.dot(&to_goal) > 0.0
    {
        let fraction = (-position.coords()).dot(&to_goal) / to_goal.norm_squared();
        if fraction > 0.0 {
            return Some(position + to_goal * fraction.min(0.9));
        }
    }
    let time = (-position.coords().dot(&velocity) / velocity.norm_squared()).clamp(0.0, horizon);
    (time > 0.0 || velocity.norm() < walk_speed)
        .then(|| position + velocity * time + contact_offset)
}

fn travel_time(
    target: Point2<Ground>,
    orientation: Orientation2<Ground>,
    limits: WalkingVelocityLimits,
    walk_speed: f32,
) -> f32 {
    let mut time = orientation.angle().abs() / limits.angular_velocity_limit.max(f32::EPSILON);
    for distance in [
        target.coords(),
        orientation.as_transform::<Ground>().inverse() * target.coords(),
    ] {
        let forward_limit = if distance.x() >= 0.0 {
            limits.forward_velocity_limits[1]
        } else {
            -limits.forward_velocity_limits[0]
        };
        time = time
            .max(distance.x().abs() / forward_limit.min(walk_speed).max(f32::EPSILON))
            .max(
                distance.y().abs()
                    / limits
                        .lateral_velocity_limit
                        .min(walk_speed)
                        .max(f32::EPSILON),
            );
    }
    time
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use hsl_network_messages::PlayerNumber;
    use linear_algebra::{point, vector};
    use types::{
        field_dimensions::{FieldDimensions, Side},
        motion_command::BodyMotion,
        obstacles::Obstacle,
        world_state::{BallSource, BallState},
    };

    use crate::node::LastBall;

    #[test]
    fn receiver_blocks_until_controlled_or_pressed_then_releases_quickly() {
        let mut blackboard = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            last_motion_type: Some(MotionType::Walk),
            ball: Some(LastBall {
                position: point![0.8, -0.7],
                velocity: vector![0.0, 1.0],
                age: Default::default(),
                field_side: Side::Right,
                source: BallSource::Own,
            }),
            ..Default::default()
        };
        blackboard.player_number = PlayerNumber::Two;
        blackboard.world_state.robot.ground_to_field = Some(Default::default());
        blackboard.parameters.goalkeeper.player_number = PlayerNumber::One;
        blackboard.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        blackboard
            .parameters
            .ball
            .interception
            .minimum_ball_velocity = 0.3;
        blackboard.parameters.walking.path_planning.robot_radius = 0.4;
        blackboard.parameters.kicking.kick_activation_distance = 1.3;
        blackboard.parameters.kicking.approach_ball_standoff = 0.3;
        blackboard.parameters.kicking.strategy.maximum_shot_distance = 6.0;
        blackboard.parameters.kicking.target_speed = 3.4;
        blackboard
            .parameters
            .kicking
            .strategy
            .passing
            .maximum_distance = 8.0;
        blackboard.world_state.player_states.three = Some(types::time_wrapper::TimeWrapper {
            time: blackboard.world_state.now,
            inner: types::world_state::PlayerState {
                pose: point![-2.0, 0.0].into(),
                pass: Some(hsl_network_messages::PassIntent {
                    receiver: PlayerNumber::Two,
                    target: Point2::origin(),
                }),
                ..crate::test_support::player_state()
            },
        });
        assert!(super::super::passing::is_pass_receiver(&mut blackboard));
        blackboard.parameters.kicking.allow_strong_kicks = true;
        blackboard
            .parameters
            .kicking
            .strong_kick_min_target_distance = 0.1;
        blackboard.walking_velocity_limits = WalkingVelocityLimits {
            forward_velocity_limits: [-0.8, 1.8],
            lateral_velocity_limit: 1.0,
            angular_velocity_limit: 2.0,
        };
        assert!(!is_ready_to_receive_kick(&blackboard));
        crate::tree::striker_subtree().tick_with_trace(&mut blackboard);
        assert!(
            matches!(blackboard.body_motion, Some(BodyMotion::WalkWithVelocity { velocity, angular_velocity })
            if velocity == Vector2::zeros() && angular_velocity < 0.0)
        );
        // Already in the lane: hold feet still facing the ball, even inside kick range.
        let foot_offset = RobotDimensions::ROBOT_TO_LEFT_PELVIS.y();
        blackboard.ball.as_mut().unwrap().position = point![foot_offset, -0.6];
        crate::tree::striker_subtree().tick_with_trace(&mut blackboard);
        assert!(
            matches!(blackboard.body_motion, Some(BodyMotion::WalkWithVelocity { velocity, .. })
            if velocity == Vector2::zeros())
        );
        blackboard.ball.as_mut().unwrap().position = point![0.1, foot_offset];
        blackboard.ball.as_mut().unwrap().velocity = vector![-1.0, 0.0];
        crate::tree::striker_subtree().tick_with_trace(&mut blackboard);
        assert!(
            matches!(blackboard.body_motion, Some(BodyMotion::WalkWithVelocity { velocity, .. })
            if velocity == Vector2::zeros())
        );

        // Far opponents do not rush the reception; the radius is configurable.
        blackboard.ball.as_mut().unwrap().position = point![0.3, -0.2];
        blackboard.ball.as_mut().unwrap().velocity = vector![0.0, 1.0];
        blackboard.world_state.obstacles = vec![Obstacle::robot(point![3.5, -0.2], 0.2, 0.2)];
        assert!(!is_ready_to_receive_kick(&blackboard));
        blackboard.world_state.obstacles[0].position = point![0.3, 0.6];
        blackboard.parameters.kicking.receiving_opponent_distance = 0.3;
        assert!(!is_ready_to_receive_kick(&blackboard));
        blackboard.parameters.kicking.receiving_opponent_distance = 2.0;
        assert!(is_ready_to_receive_kick(&blackboard));
        crate::tree::striker_subtree().tick_with_trace(&mut blackboard);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick {
                quick: true,
                strong: true,
                ..
            })
        ));

        // A controlled touch at the feet can continue without waiting for a complete stop.
        blackboard.world_state.obstacles.clear();
        blackboard.ball.as_mut().unwrap().position = point![0.4, 0.05];
        blackboard.ball.as_mut().unwrap().velocity = vector![-0.45, 0.0];
        assert!(is_ready_to_receive_kick(&blackboard));
        crate::tree::striker_subtree().tick_with_trace(&mut blackboard);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick { quick: true, ball_velocity, .. })
                if ball_velocity == vector![-0.45, 0.0]
        ));
        for (position, velocity) in [
            (point![0.4, 0.3], vector![-0.45, 0.0]),
            (point![0.8, 0.0], vector![-0.45, 0.0]),
            (point![0.4, 0.05], vector![-1.0, 0.0]),
        ] {
            let ball = blackboard.ball.as_mut().unwrap();
            ball.position = position;
            ball.velocity = velocity;
            assert!(!is_ready_to_receive_kick(&blackboard));
            assert_eq!(receive_rolling_ball(&mut blackboard), Status::Success);
        }

        // Settled possession also resumes the ordinary action immediately.
        blackboard.ball.as_mut().unwrap().position = point![0.4, 0.05];
        blackboard.ball.as_mut().unwrap().velocity = vector![0.0, 0.0];
        blackboard.world_state.now = blackboard.world_state.now + Duration::from_millis(1);
        crate::tree::striker_subtree().tick_with_trace(&mut blackboard);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick {
                strong: true,
                target_speed: 3.4,
                ..
            })
        ));
        for (position, velocity) in [
            (point![1.4, 0.0], vector![-0.5, 0.0]),
            (point![0.8, 0.7], vector![0.4, 0.3]),
            (point![-0.5, -0.7], vector![0.3, 0.5]),
        ] {
            let ball = blackboard.ball.as_mut().unwrap();
            ball.position = position;
            ball.velocity = velocity;
            assert!(!is_ready_to_receive_kick(&blackboard));
        }
        blackboard.player_number = PlayerNumber::One;
        blackboard.ball.as_mut().unwrap().position = point![0.8, -0.7];
        blackboard.ball.as_mut().unwrap().velocity = vector![0.0, 1.0];
        assert!(!is_ready_to_receive_kick(&blackboard));
        blackboard.ball.as_mut().unwrap().position = point![0.3, -0.2];
        assert!(is_ready_to_receive_kick(&blackboard));
    }

    fn addressed_receiver() -> Blackboard {
        let mut board = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            last_motion_type: Some(MotionType::Walk),
            ball: Some(LastBall {
                position: point![2.0, 0.5],
                velocity: vector![-1.0, 0.0],
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            }),
            walking_velocity_limits: WalkingVelocityLimits {
                forward_velocity_limits: [-0.8, 1.8],
                lateral_velocity_limit: 1.0,
                angular_velocity_limit: 2.0,
            },
            ..Default::default()
        };
        board.player_number = PlayerNumber::Two;
        board.world_state.robot.ground_to_field = Some(Default::default());
        board.parameters.goalkeeper.player_number = PlayerNumber::One;
        board.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        board.parameters.ball.interception.minimum_ball_velocity = 0.3;
        board.parameters.kicking.kick_activation_distance = 1.3;
        board.parameters.kicking.approach_ball_standoff = 0.3;
        board.parameters.kicking.receiving_opponent_distance = 1.0;
        board.parameters.walking.path_planning.robot_radius = 0.4;
        board.parameters.walking.speed.kicking = 1.8;
        board.parameters.walking.speed.minimum_speed = 0.2;
        board
            .parameters
            .walking
            .walk_and_stand
            .orientation_tolerance = 0.2;
        board.parameters.kicking.strategy.passing.maximum_distance = 8.0;
        board.world_state.player_states.three = Some(types::time_wrapper::TimeWrapper {
            time: board.world_state.now,
            inner: types::world_state::PlayerState {
                pose: point![-2.0, 0.0].into(),
                pass: Some(hsl_network_messages::PassIntent {
                    receiver: PlayerNumber::Two,
                    target: Point2::origin(),
                }),
                ..crate::test_support::player_state()
            },
        });
        board
    }

    #[test]
    fn addressed_pass_aligns_a_foot_lane_without_advancing_into_the_arrival() {
        let mut board = addressed_receiver();
        for side in [-1.0, 1.0] {
            board.ball.as_mut().unwrap().position = point![2.0, side * 0.5];
            board.world_state.obstacles = vec![Obstacle::robot(point![2.0, side * 0.5], 0.2, 0.2)];
            assert!(crate::conditions::opponent_near_ball(&board, 1.0));
            assert!(!is_receiver_under_pressure(&board));
            assert!(super::super::passing::is_pass_receiver(&mut board));
            assert_eq!(receive_rolling_ball(&mut board), Status::Success);
            assert!(matches!(board.body_motion,
                Some(BodyMotion::WalkWithVelocity { velocity, .. })
                if velocity.x().abs() < 1e-5 && velocity.y() * side > 0.0));

            let foot_offset = side * RobotDimensions::ROBOT_TO_LEFT_PELVIS.y();
            board.ball.as_mut().unwrap().position = point![0.4, foot_offset];
            assert_eq!(receive_rolling_ball(&mut board), Status::Success);
            assert!(matches!(board.body_motion,
                Some(BodyMotion::WalkWithVelocity { velocity, .. })
                if velocity == Vector2::zeros()));

            // Modest lane noise must not start another step into the arrival.
            board.ball.as_mut().unwrap().position = point![2.0, side * 0.17];
            assert_eq!(receive_rolling_ball(&mut board), Status::Success);
            assert!(matches!(board.body_motion,
                Some(BodyMotion::WalkWithVelocity { velocity, .. })
                if velocity == Vector2::zeros()));
        }
        // The middle-sized ball must not make the hold tolerance wider than the foot offset.
        board.field_dimensions.ball_radius = 0.105;
        board
            .parameters
            .walking
            .walk_and_stand
            .target_reached_thresholds
            .x = 0.05;
        board.ball.as_mut().unwrap().position = point![2.0, 0.0];
        assert_eq!(receive_rolling_ball(&mut board), Status::Success);
        assert!(matches!(board.body_motion,
            Some(BodyMotion::WalkWithVelocity { velocity, .. })
            if velocity.x().abs() < 1e-5 && velocity.y().abs() > 0.0));
    }

    #[test]
    fn addressed_receiver_stops_translation_during_turns_and_contact() {
        let mut board = addressed_receiver();
        for angle in [-1.0, -0.35, 0.35, 1.0] {
            let rotation = Orientation2::new(angle).as_transform::<Ground>();
            board.ball.as_mut().unwrap().position = rotation * point![2.0, 0.5];
            board.ball.as_mut().unwrap().velocity = rotation * vector![-1.0, 0.0];
            assert_eq!(receive_rolling_ball(&mut board), Status::Success);
            assert!(matches!(board.body_motion,
                Some(BodyMotion::WalkWithVelocity { velocity, angular_velocity })
                if velocity == Vector2::zeros() && angular_velocity * angle > 0.0));
        }

        // Brake for contact even when the remaining turn is inside the alignment tolerance.
        for angle in [-0.15, 0.0, 0.15, std::f32::consts::PI] {
            let rotation = Orientation2::new(angle).as_transform::<Ground>();
            board.ball.as_mut().unwrap().position = rotation * point![0.55, 0.19];
            board.ball.as_mut().unwrap().velocity = rotation * vector![-1.0, 0.0];
            assert_eq!(receive_rolling_ball(&mut board), Status::Success);
            assert!(matches!(board.body_motion,
                Some(BodyMotion::WalkWithVelocity { velocity, .. })
                if velocity == Vector2::zeros()));
        }

        // A ball passing outside the body still needs a sidestep, even near the contact plane.
        board.ball.as_mut().unwrap().position = point![0.4, 0.5];
        board.ball.as_mut().unwrap().velocity = vector![-1.0, 0.0];
        assert_eq!(receive_rolling_ball(&mut board), Status::Success);
        assert!(matches!(board.body_motion,
            Some(BodyMotion::WalkWithVelocity { velocity, .. })
            if velocity.x().abs() < 1e-5 && velocity.y() > 0.0));
    }

    #[test]
    fn addressed_receiver_holds_near_stop_and_receding_passes_until_controlled() {
        let mut board = addressed_receiver();
        for speed in [-0.31, -0.29, 0.0, 0.29, 1.0] {
            board.ball.as_mut().unwrap().position = point![0.8, 0.17];
            board.ball.as_mut().unwrap().velocity = vector![speed, 0.0];
            assert!(is_settling_pass(&mut board));
            assert_eq!(receive_rolling_ball(&mut board), Status::Success);
            assert!(matches!(board.body_motion,
                Some(BodyMotion::WalkWithVelocity { velocity, .. })
                if velocity == Vector2::zeros()));
        }

        // Controlled traps hand off for an outgoing turn, including below the rolling threshold.
        board.world_state.robot.ground_to_field = Some(linear_algebra::Isometry2::from_parts(
            vector![0.0, 0.0],
            std::f32::consts::PI,
        ));
        board.parameters.kicking.strategy.maximum_shot_distance = 6.0;
        for speed in [0.45, 0.29, 0.0] {
            board.ball.as_mut().unwrap().position = point![-0.4, -0.05];
            board.ball.as_mut().unwrap().velocity = vector![speed, 0.0];
            assert!(!is_ready_to_receive_kick(&board));
            assert!(is_settling_pass(&mut board));
            assert_eq!(receive_rolling_ball(&mut board), Status::Failure);
            assert!(
                super::super::execution::resolve_target(&board).is_some_and(|target| {
                    let transform = board.world_state.robot.ground_to_field.unwrap();
                    (transform.inverse() * target).x() < 0.0
                })
            );
        }
    }

    #[test]
    fn loose_ball_uses_quick_deflection_without_waiting_for_shot_or_walk_dwell() {
        let mut board = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            last_motion_type: Some(MotionType::Walk),
            ball: Some(LastBall {
                position: point![1.7, 0.05],
                velocity: vector![-2.0, 0.0],
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            }),
            ..Default::default()
        };
        board.player_number = PlayerNumber::Two;
        board.parameters.goalkeeper.player_number = PlayerNumber::One;
        board.world_state.robot.ground_to_field = Some(Default::default());
        board.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        board.parameters.ball.interception.minimum_ball_velocity = 0.3;
        board.parameters.kicking.kick_activation_distance = 1.3;
        board.parameters.kicking.approach_ball_standoff = 0.3;
        board.parameters.walking.path_planning.robot_radius = 0.4;
        board.parameters.control.allow_switch.walk = Duration::from_secs(1);
        board.parameters.control.allow_switch.kick = Duration::from_secs(1);
        assert!(!board.parameters.kicking.quick);
        for _ in 0..2 {
            assert_eq!(receive_rolling_ball(&mut board), Status::Success);
            assert!(matches!(board.body_motion,
                Some(BodyMotion::Kick { quick: true, strong: false, soft: false,
                    ball_position, ball_velocity, .. })
                if ball_position == point![1.7, 0.05] && ball_velocity == vector![-2.0, 0.0]));
            assert!(board.ball_action.is_none());
            board.last_motion_command =
                Some(types::motion_command::MotionCommand::from_partial_motions(
                    board.body_motion.clone().unwrap(),
                    board.head_motion.unwrap(),
                ));
            board.last_motion_type = Some(MotionType::Kick);
        }
        for (position, velocity) in [
            (point![1.7, 0.6], vector![-2.0, 0.0]),
            (point![1.7, 0.05], vector![2.0, 0.0]),
            (point![2.01, 0.05], vector![-6.0, 0.0]),
        ] {
            let mut missed = board.clone();
            missed.last_motion_type = Some(MotionType::Walk);
            missed.body_motion = None;
            missed.ball.as_mut().unwrap().position = position;
            missed.ball.as_mut().unwrap().velocity = velocity;
            receive_rolling_ball(&mut missed);
            assert!(!matches!(missed.body_motion, Some(BodyMotion::Kick { .. })));
        }
    }

    #[test]
    fn side_arrivals_use_quick_deflection_only_inside_existing_contact_window() {
        let mut board = addressed_receiver();
        board.world_state.player_states.three = None;
        for side in [-1.0, 1.0] {
            let position = point![0.3, side * 0.6];
            let velocity = vector![0.0, -side * 2.5];
            board.last_motion_type = Some(MotionType::Walk);
            board.ball.as_mut().unwrap().position = position;
            board.ball.as_mut().unwrap().velocity = velocity;
            assert_eq!(receive_rolling_ball(&mut board), Status::Success);
            assert!(matches!(board.body_motion,
                Some(BodyMotion::Kick { ball_position, ball_velocity, kick_direction,
                    quick: true, strong: false, soft: false, .. })
                if ball_position == board.world_state.robot.ground_to_field.unwrap().inverse() * position
                    && ball_velocity == board.world_state.robot.ground_to_field.unwrap().inverse() * velocity
                    && kick_direction == Orientation2::default()));
            for missed in [
                point![0.8, side * 0.6],
                point![0.3, -side * 0.6],
                point![0.3, side * 0.9],
            ] {
                board.body_motion = None;
                board.ball.as_mut().unwrap().position = missed;
                receive_rolling_ball(&mut board);
                assert!(!matches!(board.body_motion, Some(BodyMotion::Kick { .. })));
            }
        }

        let mut addressed = addressed_receiver();
        addressed.ball.as_mut().unwrap().position = point![0.3, -0.6];
        addressed.ball.as_mut().unwrap().velocity = vector![0.0, 2.5];
        assert!(super::super::passing::is_pass_receiver(&mut addressed));
        assert_eq!(receive_rolling_ball(&mut addressed), Status::Success);
        assert!(matches!(addressed.body_motion,
            Some(BodyMotion::WalkWithVelocity { velocity, .. }) if velocity == Vector2::zeros()));
    }

    #[test]
    fn side_arriving_keeper_clearance_is_excluded_from_deflection() {
        let mut board = addressed_receiver();
        board.world_state.player_states.three = None;
        let transform =
            linear_algebra::Isometry2::from_parts(vector![-3.0, -0.3], std::f32::consts::FRAC_PI_2);
        board.world_state.robot.ground_to_field = Some(transform);
        board.ball.as_mut().unwrap().position = point![-3.6, 0.0];
        board.ball.as_mut().unwrap().velocity = vector![2.5, 0.0];
        board
            .parameters
            .kicking
            .strategy
            .passing
            .maximum_receiver_age = Duration::from_secs(3);
        board.world_state.player_states.one = Some(types::time_wrapper::TimeWrapper {
            time: board.world_state.now,
            inner: types::world_state::PlayerState {
                pose: point![-4.2, 0.0].into(),
                ..crate::test_support::player_state()
            },
        });
        let ball = board.ball.as_ref().unwrap();
        assert!(is_on_contact_path(
            &board,
            transform.inverse() * ball.position,
            transform.inverse() * ball.velocity
        ));
        assert!(goalkeeper_clearance_direction(&board).is_some());
        assert!(!is_receiving_ball(&board));
        assert_eq!(receive_rolling_ball(&mut board), Status::Failure);
        assert!(!matches!(board.body_motion, Some(BodyMotion::Kick { .. })));
    }

    #[test]
    fn walks_to_incoming_ball_and_hands_off_close_ball() {
        let mut blackboard = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            last_closest_to_ball: true,
            ball: Some(LastBall {
                position: point![2.0, 0.3],
                velocity: vector![-2.0, 0.0],
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            }),
            ..Default::default()
        };
        blackboard.world_state.robot.ground_to_field = Some(Default::default());
        blackboard.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        blackboard.parameters.kicking.approach_ball_standoff = 0.2;
        blackboard.parameters.kicking.kick_activation_distance = 0.75;
        blackboard.walking_velocity_limits.forward_velocity_limits = [-0.5, 0.5];
        blackboard.walking_velocity_limits.lateral_velocity_limit = 0.5;
        assert_eq!(receive_rolling_ball(&mut blackboard), Status::Success);
        let Some(BodyMotion::Walk {
            target_orientation,
            orientation_mode: OrientationMode::LookAt { target, .. },
            ..
        }) = blackboard.body_motion
        else {
            panic!("expected ball-facing interception")
        };
        assert_eq!(target, point![2.0, 0.3]);
        assert!(
            (target_orientation.angle() - Orientation2::from_vector(target.coords()).angle()).abs()
                < 1e-6
        );

        blackboard.ball.as_mut().unwrap().position = point![0.5, 0.0];
        assert_eq!(receive_rolling_ball(&mut blackboard), Status::Failure);
        blackboard.ball.as_mut().unwrap().position = point![2.0, 0.3];
        blackboard.ball.as_mut().unwrap().velocity = vector![2.0, 0.0];
        assert_eq!(receive_rolling_ball(&mut blackboard), Status::Failure);
        blackboard.ball.as_mut().unwrap().velocity = vector![-2.0, 0.0];
        blackboard.world_state.now = blackboard.world_state.now + Duration::from_secs(2);
        assert_eq!(receive_rolling_ball(&mut blackboard), Status::Failure);

        blackboard.ball.as_mut().unwrap().age = blackboard.world_state.now;
        blackboard.last_motion_type = Some(MotionType::Kick);
        blackboard.last_motion_switch_time = blackboard.world_state.now;
        blackboard.parameters.control.allow_switch.kick = Duration::from_secs(1);
        assert_eq!(receive_rolling_ball(&mut blackboard), Status::Failure);
        blackboard.world_state.now = blackboard.world_state.now + Duration::from_millis(1001);
        blackboard.ball.as_mut().unwrap().age = blackboard.world_state.now;
        assert_eq!(receive_rolling_ball(&mut blackboard), Status::Success);
        blackboard.last_motion_type = Some(MotionType::Walk);
        blackboard.last_closest_to_ball = false;
        assert_eq!(receive_rolling_ball(&mut blackboard), Status::Success);
    }

    #[test]
    fn hands_off_incoming_ball_directly_to_quick_goalward_kick() {
        let mut blackboard = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            last_closest_to_ball: true,
            last_motion_type: Some(MotionType::Walk),
            ball: Some(LastBall {
                position: point![1.0, 0.15],
                velocity: vector![-2.0, 0.0],
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            }),
            ..Default::default()
        };
        blackboard.world_state.robot.ground_to_field = Some(Default::default());
        blackboard.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        blackboard.parameters.kicking.kick_activation_distance = 1.3;
        blackboard.parameters.kicking.quick = true;
        blackboard.parameters.walking.path_planning.robot_radius = 0.4;
        blackboard.world_state.ball = Some(BallState {
            ball_in_ground: point![1.0, 0.15],
            ball_in_ground_velocity: vector![-2.0, 0.0],
            ..types::world_state::BallState::new_at_center(Default::default())
        });
        blackboard.parameters.kicking.strategy.maximum_shot_distance = 6.0;
        blackboard.parameters.control.allow_switch.walk = Duration::from_secs(1);
        assert_eq!(receive_rolling_ball(&mut blackboard), Status::Failure);
        assert_eq!(
            crate::tree::striker_subtree()
                .tick_with_trace(&mut blackboard)
                .0,
            Status::Success
        );
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick { quick: true, ball_velocity, .. })
                if ball_velocity == vector![-2.0, 0.0]
        ));
        assert!(blackboard.kick_target.unwrap().x() > 1.0);
        assert!(blackboard.ball_action.is_some());

        blackboard.last_motion_command =
            Some(types::motion_command::MotionCommand::from_partial_motions(
                blackboard.body_motion.clone().unwrap(),
                blackboard.head_motion.unwrap(),
            ));
        blackboard.last_motion_type = Some(MotionType::Kick);
        blackboard.ball.as_mut().unwrap().velocity = vector![-0.2, 0.0];
        blackboard
            .world_state
            .ball
            .as_mut()
            .unwrap()
            .ball_in_ground_velocity = vector![-0.2, 0.0];
        crate::tree::striker_subtree().tick_with_trace(&mut blackboard);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick { quick: true, .. })
        ));
    }

    #[test]
    fn meets_a_diagonal_trajectory_before_closest_approach() {
        let limits = WalkingVelocityLimits {
            forward_velocity_limits: [-0.5, 0.5],
            lateral_velocity_limit: 0.5,
            angular_velocity_limit: 2.0,
        };
        let position = point![2.0, 0.7];
        let velocity = vector![-1.0, -0.3];
        let direction = velocity.normalize();
        let target = blocking_target(
            position,
            velocity,
            Some(point![-4.5, 0.0]),
            0.2,
            limits,
            1.0,
            Orientation2::from_vector(-direction),
        )
        .unwrap();
        let time = (target.x() - position.x() - direction.x() * 0.2) / velocity.x();
        assert!(time > 0.0 && time < 2.0);
        assert!((target - (position + velocity * time + direction * 0.2)).norm() < 1e-5);
        assert!(travel_time(target, Orientation2::from_vector(-direction), limits, 1.0) <= time);
    }

    #[test]
    fn occupies_goal_lane_when_a_fast_crossing_ball_is_unreachable() {
        let position = point![2.0, 2.0];
        let goal = point![-4.5, 0.0];
        let limits = WalkingVelocityLimits {
            forward_velocity_limits: [-0.2, 0.2],
            lateral_velocity_limit: 0.2,
            angular_velocity_limit: 1.0,
        };
        let target = blocking_target(
            position,
            vector![-3.0, 0.0],
            Some(goal),
            0.2,
            limits,
            1.0,
            Orientation2::identity(),
        )
        .unwrap();
        let lane = goal - position;
        let fraction = (target - position).dot(&lane) / lane.norm_squared();
        assert!((0.0..1.0).contains(&fraction));
        assert!((target - (position + lane * fraction)).norm() < 1e-5);
        assert!(target.y() < position.y());

        let reception = blocking_target(
            position,
            vector![-3.0, 0.0],
            None,
            0.2,
            limits,
            1.0,
            Orientation2::identity(),
        )
        .unwrap();
        assert_eq!(reception.y(), position.y());
        assert!((reception.x() + 0.2).abs() < 1e-5);
    }

    #[test]
    fn fast_distant_ball_cannot_pull_reception_target_beyond_two_meters() {
        let position = point![6.0, 2.0];
        for speed in [4.0, 20.0] {
            let target = blocking_target(
                position,
                vector![-speed, 0.0],
                None,
                0.3,
                WalkingVelocityLimits::default(),
                1.0,
                Orientation2::identity(),
            )
            .unwrap();
            approx::assert_relative_eq!(
                target + vector![0.3, 0.0],
                point![4.0, 2.0],
                epsilon = 1e-5
            );
        }
    }

    #[test]
    fn gets_goal_side_of_a_slow_receding_ball_when_reachable() {
        let position = point![-1.0, 0.0];
        let limits = WalkingVelocityLimits {
            forward_velocity_limits: [-0.8, 1.8],
            lateral_velocity_limit: 1.0,
            angular_velocity_limit: 2.0,
        };
        let target = blocking_target(
            position,
            vector![-0.4, 0.0],
            Some(point![-4.5, 0.0]),
            0.2,
            limits,
            1.8,
            Orientation2::identity(),
        )
        .unwrap();
        assert!(target.x() < position.x());
        assert!(target.x() > -4.5);
        assert!(target.y().abs() < 1e-6);
    }

    #[test]
    fn pursues_present_ball_when_meeting_it_would_require_a_long_forecast() {
        let position = point![1.0, 0.0];
        let velocity = vector![0.4, 0.0];
        let limits = WalkingVelocityLimits {
            forward_velocity_limits: [-0.8, 1.8],
            lateral_velocity_limit: 1.0,
            angular_velocity_limit: 2.0,
        };
        let orientation = Orientation2::from_vector(-velocity);
        let target = blocking_target(
            position,
            velocity,
            Some(point![-4.5, 0.0]),
            0.3,
            limits,
            1.8,
            orientation,
        )
        .unwrap();
        approx::assert_relative_eq!(target, position + vector![0.3, 0.0], epsilon = 1e-5);

        assert!(
            blocking_target(
                position,
                vector![2.0, 0.0],
                Some(point![-4.5, 0.0]),
                0.3,
                limits,
                1.8,
                orientation,
            )
            .is_none()
        );
    }

    #[test]
    fn striker_meets_predicted_path_even_with_opponent_at_the_ball() {
        let mut board = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            player_number: PlayerNumber::Two,
            last_motion_type: Some(MotionType::Walk),
            ball: Some(LastBall {
                position: point![2.0, 0.7],
                velocity: vector![-0.5, 0.0],
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            }),
            walking_velocity_limits: WalkingVelocityLimits {
                forward_velocity_limits: [-0.8, 1.8],
                lateral_velocity_limit: 1.0,
                angular_velocity_limit: 2.0,
            },
            ..Default::default()
        };
        board.world_state.robot.ground_to_field = Some(Default::default());
        board.parameters.goalkeeper.player_number = PlayerNumber::One;
        board.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        board.parameters.ball.interception.minimum_ball_velocity = 0.3;
        board.parameters.kicking.strategy.pressure_distance = 0.8;
        board.parameters.kicking.receiving_opponent_distance = 1.0;
        board.parameters.kicking.kick_activation_distance = 1.3;
        board.parameters.kicking.approach_ball_standoff = 0.3;
        board.parameters.walking.path_planning.robot_radius = 0.4;
        board.parameters.walking.speed.kicking = 1.8;
        for contested in [false, true] {
            board.world_state.obstacles = if contested {
                vec![Obstacle::robot(point![2.1, 0.8], 0.2, 0.2)]
            } else {
                vec![]
            };
            assert_eq!(
                crate::conditions::opponent_near_ball(&board, 0.8),
                contested
            );
            assert!(!is_receiver_under_pressure(&board));
            assert_eq!(receive_rolling_ball(&mut board), Status::Success);
            let Some(BodyMotion::Walk {
                ref path, speed, ..
            }) = board.body_motion
            else {
                panic!("expected active interception, not a passive trap")
            };
            let types::path::PathSegment::LineSegment(segment) = path.last_segment() else {
                panic!("expected final approach to predicted contact")
            };
            let target = segment.1;
            let ball = board.ball.as_ref().unwrap();
            let transform = board.world_state.robot.ground_to_field.unwrap().inverse();
            let position = transform * ball.position;
            let velocity = transform * ball.velocity;
            let time = (target.x() - position.x() + 0.3) / velocity.x();
            assert!(time > 0.0 && time <= MAXIMUM_BALL_PREDICTION_TIME);
            assert!(target.x() > 0.0 && target.x() < position.x() - 0.3);
            assert!((target + vector![0.3, 0.0] - position - velocity * time).norm() < 1e-5);
            assert!(
                travel_time(
                    target,
                    Orientation2::identity(),
                    board.walking_velocity_limits,
                    speed
                ) <= time + 1e-5
            );
        }
    }

    #[test]
    fn overshot_interception_turns_toward_observed_ball_before_approaching() {
        let mut blackboard = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            last_motion_type: Some(MotionType::Walk),
            ball: Some(LastBall {
                position: point![-0.5, 1.0],
                velocity: vector![0.6, -2.0],
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            }),
            walking_velocity_limits: WalkingVelocityLimits {
                forward_velocity_limits: [-0.8, 1.8],
                lateral_velocity_limit: 1.0,
                angular_velocity_limit: 2.0,
            },
            ..Default::default()
        };
        blackboard.world_state.robot.ground_to_field = Some(Default::default());
        blackboard.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        blackboard.parameters.kicking.approach_ball_standoff = 0.3;
        blackboard.parameters.walking.speed.kicking = 1.8;
        assert_eq!(receive_rolling_ball(&mut blackboard), Status::Success);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::WalkWithVelocity { velocity, angular_velocity })
                if velocity == Vector2::zeros() && angular_velocity > 0.0
        ));
    }

    #[test]
    fn rejects_receding_balls_without_a_useful_forward_block() {
        for (position, velocity) in [
            (point![2.0, 0.0], vector![2.0, 0.0]),
            (point![-2.0, 0.0], vector![-2.0, 0.0]),
            (point![f32::NAN, 0.0], vector![-2.0, 0.0]),
        ] {
            assert!(
                blocking_target(
                    position,
                    velocity,
                    Some(point![-4.5, 0.0]),
                    0.2,
                    WalkingVelocityLimits::default(),
                    1.0,
                    Orientation2::from_vector(-velocity),
                )
                .is_none()
            );
        }
    }

    #[test]
    fn slow_crossing_ball_keeps_walking_until_facing_its_path() {
        let mut blackboard = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            last_motion_type: Some(MotionType::Walk),
            ball: Some(LastBall {
                position: point![0.8, 0.7],
                velocity: vector![-0.4, -0.3],
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            }),
            walking_velocity_limits: WalkingVelocityLimits {
                forward_velocity_limits: [-0.5, 0.5],
                lateral_velocity_limit: 0.5,
                angular_velocity_limit: 1.0,
            },
            ..Default::default()
        };
        blackboard.world_state.robot.ground_to_field = Some(Default::default());
        blackboard.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        blackboard
            .parameters
            .ball
            .interception
            .minimum_ball_velocity = 0.3;
        blackboard.parameters.kicking.kick_activation_distance = 1.3;
        blackboard.parameters.kicking.approach_ball_standoff = 0.2;
        blackboard.parameters.walking.path_planning.robot_radius = 0.4;
        assert!(is_receiving_ball(&blackboard));
        assert!(!is_ready_to_receive_kick(&blackboard));
        assert_eq!(receive_rolling_ball(&mut blackboard), Status::Success);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Walk { .. })
        ));

        blackboard.ball.as_mut().unwrap().position = point![0.8, 0.05];
        blackboard.ball.as_mut().unwrap().velocity = vector![-0.5, 0.0];
        assert!(is_ready_to_receive_kick(&blackboard));
        assert_eq!(receive_rolling_ball(&mut blackboard), Status::Failure);
    }

    #[test]
    fn crossing_ball_approach_turns_toward_observed_ball_instead_of_outgoing_shot() {
        let mut blackboard = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            last_motion_type: Some(MotionType::Walk),
            ball: Some(LastBall {
                position: point![0.8, -0.6],
                velocity: vector![0.0, 1.0],
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            }),
            walking_velocity_limits: WalkingVelocityLimits {
                forward_velocity_limits: [-0.5, 0.8],
                lateral_velocity_limit: 0.5,
                angular_velocity_limit: 2.0,
            },
            ..Default::default()
        };
        blackboard.world_state.robot.ground_to_field = Some(Default::default());
        blackboard.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        blackboard.parameters.kicking.strategy.maximum_shot_distance = 6.0;
        blackboard.parameters.kicking.approach_ball_standoff = 0.3;
        blackboard.parameters.kicking.kick_activation_distance = 1.3;
        blackboard.parameters.walking.speed.kicking = 1.0;
        assert_eq!(receive_rolling_ball(&mut blackboard), Status::Success);
        let Some(BodyMotion::WalkWithVelocity {
            velocity,
            angular_velocity,
        }) = blackboard.body_motion
        else {
            panic!("expected observed-ball alignment")
        };
        assert_eq!(velocity, Vector2::zeros());
        let bearing = Orientation2::<Ground>::from_vector(vector![0.8, -0.6]);
        assert_eq!(
            angular_velocity,
            blackboard
                .walking_velocity_limits
                .scale_angular_axis(bearing.angle())
        );
    }

    #[test]
    fn raw_visual_percept_cannot_supply_a_mixed_reception_state() {
        let mut blackboard = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            visual_kick_ball_position: Some(types::ball_position::BallPosition {
                position: point![0.5, 0.05],
                velocity: Vector2::zeros(),
                last_seen: Default::default(),
            }),
            ..Default::default()
        };
        blackboard.world_state.robot.ground_to_field = Some(Default::default());
        blackboard.world_state.ball = Some(BallState {
            ball_in_ground: point![0.5, 0.05],
            ball_in_ground_velocity: vector![-0.5, 0.0],
            ..types::world_state::BallState::new_at_center(Default::default())
        });
        blackboard.parameters.kicking.kick_activation_distance = 1.3;
        assert!(!is_receiving_ball(&blackboard));
        assert!(!is_ready_to_receive_kick(&blackboard));

        blackboard.parameters.ball.last_ball_timeout = Duration::from_millis(100);
        blackboard.ball = Some(LastBall {
            position: point![0.5, 0.05],
            velocity: vector![-0.5, 0.0],
            age: blackboard.world_state.now,
            field_side: Side::Left,
            source: BallSource::Own,
        });
        assert!(is_receiving_ball(&blackboard));
        assert!(is_ready_to_receive_kick(&blackboard));

        blackboard.world_state.now = blackboard.world_state.now + Duration::from_millis(101);
        assert!(!is_receiving_ball(&blackboard));
        assert!(!is_ready_to_receive_kick(&blackboard));
    }

    #[test]
    fn fast_incoming_ball_starts_quick_kick_with_preparation_lead() {
        let mut blackboard = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            last_motion_type: Some(MotionType::Walk),
            ball: Some(LastBall {
                position: point![1.7, 0.05],
                velocity: vector![-2.0, 0.0],
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            }),
            ..Default::default()
        };
        blackboard.world_state.robot.ground_to_field = Some(Default::default());
        blackboard.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        blackboard.parameters.kicking.kick_activation_distance = 1.3;
        blackboard.parameters.kicking.quick = true;
        blackboard.parameters.kicking.approach_ball_standoff = 0.3;
        blackboard.parameters.kicking.strategy.maximum_shot_distance = 6.0;
        blackboard.parameters.walking.path_planning.robot_radius = 0.4;
        assert!(is_ready_to_receive_kick(&blackboard));
        assert_eq!(receive_rolling_ball(&mut blackboard), Status::Failure);
        crate::tree::striker_subtree().tick_with_trace(&mut blackboard);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick { quick: true, ball_position, ball_velocity, .. })
                if ball_position == point![1.7, 0.05] && ball_velocity == vector![-2.0, 0.0]
        ));

        blackboard.ball.as_mut().unwrap().velocity = vector![-0.5, 0.0];
        blackboard.ball.as_mut().unwrap().position = point![1.4, 0.05];
        assert!(!is_ready_to_receive_kick(&blackboard));
        blackboard.ball.as_mut().unwrap().velocity = vector![-6.0, 0.0];
        blackboard.ball.as_mut().unwrap().position = point![2.01, 0.05];
        assert!(!is_ready_to_receive_kick(&blackboard));
        blackboard.ball.as_mut().unwrap().position = point![1.7, 0.6];
        assert!(!is_ready_to_receive_kick(&blackboard));
    }

    #[test]
    fn prepares_for_side_arrival_before_it_crosses_the_feet() {
        let mut blackboard = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            last_motion_type: Some(MotionType::Walk),
            ball: Some(LastBall {
                position: point![0.3, -0.6],
                velocity: vector![0.0, 2.5],
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            }),
            ..Default::default()
        };
        blackboard.world_state.robot.ground_to_field = Some(Default::default());
        blackboard.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        blackboard.parameters.kicking.kick_activation_distance = 1.3;
        blackboard.parameters.kicking.approach_ball_standoff = 0.3;
        blackboard.parameters.kicking.strategy.maximum_shot_distance = 6.0;
        blackboard.parameters.walking.path_planning.robot_radius = 0.4;
        assert!(is_ready_to_receive_kick(&blackboard));
        assert_eq!(receive_rolling_ball(&mut blackboard), Status::Failure);
        crate::tree::striker_subtree().tick_with_trace(&mut blackboard);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick { quick: false, .. })
        ));
        for position in [point![0.8, -0.6], point![0.3, 0.6], point![0.3, -0.9]] {
            blackboard.ball.as_mut().unwrap().position = position;
            assert!(!is_ready_to_receive_kick(&blackboard));
        }
    }
}
