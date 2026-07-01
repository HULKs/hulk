use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use coordinate_systems::{Ground, World};
use hsl_network_messages::{GameState, Team};
use linear_algebra::{Isometry2, Orientation2, Point2, Vector2, vector};
use motion::booster::walking::step_from_motion_command;
use types::{
    motion_command::{HeadMotion, KickPower, MotionCommand},
    step::Step,
};

use crate::behavior_tree_simulator::{
    SimulatedBall, SimulationConfig, SimulatorBall, SimulatorClock, SimulatorFallDownState,
    SimulatorFieldDimensions, SimulatorGameState, SimulatorGroundToWorld, SimulatorHeadYaw,
    SimulatorLastKickTime, SimulatorRobot, SimulatorRobotFrames, SimulatorRobotId,
    SimulatorRobotParameters,
};

pub fn resolve_collisions(
    field_dimensions: Res<SimulatorFieldDimensions>,
    game_state: Res<SimulatorGameState>,
    config: Res<SimulationConfig>,
    mut ball: ResMut<SimulatorBall>,
    mut robots: Query<(Entity, &SimulatorRobot, &mut SimulatorGroundToWorld)>,
) {
    let mut robot_positions = robots
        .iter()
        .map(|(entity, robot, ground_to_world)| {
            (
                entity,
                robot.id(),
                ground_to_world.ground_to_world.translation(),
            )
        })
        .collect::<Vec<_>>();
    robot_positions.sort_by_key(|(_, robot_id, _)| *robot_id);

    resolve_robot_robot_collisions(&mut robot_positions, config.robot_radius);

    for (entity, _, resolved_position) in robot_positions.iter().copied() {
        let Ok((_, _, mut ground_to_world)) = robots.get_mut(entity) else {
            continue;
        };
        let rotation = ground_to_world.ground_to_world.orientation().angle();
        ground_to_world.ground_to_world =
            Isometry2::from_parts(resolved_position.coords(), rotation);
    }

    if let Some(simulated_ball) = &mut ball.state {
        let mut last_collision = None;
        for (_, robot_id, robot_position) in robot_positions {
            if let Some(contact_distance) = resolve_ball_robot_collision(
                simulated_ball,
                robot_position,
                config.robot_radius,
                field_dimensions.0.ball_radius,
            ) {
                last_collision = closest_collision(last_collision, robot_id, contact_distance);
            }
        }

        if game_state.game_controller_state.game_state == GameState::Playing
            && let Some((robot_id, _)) = last_collision
        {
            ball.last_touched_by = Some(robot_id.team);
        }
    }
}

pub fn move_robots(
    clock: Res<SimulatorClock>,
    config: Res<SimulationConfig>,
    field_dimensions: Res<SimulatorFieldDimensions>,
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
                let step = step_from_motion_command(&frame.motion_command, &parameters.walking);
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
            } => {
                let ball = &mut *ball;
                apply_visual_kick_kinematics(
                    clock.now,
                    clock.tick_duration,
                    &mut ball.state,
                    &mut ball.last_touched_by,
                    robot.team,
                    &config,
                    field_dimensions.0.ball_radius,
                    &mut ground_to_world.ground_to_world,
                    &mut last_kick_time.last_kick_time,
                    *ball_position,
                    *kick_direction,
                    *kick_power,
                )
            }
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

fn resolve_robot_robot_collisions(
    robot_positions: &mut [(Entity, SimulatorRobotId, Point2<World>)],
    robot_radius: f32,
) {
    let minimum_distance = 2.0 * robot_radius;
    if minimum_distance <= 0.0 {
        return;
    }

    for _ in 0..4 {
        for i in 0..robot_positions.len() {
            for j in (i + 1)..robot_positions.len() {
                let delta = robot_positions[j].2 - robot_positions[i].2;
                let distance = delta.norm();
                if distance >= minimum_distance {
                    continue;
                }

                let normal = collision_normal(delta, robot_positions[i].1, robot_positions[j].1);
                let overlap = minimum_distance - distance;
                robot_positions[i].2 -= normal * (overlap / 2.0);
                robot_positions[j].2 += normal * (overlap / 2.0);
            }
        }
    }
}

fn resolve_ball_robot_collision(
    ball: &mut SimulatedBall,
    robot_position: Point2<World>,
    robot_radius: f32,
    ball_radius: f32,
) -> Option<f32> {
    let minimum_distance = robot_radius + ball_radius;
    if minimum_distance <= 0.0 {
        return None;
    }

    let delta = ball.position - robot_position;
    let distance = delta.norm();
    if distance >= minimum_distance {
        return None;
    }

    let normal = if distance > f32::EPSILON {
        delta / distance
    } else if ball.velocity.norm() > f32::EPSILON {
        ball.velocity.normalize()
    } else {
        vector![1.0, 0.0]
    };

    ball.position = robot_position + normal * minimum_distance;
    let velocity_into_robot = ball.velocity.dot(&normal) < 0.0;
    if velocity_into_robot {
        ball.velocity -= normal * (2.0 * ball.velocity.dot(&normal));
    }

    Some(distance)
}

fn closest_collision(
    current: Option<(SimulatorRobotId, f32)>,
    robot_id: SimulatorRobotId,
    contact_distance: f32,
) -> Option<(SimulatorRobotId, f32)> {
    match current {
        Some((current_id, current_distance))
            if current_distance
                .total_cmp(&contact_distance)
                .then_with(|| current_id.cmp(&robot_id))
                .is_le() =>
        {
            Some((current_id, current_distance))
        }
        _ => Some((robot_id, contact_distance)),
    }
}

