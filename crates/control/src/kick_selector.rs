use std::cmp::Ordering;

use color_eyre::Result;
use itertools::iproduct;
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{AdditionalOutput, MainOutput};
use geometry::{
    circle::Circle, line_segment::LineSegment, look_at::LookAt, two_line_segments::TwoLineSegments,
};
use linear_algebra::{
    distance, point, vector, IntoFramed, Isometry2, Orientation2, Point, Point2, Pose, UnitComplex,
    Vector2,
};
use spl_network_messages::GamePhase;
use types::{
    field_dimensions::FieldDimensions,
    kick_decision::KickDecision,
    kick_target::KickTarget,
    motion_command::KickVariant,
    obstacles::Obstacle,
    parameters::{FindKickTargetsParameters, InWalkKickInfoParameters, InWalkKicksParameters},
    support_foot::Side,
    world_state::BallState,
};

#[derive(Deserialize, Serialize)]
pub struct KickSelector {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    ground_to_field: RequiredInput<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    ball_state: RequiredInput<Option<BallState>, "ball_state?">,
    obstacles: Input<Vec<Obstacle>, "obstacles">,
    game_phase: Input<Option<GamePhase>, "filtered_game_controller_state?.game_phase">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,

    in_walk_kicks: Parameter<InWalkKicksParameters, "in_walk_kicks">,
    angle_distance_weight: Parameter<f32, "kick_selector.angle_distance_weight">,
    max_kick_around_obstacle_angle: Parameter<f32, "kick_selector.max_kick_around_obstacle_angle">,
    kick_pose_obstacle_radius: Parameter<f32, "kick_selector.kick_pose_obstacle_radius">,
    ball_radius_for_kick_target_selection:
        Parameter<f32, "kick_selector.ball_radius_for_kick_target_selection">,
    closer_threshold: Parameter<f32, "kick_selector.closer_threshold">,
    find_kick_targets: Parameter<FindKickTargetsParameters, "kick_selector.find_kick_targets">,
    goal_accuracy_margin: Parameter<f32, "kick_selector.goal_accuracy_margin">,

    default_kick_strength: Parameter<f32, "kick_selector.default_kick_strength">,
    corner_kick_strength: Parameter<f32, "kick_selector.corner_kick_strength">,

    kick_targets: AdditionalOutput<Vec<KickTarget>, "kick_targets">,
    instant_kick_targets: AdditionalOutput<Vec<Point2<Ground>>, "instant_kick_targets">,
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
        let penalty_shootout =
            matches!(context.game_phase, Some(GamePhase::PenaltyShootout { .. }));

        let sides = [Side::Left, Side::Right];
        let mut kick_variants = Vec::new();
        if context.in_walk_kicks.forward.enabled {
            kick_variants.push(KickVariant::Forward)
        }
        if context.in_walk_kicks.turn.enabled && !penalty_shootout {
            kick_variants.push(KickVariant::Turn)
        }
        if context.in_walk_kicks.side.enabled && !penalty_shootout {
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
            *context.ground_to_field,
            *context.closer_threshold,
            &mut context.instant_kick_targets,
            *context.default_kick_strength,
            *context.goal_accuracy_margin,
        );

        let kick_targets = collect_kick_targets(
            *context.ground_to_field,
            context.field_dimensions,
            &obstacle_circles,
            ball_position,
            *context.max_kick_around_obstacle_angle,
            context.find_kick_targets,
            *context.corner_kick_strength,
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
                    *context.default_kick_strength,
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
                right.kick_pose,
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

        Ok(MainOutputs {
            kick_decisions: Some(kick_decisions).into(),
            instant_kick_decisions: Some(instant_kick_decisions).into(),
        })
    }
}

