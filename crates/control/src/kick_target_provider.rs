use color_eyre::Result;
use ordered_float::NotNan;

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{AdditionalOutput, MainOutput};
use geometry::{circle::Circle, line_segment::LineSegment, two_line_segments::TwoLineSegments};
use linear_algebra::{distance, point, Isometry2, Point2};
use serde::{Deserialize, Serialize};
use spl_network_messages::{GamePhase, SubState, Team};
use types::{
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState,
    kick_target::{KickTarget, KickTargetWithKickVariants, PlayingSituation},
    motion_command::KickVariant,
    obstacles::Obstacle,
    parameters::FindKickTargetsParameters,
    world_state::BallState,
};

trait WithVariants {
    fn with_variants(self, variants: &[KickVariant]) -> Vec<KickTargetWithKickVariants>;
}
impl WithVariants for Vec<KickTarget> {
    fn with_variants(self, variants: &[KickVariant]) -> Vec<KickTargetWithKickVariants> {
        self.into_iter()
            .map(|kick_target| KickTargetWithKickVariants {
                kick_target,
                kick_variants: variants.to_vec(),
            })
            .collect()
    }
}

#[derive(Deserialize, Serialize)]
pub struct KickTargetProvider;

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    ball_state: RequiredInput<Option<BallState>, "ball_state?">,
    ground_to_field: RequiredInput<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    obstacles: Input<Vec<Obstacle>, "obstacles">,
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,

    ball_radius_for_kick_target_selection:
        Parameter<f32, "kick_target_provider.ball_radius_for_kick_target_selection">,
    find_kick_targets_parameters:
        Parameter<FindKickTargetsParameters, "kick_target_provider.find_kick_targets">,
    max_kick_around_obstacle_angle:
        Parameter<f32, "kick_target_provider.max_kick_around_obstacle_angle">,
    corner_kick_strength: Parameter<f32, "kick_target_provider.corner_kick_strength">,
    kick_off_kick_strength: Parameter<f32, "kick_target_provider.kick_off_kick_strength">,
    penalty_shot_kick_strength: Parameter<f32, "kick_target_provider.penalty_shot_kick_strength">,
    kick_off_kick_variants:
        Parameter<Vec<KickVariant>, "kick_target_provider.kick_off_kick_variants">,
    corner_kick_variants: Parameter<Vec<KickVariant>, "kick_target_provider.corner_kick_variants">,
    penalty_kick_kick_variants:
        Parameter<Vec<KickVariant>, "kick_target_provider.penalty_kick_kick_variants">,
    goal_line_kick_variants:
        Parameter<Vec<KickVariant>, "kick_target_provider.goal_line_kick_variants">,
    playing_situation: AdditionalOutput<PlayingSituation, "playing_situation">,
}

struct CollectKickTargetsParameter<'cycle> {
    find_kick_targets_parameter: &'cycle FindKickTargetsParameters,
    max_kick_around_obstacle_angle: f32,
    corner_kick_strength: f32,
    kick_off_kick_strength: f32,
    penalty_shot_kick_strength: f32,
    kick_off_kick_variants: Vec<KickVariant>,
    corner_kick_variants: Vec<KickVariant>,
    penalty_kick_kick_variants: Vec<KickVariant>,
    goal_line_kick_variants: Vec<KickVariant>,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub kick_opportunities: MainOutput<Vec<KickTargetWithKickVariants>>,
    pub obstacle_circles: MainOutput<Vec<Circle<Ground>>>,
    pub allow_instant_kicks: MainOutput<bool>,
}

impl KickTargetProvider {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&self, mut context: CycleContext) -> Result<MainOutputs> {
        let obstacle_circles = generate_obstacle_circles(
            context.obstacles,
            *context.ball_radius_for_kick_target_selection,
        );

        let playing_situation = get_playing_situation(
            context.filtered_game_controller_state,
            context.ball_state.ball_in_field,
            context.find_kick_targets_parameters,
            context.field_dimensions,
        );

        context
            .playing_situation
            .fill_if_subscribed(|| playing_situation);

