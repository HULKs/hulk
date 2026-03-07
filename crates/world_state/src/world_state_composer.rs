use booster::FallDownState;
use color_eyre::Result;
use coordinate_systems::{Field, Ground};
use linear_algebra::Isometry2;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::{MainOutput, PerceptionInput};
use types::{
    filtered_game_controller_state::FilteredGameControllerState,
    primary_state::PrimaryState,
    world_state::{BallState, RobotState, WorldState},
};

#[derive(Deserialize, Serialize)]
pub struct WorldStateComposer {
    last_fall_down_state: Option<FallDownState>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    fall_down_state: PerceptionInput<Option<FallDownState>, "FallDownState", "fall_down_state?">,

    ball: Input<Option<BallState>, "ball_state?">,
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    primary_state: Input<PrimaryState, "primary_state">,
    rule_ball: Input<Option<BallState>, "rule_ball_state?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub world_state: MainOutput<WorldState>,
}

impl WorldStateComposer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_fall_down_state: None,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let fall_down_state = context
            .fall_down_state
            .persistent
            .into_iter()
            .chain(context.fall_down_state.temporary)
            .flat_map(|(_time, fall_down_states)| fall_down_states)
            .last()
            .flatten()
            .copied()
            .map_or(self.last_fall_down_state, |fall_down_state| {
                Some(fall_down_state)
            });

        if fall_down_state.is_some() {
            self.last_fall_down_state = fall_down_state;
        }

        let robot: RobotState = RobotState {
            ground_to_field: context.ground_to_field.copied(),
            primary_state: *context.primary_state,
        };

        let world_state = WorldState {
            ball: context.ball.copied(),
            filtered_game_controller_state: context.filtered_game_controller_state.cloned(),
            robot,
            rule_ball: context.rule_ball.copied(),
            fall_down_state,
        };

        Ok(MainOutputs {
            world_state: world_state.into(),
        })
    }
}
