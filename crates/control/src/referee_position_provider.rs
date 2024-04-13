use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::MainOutput;
use linear_algebra::{point, Isometry2, Point2};
use serde::{Deserialize, Serialize};
use types::{field_dimensions::FieldDimensions, world_state::WorldState};

#[derive(Deserialize, Serialize)]
pub struct RefereePositionProvider {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    world_state: Input<WorldState, "world_state">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    normed_expected_referee_position:
        Parameter<Point2<Ground>, "detection.detection_top.normed_expected_referee_position">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub expected_referee_position: MainOutput<Point2<Ground>>,
}

impl RefereePositionProvider {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let Some(ground_to_field) = context.ground_to_field else {
            return Ok(MainOutputs {
                expected_referee_position: point![0.0, 0.0].into(),
            });
        };
        let mut normed_expected_referee_position = *context.normed_expected_referee_position;
        if let Some(filtered_game_controller_state) =
            context.world_state.filtered_game_controller_state
        {
            if !filtered_game_controller_state.own_team_is_home_after_coin_toss {
                normed_expected_referee_position = point![
                    normed_expected_referee_position.x(),
                    normed_expected_referee_position.y() * -1.0
                ];
            }
        }

        let expected_referee_position = point![
            normed_expected_referee_position.x() * context.field_dimensions.length / 2.0,
            normed_expected_referee_position.y() * context.field_dimensions.width / 2.0,
        ];

        Ok(MainOutputs {
            expected_referee_position: (ground_to_field.inverse() * expected_referee_position)
                .into(),
        })
    }
}