fn generate_obstacle_circles(
    obstacles: &[Obstacle],
    ball_radius_for_kick_target_selection: f32,
) -> Vec<Circle<Ground>> {
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

#[allow(clippy::too_many_arguments)]
fn generate_decisions_for_instant_kicks(
    sides: &[Side; 2],
    kick_variants: &[KickVariant],
    in_walk_kicks: &InWalkKicksParameters,
    ball_position: Point2<Ground>,
    obstacle_circles: &[Circle<Ground>],
    field_dimensions: &FieldDimensions,
    ground_to_field: Isometry2<Ground, Field>,
    closer_threshold: f32,
    instant_kick_targets: &mut AdditionalOutput<Vec<Point2<Ground>>>,
    default_kick_strength: f32,
    goal_accuracy_margin: f32,
) -> Vec<KickDecision> {
    let field_to_ground = ground_to_field.inverse();
    instant_kick_targets.fill_if_subscribed(Default::default);
    iproduct!(sides, kick_variants)
        .filter_map(|(&kicking_side, &variant)| {
            let kick_info = &in_walk_kicks[variant];
            let shot_angle = match kicking_side {
                Side::Left => UnitComplex::new(kick_info.orientation),
                Side::Right => UnitComplex::new(kick_info.orientation).inverse(),
            };
            let shot_distance: Vector2<Ground> = vector![kick_info.shot_distance, 0.0];
            let target = ball_position + shot_angle * shot_distance;

            let is_inside_field = field_dimensions.is_inside_field(ground_to_field * target);
            let ball_to_target = LineSegment(ball_position, target);
            let is_intersecting_with_an_obstacle = obstacle_circles
                .iter()
                .any(|circle| circle.intersects_line_segment(&ball_to_target));
            let opponent_goal_center = field_to_ground * point![field_dimensions.length / 2.0, 0.0];
            let own_goal_center = field_to_ground * point![-field_dimensions.length / 2.0, 0.0];
            let is_target_closer_to_opponent_goal = (distance(target, opponent_goal_center)
                + closer_threshold)
                < distance(ball_position, opponent_goal_center);
            let goal_box_radius = nalgebra::vector![
                field_dimensions.goal_box_area_length,
                field_dimensions.goal_box_area_width / 2.0
            ]
            .norm();
            let is_ball_close_to_own_goal =
                distance(ball_position, own_goal_center) < goal_box_radius;
            let is_target_farer_away_from_our_goal = distance(target, own_goal_center)
                > (distance(ball_position, own_goal_center) + closer_threshold);
            let scores_goal = is_scoring_goal(
                target,
                ball_position,
                field_dimensions,
                ground_to_field,
                goal_accuracy_margin,
            );
            let is_good_emergency_target =
                is_ball_close_to_own_goal && is_target_farer_away_from_our_goal;
            let is_strategic_target = is_target_closer_to_opponent_goal || is_good_emergency_target;
            if (is_inside_field || scores_goal)
                && !is_intersecting_with_an_obstacle
                && is_strategic_target
            {
                instant_kick_targets
                    .mutate_if_subscribed(|targets| targets.as_mut().unwrap().push(target));
                let kick_pose = compute_kick_pose(ball_position, target, kick_info, kicking_side);
                Some(KickDecision {
                    variant,
                    kicking_side,
                    kick_pose,
                    strength: default_kick_strength,
                })
            } else {
                None
            }
        })
        .collect()
}

fn is_scoring_goal(
    target: Point2<Ground>,
    ball_position: Point2<Ground>,
    field_dimensions: &FieldDimensions,
    ground_to_field: Isometry2<Ground, Field>,
    goal_accuracy_margin: f32,
) -> bool {
    let ball_to_target =
        LineSegment::new(ground_to_field * ball_position, ground_to_field * target);
    let opponent_goal_line = LineSegment::new(
        point![
            field_dimensions.length / 2.0,
            field_dimensions.goal_inner_width / 2.0 - goal_accuracy_margin
        ],
        point![
            field_dimensions.length / 2.0,
            -field_dimensions.goal_inner_width / 2.0 + goal_accuracy_margin
        ],
    );
    ball_to_target.intersects_line_segment(opponent_goal_line)
}

fn collect_kick_targets(
    ground_to_field: Isometry2<Ground, Field>,
    field_dimensions: &FieldDimensions,
    obstacle_circles: &[Circle<Ground>],
    ball_position: Point2<Ground>,
    max_kick_around_obstacle_angle: f32,
    parameters: &FindKickTargetsParameters,
    corner_kick_strength: f32,
) -> Vec<KickTarget> {
    let field_to_ground = ground_to_field.inverse();
    let mut kick_targets = Vec::new();

    if is_ball_in_opponents_corners(ball_position, parameters, field_dimensions, ground_to_field) {
        kick_targets.extend(generate_corner_kick_targets(
            parameters,
            field_dimensions,
            field_to_ground,
            corner_kick_strength,
        ));
    } else {
        kick_targets.extend(generate_goal_line_kick_targets(
            field_dimensions,
            field_to_ground,
        ));
    }

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
            let ball_to_target = LineSegment(ball_position, target.position);
            let closest_intersecting_obstacle = obstacle_circles
                .iter()
                .filter(|circle| circle.intersects_line_segment(&ball_to_target))
                .min_by_key(|circle| NotNan::new(circle.center.coords().norm()).unwrap());
            match closest_intersecting_obstacle {
                Some(circle) => {
                    let TwoLineSegments(left_tangent, right_tangent) =
                        circle.tangents_with_point(ball_position).unwrap();
                    [left_tangent, right_tangent]
                        .into_iter()
                        .map(|tangent| {
                            let kick_direction = (tangent.0 - ball_position).normalize();
                            // TODO: drop this constant?
                            ball_position + kick_direction * 2.0
                        })
                        .filter(|&position| {
                            field_dimensions.is_inside_field(ground_to_field * position)
                        })
                        .map(KickTarget::new)
                        .collect()
                }
                None => vec![target],
            }
        })
        .collect()
}

