use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use nalgebra::{point, vector};
use spl_network_messages::{GameState, SubState, Team};
use types::{FieldDimensions, GameControllerState, Rectangle, RuleObstacle};

pub struct RuleObstacleComposer {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub game_controller_state: RequiredInput<Option<GameControllerState>, "game_controller_state?">,
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
        let mut rule_obstacles = Vec::new();
        if let GameControllerState {
            sub_state: Some(SubState::PenaltyKick),
            game_state: GameState::Playing,
            ..
        } = context.game_controller_state
        {
            let penalty_box_obstacle = create_penalty_box(
                context.field_dimensions,
                context.game_controller_state.kicking_team,
            );
            rule_obstacles.push(penalty_box_obstacle);
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
        //Striker may still enter opponent penalty box so this doesn't stop us from defending our own goal
        Team::Uncertain => 1.0,
    };
    let half_field_length = field_dimensions.length / 2.0;
    let half_penalty_area_length = field_dimensions.penalty_area_length / 2.0;
    let center_x = side_factor * (half_field_length - half_penalty_area_length);
    RuleObstacle::Rectangle(Rectangle::new_with_center(
        point![center_x, 0.0],
        vector![
            field_dimensions.penalty_area_length,
            field_dimensions.penalty_area_width
        ],
    ))
}
