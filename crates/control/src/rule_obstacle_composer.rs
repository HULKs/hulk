use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use geometry::{circle::Circle, rectangle::Rectangle};
use nalgebra::{point, vector, Point2};
use serde::{Deserialize, Serialize};
use spl_network_messages::{SubState, Team};
use types::{
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_states::FilteredGameState,
    rule_obstacles::RuleObstacle,
    world_state::BallState,
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

    center_circle_obstacle_increasement: Parameter<f32, "center_circle_obstacle_increasement">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
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
        let free_kick_obstacle_radius = 0.75;

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
                    kicking_team: Team::Opponent | Team::Uncertain,
                    game_state: FilteredGameState::Playing { .. },
                    ..
                },
                Some(ball),
            ) => {
                let free_kick_obstacle = RuleObstacle::Circle(Circle::new(
                    ball.ball_in_field,
                    free_kick_obstacle_radius,
                ));
                rule_obstacles.push(free_kick_obstacle);
            }
            (
                FilteredGameControllerState {
                    game_state: FilteredGameState::Playing { ball_is_free:false, kick_off:true },
                    ..
                },
                _,
            ) => {
                let center_circle_obstacle = RuleObstacle::Circle(Circle::new(
                    Point2::origin(),
                    context.field_dimensions.center_circle_diameter / 2.0
                        * context.center_circle_obstacle_increasement,
                ));
                dbg!("center circle obstacle created");
                rule_obstacles.push(center_circle_obstacle);
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
                );
                rule_obstacles.push(penalty_box_obstacle);
            }
            _ => (),
        };

        Ok(MainOutputs {
            rule_obstacles: rule_obstacles.into(),
        })
    }
}

pub fn create_penalty_box(field_dimensions: &FieldDimensions, kicking_team: Team) -> RuleObstacle {
    let side_factor: f32 = match kicking_team {
        Team::Hulks => 1.0,
        Team::Opponent => -1.0,
        // Striker may still enter opponent penalty box so this doesn't stop us from defending our own goal
        Team::Uncertain => 1.0,
    };
    let half_field_length = field_dimensions.length / 2.0;
    let half_penalty_area_length = field_dimensions.penalty_area_length / 2.0;
    let center_x = side_factor * (half_field_length - half_penalty_area_length);
    RuleObstacle::Rectangle(Rectangle::new_with_center_and_size(
        point![center_x, 0.0],
        vector![
            field_dimensions.penalty_area_length,
            field_dimensions.penalty_area_width
        ],
    ))
}
