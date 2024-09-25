use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::MainOutput;
use geometry::{circle::Circle, rectangle::Rectangle};
use linear_algebra::{point, vector, Point};
use spl_network_messages::{SubState, Team};
use types::{
    field_dimensions::FieldDimensions, filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState, rule_obstacles::RuleObstacle, world_state::BallState,
};

#[derive(Deserialize, Serialize)]
pub struct RuleObstacleComposer {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    filtered_game_controller_state:
        RequiredInput<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    ball_state: Input<Option<BallState>, "ball_state?">,

    center_circle_obstacle_radius_increase:
        Parameter<f32, "rule_obstacles.center_circle_obstacle_radius_increase">,
    center_circle_ballspace_free_obstacle_radius:
        Parameter<f32, "rule_obstacles.center_circle_ballspace_free_obstacle_radius">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    free_kick_obstacle_radius: Parameter<f32, "rule_obstacles.free_kick_obstacle_radius">,
    penaltykick_box_extension: Parameter<f32, "rule_obstacles.penaltykick_box_extension">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub rule_obstacles: MainOutput<Vec<RuleObstacle>>,
}

impl RuleObstacleComposer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let mut rule_obstacles = Vec::new();
        match (context.filtered_game_controller_state, context.ball_state) {
            (
                FilteredGameControllerState {
                    sub_state:
                        Some(
                            SubState::KickIn
                            | SubState::CornerKick
                            | SubState::GoalKick
                            | SubState::PushingFreeKick,
                        ),
                    kicking_team: Team::Opponent,
                    game_state: FilteredGameState::Playing { .. },
                    ..
                },
                Some(ball),
            ) => {
                let free_kick_obstacle = RuleObstacle::Circle(Circle::new(
                    ball.ball_in_field,
                    *context.free_kick_obstacle_radius,
                ));
                rule_obstacles.push(free_kick_obstacle);
            }
            (
                FilteredGameControllerState {
                    game_state:
                        FilteredGameState::Playing {
                            ball_is_free: false,
                            kick_off: true,
                        },
                    ..
                },
                _,
            ) => {
                let center_circle_obstacle = RuleObstacle::Circle(Circle::new(
                    Point::origin(),
                    context.field_dimensions.center_circle_diameter / 2.0
                        + context.center_circle_obstacle_radius_increase,
                ));
                rule_obstacles.push(center_circle_obstacle);

                let opponent_half_obstacle = RuleObstacle::Rectangle(Rectangle {
                    min: point!(0.0, -context.field_dimensions.width / 2.0),
                    max: point!(
                        context.field_dimensions.length / 2.0,
                        context.field_dimensions.width / 2.0
                    ),
                });
                rule_obstacles.push(opponent_half_obstacle);
            }
            (
                FilteredGameControllerState {
                    sub_state: Some(SubState::PenaltyKick),
                    game_state: FilteredGameState::Playing { .. },
                    ..
                },
                _,
            ) => {
                let penalty_box_obstacle = create_penalty_box(
                    context.field_dimensions,
                    context.filtered_game_controller_state.kicking_team,
                    *context.penaltykick_box_extension,
                );
                rule_obstacles.push(penalty_box_obstacle);
            }
            (
                FilteredGameControllerState {
                    game_state:
                        FilteredGameState::Ready {
                            kicking_team: Some(Team::Hulks),
                        },
                    sub_state: None,
                    ..
                },
                _,
            ) => {
                let center_circle_ballspace_free_obstacle = RuleObstacle::Circle(Circle {
                    center: Point::origin(),
                    radius: *context.center_circle_ballspace_free_obstacle_radius,
                });

                rule_obstacles.push(center_circle_ballspace_free_obstacle);
            }
            (
                FilteredGameControllerState {
                    game_state:
                        FilteredGameState::Ready {
                            kicking_team: Some(Team::Opponent),
                        },
                    sub_state: None,
                    ..
                },
                _,
            ) => {
                let center_circle_obstacle = RuleObstacle::Circle(Circle {
                    center: Point::origin(),
                    radius: context.field_dimensions.center_circle_diameter / 2.0
                        + context.center_circle_obstacle_radius_increase,
                });

                rule_obstacles.push(center_circle_obstacle);
            }
            _ => (),
        };

        Ok(MainOutputs {
            rule_obstacles: rule_obstacles.into(),
        })
    }
}

pub fn create_penalty_box(
    field_dimensions: &FieldDimensions,
    kicking_team: Team,
    penaltykick_box_extension: f32,
) -> RuleObstacle {
    let side_factor: f32 = match kicking_team {
        Team::Hulks => 1.0,
        Team::Opponent => -1.0,
    };
    let half_field_length = field_dimensions.length / 2.0;
    let half_penalty_area_length = field_dimensions.penalty_area_length / 2.0;
    let center_x = side_factor
        * (half_field_length - half_penalty_area_length + penaltykick_box_extension / 2.0);
    RuleObstacle::Rectangle(Rectangle::new_with_center_and_size(
        point![center_x, 0.0],
        vector![
            field_dimensions.penalty_area_length + penaltykick_box_extension,
            field_dimensions.penalty_area_width
        ],
    ))
}
