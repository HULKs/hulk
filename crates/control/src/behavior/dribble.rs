use std::cmp::Ordering;

use framework::AdditionalOutput;
use itertools::iproduct;
use nalgebra::{point, vector, Isometry2, Point2, Rotation2, UnitComplex};
use ordered_float::NotNan;
use types::{
    configuration::{Dribbling as DribblingConfiguration, InWalkKickInfo, InWalkKicks},
    rotate_towards, Circle, FieldDimensions, HeadMotion, KickDecision, KickVariant, LineSegment,
    MotionCommand, Obstacle,
    OrientationMode::{self, AlignWithPath},
    PathObstacle, Side, TwoLineSegments, WorldState,
};

use super::walk_to_pose::{hybrid_alignment, WalkPathPlanner};

fn kick_decisions_from_targets(
    targets_to_kick_to: &[Point2<f32>],
    parameters: &InWalkKicks,
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
                let kick_info = &parameters[variant];
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
    parameters: &DribblingConfiguration,
    walk_path_planner: &WalkPathPlanner,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    kick_targets_output: &mut AdditionalOutput<Vec<Point2<f32>>>,
    kick_decisions_output: &mut AdditionalOutput<Vec<KickDecision>>,
) -> Option<MotionCommand> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let relative_ball_position = world_state.ball?.position;
    let head = HeadMotion::LookAt {
        target: world_state.position_of_interest,
    };

    let targets_to_kick_to = find_targets_to_kick_to(
        relative_ball_position,
        robot_to_field,
        field_dimensions,
        &world_state.obstacles,
        parameters,
    );
    kick_targets_output.fill_if_subscribed(|| targets_to_kick_to.clone());

    let sides = [Side::Left, Side::Right];
    let mut kick_variants = Vec::new();
    if parameters.in_walk_kicks.forward.enabled {
        kick_variants.push(KickVariant::Forward)
    }
    if parameters.in_walk_kicks.turn.enabled {
        kick_variants.push(KickVariant::Turn)
    }
    if parameters.in_walk_kicks.side.enabled
        && field_dimensions.is_inside_any_goal_box(robot_to_field * relative_ball_position)
    {
        kick_variants.push(KickVariant::Side)
    }
    let kick_decisions: Vec<_> = iproduct!(sides, kick_variants)
        .filter_map(|(side, kick_variant)| {
            kick_decisions_from_targets(
                &targets_to_kick_to,
                &parameters.in_walk_kicks,
                kick_variant,
                side,
                world_state,
            )
        })
        .flatten()
        .collect();

    kick_decisions_output.fill_if_subscribed(|| {
        kick_decisions
            .iter()
            .filter(|decision| {
                !is_inside_any_obstacle(
                    decision.relative_kick_pose,
                    &world_state.obstacles,
                    parameters.kick_pose_obstacle_radius,
                )
            })
            .cloned()
            .collect()
    });

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
        let left_in_obstacle = is_inside_any_obstacle(
            left.relative_kick_pose,
            &world_state.obstacles,
            parameters.kick_pose_obstacle_radius,
        );
        let right_in_obstacle = is_inside_any_obstacle(
            left.relative_kick_pose,
            &world_state.obstacles,
            parameters.kick_pose_obstacle_radius,
        );
        let distance_to_left =
            distance_to_kick_pose(left.relative_kick_pose, parameters.angle_distance_weight);
        let distance_to_right =
            distance_to_kick_pose(right.relative_kick_pose, parameters.angle_distance_weight);
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

    let hybrid_orientation_mode = hybrid_alignment(
        relative_best_pose,
        parameters.hybrid_align_distance,
        parameters.distance_to_be_aligned,
    );
    let orientation_mode = match hybrid_orientation_mode {
        AlignWithPath if relative_ball_position.coords.norm() > 0.0 => {
            OrientationMode::Override(rotate_towards(Point2::origin(), relative_ball_position))
        }
        orientation_mode => orientation_mode,
    };

    let robot_to_ball = relative_ball_position.coords;
    let dribble_pose_to_ball =
        relative_ball_position.coords - relative_best_pose.translation.vector;
    let angle = robot_to_ball.angle(&dribble_pose_to_ball);
    let should_avoid_ball = angle > parameters.angle_to_approach_ball_from_threshold;
    let ball_obstacle = should_avoid_ball.then_some(relative_ball_position);

    let is_near_ball = matches!(
        world_state.ball,
        Some(ball) if ball.position.coords.norm() < parameters.ignore_robot_when_near_ball_radius,
    );
    let obstacles = if is_near_ball {
        &[]
    } else {
        world_state.obstacles.as_slice()
    };
    let path = walk_path_planner.plan(
        relative_best_pose * Point2::origin(),
        robot_to_field,
        ball_obstacle,
        obstacles,
        path_obstacles_output,
    );
    Some(walk_path_planner.walk_with_obstacle_avoiding_arms(head, orientation_mode, path))
}

