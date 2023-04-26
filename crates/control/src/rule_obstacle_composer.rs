use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use nalgebra::point;
use spl_network_messages::{SubState, Team};
use types::{FieldDimensions, GameControllerState, Rectangle, RuleObstacle};

pub struct RuleObstacleComposer {
    obstacle_list: Vec<RuleObstacle>,
}

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
        Ok(Self {
            obstacle_list: Vec::new(),
        })
    }
    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        if let Some(SubState::PenaltyKick) = context.game_controller_state.sub_state {
            let penalty_box_obstacle = create_penalty_box(
                context.field_dimensions,
                context.game_controller_state.kicking_team,
            );
            self.obstacle_list.push(penalty_box_obstacle);
        };

        Ok(MainOutputs {
            rule_obstacles: self.obstacle_list.clone().into(),
        })
    }
}

pub fn create_penalty_box(field_dimensions: &FieldDimensions, kicking_team: Team) -> RuleObstacle {
    let side_factor: f32 = match kicking_team {
        Team::Hulks => 1.0,
        _ => -1.0,
    };
    let half_penalty_area_width = field_dimensions.penalty_area_width / 2.0;
    let half_field_length = field_dimensions.length / 2.0;
    let top_left = point![
        side_factor * (half_field_length - field_dimensions.penalty_area_length),
        half_penalty_area_width
    ];
    let bottom_right = point![side_factor * half_field_length, -half_penalty_area_width];
    RuleObstacle::Rectangle(Rectangle {
        top_left,
        bottom_right,
    })
}
