use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::MainOutput;
use linear_algebra::{point, Isometry2, Point2};
use serde::{Deserialize, Serialize};
use types::field_dimensions::FieldDimensions;

#[derive(Deserialize, Serialize)]
pub struct RefereePositionProvider {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "ground_to_field?">,

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
        if let Some(ground_to_field) = context.ground_to_field {
            let expected_referee_position: Point2<Field> = point![
                context.normed_expected_referee_position.x() * context.field_dimensions.length,
                context.normed_expected_referee_position.y() * context.field_dimensions.width,
            ];

            Ok(MainOutputs {
                expected_referee_position: (ground_to_field.inverse() * expected_referee_position)
                    .into(),
            })
        } else {
            Ok(MainOutputs {
                expected_referee_position: point![0.0, 0.0].into(),
            })
        }
    }
}
