use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use coordinate_systems::{Ground, World};
use linear_algebra::{Isometry2, Orientation2, Point2, Vector2, vector};
use types::{
    motion_command::{HeadMotion, KickPower, MotionCommand},
    path::PathSegment,
    step::Step,
};

use crate::behavior_tree_simulator::{
    SimulatedBall, SimulationConfig, SimulatorBall, SimulatorClock, SimulatorFallDownState,
    SimulatorGroundToWorld, SimulatorHeadYaw, SimulatorLastKickTime, SimulatorRobot,
    SimulatorRobotFrames, SimulatorRobotParameters,
};

pub fn move_robots(
    clock: Res<SimulatorClock>,
    config: Res<SimulationConfig>,
    robot_frames: Res<SimulatorRobotFrames>,
    mut ball: ResMut<SimulatorBall>,
    mut robots: Query<(
        &SimulatorRobot,
        &SimulatorRobotParameters,
        &mut SimulatorGroundToWorld,
        &mut SimulatorHeadYaw,
        &mut SimulatorFallDownState,
        &mut SimulatorLastKickTime,
    )>,
) {
    for (
        robot,
        parameters,
        mut ground_to_world,
        mut head_yaw,
        mut fall_down_state,
        mut last_kick_time,
    ) in &mut robots
    {
        let Some(frame) = robot_frames.0.get(&robot.id()) else {
            continue;
        };

        match &frame.motion_command {
            MotionCommand::Walk { .. } => {
                let step = motion::booster::walking::step_from_motion_command(
                    &frame.motion_command,
                    &parameters.walking,
                );
                ground_to_world.ground_to_world = apply_walk_to_pose(
                    ground_to_world.ground_to_world,
                    step,
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

pub fn first_path_target(path: &types::path::Path) -> Option<Point2<Ground>> {
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
    step: Step,
    tick_duration: Duration,
    config: &SimulationConfig,
) -> Isometry2<Ground, Frame> {
    let dt = tick_duration.as_secs_f32();
    let max_distance = config.walk_translation_speed * dt;
    let target_vector = vector![step.forward, step.left] * dt;
    let step_translation =
        if target_vector.norm() > max_distance && target_vector.norm() > f32::EPSILON {
            target_vector.normalize() * max_distance
        } else {
            target_vector
        };

    let max_rotation = config.walk_rotation_speed * dt;
    let step_rotation = (step.turn * dt).clamp(-max_rotation, max_rotation);
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
        step_towards_target(kick_pose, kick_direction, 1.0, config, tick_duration),
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

fn step_towards_target(
    target: Point2<Ground>,
    target_orientation: Orientation2<Ground>,
    speed: f32,
    config: &SimulationConfig,
    tick_duration: Duration,
) -> Step {
    let dt = tick_duration.as_secs_f32();
    let max_distance = config.walk_translation_speed * speed * dt;
    let target_vector = target.coords();
    let velocity = if target_vector.norm() > max_distance && target_vector.norm() > f32::EPSILON {
        target_vector.normalize() * config.walk_translation_speed * speed
    } else if dt > f32::EPSILON {
        target_vector / dt
    } else {
        target_vector
    };

    Step {
        forward: velocity.x(),
        left: velocity.y(),
        turn: target_orientation.angle() / dt.max(f32::EPSILON),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        f32::consts::FRAC_PI_2,
        time::{Duration, SystemTime},
    };

    use coordinate_systems::{Ground, World};
    use hsl_network_messages::{PlayerNumber, Team};
    use linear_algebra::{Isometry2, Orientation2, point, vector};
    use types::{
        behavior_tree::{NodeTrace, Status},
        field_dimensions::Side,
        motion_command::{HeadMotion, KickPower, MotionCommand, OrientationMode},
        parameters::{BehaviorParameters, RLWalkingParameters},
        path::direct_path,
        world_state::WorldState,
    };

    use super::*;
    use crate::behavior_tree_simulator::{
        DEFAULT_TICK_DURATION, RobotFrame, SimulatedBall, SimulationConfig, SimulatorRobotId,
    };

    #[test]
    fn walk_kinematics_uses_booster_hybrid_alignment() {
        let robot_id = SimulatorRobotId::new(Team::Hulks, PlayerNumber::Three);
        let mut app = App::new();
        app.insert_resource(SimulatorClock {
            now: SystemTime::UNIX_EPOCH,
            tick_duration: DEFAULT_TICK_DURATION,
        })
        .insert_resource(SimulationConfig {
            walk_rotation_speed: 10.0,
            ..Default::default()
        })
        .insert_resource(SimulatorBall::default())
        .insert_resource(SimulatorRobotFrames(BTreeMap::from([(
            robot_id,
            RobotFrame {
                world_state: WorldState::default(),
                motion_command: MotionCommand::Walk {
                    head: HeadMotion::ZeroAngles,
                    path: direct_path(point![0.0, 0.0], point![2.0, 0.0]),
                    orientation_mode: OrientationMode::AlignWithPath,
                    target_orientation: Orientation2::new(FRAC_PI_2),
                    distance_to_be_aligned: 0.05,
                    speed: 1.0,
                },
                trace: empty_trace(),
                static_layout: empty_trace(),
                path_obstacles: Vec::new(),
                time_since_last_switch: Duration::ZERO,
                direction_difference: 0.0,
                walk_position: None,
                voronoi_map: None,
                voronoi_inputs: Vec::new(),
                outgoing_messages: Vec::new(),
            },
        )])))
        .add_systems(Update, move_robots);

        app.world_mut().spawn((
            SimulatorRobot {
                team: Team::Hulks,
                player_number: PlayerNumber::Three,
            },
            SimulatorRobotParameters {
                behavior: BehaviorParameters::default(),
                walking: RLWalkingParameters {
                    hybrid_align_distance: 1.0,
                    max_alignment_rate: 1.0,
                    deceleration_distance: 0.5,
                    ..Default::default()
                },
            },
            SimulatorGroundToWorld {
                ground_to_world: Isometry2::identity(),
            },
            SimulatorHeadYaw::default(),
            SimulatorFallDownState::default(),
            SimulatorLastKickTime {
                last_kick_time: SystemTime::UNIX_EPOCH,
            },
        ));

        app.update();

        let mut query = app.world_mut().query::<&SimulatorGroundToWorld>();
        let ground_to_world = query.single(app.world()).expect("robot should exist");
        assert_eq!(ground_to_world.ground_to_world.orientation().angle(), 0.0);
        assert!(ground_to_world.ground_to_world.translation().x() > 0.0);
    }

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

    fn empty_trace() -> NodeTrace {
        NodeTrace {
            name: String::new(),
            status: Status::Success,
            children: Vec::new(),
        }
    }
}
