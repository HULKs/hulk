use std::time::SystemTime;

use color_eyre::Result;
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::MainOutput;
use linear_algebra::{point, Isometry2, Point2, Vector2};
use types::{
    cycle_time::CycleTime,
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState,
    obstacles::{Obstacle, ObstacleKind},
    parameters::LookActionParameters,
    point_of_interest::PointOfInterest,
    world_state::BallState,
};

#[derive(Deserialize, Serialize)]
pub struct ActiveVision {
    field_mark_positions: Vec<Point2<Field>>,
    last_point_of_interest_switch: Option<SystemTime>,
    current_point_of_interest: PointOfInterest,
}

#[context]
pub struct CreationContext {
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
}

#[context]
pub struct CycleContext {
    ball: Input<Option<BallState>, "ball_state?">,
    rule_ball: Input<Option<BallState>, "rule_ball_state?">,
    cycle_time: Input<CycleTime, "cycle_time">,
    obstacles: Input<Vec<Obstacle>, "obstacles">,
    parameters: Parameter<LookActionParameters, "behavior.look_action">,
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub position_of_interest: MainOutput<Point2<Ground>>,
}

impl ActiveVision {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            field_mark_positions: generate_field_mark_positions(context.field_dimensions),
            last_point_of_interest_switch: None,
            current_point_of_interest: PointOfInterest::default(),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let cycle_start_time = context.cycle_time.start_time;

        if let Some(&ground_to_field) = context.ground_to_field {
            if let Some(FilteredGameControllerState {
                game_state:
                    FilteredGameState::Playing {
                        ball_is_free: false,
                        kick_off: true,
                    },
                ..
            }) = context.filtered_game_controller_state
            {
                self.current_point_of_interest = PointOfInterest::Ball
            } else if self.last_point_of_interest_switch.is_none()
                || cycle_start_time.duration_since(self.last_point_of_interest_switch.unwrap())?
                    > context.parameters.position_of_interest_switch_interval
            {
                self.current_point_of_interest = next_point_of_interest(
                    self.current_point_of_interest,
                    &self.field_mark_positions,
                    context.obstacles,
                    *context.parameters,
                    ground_to_field,
                    context.rule_ball.or(context.ball),
                );

                self.last_point_of_interest_switch = Some(cycle_start_time);
            }
            let position_of_interest = match self.current_point_of_interest {
                PointOfInterest::Forward => context.parameters.look_forward_position,
                PointOfInterest::FieldMark { absolute_position } => {
                    ground_to_field.inverse() * absolute_position
                }
                PointOfInterest::Ball => {
                    if let Some(ball_state) = context.ball {
                        ball_state.ball_in_ground
                    } else {
                        context.parameters.look_forward_position
                    }
                }
                PointOfInterest::Obstacle { absolute_position } => {
                    ground_to_field.inverse() * absolute_position
                }
            };

            Ok(MainOutputs {
                position_of_interest: position_of_interest.into(),
            })
        } else {
            Ok(MainOutputs {
                position_of_interest: context.parameters.look_forward_position.into(),
            })
        }
    }
}

fn is_position_visible(position: Point2<Ground>, parameters: LookActionParameters) -> bool {
    Vector2::x_axis().angle(position.coords()).abs() < parameters.angle_threshold
        && position.coords().norm() < parameters.distance_threshold
}

fn closest_field_mark_visible(
    field_mark_positions: &[Point2<Field>],
    parameters: LookActionParameters,
    ground_to_field: Isometry2<Ground, Field>,
) -> Option<Point2<Ground>> {
    field_mark_positions
        .iter()
        .map(|position| ground_to_field.inverse() * position)
        .filter(|position| is_position_visible(*position, parameters))
        .min_by_key(|position| NotNan::new(position.coords().norm()).unwrap())
}

