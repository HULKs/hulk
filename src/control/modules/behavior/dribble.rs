use std::cmp::Ordering;

use itertools::iproduct;
use nalgebra::{point, vector, Isometry2, Point2, UnitComplex};
use ordered_float::NotNan;
use types::LineSegment;
use types::{
    rotate_towards, Circle, FieldDimensions, HeadMotion, KickDecision, KickVariant, MotionCommand,
    Obstacle, PathObstacle, Side, WorldState,
};

use crate::framework::{
    configuration::{self, Dribbling, InWalkKickInfo},
    AdditionalOutput,
};

use super::walk_to_pose::{hybrid_alignment, WalkPathPlanner};

fn kick_decisions_from_targets(
    targets_to_kick_to: &[Point2<f32>],
    config: &configuration::InWalkKicks,
    variant: KickVariant,
    kicking_side: Side,
    world_state: &WorldState,
) -> Option<Vec<KickDecision>> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let relative_ball_position = world_state.ball?.position;
    let absolute_ball_position = robot_to_field * relative_ball_position;
    Some(
        targets_to_kick_to
            .iter()
            .map(|&target| {
                let kick_info = &config[variant];
                let absolute_kick_pose = compute_kick_pose(
                    absolute_ball_position,
                    robot_to_field * target,
                    kick_info,
                    kicking_side,
                );
                let relative_kick_pose = robot_to_field.inverse() * absolute_kick_pose;
                let is_reached = is_kick_pose_reached(relative_kick_pose, kick_info);
                KickDecision {
                    variant,
                    kicking_side,
                    relative_kick_pose,
                    is_reached,
                }
            })
            .collect(),
    )
}

pub fn execute(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    config: &Dribbling,
    walk_path_planner: &WalkPathPlanner,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    kick_targets_output: &mut AdditionalOutput<Vec<Point2<f32>>>,
    kick_decisions_output: &mut AdditionalOutput<Vec<KickDecision>>,
) -> Option<MotionCommand> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let relative_ball_position = world_state.ball?.position;
    let head = HeadMotion::LookAt {
        target: relative_ball_position,
    };

    let targets_to_kick_to = find_targets_to_kick_to(
        relative_ball_position,
        robot_to_field,
        field_dimensions,
        &world_state.obstacles,
        config.max_kick_around_obstacle_angle,
    );
    kick_targets_output.fill_on_subscription(|| targets_to_kick_to.clone());

    let sides = [Side::Left, Side::Right];
    let mut kick_variants = Vec::new();
    if config.in_walk_kicks.forward.enabled {
        kick_variants.push(KickVariant::Forward)
    }
    if config.in_walk_kicks.turn.enabled {
        kick_variants.push(KickVariant::Turn)
    }
    if config.in_walk_kicks.side.enabled
        && field_dimensions.is_inside_any_goal_box(robot_to_field * relative_ball_position)
    {
        kick_variants.push(KickVariant::Side)
    }
    let kick_decisions: Vec<_> = iproduct!(sides, kick_variants)
        .filter_map(|(side, kick_variant)| {
            kick_decisions_from_targets(
                &targets_to_kick_to,
                &config.in_walk_kicks,
                kick_variant,
                side,
                world_state,
            )
        })
        .flatten()
        .collect();

    kick_decisions_output.fill_on_subscription(|| kick_decisions.clone());

    let available_kick = kick_decisions.iter().find(|decision| decision.is_reached);
    if let Some(kick) = available_kick {
        let command = MotionCommand::InWalkKick {
            head,
            kick: kick.variant,
            kicking_side: kick.kicking_side,
        };
        return Some(command);
    }

    let best_kick_decision = kick_decisions.iter().min_by(|left, right| {
        let left_in_obstacle =
            is_inside_any_obstacle(left.relative_kick_pose, &world_state.obstacles);
        let right_in_obstacle =
            is_inside_any_obstacle(left.relative_kick_pose, &world_state.obstacles);
        let distance_to_left =
            distance_to_kick_pose(left.relative_kick_pose, config.angle_distance_weight);
        let distance_to_right =
            distance_to_kick_pose(right.relative_kick_pose, config.angle_distance_weight);
        match (left_in_obstacle, right_in_obstacle) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => distance_to_left.total_cmp(&distance_to_right),
        }
    });
    let best_kick_decision = match best_kick_decision {
        Some(decision) => decision,
        None => return Some(MotionCommand::Stand { head }),
    };

    let relative_best_pose = best_kick_decision.relative_kick_pose;

    let orientation_mode = hybrid_alignment(
        relative_best_pose,
        config.hybrid_align_distance,
        config.distance_to_be_aligned,
    );
    let ball_position = world_state.ball.and_then(|ball| {
        let robot_to_ball = ball.position.coords;
        let dribble_pose_to_ball = ball.position.coords - relative_best_pose.translation.vector;
        let angle = robot_to_ball.angle(&dribble_pose_to_ball);
        if angle > config.angle_to_approach_ball_from_threshold {
            Some(ball.position)
        } else {
            None
        }
    });
    let is_near_ball = matches!(
        world_state.ball,
        Some(ball) if ball.position.coords.norm() < config.ignore_robot_when_near_ball_radius,
    );
    let obstacles = if is_near_ball {
        &[]
    } else {
        world_state.obstacles.as_slice()
    };
    let path = walk_path_planner.plan(
        relative_best_pose * Point2::origin(),
        robot_to_field,
        ball_position,
        obstacles,
        path_obstacles_output,
    );
    Some(MotionCommand::Walk {
        head,
        orientation_mode,
        path,
    })
}

