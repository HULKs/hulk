use std::cmp::Ordering;

use color_eyre::Result;
use itertools::iproduct;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground, UpcomingSupport};
use framework::{AdditionalOutput, MainOutput};
use geometry::{circle::Circle, line_segment::LineSegment, look_at::LookAt};
use linear_algebra::{
    distance, point, vector, IntoFramed, Isometry2, Orientation2, Point, Point2, Pose2, Vector2,
};
use types::{
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState,
    kick_decision::KickDecision,
    kick_target::{KickTarget, KickTargetWithKickVariants},
    motion_command::KickVariant,
    obstacles::Obstacle,
    parameters::{InWalkKickInfoParameters, InWalkKicksParameters},
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
    kick_opportunities: Input<Vec<KickTargetWithKickVariants>, "kick_opportunities">,
    obstacles: Input<Vec<Obstacle>, "obstacles">,
    obstacle_circles: Input<Vec<Circle<Ground>>, "obstacle_circles">,
    allow_instant_kicks: Input<bool, "allow_instant_kicks">,
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    ground_to_upcoming_support:
        CyclerState<Isometry2<Ground, UpcomingSupport>, "ground_to_upcoming_support">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,

    in_walk_kicks: Parameter<InWalkKicksParameters, "in_walk_kicks">,
    angle_distance_weight: Parameter<f32, "kick_selector.angle_distance_weight">,
    kick_pose_obstacle_radius: Parameter<f32, "kick_selector.kick_pose_obstacle_radius">,
    closer_threshold: Parameter<f32, "kick_selector.closer_threshold">,
    goal_accuracy_margin: Parameter<f32, "kick_selector.goal_accuracy_margin">,

    default_kick_strength: Parameter<f32, "kick_selector.default_kick_strength">,

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
        let sides = [Side::Left, Side::Right];

        let mut instant_kick_decisions = if *context.allow_instant_kicks {
            generate_decisions_for_instant_kicks(
                &sides,
                context.in_walk_kicks,
                ball_position,
                context.obstacle_circles,
                context.field_dimensions,
                *context.ground_to_field,
                *context.closer_threshold,
                &mut context.instant_kick_targets,
                *context.default_kick_strength,
                *context.goal_accuracy_margin,
                context.filtered_game_controller_state,
            )
        } else {
            context
                .instant_kick_targets
                .fill_if_subscribed(Default::default);
            vec![]
        };

        let mut kick_decisions: Vec<_> = sides
            .iter()
            .filter_map(|&side| {
                kick_decisions_from_targets(
                    context.kick_opportunities,
                    context.in_walk_kicks,
                    side,
                    ball_position,
                    *context.default_kick_strength,
                )
            })
            .flatten()
            .collect();

        kick_decisions.retain(|target| match target.variant {
            KickVariant::Forward => context.in_walk_kicks.forward.enabled,
            KickVariant::Turn => context.in_walk_kicks.turn.enabled,
            KickVariant::Side => context.in_walk_kicks.side.enabled,
        });

        kick_decisions.sort_by(|left, right| {
            compare_decisions(left, right, &context, *context.ground_to_upcoming_support)
        });
        instant_kick_decisions.sort_by(|left, right| {
            compare_decisions(left, right, &context, *context.ground_to_upcoming_support)
        });

        Ok(MainOutputs {
            kick_decisions: Some(kick_decisions).into(),
            instant_kick_decisions: Some(instant_kick_decisions).into(),
        })
    }
}

fn compare_decisions(
    left: &KickDecision,
    right: &KickDecision,
    context: &CycleContext,
    ground_to_upcoming_support: Isometry2<Ground, UpcomingSupport>,
) -> Ordering {
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
    let distance_to_left = distance_to_kick_pose(
        ground_to_upcoming_support * left.kick_pose,
        *context.angle_distance_weight,
    );
    let distance_to_right = distance_to_kick_pose(
        ground_to_upcoming_support * right.kick_pose,
        *context.angle_distance_weight,
    );
    match (left_in_obstacle, right_in_obstacle) {
        (false, true) => Ordering::Less,
        (true, false) => Ordering::Greater,
        _ => distance_to_left.total_cmp(&distance_to_right),
    }
}

