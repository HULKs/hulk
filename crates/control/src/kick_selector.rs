use std::cmp::Ordering;

use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use itertools::iproduct;
use nalgebra::{distance, point, vector, Isometry2, Point2, UnitComplex, Vector2};
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
    pub ball_radius_for_kick_target_selection:
        Parameter<f32, "kick_selector.ball_radius_for_kick_target_selection">,

    pub kick_decisions: AdditionalOutput<Vec<KickDecision>, "kick_decisions">,
    pub kick_targets: AdditionalOutput<Vec<Point2<f32>>, "kick_targets">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub kick_decisions: MainOutput<Option<Vec<KickDecision>>>,
    pub instant_kick_decisions: MainOutput<Option<Vec<KickDecision>>>,
}

impl KickSelector {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let ball_position = context.ball_state.ball_in_ground;

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

        let obstacle_circles = generate_obstacle_circles(
            context.obstacles,
            *context.ball_radius_for_kick_target_selection,
        );

        let instant_kick_decisions = generate_decisions_for_instant_kicks(
            &sides,
            &kick_variants,
            context.in_walk_kicks,
            ball_position,
            &obstacle_circles,
            context.field_dimensions,
            *context.robot_to_field,
        );

        let kick_targets = collect_kick_targets(
            *context.robot_to_field,
            context.field_dimensions,
            &obstacle_circles,
            ball_position,
            *context.max_kick_around_obstacle_angle,
        );

        context
            .kick_targets
            .fill_if_subscribed(|| kick_targets.clone());

        let mut kick_decisions: Vec<_> = iproduct!(sides, kick_variants)
            .filter_map(|(side, kick_variant)| {
                kick_decisions_from_targets(
                    &kick_targets,
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
            instant_kick_decisions: Some(instant_kick_decisions).into(),
        })
    }
}

fn generate_obstacle_circles(
    obstacles: &[Obstacle],
    ball_radius_for_kick_target_selection: f32,
) -> Vec<Circle> {
    obstacles
        .iter()
        .map(|obstacle| {
            let obstacle_radius =
                obstacle.radius_at_foot_height + ball_radius_for_kick_target_selection;
            Circle {
                center: obstacle.position,
                radius: obstacle_radius,
            }
        })
        .collect()
}

fn generate_decisions_for_instant_kicks(
    sides: &[Side; 2],
    kick_variants: &[KickVariant],
    in_walk_kicks: &InWalkKicks,
    ball_position: Point2<f32>,
    obstacle_circles: &[Circle],
    field_dimensions: &FieldDimensions,
    robot_to_field: Isometry2<f32>,
) -> Vec<KickDecision> {
    iproduct!(sides, kick_variants)
        .filter_map(|(&kicking_side, &variant)| {
            let kick_info = &in_walk_kicks[variant];
            let shot_angle = match kicking_side {
                Side::Left => UnitComplex::new(kick_info.shot_angle),
                Side::Right => UnitComplex::new(kick_info.shot_angle).inverse(),
            };
            let shot_distance = vector![2.0, 0.0];
            let target = ball_position + shot_angle * shot_distance;

            let is_inside_field = field_dimensions.is_inside_field(robot_to_field * target);
            let ball_to_target = LineSegment(ball_position, target);
            let is_intersecting_with_an_obstacle = obstacle_circles
                .iter()
                .any(|circle| circle.intersects_line_segment(&ball_to_target));
            let opponent_goal_center = point![field_dimensions.length / 2.0, 0.0];
            let own_goal_center = point![-field_dimensions.length / 2.0, 0.0];
            let is_target_closer_to_opponent_goal = (distance(&target, &opponent_goal_center)
                + 1.0)
                < distance(&ball_position, &opponent_goal_center);
            let goal_box_radius = vector![
                field_dimensions.goal_box_area_length,
                field_dimensions.goal_box_area_width / 2.0
            ]
            .norm();
            let is_ball_close_to_own_goal =
                distance(&ball_position, &own_goal_center) < goal_box_radius;
            let is_target_farer_away_from_our_goal = distance(&target, &own_goal_center)
                > (distance(&ball_position, &own_goal_center) + 1.0);
            if is_inside_field
                && !is_intersecting_with_an_obstacle
                && (is_target_closer_to_opponent_goal
                    || (is_ball_close_to_own_goal && is_target_farer_away_from_our_goal))
            {
                let kick_pose = compute_kick_pose(ball_position, target, kick_info, kicking_side);
                Some(KickDecision {
                    variant,
                    kicking_side,
                    kick_pose,
                })
            } else {
                None
            }
        })
        .collect()
}

fn collect_kick_targets(
    robot_to_field: Isometry2<f32>,
    field_dimensions: &FieldDimensions,
    obstacle_circles: &[Circle],
    ball_position: Point2<f32>,
    max_kick_around_obstacle_angle: f32,
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
    let kick_targets = vec![left_goal_half, right_goal_half];

    let obstacle_circles: Vec<_> = obstacle_circles
        .iter()
        .map(|circle| {
            let ball_to_obstacle = circle.center - ball_position;
            let safety_radius = circle.radius / max_kick_around_obstacle_angle.sin();
            let distance_to_obstacle = ball_to_obstacle.norm();
            let center = if distance_to_obstacle < safety_radius {
                circle.center
                    + ball_to_obstacle.normalize() * (safety_radius - distance_to_obstacle)
            } else {
                circle.center
            };
            Circle {
                center,
                radius: circle.radius,
            }
        })
        .collect();

    kick_targets
        .iter()
        .flat_map(|&target| {
            let ball_to_target = LineSegment(ball_position, target);
            let closest_intersecting_obstacle = obstacle_circles
                .iter()
                .filter(|circle| circle.intersects_line_segment(&ball_to_target))
                .min_by_key(|circle| NotNan::new(circle.center.coords.norm()).unwrap());
            match closest_intersecting_obstacle {
                Some(circle) => {
                    let TwoLineSegments(left_tangent, right_tangent) =
                        circle.tangents_with_point(ball_position).unwrap();
                    [left_tangent, right_tangent]
                        .into_iter()
                        .map(|tangent| {
                            let kick_direction = (tangent.0 - ball_position).normalize();
                            ball_position + kick_direction * 2.0
                        })
                        .filter(|&target| field_dimensions.is_inside_field(target))
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

fn mirror_kick_offset(kick_offset: Vector2<f32>) -> Vector2<f32> {
    vector![kick_offset.x, -kick_offset.y]
}

fn compute_kick_pose(
    ball_position: Point2<f32>,
    target_to_kick_to: Point2<f32>,
    kick_info: &InWalkKickInfo,
    side: Side,
) -> Isometry2<f32> {
    let kick_rotation = rotate_towards(ball_position, target_to_kick_to);
    let ball_to_ground = Isometry2::from(ball_position.coords);
    let shot_angle = UnitComplex::new(kick_info.shot_angle);
    let offset_to_ball = kick_info.offset;
    match side {
        Side::Left => ball_to_ground * shot_angle * kick_rotation * Isometry2::from(offset_to_ball),
        Side::Right => {
            ball_to_ground
                * shot_angle.inverse()
                * kick_rotation
                * Isometry2::from(mirror_kick_offset(offset_to_ball))
        }
    }
}