fn generate_corner_kick_targets(
    parameters: &FindKickTargetsParameters,
    field_dimensions: &FieldDimensions,
    field_to_ground: Isometry2<Field, Ground>,
    corner_kick_strength: f32,
) -> Vec<KickTarget> {
    let from_corner_kick_target_x =
        field_dimensions.length / 2.0 - parameters.corner_kick_target_distance_to_goal;
    let position = field_to_ground * point![from_corner_kick_target_x, 0.0];
    vec![KickTarget {
        position,
        strength: Some(corner_kick_strength),
    }]
}

fn generate_goal_line_kick_targets(
    field_dimensions: &FieldDimensions,
    field_to_ground: Isometry2<Field, Ground>,
) -> Vec<KickTarget> {
    let left_goal_half = field_to_ground
        * point![
            field_dimensions.length / 2.0,
            field_dimensions.goal_inner_width / 4.0
        ];
    let right_goal_half = field_to_ground
        * point![
            field_dimensions.length / 2.0,
            -field_dimensions.goal_inner_width / 4.0
        ];
    vec![
        KickTarget::new(left_goal_half),
        KickTarget::new(right_goal_half),
    ]
}

fn kick_decisions_from_targets(
    targets_to_kick_to: &[KickTarget],
    in_walk_kicks: &InWalkKicksParameters,
    variant: KickVariant,
    kicking_side: Side,
    ball_position: Point2<Ground>,
    default_strength: f32,
) -> Option<Vec<KickDecision>> {
    Some(
        targets_to_kick_to
            .iter()
            .map(|&KickTarget { position, strength }| {
                let kick_info = &in_walk_kicks[variant];
                let kick_pose = compute_kick_pose(ball_position, position, kick_info, kicking_side);
                KickDecision {
                    variant,
                    kicking_side,
                    kick_pose,
                    strength: strength.unwrap_or(default_strength),
                }
            })
            .collect(),
    )
}

fn distance_to_kick_pose(kick_pose: Pose<Ground>, angle_distance_weight: f32) -> f32 {
    kick_pose.position().coords().norm()
        + angle_distance_weight * kick_pose.orientation().angle().abs()
}

fn is_inside_any_obstacle(
    kick_pose: Pose<Ground>,
    obstacles: &[Obstacle],
    kick_pose_obstacle_radius: f32,
) -> bool {
    let position = kick_pose.position();
    obstacles.iter().any(|obstacle| {
        let circle = Circle {
            center: obstacle.position,
            radius: obstacle.radius_at_foot_height + kick_pose_obstacle_radius,
        };
        circle.contains(position)
    })
}

fn mirror_kick_pose<Frame>(kick_pose: Pose<Frame>) -> Pose<Frame> {
    Pose::from_parts(
        point![kick_pose.position().x(), -kick_pose.position().y()],
        kick_pose.orientation().inverse(),
    )
}

fn compute_kick_pose(
    ball_position: Point2<Ground>,
    target_to_kick_to: Point2<Ground>,
    kick_info: &InWalkKickInfoParameters,
    side: Side,
) -> Pose<Ground> {
    struct TargetAlignedBall;
    struct Ball;

    let ball_to_ground = Isometry2::<Ball, Ground>::new(ball_position.coords(), 0.0);
    let aligned_ball_to_ball = Point::origin()
        .look_at(&(ball_to_ground.inverse() * target_to_kick_to))
        .as_transform::<TargetAlignedBall>();
    let kick_pose_in_target_aligned_ball = Pose::<TargetAlignedBall>::from_parts(
        kick_info.position.framed(),
        Orientation2::new(kick_info.orientation),
    );

    ball_to_ground
        * aligned_ball_to_ball
        * match side {
            Side::Left => kick_pose_in_target_aligned_ball,
            Side::Right => mirror_kick_pose(kick_pose_in_target_aligned_ball),
        }
}

fn is_ball_in_opponents_corners(
    ball_position: Point2<Ground>,
    parameters: &FindKickTargetsParameters,
    field_dimensions: &FieldDimensions,
    ground_to_field: Isometry2<Ground, Field>,
) -> bool {
    let ball_in_field = ground_to_field * ball_position;
    let left_opponent_corner = point![field_dimensions.length / 2.0, field_dimensions.width / 2.0];
    let right_opponent_corner =
        point![field_dimensions.length / 2.0, -field_dimensions.width / 2.0];
    let ball_near_left_opponent_corner =
        distance(ball_in_field, left_opponent_corner) < parameters.distance_from_corner;
    let ball_near_right_opponent_corner =
        distance(ball_in_field, right_opponent_corner) < parameters.distance_from_corner;
    ball_near_left_opponent_corner || ball_near_right_opponent_corner
}
