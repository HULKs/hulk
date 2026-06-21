use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use coordinate_systems::{Ground, World};
use linear_algebra::{Isometry2, Orientation2, Point2, Vector2};
use types::{
    motion_command::{HeadMotion, KickPower, MotionCommand, OrientationMode},
    path::PathSegment,
};

use crate::behavior_tree_simulator::{
    SimulatedBall, SimulationConfig, SimulatorBall, SimulatorClock, SimulatorFallDownState,
    SimulatorGroundToWorld, SimulatorHeadYaw, SimulatorLastKickTime, SimulatorRobot,
    SimulatorRobotFrames,
};

pub(crate) fn apply_motion_kinematics(
    clock: Res<SimulatorClock>,
    config: Res<SimulationConfig>,
    robot_frames: Res<SimulatorRobotFrames>,
    mut ball: ResMut<SimulatorBall>,
    mut robots: Query<(
        &SimulatorRobot,
        &mut SimulatorGroundToWorld,
        &mut SimulatorHeadYaw,
        &mut SimulatorFallDownState,
        &mut SimulatorLastKickTime,
    )>,
) {
    for (robot, mut ground_to_world, mut head_yaw, mut fall_down_state, mut last_kick_time) in
        &mut robots
    {
        let Some(frame) = robot_frames.0.get(&robot.player_number) else {
            continue;
        };

        match &frame.motion_command {
            MotionCommand::Walk {
                path,
                orientation_mode,
                target_orientation,
                speed,
                ..
            } => {
                let target = first_path_target(path).unwrap_or_else(Point2::origin);
                ground_to_world.ground_to_world = apply_walk_to_pose(
                    ground_to_world.ground_to_world,
                    target,
                    *target_orientation,
                    *orientation_mode,
                    *speed,
                    clock.tick_duration,
                    &config,
                );
            }
            MotionCommand::WalkWithVelocity {
                velocity,
                angular_velocity,
                ..
            } => {
                ground_to_world.ground_to_world = apply_walk_with_velocity_to_pose(
                    ground_to_world.ground_to_world,
                    *velocity,
                    *angular_velocity,
                    clock.tick_duration,
                    &config,
                );
            }
            MotionCommand::VisualKick {
                ball_position,
                kick_direction,
                kick_power,
                ..
            } => apply_visual_kick_kinematics(
                clock.now,
                clock.tick_duration,
                &mut ball.state,
                &config,
                &mut ground_to_world.ground_to_world,
                &mut last_kick_time.last_kick_time,
                *ball_position,
                *kick_direction,
                *kick_power,
            ),
            MotionCommand::StandUp => fall_down_state.fall_down_state = None,
            MotionCommand::Damping | MotionCommand::Prepare | MotionCommand::Stand { .. } => {}
        }

        head_yaw.yaw = apply_head_motion(
            head_yaw.yaw,
            frame.motion_command.head_motion(),
            clock.now,
            clock.tick_duration,
            &config,
        );
    }
}

fn apply_head_motion(
    current_yaw: Orientation2<Ground>,
    head_motion: Option<HeadMotion>,
    now: SystemTime,
    tick_duration: Duration,
    config: &SimulationConfig,
) -> Orientation2<Ground> {
    let desired_yaw = desired_head_yaw(head_motion, now, config)
        .clamp(config.head_yaw_minimum, config.head_yaw_maximum);
    let maximum_movement = config.head_yaw_velocity * tick_duration.as_secs_f32();
    let movement =
        wrap_angle(desired_yaw - current_yaw.angle()).clamp(-maximum_movement, maximum_movement);
    Orientation2::new(
        (current_yaw.angle() + movement).clamp(config.head_yaw_minimum, config.head_yaw_maximum),
    )
}

