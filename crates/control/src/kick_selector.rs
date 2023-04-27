use std::cmp::Ordering;

use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use itertools::iproduct;
use nalgebra::{point, vector, Isometry2, Point2, Rotation2, UnitComplex};
use ordered_float::NotNan;
use types::{
    configuration::{InWalkKickInfo, InWalkKicks},
    rotate_towards, BallState, Circle, FieldDimensions, KickDecision, KickVariant, LineSegment,
    Obstacle, Side, TwoLineSegments,
};

pub struct KickSelector {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub robot_to_field: RequiredInput<Option<Isometry2<f32>>, "robot_to_field?">,
    pub ball_state: RequiredInput<Option<BallState>, "ball_state?">,
    pub obstacles: Input<Vec<Obstacle>, "obstacles">,

    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,

    pub in_walk_kicks: Parameter<InWalkKicks, "in_walk_kicks">,
    pub angle_distance_weight: Parameter<f32, "kick_selector.angle_distance_weight">,
    pub max_kick_around_obstacle_angle:
        Parameter<f32, "kick_selector.max_kick_around_obstacle_angle">,
    pub kick_pose_obstacle_radius: Parameter<f32, "kick_selector.kick_pose_obstacle_radius">,
    pub emergency_kick_target_angles:
        Parameter<Vec<f32>, "kick_selector.emergency_kick_target_angles">,
    pub ball_radius_for_kick_target_selection:
        Parameter<f32, "kick_selector.ball_radius_for_kick_target_selection">,

    pub kick_decisions: AdditionalOutput<Vec<KickDecision>, "kick_decisions">,
    pub kick_targets: AdditionalOutput<Vec<Point2<f32>>, "kick_targets">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub kick_decisions: MainOutput<Option<Vec<KickDecision>>>,
}

impl KickSelector {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let ball_position = context.ball_state.position;
        let targets_to_kick_to = find_targets_to_kick_to(
            ball_position,
            *context.robot_to_field,
            context.field_dimensions,
            context.obstacles,
            *context.ball_radius_for_kick_target_selection,
            *context.max_kick_around_obstacle_angle,
            context.emergency_kick_target_angles,
        );
        context
            .kick_targets
            .fill_if_subscribed(|| targets_to_kick_to.clone());

        let sides = [Side::Left, Side::Right];
        let mut kick_variants = Vec::new();
        if context.in_walk_kicks.forward.enabled {
            kick_variants.push(KickVariant::Forward)
        }
        if context.in_walk_kicks.turn.enabled {
            kick_variants.push(KickVariant::Turn)
        }
        if context.in_walk_kicks.side.enabled {
            kick_variants.push(KickVariant::Side)
        }
        let mut kick_decisions: Vec<_> = iproduct!(sides, kick_variants)
            .filter_map(|(side, kick_variant)| {
                kick_decisions_from_targets(
                    &targets_to_kick_to,
                    context.in_walk_kicks,
                    kick_variant,
                    side,
                    ball_position,
                )
            })
            .flatten()
            .collect();
        kick_decisions.sort_by(|left, right| {
            let left_in_obstacle = is_inside_any_obstacle(
                left.kick_pose,
                context.obstacles,
                *context.kick_pose_obstacle_radius,
            );
            let right_in_obstacle = is_inside_any_obstacle(
                left.kick_pose,
                context.obstacles,
                *context.kick_pose_obstacle_radius,
            );
            let distance_to_left =
                distance_to_kick_pose(left.kick_pose, *context.angle_distance_weight);
            let distance_to_right =
                distance_to_kick_pose(right.kick_pose, *context.angle_distance_weight);
            match (left_in_obstacle, right_in_obstacle) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => distance_to_left.total_cmp(&distance_to_right),
            }
        });
        context
            .kick_decisions
            .fill_if_subscribed(|| kick_decisions.clone());

        Ok(MainOutputs {
            kick_decisions: Some(kick_decisions).into(),
        })
    }
}

fn find_targets_to_kick_to(
    ball_position: Point2<f32>,
    robot_to_field: Isometry2<f32>,
    field_dimensions: &FieldDimensions,
    obstacles: &[Obstacle],
    ball_radius_for_kick_target_selection: f32,
    max_kick_around_obstacle_angle: f32,
    emergency_kick_target_angles: &[f32],
) -> Vec<Point2<f32>> {
    let obstacle_circles: Vec<_> = obstacles
        .iter()
        .map(|obstacle| {
            let ball_to_obstacle = obstacle.position - ball_position;
            let obstacle_radius =
                obstacle.radius_at_foot_height + ball_radius_for_kick_target_selection;
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

        let emergency_targets = emergency_kick_target_angles
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

fn kick_decisions_from_targets(
    targets_to_kick_to: &[Point2<f32>],
    in_walk_kicks: &InWalkKicks,
    variant: KickVariant,
    kicking_side: Side,
    ball_position: Point2<f32>,
) -> Option<Vec<KickDecision>> {
    Some(
        targets_to_kick_to
            .iter()
            .map(|&target| {
                let kick_info = &in_walk_kicks[variant];
                let kick_pose = compute_kick_pose(ball_position, target, kick_info, kicking_side);
                KickDecision {
                    variant,
                    kicking_side,
                    kick_pose,
                }
            })
            .collect(),
    )
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

fn distance_to_kick_pose(kick_pose: Isometry2<f32>, angle_distance_weight: f32) -> f32 {
    kick_pose.translation.vector.norm() + angle_distance_weight * kick_pose.rotation.angle().abs()
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
