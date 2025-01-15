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
use spl_network_messages::{GamePhase, SubState, Team};
use types::{
    field_dimensions::{self, FieldDimensions, Half},
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState,
    kick_decision::{DecisionParameters, KickDecision, PlayingSituation},
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
    obstacles: Input<Vec<Obstacle>, "obstacles">,
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    ground_to_upcoming_support:
        CyclerState<Isometry2<Ground, UpcomingSupport>, "ground_to_upcoming_support">,

    decision_parameters: Parameter<DecisionParameters, "kick_selector">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    in_walk_kicks: Parameter<InWalkKicksParameters, "in_walk_kicks">,

    playing_situation: AdditionalOutput<PlayingSituation, "playing_situation">,
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
        let ground_to_field = *context.ground_to_field;
        let ball_position = context.ball_state.ball_in_ground;
        let sides = [Side::Left, Side::Right];
        let playing_situation = determine_playing_situation(
            context.filtered_game_controller_state,
            ground_to_field * ball_position,
            context.field_dimensions,
            context.decision_parameters,
        );
        context
            .playing_situation
            .fill_if_subscribed(|| playing_situation);

        let variants = match playing_situation {
            PlayingSituation::KickOff => &context.decision_parameters.kick_off_kick_variants,
            PlayingSituation::CornerKick => &context.decision_parameters.corner_kick_variants,
            PlayingSituation::PenaltyShot => {
                &context.decision_parameters.penalty_shot_kick_variants
            }
            PlayingSituation::Normal => &context.decision_parameters.default_kick_variants,
        }
        .iter()
        .filter(|variant| match variant {
            KickVariant::Forward => context.in_walk_kicks.forward.enabled,
            KickVariant::Turn => context.in_walk_kicks.turn.enabled,
            KickVariant::Side => context.in_walk_kicks.side.enabled,
        })
        .copied()
        .collect::<Vec<_>>();

        let strength = match playing_situation {
            PlayingSituation::KickOff => context.decision_parameters.kick_off_kick_strength,
            PlayingSituation::CornerKick => context.decision_parameters.corner_kick_strength,
            PlayingSituation::PenaltyShot => context.decision_parameters.penalty_shot_kick_strength,
            PlayingSituation::Normal => context.decision_parameters.default_kick_strength,
        };

        let targets = collect_kick_targets(&context, playing_situation);

        let mut kick_decisions = kick_decisions_from_targets(
            &targets,
            &variants,
            &sides,
            strength,
            ball_position,
            context.in_walk_kicks,
        );

        kick_decisions.sort_by(|left, right| {
            compare_decisions(
                left,
                right,
                ball_position,
                *context.ground_to_upcoming_support,
                context.obstacles,
                context.decision_parameters,
            )
        });

        let mut instant_kick_decisions = generate_decisions_for_instant_kicks(
            &variants,
            &sides,
            context.in_walk_kicks,
            ball_position,
            context.field_dimensions,
            *context.ground_to_field,
            context.filtered_game_controller_state,
            context.decision_parameters,
        );
        instant_kick_decisions.sort_by(|left, right| {
            compare_decisions(
                left,
                right,
                ball_position,
                *context.ground_to_upcoming_support,
                context.obstacles,
                context.decision_parameters,
            )
        });

        Ok(MainOutputs {
            kick_decisions: Some(kick_decisions).into(),
            instant_kick_decisions: Some(instant_kick_decisions).into(),
        })
    }
}

fn is_ball_in_opponents_corners(
    ball_position: Point2<Field>,
    field_dimensions: &FieldDimensions,
    parameters: &DecisionParameters,
) -> bool {
    let left_opponent_corner =
        field_dimensions.corner(Half::Opponent, field_dimensions::Side::Left);
    let right_opponent_corner =
        field_dimensions.corner(Half::Opponent, field_dimensions::Side::Right);
    let ball_near_left_opponent_corner =
        distance(ball_position, left_opponent_corner) < parameters.distance_to_corner;
    let ball_near_right_opponent_corner =
        distance(ball_position, right_opponent_corner) < parameters.distance_to_corner;
    ball_near_left_opponent_corner || ball_near_right_opponent_corner
}