fn find_targets_to_kick_to(
    ball_position: Point2<f32>,
    robot_to_field: Isometry2<f32>,
    field_dimensions: &FieldDimensions,
    obstacles: &[Obstacle],
    max_kick_around_obstacle_angle: f32,
) -> Vec<Point2<f32>> {
    let field_to_robot = robot_to_field.inverse();
    let goal_center = field_to_robot * point![field_dimensions.length / 2.0, 0.0];
    let left_goal_half = field_to_robot
        * point![
            field_dimensions.length / 2.0,
            field_dimensions.goal_inner_width / 4.0
        ];
    let right_goal_half = field_to_robot
        * point![
            field_dimensions.length / 2.0,
            -field_dimensions.goal_inner_width / 4.0
        ];
    let obstacle_circles: Vec<_> = obstacles
        .iter()
        .map(|obstacle| {
            let ball_to_obstacle = obstacle.position - ball_position;
            let obstacle_radius = obstacle.radius_at_foot_height;
            let safety_radius = obstacle_radius / max_kick_around_obstacle_angle.sin();
            let distance_to_obstacle = ball_to_obstacle.norm();
            let center = if distance_to_obstacle < safety_radius {
                obstacle.position
                    + ball_to_obstacle.normalize() * (safety_radius - distance_to_obstacle)
            } else {
                obstacle.position
            };
            Circle {
                center,
                radius: obstacle_radius,
            }
        })
        .collect();

    [goal_center, left_goal_half, right_goal_half]
        .into_iter()
        .flat_map(|target| {
            let ball_to_target = LineSegment(ball_position, target);
            let closest_intersecting_obstacle = obstacle_circles
                .iter()
                .filter(|circle| circle.intersects_line_segment(&ball_to_target))
                .min_by_key(|circle| NotNan::new(circle.center.coords.norm()).unwrap());
            match closest_intersecting_obstacle {
                Some(circle) => {
                    let (left_tangent, right_tangent) =
                        circle.tangents_with_point(ball_position).unwrap();
                    [left_tangent.0, right_tangent.0]
                        .into_iter()
                        .filter(|target| field_dimensions.is_inside_field(robot_to_field * target))
                        .collect()
                }
                None => vec![target],
            }
        })
        .collect()
}

fn is_inside_any_obstacle(kick_pose: Isometry2<f32>, obstacles: &[Obstacle]) -> bool {
    let position = Point2::from(kick_pose.translation.vector);
    obstacles.iter().any(|obstacle| {
        let circle = Circle {
            center: obstacle.position,
            radius: obstacle.radius_at_foot_height,
        };
        circle.contains(position)
    })
}

fn mirror_kick_offset(kick_offset: Isometry2<f32>) -> Isometry2<f32> {
    let translation = kick_offset
        .translation
        .vector
        .component_mul(&vector![1.0, -1.0]);
    let rotation = kick_offset.rotation.inverse();
    Isometry2::new(translation, rotation.angle())
}

fn compute_kick_pose(
    ball_position: Point2<f32>,
    target_to_kick_to: Point2<f32>,
    kick_info: &InWalkKickInfo,
    side: Side,
) -> Isometry2<f32> {
    let kick_rotation = rotate_towards(ball_position, target_to_kick_to);
    let ball_to_field = Isometry2::from(ball_position.coords);
    let shot_angle = UnitComplex::new(kick_info.shot_angle);
    let offset_to_ball = Isometry2::new(
        vector![kick_info.offset.x, kick_info.offset.y],
        kick_info.offset.z,
    );
    match side {
        Side::Left => ball_to_field * shot_angle * kick_rotation * offset_to_ball,
        Side::Right => {
            ball_to_field
                * shot_angle.inverse()
                * kick_rotation
                * mirror_kick_offset(offset_to_ball)
        }
    }
}

fn is_kick_pose_reached(kick_pose_to_robot: Isometry2<f32>, kick_info: &InWalkKickInfo) -> bool {
    let is_x_reached = kick_pose_to_robot.translation.x.abs() < kick_info.reached_thresholds.x;
    let is_y_reached = kick_pose_to_robot.translation.y.abs() < kick_info.reached_thresholds.y;
    let is_orientation_reached =
        kick_pose_to_robot.rotation.angle().abs() < kick_info.reached_thresholds.z;
    is_x_reached && is_y_reached && is_orientation_reached
}

fn distance_to_kick_pose(kick_pose: Isometry2<f32>, angle_distance_weight: f32) -> f32 {
    kick_pose.translation.vector.norm() + angle_distance_weight * kick_pose.rotation.angle().abs()
}