fn collision_normal(
    delta: Vector2<World>,
    first_id: SimulatorRobotId,
    second_id: SimulatorRobotId,
) -> Vector2<World> {
    if delta.norm() > f32::EPSILON {
        return delta.normalize();
    }

    if first_id <= second_id {
        vector![1.0, 0.0]
    } else {
        vector![-1.0, 0.0]
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
        | Some(HeadMotion::Unstiff)
        | None => 0.0,
    }
}

fn wrap_angle(angle: f32) -> f32 {
    Orientation2::<Ground>::new(angle).angle()
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

enum KickAttempt {
    Kicked,
    CoolingDown,
    NotInRange,
}

fn apply_kick_to_ball(
    now: SystemTime,
    ball: &mut Option<SimulatedBall>,
    last_touched_by: &mut Option<Team>,
    kicking_team: Team,
    config: &SimulationConfig,
    ground_to_world: Isometry2<Ground, World>,
    last_kick_time: &mut SystemTime,
    expected_ball_position: Point2<Ground>,
    kick_direction: Orientation2<Ground>,
    kick_power: KickPower,
) -> KickAttempt {
    let Some(ball) = ball else {
        return KickAttempt::NotInRange;
    };
    if now.duration_since(*last_kick_time).unwrap_or_default() < config.kick_cooldown {
        return KickAttempt::CoolingDown;
    }

    let expected_ball_in_world = ground_to_world * expected_ball_position;
    if (ball.position - expected_ball_in_world).norm() > config.kick_radius {
        return KickAttempt::NotInRange;
    }

    let actual_ball_in_ground = ground_to_world.inverse() * ball.position;
    if actual_ball_in_ground.coords().norm() > config.kick_radius {
        return KickAttempt::NotInRange;
    }

    let speed = match kick_power {
        KickPower::Rumpelstilzchen => config.kick_ball_speed_rumpelstilzchen,
        KickPower::Schlong => config.kick_ball_speed_schlong,
    };
    ball.velocity = ground_to_world * (kick_direction.as_unit_vector() * speed);
    *last_touched_by = Some(kicking_team);
    *last_kick_time = now;
    KickAttempt::Kicked
}

fn apply_visual_kick_kinematics(
    now: SystemTime,
    tick_duration: Duration,
    ball: &mut Option<SimulatedBall>,
    last_touched_by: &mut Option<Team>,
    kicking_team: Team,
    config: &SimulationConfig,
    ball_radius: f32,
    ground_to_world: &mut Isometry2<Ground, World>,
    last_kick_time: &mut SystemTime,
    ball_position: Point2<Ground>,
    kick_direction: Orientation2<Ground>,
    kick_power: KickPower,
) {
    let kick_direction_vector = kick_direction.as_unit_vector();
    match apply_kick_to_ball(
        now,
        ball,
        last_touched_by,
        kicking_team,
        config,
        *ground_to_world,
        last_kick_time,
        ball_position,
        kick_direction,
        kick_power,
    ) {
        KickAttempt::Kicked | KickAttempt::CoolingDown => return,
        KickAttempt::NotInRange => {}
    }

    let standoff_distance = (config.kick_radius * 0.9)
        .min(ball_position.coords().dot(&kick_direction_vector))
        .max(config.robot_radius + ball_radius);
    let kick_pose = ball_position - kick_direction_vector * standoff_distance;
    *ground_to_world = apply_walk_to_pose(
        *ground_to_world,
        step_towards_target(kick_pose, kick_direction, 1.0, config, tick_duration),
        tick_duration,
        config,
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
        field_dimensions::{FieldDimensions, Side},
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
        .insert_resource(SimulatorFieldDimensions(FieldDimensions::SPL_2025))
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
        let mut last_touched_by = None;
        let mut last_kick_time = SystemTime::UNIX_EPOCH;

        apply_kick_to_ball(
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            &mut ball,
            &mut last_touched_by,
            Team::Hulks,
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
        assert_eq!(last_touched_by, None);
    }

    #[test]
    fn kick_moves_ball_inside_contact_range() {
        let mut ball = Some(SimulatedBall {
            position: point![0.2, 0.0],
            velocity: vector![0.0, 0.0],
            field_side: Side::Left,
        });
        let mut last_touched_by = None;
        let mut last_kick_time = SystemTime::UNIX_EPOCH;

        apply_kick_to_ball(
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            &mut ball,
            &mut last_touched_by,
            Team::Hulks,
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
        assert_eq!(last_touched_by, Some(Team::Hulks));
    }

    #[test]
    fn visual_kick_walks_toward_ball_without_moving_far_ball() {
        let mut ball = Some(SimulatedBall {
            position: point![1.0, 0.0],
            velocity: vector![0.0, 0.0],
            field_side: Side::Left,
        });
        let mut ground_to_field: Isometry2<Ground, World> = Isometry2::identity();
        let mut last_touched_by = None;
        let mut last_kick_time = SystemTime::UNIX_EPOCH;

        apply_visual_kick_kinematics(
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            DEFAULT_TICK_DURATION,
            &mut ball,
            &mut last_touched_by,
            Team::Hulks,
            &SimulationConfig::default(),
            FieldDimensions::SPL_2025.ball_radius,
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