fn desired_head_yaw(
    head_motion: Option<HeadMotion>,
    now: SystemTime,
    config: &SimulationConfig,
) -> f32 {
    match head_motion {
        Some(HeadMotion::LookAt { target, .. }) => {
            Orientation2::from_vector(target.coords()).angle()
        }
        Some(HeadMotion::LookLeftAndRightOf { target }) => {
            let elapsed = now
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f32();
            Orientation2::from_vector(target.coords()).angle()
                + elapsed.sin() * config.head_glance_angle
        }
        Some(HeadMotion::LookAround) | Some(HeadMotion::SearchForLostBall) => {
            let period = config.head_scan_period.as_secs_f32();
            if period <= f32::EPSILON {
                0.0
            } else {
                let elapsed = now
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f32();
                let amplitude = config
                    .head_yaw_minimum
                    .abs()
                    .max(config.head_yaw_maximum.abs());
                (elapsed * std::f32::consts::TAU / period).sin() * amplitude
            }
        }
        Some(HeadMotion::ZeroAngles)
        | Some(HeadMotion::Center { .. })
        | Some(HeadMotion::LookAtReferee { .. })
        | Some(HeadMotion::Unstiff)
        | None => 0.0,
    }
}

fn wrap_angle(angle: f32) -> f32 {
    Orientation2::<Ground>::new(angle).angle()
}

pub(crate) fn first_path_target(path: &types::path::Path) -> Option<Point2<Ground>> {
    let segment = path.segments.first()?;
    match segment {
        PathSegment::LineSegment(segment) => Some(segment.1),
        PathSegment::Arc(arc) => {
            Some(arc.circle.center + arc.end.as_unit_vector() * arc.circle.radius)
        }
    }
}

fn apply_walk_to_pose<Frame>(
    ground_to_frame: Isometry2<Ground, Frame>,
    target: Point2<Ground>,
    target_orientation: Orientation2<Ground>,
    orientation_mode: OrientationMode,
    speed: f32,
    tick_duration: Duration,
    config: &SimulationConfig,
) -> Isometry2<Ground, Frame> {
    let dt = tick_duration.as_secs_f32();
    let max_distance = config.walk_translation_speed * speed * dt;
    let target_vector = target.coords();
    let step_translation =
        if target_vector.norm() > max_distance && target_vector.norm() > f32::EPSILON {
            target_vector.normalize() * max_distance
        } else {
            target_vector
        };

    let desired_orientation = match orientation_mode {
        OrientationMode::LookTowards { direction, .. } => direction,
        OrientationMode::LookAt { target, .. } => Orientation2::from_vector(target.coords()),
        OrientationMode::AlignWithPath | OrientationMode::Unspecified => target_orientation,
    };
    let max_rotation = config.walk_rotation_speed * dt;
    let step_rotation = desired_orientation
        .angle()
        .clamp(-max_rotation, max_rotation);
    let delta = Isometry2::from_parts(step_translation, step_rotation);
    ground_to_frame * delta
}

fn apply_walk_with_velocity_to_pose<Frame>(
    ground_to_frame: Isometry2<Ground, Frame>,
    velocity: Vector2<Ground>,
    angular_velocity: f32,
    tick_duration: Duration,
    config: &SimulationConfig,
) -> Isometry2<Ground, Frame> {
    let dt = tick_duration.as_secs_f32();
    let translation = velocity * config.walk_with_velocity_scale * dt;
    let rotation = angular_velocity * config.walk_with_velocity_scale * dt;
    let delta = Isometry2::from_parts(translation, rotation);
    ground_to_frame * delta
}

fn apply_kick_to_ball(
    now: SystemTime,
    ball: &mut Option<SimulatedBall>,
    config: &SimulationConfig,
    ground_to_world: Isometry2<Ground, World>,
    last_kick_time: &mut SystemTime,
    expected_ball_position: Point2<Ground>,
    kick_direction: Orientation2<Ground>,
    kick_power: KickPower,
) {
    let Some(ball) = ball else { return };
    if now.duration_since(*last_kick_time).unwrap_or_default() < config.kick_cooldown {
        return;
    }

    let expected_ball_in_world = ground_to_world * expected_ball_position;
    if (ball.position - expected_ball_in_world).norm() > config.kick_radius {
        return;
    }

    let actual_ball_in_ground = ground_to_world.inverse() * ball.position;
    if actual_ball_in_ground.coords().norm() > config.kick_radius {
        return;
    }

    let speed = match kick_power {
        KickPower::Rumpelstilzchen => config.kick_ball_speed_rumpelstilzchen,
        KickPower::Schlong => config.kick_ball_speed_schlong,
    };
    ball.velocity = ground_to_world * (kick_direction.as_unit_vector() * speed);
    *last_kick_time = now;
}