        let collect_kick_targets_parameters = CollectKickTargetsParameter {
            find_kick_targets_parameter: context.find_kick_targets_parameters,
            max_kick_around_obstacle_angle: *context.max_kick_around_obstacle_angle,
            corner_kick_strength: *context.corner_kick_strength,
            kick_off_kick_strength: *context.kick_off_kick_strength,
            penalty_shot_kick_strength: *context.penalty_shot_kick_strength,
            kick_off_kick_variants: context.kick_off_kick_variants.clone(),
            corner_kick_variants: context.corner_kick_variants.clone(),
            penalty_kick_kick_variants: context.penalty_kick_kick_variants.clone(),
            goal_line_kick_variants: context.goal_line_kick_variants.clone(),
        };

        let (kick_opportunities, allow_instant_kicks) = collect_kick_targets(
            *context.ground_to_field,
            context.field_dimensions,
            &obstacle_circles,
            *context.ball_state,
            collect_kick_targets_parameters,
            playing_situation,
        );

        Ok(MainOutputs {
            kick_opportunities: kick_opportunities.into(),
            obstacle_circles: obstacle_circles.into(),
            allow_instant_kicks: allow_instant_kicks.into(),
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

fn collect_kick_targets(
    ground_to_field: Isometry2<Ground, Field>,
    field_dimensions: &FieldDimensions,
    obstacle_circles: &[Circle<Ground>],
    ball_state: BallState,
    collect_kick_targets_parameters: CollectKickTargetsParameter<'_>,
    playing_situation: PlayingSituation,
) -> (Vec<KickTargetWithKickVariants>, bool) {
    let field_to_ground = ground_to_field.inverse();

    let (mut kick_opportunities, allow_instant_kicks, respect_obstacles) = match playing_situation {
        PlayingSituation::KickOff => (
            generate_kick_off_kick_targets(
                field_dimensions,
                field_to_ground,
                collect_kick_targets_parameters.kick_off_kick_strength,
            )
            .with_variants(&collect_kick_targets_parameters.kick_off_kick_variants),
            true,
            false,
        ),
        PlayingSituation::CornerKick => (
            generate_corner_kick_targets(
                collect_kick_targets_parameters.find_kick_targets_parameter,
                field_dimensions,
                field_to_ground,
                collect_kick_targets_parameters.corner_kick_strength,
            )
            .with_variants(&collect_kick_targets_parameters.corner_kick_variants),
            true,
            true,
        ),
        PlayingSituation::PenaltyShot => (
            generate_penalty_shot_kick_targets(
                field_dimensions,
                field_to_ground,
                collect_kick_targets_parameters.penalty_shot_kick_strength,
            )
            .with_variants(&collect_kick_targets_parameters.penalty_kick_kick_variants),
            true,
            false,
        ),
        PlayingSituation::Normal => (
            generate_goal_line_kick_targets(field_dimensions, field_to_ground)
                .with_variants(&collect_kick_targets_parameters.goal_line_kick_variants),
            true,
            true,
        ),
    };

    if respect_obstacles {
        kick_opportunities = modify_with_obstacles(
            kick_opportunities,
            obstacle_circles,
            collect_kick_targets_parameters,
            ball_state,
            field_dimensions,
            ground_to_field,
        );
    }

    (kick_opportunities, allow_instant_kicks)
}

fn is_ball_in_opponents_corners(
    ball_position: Point2<Field>,
    parameters: &FindKickTargetsParameters,
    field_dimensions: &FieldDimensions,
) -> bool {
    let left_opponent_corner = point![field_dimensions.length / 2.0, field_dimensions.width / 2.0];
    let right_opponent_corner =
        point![field_dimensions.length / 2.0, -field_dimensions.width / 2.0];
    let ball_near_left_opponent_corner =
        distance(ball_position, left_opponent_corner) < parameters.distance_from_corner;
    let ball_near_right_opponent_corner =
        distance(ball_position, right_opponent_corner) < parameters.distance_from_corner;
    ball_near_left_opponent_corner || ball_near_right_opponent_corner
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

fn generate_kick_off_kick_targets(
    field_dimensions: &FieldDimensions,
    field_to_ground: Isometry2<Field, Ground>,
    kick_off_kick_strength: f32,
) -> Vec<KickTarget> {
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

    vec![
        KickTarget::new_with_strength(left_kick_off_target, kick_off_kick_strength),
        KickTarget::new_with_strength(right_kick_off_target, kick_off_kick_strength),
    ]
}

fn generate_penalty_shot_kick_targets(
    field_dimensions: &FieldDimensions,
    field_to_ground: Isometry2<Field, Ground>,
    penalty_shot_kick_strength: f32,
) -> Vec<KickTarget> {
    let left_kick_off_target = field_to_ground
        * point![
            field_dimensions.length / 2.0,
            field_dimensions.goal_inner_width / 4.0
        ];
    let right_kick_off_target = field_to_ground
        * point![
            field_dimensions.length / 2.0,
            -field_dimensions.goal_inner_width / 4.0
        ];

    vec![
        KickTarget::new_with_strength(left_kick_off_target, penalty_shot_kick_strength),
        KickTarget::new_with_strength(right_kick_off_target, penalty_shot_kick_strength),
    ]
}

fn get_playing_situation(
    filtered_game_controller_state: Option<&FilteredGameControllerState>,
    ball_position: Point2<Field>,
    parameters: &FindKickTargetsParameters,
    field_dimensions: &FieldDimensions,
) -> PlayingSituation {
    let is_ball_in_opponent_corner =
        is_ball_in_opponents_corners(ball_position, parameters, field_dimensions);
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
            kicking_team: Team::Hulks,
            ..
        })
        | Some(FilteredGameControllerState {
            sub_state: Some(SubState::PenaltyKick),
            kicking_team: Team::Hulks,
            ..
        }) => PlayingSituation::PenaltyShot,
        _ if is_ball_in_opponent_corner => PlayingSituation::CornerKick,
        _ => PlayingSituation::Normal,
    }
}

fn modify_with_obstacles(
    kick_opportunities: Vec<KickTargetWithKickVariants>,
    obstacle_circles: &[Circle<Ground>],
    collect_kick_targets_parameters: CollectKickTargetsParameter<'_>,
    ball_state: BallState,
    field_dimensions: &FieldDimensions,
    ground_to_field: Isometry2<Ground, Field>,
) -> Vec<KickTargetWithKickVariants> {
    let obstacle_circles: Vec<_> = obstacle_circles
        .iter()
        .map(|circle| {
            let ball_to_obstacle = circle.center - ball_state.ball_in_ground;
            let safety_radius = circle.radius
                / collect_kick_targets_parameters
                    .max_kick_around_obstacle_angle
                    .sin();
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

    kick_opportunities
        .iter()
        .flat_map(|kick_opportunity| {
            let ball_to_target = LineSegment(
                ball_state.ball_in_ground,
                kick_opportunity.kick_target.position,
            );
            let closest_intersecting_obstacle = obstacle_circles
                .iter()
                .filter(|circle| circle.intersects_line_segment(&ball_to_target))
                .min_by_key(|circle| NotNan::new(circle.center.coords().norm()).unwrap());
            let targets = match closest_intersecting_obstacle {
                Some(circle) => {
                    let TwoLineSegments(left_tangent, right_tangent) = circle
                        .tangents_with_point(ball_state.ball_in_ground)
                        .unwrap();
                    [left_tangent, right_tangent]
                        .into_iter()
                        .map(|tangent| {
                            let kick_direction =
                                (tangent.0 - ball_state.ball_in_ground).normalize();
                            // TODO: drop this constant?
                            ball_state.ball_in_ground + kick_direction * 2.0
                        })
                        .filter(|&position| {
                            field_dimensions.is_inside_field(ground_to_field * position)
                        })
                        .map(KickTarget::new)
                        .collect()
                }
                None => vec![kick_opportunity.kick_target],
            };
            targets
                .into_iter()
                .map(|target| KickTargetWithKickVariants {
                    kick_target: target,
                    kick_variants: kick_opportunity.kick_variants.clone(),
                })
        })
        .collect()
}
