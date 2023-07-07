use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use nalgebra::{point, vector, Point2};
use spl_network_messages::{GameState, SubState, Team};
use types::{
    BallState, Circle, FieldDimensions, FilteredGameState, GameControllerState, Rectangle,
    RuleObstacle,
};

pub struct RuleObstacleComposer {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub game_controller_state: RequiredInput<Option<GameControllerState>, "game_controller_state?">,
    pub filtered_game_state: RequiredInput<Option<FilteredGameState>, "filtered_game_state?">,
    pub ball_state: Input<Option<BallState>, "ball_state?">,
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
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
        match (
            context.game_controller_state,
            context.filtered_game_state,
            context.ball_state,
        ) {
            (
                GameControllerState {
                    sub_state:
                        Some(
                            SubState::KickIn
                            | SubState::CornerKick
                            | SubState::GoalKick
                            | SubState::PushingFreeKick,
                        ),
                    kicking_team: Team::Opponent | Team::Uncertain,
                    game_state: GameState::Playing,
                    ..
                },
                _,
                Some(ball),
            ) => {
                let obstacle = RuleObstacle::Circle(Circle::new(
                    ball.ball_in_field,
                    free_kick_obstacle_radius,
                ));
                rule_obstacles.push(obstacle);
            }
            (
                GameControllerState {
                    game_state: GameState::Playing,
                    sub_state: None,
                    ..
                },
                FilteredGameState::Playing {
                    ball_is_free: false,
                },
                _,
            ) => {
                let obstacle = RuleObstacle::Circle(Circle::new(
                    Point2::origin(),
                    context.field_dimensions.center_circle_diameter / 2.0,
                ));
                rule_obstacles.push(obstacle);
            }
            (
                GameControllerState {
                    sub_state: Some(SubState::PenaltyKick),
                    game_state: GameState::Playing,
                    ..
                },
                _,
                _,
            ) => {
                let penalty_box_obstacle = create_penalty_box(
                    context.field_dimensions,
                    context.game_controller_state.kicking_team,
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
