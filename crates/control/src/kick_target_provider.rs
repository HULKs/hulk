use color_eyre::Result;
use ordered_float::NotNan;

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{AdditionalOutput, MainOutput};
use geometry::{circle::Circle, line_segment::LineSegment, two_line_segments::TwoLineSegments};
use linear_algebra::{distance, point, Isometry2, Point2};
use serde::{Deserialize, Serialize};
use types::{
    field_dimensions::FieldDimensions, kick_target::KickTarget, obstacles::Obstacle,
    parameters::FindKickTargetsParameters, world_state::BallState,
};

#[derive(Deserialize, Serialize)]
pub struct KickTargetProvider;

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    ball_state: RequiredInput<Option<BallState>, "ball_state?">,
    ground_to_field: RequiredInput<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    obstacles: Input<Vec<Obstacle>, "obstacles">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,

    ball_radius_for_kick_target_selection:
        Parameter<f32, "kick_target_provider.ball_radius_for_kick_target_selection">,
    find_kick_targets:
        Parameter<FindKickTargetsParameters, "kick_target_provider.find_kick_targets">,
    max_kick_around_obstacle_angle:
        Parameter<f32, "kick_target_provider.max_kick_around_obstacle_angle">,

    corner_kick_strength: Parameter<f32, "kick_target_provider.corner_kick_strength">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub kick_targets: MainOutput<Vec<KickTarget>>,
    pub obstacle_circles: MainOutput<Vec<Circle<Ground>>>,
}

impl KickTargetProvider {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&self, context: CycleContext) -> Result<MainOutputs> {
        let ball_position = context.ball_state.ball_in_ground;

        let obstacle_circles = generate_obstacle_circles(
            context.obstacles,
            *context.ball_radius_for_kick_target_selection,
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

        Ok(MainOutputs {
            kick_targets: kick_targets.into(),
            obstacle_circles: obstacle_circles.into(),
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
    ball_position: Point2<Ground>,
    max_kick_around_obstacle_angle: f32,
    parameters: &FindKickTargetsParameters,
    corner_kick_strength: f32,
) -> Vec<KickTarget> {
    let field_to_ground = ground_to_field.inverse();

    let kick_targets = if is_ball_in_opponents_corners(
        ball_position,
        parameters,
        field_dimensions,
        ground_to_field,
    ) {
        generate_corner_kick_targets(
            parameters,
            field_dimensions,
            field_to_ground,
            corner_kick_strength,
        )
    } else {
        generate_goal_line_kick_targets(field_dimensions, field_to_ground)
    };

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
