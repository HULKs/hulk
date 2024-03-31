use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::Ground;
use framework::MainOutput;
use linear_algebra::{point, Point2};
use serde::{Deserialize, Serialize};
use types::field_dimensions::FieldDimensions;

#[derive(Deserialize, Serialize)]
pub struct RefereePositionProvider {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
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
        let field_dimensions = context.field_dimensions;
        let normed_expected_referee_position = context.normed_expected_referee_position;

        let expected_referee_position = point![
            normed_expected_referee_position.x() * field_dimensions.length,
            normed_expected_referee_position.y() * field_dimensions.width,
        ];

        Ok(MainOutputs {
            expected_referee_position: expected_referee_position.into(),
        })
    }
}