fn determine_playing_situation(
    filtered_game_controller_state: Option<&FilteredGameControllerState>,
    ball_position: Point2<Field>,
    field_dimensions: &FieldDimensions,
    parameters: &DecisionParameters,
) -> PlayingSituation {
    let is_ball_in_opponent_corner =
        is_ball_in_opponents_corners(ball_position, field_dimensions, parameters);
    match filtered_game_controller_state {
        Some(FilteredGameControllerState {
            game_state: FilteredGameState::Playing { kick_off: true, .. },
            game_phase: GamePhase::Normal,
            opponent_game_state:
                FilteredGameState::Playing {
                    ball_is_free: false,
                    ..
                },
            ..
        }) => PlayingSituation::KickOff,
        Some(FilteredGameControllerState {
            game_phase: GamePhase::PenaltyShootout { .. },
            kicking_team: Some(Team::Hulks),
            ..
        })
        | Some(FilteredGameControllerState {
            sub_state: Some(SubState::PenaltyKick),
            kicking_team: Some(Team::Hulks),
            ..
        }) => PlayingSituation::PenaltyShot,
        _ if is_ball_in_opponent_corner => PlayingSituation::CornerKick,
        _ => PlayingSituation::Normal,
    }
}

fn collect_kick_targets(
    context: &CycleContext,
    playing_situation: PlayingSituation,
) -> Vec<Point2<Ground>> {
    match playing_situation {
        PlayingSituation::KickOff => generate_kick_off_kick_targets(context),
        PlayingSituation::CornerKick => generate_corner_kick_targets(context),
        PlayingSituation::PenaltyShot => generate_penalty_shot_kick_targets(context),
        PlayingSituation::Normal => generate_goal_line_kick_targets(context),
    }
}

fn generate_corner_kick_targets(context: &CycleContext) -> Vec<Point2<Ground>> {
    let field_to_ground = context.ground_to_field.inverse();
    let field_dimensions = &context.field_dimensions;
    let parameters = &context.decision_parameters;

    let from_corner_kick_target_x =
        field_dimensions.length / 2.0 - parameters.corner_kick_target_distance_to_goal;
    let target = field_to_ground * point![from_corner_kick_target_x, 0.0];
    vec![target]
}

fn generate_goal_line_kick_targets(context: &CycleContext) -> Vec<Point2<Ground>> {
    let field_to_ground = context.ground_to_field.inverse();
    let field_dimensions = &context.field_dimensions;

    let left_goal_half = field_to_ground
        * point![
            field_dimensions.length / 2.0 + 0.1,
            field_dimensions.goal_inner_width / 4.0
        ];
    let right_goal_half = field_to_ground
        * point![
            field_dimensions.length / 2.0 + 0.1,
            -field_dimensions.goal_inner_width / 4.0
        ];
    vec![left_goal_half, right_goal_half]
}

fn generate_kick_off_kick_targets(context: &CycleContext) -> Vec<Point2<Ground>> {
    let field_to_ground = context.ground_to_field.inverse();
    let field_dimensions = &context.field_dimensions;

    let left_kick_off_target = field_to_ground
        * point![
            0.0,
            field_dimensions.width / 2.0 - field_dimensions.center_circle_diameter,
        ];
    let right_kick_off_target = field_to_ground
        * point![
            0.0,
            -(field_dimensions.width / 2.0 - field_dimensions.center_circle_diameter),
        ];

    vec![left_kick_off_target, right_kick_off_target]
}