fn apply_visual_kick_kinematics(
    now: SystemTime,
    tick_duration: Duration,
    ball: &mut Option<SimulatedBall>,
    config: &SimulationConfig,
    ground_to_world: &mut Isometry2<Ground, World>,
    last_kick_time: &mut SystemTime,
    ball_position: Point2<Ground>,
    kick_direction: Orientation2<Ground>,
    kick_power: KickPower,
) {
    let kick_pose = ball_position - kick_direction.as_unit_vector() * config.kick_radius;
    *ground_to_world = apply_walk_to_pose(
        *ground_to_world,
        kick_pose,
        kick_direction,
        OrientationMode::AlignWithPath,
        1.0,
        tick_duration,
        config,
    );

    apply_kick_to_ball(
        now,
        ball,
        config,
        *ground_to_world,
        last_kick_time,
        ball_position,
        kick_direction,
        kick_power,
    );
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use coordinate_systems::{Ground, World};
    use linear_algebra::{Isometry2, Orientation2, point, vector};
    use types::{field_dimensions::Side, motion_command::KickPower};

    use super::*;
    use crate::behavior_tree_simulator::{DEFAULT_TICK_DURATION, SimulatedBall, SimulationConfig};

    #[test]
    fn kick_does_not_move_ball_outside_contact_range() {
        let mut ball = Some(SimulatedBall {
            position: point![1.0, 0.0],
            velocity: vector![0.0, 0.0],
            field_side: Side::Left,
        });
        let mut last_kick_time = SystemTime::UNIX_EPOCH;

        apply_kick_to_ball(
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            &mut ball,
            &SimulationConfig::default(),
            Isometry2::identity(),
            &mut last_kick_time,
            point![1.0, 0.0],
            Orientation2::identity(),
            KickPower::Rumpelstilzchen,
        );

        assert_eq!(
            ball.expect("ball should still exist").velocity,
            vector![0.0, 0.0]
        );
    }

    #[test]
    fn kick_moves_ball_inside_contact_range() {
        let mut ball = Some(SimulatedBall {
            position: point![0.2, 0.0],
            velocity: vector![0.0, 0.0],
            field_side: Side::Left,
        });
        let mut last_kick_time = SystemTime::UNIX_EPOCH;

        apply_kick_to_ball(
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            &mut ball,
            &SimulationConfig::default(),
            Isometry2::identity(),
            &mut last_kick_time,
            point![0.2, 0.0],
            Orientation2::identity(),
            KickPower::Rumpelstilzchen,
        );

        assert_eq!(
            ball.expect("ball should still exist").velocity,
            vector![
                SimulationConfig::default().kick_ball_speed_rumpelstilzchen,
                0.0
            ]
        );
    }

    #[test]
    fn visual_kick_walks_toward_ball_without_moving_far_ball() {
        let mut ball = Some(SimulatedBall {
            position: point![1.0, 0.0],
            velocity: vector![0.0, 0.0],
            field_side: Side::Left,
        });
        let mut ground_to_field: Isometry2<Ground, World> = Isometry2::identity();
        let mut last_kick_time = SystemTime::UNIX_EPOCH;

        apply_visual_kick_kinematics(
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            DEFAULT_TICK_DURATION,
            &mut ball,
            &SimulationConfig::default(),
            &mut ground_to_field,
            &mut last_kick_time,
            point![1.0, 0.0],
            Orientation2::identity(),
            KickPower::Rumpelstilzchen,
        );

        assert!(ground_to_field.translation().x() > 0.0);
        assert_eq!(
            ball.expect("ball should still exist").velocity,
            vector![0.0, 0.0]
        );
    }

    #[test]
    fn head_motion_is_rate_limited() {
        let config = SimulationConfig {
            head_yaw_velocity: 0.5,
            head_yaw_maximum: 1.0,
            ..Default::default()
        };

        let yaw = apply_head_motion(
            Orientation2::identity(),
            Some(HeadMotion::LookAt {
                target: point![0.0, 1.0],
                image_region_target: Default::default(),
            }),
            SystemTime::UNIX_EPOCH,
            Duration::from_secs(1),
            &config,
        );

        assert_eq!(yaw.angle(), 0.5);
    }
}