fn find_targets_to_kick_to(
    ball_position: Point2<f32>,
    robot_to_field: Isometry2<f32>,
    field_dimensions: &FieldDimensions,
    obstacles: &[Obstacle],
    parameters: &DribblingConfiguration,
) -> Vec<Point2<f32>> {
    let field_to_robot = robot_to_field.inverse();
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
            let obstacle_radius =
                obstacle.radius_at_foot_height + parameters.ball_radius_for_kick_target_selection;
            let safety_radius = obstacle_radius / parameters.max_kick_around_obstacle_angle.sin();
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

    let mut possible_kick_targets = vec![left_goal_half, right_goal_half];

    let own_goal_center = field_to_robot
        * point![
            -field_dimensions.length / 2.0 - field_dimensions.goal_depth / 2.0,
            0.0
        ];
    if ball_is_close_to_own_goal(&ball_position, own_goal_center, field_dimensions) {
        let goal_center_to_ball = ball_position - own_goal_center;
        let target_vector_from_goal = goal_center_to_ball
            .normalize()
            .scale(field_dimensions.width / 2.0);

        let emergency_targets = parameters
            .emergency_kick_target_angles
            .iter()
            .map(|angle| {
                let rotation_matrix = Rotation2::new(*angle);
                let kick_target_vector_from_goal = rotation_matrix * target_vector_from_goal;
                own_goal_center + kick_target_vector_from_goal
            })
            .filter(|target| (robot_to_field * target).x > -field_dimensions.length / 2.0);
        possible_kick_targets.extend(emergency_targets);
    }

    possible_kick_targets
        .into_iter()
        .flat_map(|target| {
            let ball_to_target = LineSegment(ball_position, target);
            let closest_intersecting_obstacle = obstacle_circles
                .iter()
                .filter(|circle| circle.intersects_line_segment(&ball_to_target))
                .min_by_key(|circle| NotNan::new(circle.center.coords.norm()).unwrap());
            match closest_intersecting_obstacle {
                Some(circle) => {
                    let TwoLineSegments(left_tangent, right_tangent) =
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

fn is_inside_any_obstacle(
    kick_pose: Isometry2<f32>,
    obstacles: &[Obstacle],
    kick_pose_obstacle_radius: f32,
) -> bool {
    let position = Point2::from(kick_pose.translation.vector);
    obstacles.iter().any(|obstacle| {
        let circle = Circle {
            center: obstacle.position,
            radius: obstacle.radius_at_foot_height + kick_pose_obstacle_radius,
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

fn ball_is_close_to_own_goal(
    ball_position: &Point2<f32>,
    own_goal_center: Point2<f32>,
    field_dimensions: &FieldDimensions,
) -> bool {
    let is_close_threshold = vector![
        field_dimensions.goal_box_area_length,
        field_dimensions.goal_box_area_width / 2.0
    ]
    .norm();
    let goal_to_ball = ball_position - own_goal_center;
    goal_to_ball.norm() < is_close_threshold
}
