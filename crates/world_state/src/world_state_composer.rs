use color_eyre::Result;
use coordinate_systems::{Field, Ground};
use linear_algebra::Isometry2;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::MainOutput;
use types::{
    filtered_game_controller_state::FilteredGameControllerState,
    primary_state::PrimaryState,
    roles::Role,
    world_state::{BallState, RobotState, WorldState},
};

#[derive(Deserialize, Serialize)]
pub struct WorldStateComposer {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    ball: Input<Option<BallState>, "ball_state?">,
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    primary_state: Input<PrimaryState, "primary_state">,
    role: Input<Role, "role">,
    rule_ball: Input<Option<BallState>, "rule_ball_state?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub world_state: MainOutput<WorldState>,
}

impl WorldStateComposer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let robot: RobotState = RobotState {
            ground_to_field: context.ground_to_field.copied(),
            primary_state: *context.primary_state,
            role: *context.role,
        };

        let world_state = WorldState {
            ball: context.ball.copied(),
            filtered_game_controller_state: context.filtered_game_controller_state.cloned(),
            robot,
            rule_ball: context.rule_ball.copied(),
        };

        Ok(MainOutputs {
            world_state: world_state.into(),
        })
    }
}
