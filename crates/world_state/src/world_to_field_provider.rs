use std::f32::consts::PI;

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, World};
use framework::MainOutput;
use linear_algebra::Isometry2;
use types::{field_dimensions::GlobalFieldSide, game_controller_state::GameControllerState};

#[derive(Deserialize, Serialize)]
pub struct WorldToFieldProvider {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    game_controller_state: Input<Option<GameControllerState>, "game_controller_state?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub world_to_field: MainOutput<Option<Isometry2<World, Field>>>,
}

impl WorldToFieldProvider {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let game_controller_state = context.game_controller_state.cloned();
        let world_to_field = game_controller_state.map(|game_controller_state| {
            let global_field_side = game_controller_state.global_field_side;
            match global_field_side {
                GlobalFieldSide::Home => Isometry2::identity(),
                GlobalFieldSide::Away => Isometry2::rotation(PI),
            }
        });
        Ok(MainOutputs {
            world_to_field: world_to_field.into(),
        })
    }
}