fn generate_penalty_shot_kick_targets(context: &CycleContext) -> Vec<Point2<Ground>> {
    let field_to_ground = context.ground_to_field.inverse();
    let field_dimensions = &context.field_dimensions;

    let left_target = field_to_ground
        * point![
            field_dimensions.length / 2.0,
            field_dimensions.goal_inner_width / 3.5
        ];
    let right_target = field_to_ground
        * point![
            field_dimensions.length / 2.0,
            -field_dimensions.goal_inner_width / 3.5
        ];

    vec![left_target, right_target]
}

fn compare_decisions(
    left: &KickDecision,
    right: &KickDecision,
    ball_position: Point2<Ground>,
    ground_to_upcoming_support: Isometry2<Ground, UpcomingSupport>,
    obstacles: &[Obstacle],
    parameters: &DecisionParameters,
) -> Ordering {
    let left_in_obstacle = is_inside_any_obstacle(left.kick_pose, obstacles, parameters);
    let right_in_obstacle = is_inside_any_obstacle(right.kick_pose, obstacles, parameters);
    let left_is_intersecting_with_obstacle =
        is_intersecting_with_an_obstacle(obstacles, ball_position, left.target, parameters);
    let right_is_intersecting_with_obstacle =
        is_intersecting_with_an_obstacle(obstacles, ball_position, right.target, parameters);
    let distance_to_left = distance_to_kick_pose(
        ground_to_upcoming_support * left.kick_pose,
        parameters.angle_distance_weight,
    );
    let distance_to_right = distance_to_kick_pose(
        ground_to_upcoming_support * right.kick_pose,
        parameters.angle_distance_weight,
    );

    match (
        left_in_obstacle,
        right_in_obstacle,
        left_is_intersecting_with_obstacle,
        right_is_intersecting_with_obstacle,
    ) {
        (false, true, _, _) => Ordering::Less,
        (true, false, _, _) => Ordering::Greater,
        (_, _, false, true) => Ordering::Less,
        (_, _, true, false) => Ordering::Greater,
        _ => distance_to_left.total_cmp(&distance_to_right),
    }
}

fn is_intersecting_with_an_obstacle(
    obstacles: &[Obstacle],
    ball_position: Point2<Ground>,
    target: Point2<Ground>,
    parameters: &DecisionParameters,
) -> bool {
    let ball_to_target = LineSegment::new(ball_position, target);
    let closest_obstructing = obstacles
        .iter()
        .map(|obstacle| Circle::new(obstacle.position, obstacle.radius_at_foot_height))
        .filter(|circle| circle.intersects_line_segment(&ball_to_target))
        .max_by(|left, right| {
            let distance_to_left = distance(ball_position, left.center);
            let distance_to_right = distance(ball_position, right.center);
            distance_to_left.total_cmp(&distance_to_right)
        });
    if let Some(mut obstacle) = closest_obstructing {
        let distance_to_ball = distance(obstacle.center, ball_position);
        if distance_to_ball < parameters.min_obstacle_distance {
            let ball_to_obstacle = obstacle.center - ball_position;
            let push_direction = ball_to_obstacle.try_normalize(0.0).unwrap_or_default();
            obstacle.center = ball_position + push_direction * parameters.min_obstacle_distance;
        }
        obstacle.intersects_line_segment(&ball_to_target)
    } else {
        false
    }
}