#[allow(clippy::too_many_arguments)]
fn generate_decisions_for_instant_kicks(
    sides: &[Side; 2],
    in_walk_kicks: &InWalkKicksParameters,
    ball_position: Point2<Ground>,
    obstacle_circles: &[Circle<Ground>],
    field_dimensions: &FieldDimensions,
    ground_to_field: Isometry2<Ground, Field>,
    closer_threshold: f32,
    instant_kick_targets: &mut AdditionalOutput<Vec<Point2<Ground>>>,
    default_kick_strength: f32,
    goal_accuracy_margin: f32,
    filtered_game_controller_state: Option<&FilteredGameControllerState>,
) -> Vec<KickDecision> {
    let field_to_ground = ground_to_field.inverse();
    instant_kick_targets.fill_if_subscribed(Default::default);

    let kick_variants = vec![KickVariant::Forward, KickVariant::Turn, KickVariant::Side];

    iproduct!(sides, kick_variants)
        .filter_map(|(&kicking_side, variant)| {
            let kick_info = &in_walk_kicks[variant];

            struct TargetAlignedBall;
            struct KickPose;
            let position: Point2<TargetAlignedBall> = Point2::wrap(kick_info.position);
            let parameter_kick_pose =
                Pose2::from_parts(position, Orientation2::new(kick_info.orientation));
            let target_aligned_ball_to_kick_pose = match kicking_side {
                Side::Left => parameter_kick_pose,
                Side::Right => mirror_kick_pose(parameter_kick_pose),
            }
            .as_transform::<KickPose>()
            .inverse();
            let kick_pose_to_ground: Isometry2<KickPose, Ground> = Isometry2::identity();

            let target_aligned_ball_to_ground =
                kick_pose_to_ground * target_aligned_ball_to_kick_pose;

            let shot_distance: Vector2<TargetAlignedBall> = vector![kick_info.shot_distance, 0.0];
            let shot_direction = target_aligned_ball_to_ground * shot_distance;
            let target = ball_position + shot_direction;

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

            let is_inside_kick_off_target_region =
                is_inside_kick_off_target_region(ground_to_field * target, field_dimensions.width);

            let is_own_kick_off = matches!(
                filtered_game_controller_state.map(|x| x.game_state),
                Some(FilteredGameState::Playing { kick_off: true, .. })
            );

            if is_own_kick_off
                && is_inside_field
                && is_inside_kick_off_target_region
                && !is_intersecting_with_an_obstacle
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
            } else if !is_own_kick_off
                && (is_inside_field || scores_goal)
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

fn kick_decisions_from_targets(
    targets_to_kick_to: &[KickTargetWithKickVariants],
    in_walk_kicks: &InWalkKicksParameters,
    kicking_side: Side,
    ball_position: Point2<Ground>,
    default_strength: f32,
) -> Option<Vec<KickDecision>> {
    Some(
        targets_to_kick_to
            .iter()
            .flat_map(
                |KickTargetWithKickVariants {
                     kick_target: KickTarget { position, strength },
                     kick_variants,
                 }| {
                    kick_variants.iter().map(move |&variant| {
                        let kick_info = &in_walk_kicks[variant];
                        let kick_pose =
                            compute_kick_pose(ball_position, *position, kick_info, kicking_side);
                        KickDecision {
                            variant,
                            kicking_side,
                            kick_pose,
                            strength: strength.unwrap_or(default_strength),
                        }
                    })
                },
            )
            .collect(),
    )
}

fn distance_to_kick_pose(kick_pose: Pose2<UpcomingSupport>, angle_distance_weight: f32) -> f32 {
    kick_pose.position().coords().norm()
        + angle_distance_weight * kick_pose.orientation().angle().abs()
}

fn is_inside_any_obstacle(
    kick_pose: Pose2<Ground>,
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

fn mirror_kick_pose<Frame>(kick_pose: Pose2<Frame>) -> Pose2<Frame> {
    Pose2::from_parts(
        point![kick_pose.position().x(), -kick_pose.position().y()],
        kick_pose.orientation().mirror(),
    )
}

fn compute_kick_pose(
    ball_position: Point2<Ground>,
    target_to_kick_to: Point2<Ground>,
    kick_info: &InWalkKickInfoParameters,
    side: Side,
) -> Pose2<Ground> {
    struct TargetAlignedBall;
    struct Ball;

    let ball_to_ground = Isometry2::<Ball, Ground>::from_parts(ball_position.coords(), 0.0);
    let aligned_ball_to_ball = Point::origin()
        .look_at(&(ball_to_ground.inverse() * target_to_kick_to))
        .as_transform::<TargetAlignedBall>();
    let kick_pose_in_target_aligned_ball = Pose2::<TargetAlignedBall>::from_parts(
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

pub fn is_inside_kick_off_target_region(position: Point2<Field>, field_width: f32) -> bool {
    position.x().signum() == position.y().signum()
        && position.x().abs() < field_width / 2.0
        && position.x().abs() <= position.y().abs()
}
