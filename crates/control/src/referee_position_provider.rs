use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::Field;
use framework::MainOutput;
use linear_algebra::{point, Point2};
use serde::{Deserialize, Serialize};
use types::{
    field_dimensions::{FieldDimensions, GlobalFieldSide},
    filtered_game_controller_state::FilteredGameControllerState,
};

#[derive(Deserialize, Serialize)]
pub struct RefereePositionProvider {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    filtered_game_controller_state:
        RequiredInput<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub expected_referee_position: MainOutput<Option<Point2<Field>>>,
}

impl RefereePositionProvider {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let expected_referee_position =
            if context.filtered_game_controller_state.global_field_side == GlobalFieldSide::Home {
                point![0.0, context.field_dimensions.width / 2.0,]
            } else {
                point![0.0, -context.field_dimensions.width / 2.0,]
            };

        Ok(MainOutputs {
            expected_referee_position: Some(expected_referee_position).into(),
        })
    }
}