#[allow(clippy::too_many_arguments)]
fn generate_decisions_for_instant_kicks(
    kick_variants: &[KickVariant],
    sides: &[Side; 2],
    in_walk_kicks: &InWalkKicksParameters,
    ball_position: Point2<Ground>,
    field_dimensions: &FieldDimensions,
    ground_to_field: Isometry2<Ground, Field>,
    filtered_game_controller_state: Option<&FilteredGameControllerState>,
    parameters: &DecisionParameters,
) -> Vec<KickDecision> {
    let field_to_ground = ground_to_field.inverse();

    iproduct!(sides, kick_variants)
        .filter_map(|(&kicking_side, &variant)| {
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
            let opponent_goal_center = field_to_ground * point![field_dimensions.length / 2.0, 0.0];
            let own_goal_center = field_to_ground * point![-field_dimensions.length / 2.0, 0.0];
            let is_target_closer_to_opponent_goal = (distance(target, opponent_goal_center)
                + parameters.closer_to_goal_threshold)
                < distance(ball_position, opponent_goal_center);
            let goal_box_radius = nalgebra::vector![
                field_dimensions.goal_box_area_length,
                field_dimensions.goal_box_area_width / 2.0
            ]
            .norm();
            let is_ball_close_to_own_goal =
                distance(ball_position, own_goal_center) < goal_box_radius;
            let is_target_farer_away_from_our_goal = distance(target, own_goal_center)
                > (distance(ball_position, own_goal_center) + parameters.closer_to_goal_threshold);
            let scores_goal = is_scoring_goal(
                ground_to_field * target,
                ground_to_field * ball_position,
                field_dimensions,
                parameters,
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

            if is_own_kick_off && is_inside_field && is_inside_kick_off_target_region {
                let kick_pose = compute_kick_pose(ball_position, target, kick_info, kicking_side);
                Some(KickDecision {
                    target,
                    variant,
                    kicking_side,
                    kick_pose,
                    strength: parameters.kick_off_kick_strength,
                })
            } else if !is_own_kick_off && (is_inside_field && is_strategic_target || scores_goal) {
                let kick_pose = compute_kick_pose(ball_position, target, kick_info, kicking_side);
                Some(KickDecision {
                    target,
                    variant,
                    kicking_side,
                    kick_pose,
                    strength: parameters.default_kick_strength,
                })
            } else {
                None
            }
        })
        .collect()
}

fn is_scoring_goal(
    target: Point2<Field>,
    ball_position: Point2<Field>,
    field_dimensions: &FieldDimensions,
    parameters: &DecisionParameters,
) -> bool {
    let ball_to_target = LineSegment::new(ball_position, target);
    let opponent_goal_line = LineSegment::new(
        point![
            field_dimensions.length / 2.0,
            field_dimensions.goal_inner_width / 2.0 - parameters.goal_accuracy_margin
        ],
        point![
            field_dimensions.length / 2.0,
            -field_dimensions.goal_inner_width / 2.0 + parameters.goal_accuracy_margin
        ],
    );
    ball_to_target.intersects_line_segment(opponent_goal_line)
}

fn kick_decisions_from_targets(
    targets_to_kick_to: &[Point2<Ground>],
    variants: &[KickVariant],
    kicking_sides: &[Side],
    strength: f32,
    ball_position: Point2<Ground>,
    in_walk_kicks: &InWalkKicksParameters,
) -> Vec<KickDecision> {
    targets_to_kick_to
        .iter()
        .flat_map(|&target| {
            iproduct!(kicking_sides, variants).map(move |(&kicking_side, &variant)| {
                let kick_info = &in_walk_kicks[variant];
                let kick_pose = compute_kick_pose(ball_position, target, kick_info, kicking_side);
                KickDecision {
                    target,
                    variant,
                    kicking_side,
                    kick_pose,
                    strength,
                }
            })
        })
        .collect()
}

fn distance_to_kick_pose(kick_pose: Pose2<UpcomingSupport>, angle_distance_weight: f32) -> f32 {
    kick_pose.position().coords().norm()
        + angle_distance_weight * kick_pose.orientation().angle().abs()
}

fn is_inside_any_obstacle(
    kick_pose: Pose2<Ground>,
    obstacles: &[Obstacle],
    parameters: &DecisionParameters,
) -> bool {
    let position = kick_pose.position();
    obstacles.iter().any(|obstacle| {
        let circle = Circle {
            center: obstacle.position,
            radius: obstacle.radius_at_foot_height + parameters.kick_pose_robot_radius,
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
        * Isometry2::from(kick_info.position_offset.framed())
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
