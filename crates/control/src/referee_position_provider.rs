use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::Ground;
use framework::MainOutput;
use linear_algebra::{point, Point2};
use serde::{Deserialize, Serialize};
use spl_network_messages::PlayerNumber;
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
    player_number: Parameter<PlayerNumber, "player_number">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub expected_referee_position: MainOutput<Option<Point2<Ground>>>,
}

impl RefereePositionProvider {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let expected_referee_position: Option<Point2<Ground>> = match (
            context.player_number,
            context.filtered_game_controller_state.global_field_side,
        ) {
            (PlayerNumber::Four, GlobalFieldSide::Home) => {
                Some(point![2.0, context.field_dimensions.width,])
            }
            (PlayerNumber::Seven, GlobalFieldSide::Home) => {
                Some(point![1.0, context.field_dimensions.width,])
            }
            (PlayerNumber::Two, GlobalFieldSide::Away) => {
                Some(point![-2.0, context.field_dimensions.width,])
            }
            (PlayerNumber::Six, GlobalFieldSide::Away) => {
                Some(point![-1.0, context.field_dimensions.width,])
            }
            _ => None,
        };

        Ok(MainOutputs {
            expected_referee_position: expected_referee_position.into(),
        })
    }
}