fn closest_interesting_obstacle_visible(
    obstacles: &[Obstacle],
    parameters: LookActionParameters,
) -> Option<Point2<Ground>> {
    obstacles
        .iter()
        .filter(|obstacle| matches!(obstacle.kind, ObstacleKind::Robot | ObstacleKind::Unknown))
        .map(|obstacle| obstacle.position)
        .filter(|obstacle_position| is_position_visible(*obstacle_position, parameters))
        .min_by_key(|position| NotNan::new(position.coords().norm()).unwrap())
}

fn generate_field_mark_positions(field_dimensions: &FieldDimensions) -> Vec<Point2<Field>> {
    let left_center_circle_junction = point![0.0, field_dimensions.center_circle_diameter / 2.0];
    let right_center_circle_junction = point![0.0, -field_dimensions.center_circle_diameter / 2.0];
    let left_center_t_junction = point![0.0, field_dimensions.width / 2.0];
    let right_center_t_junction = point![0.0, -field_dimensions.width / 2.0];
    let left_opponent_penalty_box_corner = point![
        field_dimensions.length / 2.0 - field_dimensions.penalty_area_length,
        field_dimensions.penalty_area_width / 2.0
    ];
    let right_opponent_penalty_box_corner = point![
        field_dimensions.length / 2.0 - field_dimensions.penalty_area_length,
        -field_dimensions.penalty_area_width / 2.0
    ];
    let left_own_penalty_box_corner = point![
        -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length,
        field_dimensions.penalty_area_width / 2.0
    ];
    let right_own_penalty_box_corner = point![
        -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length,
        -field_dimensions.penalty_area_width / 2.0
    ];
    vec![
        left_center_circle_junction,
        right_center_circle_junction,
        left_center_t_junction,
        right_center_t_junction,
        left_opponent_penalty_box_corner,
        right_opponent_penalty_box_corner,
        left_own_penalty_box_corner,
        right_own_penalty_box_corner,
    ]
}

fn next_point_of_interest(
    current_point_of_interest: PointOfInterest,
    field_mark_positions: &[Point2<Field>],
    obstacles: &[Obstacle],
    parameters: LookActionParameters,
    ground_to_field: Isometry2<Ground, Field>,
    ball: Option<&BallState>,
) -> PointOfInterest {
    match current_point_of_interest {
        PointOfInterest::Forward => {
            let field_mark_of_interest =
                closest_field_mark_visible(field_mark_positions, parameters, ground_to_field);

            match (field_mark_of_interest, ball) {
                (Some(field_mark_position), _) => PointOfInterest::FieldMark {
                    absolute_position: ground_to_field * field_mark_position,
                },
                (_, Some(_)) => PointOfInterest::Ball,
                (None, None) => {
                    let closest_interesting_obstacle_position =
                        closest_interesting_obstacle_visible(obstacles, parameters);
                    match closest_interesting_obstacle_position {
                        Some(interesting_obstacle_position) => PointOfInterest::Obstacle {
                            absolute_position: ground_to_field * interesting_obstacle_position,
                        },
                        None => PointOfInterest::Forward,
                    }
                }
            }
        }
        PointOfInterest::FieldMark { .. } => match ball {
            Some(_) => PointOfInterest::Ball,
            None => {
                let closest_interesting_obstacle_position =
                    closest_interesting_obstacle_visible(obstacles, parameters);

                match closest_interesting_obstacle_position {
                    Some(interesting_obstacle_position) => PointOfInterest::Obstacle {
                        absolute_position: ground_to_field * interesting_obstacle_position,
                    },
                    None => PointOfInterest::Forward,
                }
            }
        },
        PointOfInterest::Ball => {
            let closest_interesting_obstacle_position =
                closest_interesting_obstacle_visible(obstacles, parameters);

            match closest_interesting_obstacle_position {
                Some(interesting_obstacle_position) => PointOfInterest::Obstacle {
                    absolute_position: ground_to_field * interesting_obstacle_position,
                },
                None => PointOfInterest::Forward,
            }
        }
        PointOfInterest::Obstacle { .. } => PointOfInterest::Forward,
    }
}
