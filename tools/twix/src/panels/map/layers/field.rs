use std::sync::Arc;

use color_eyre::Result;
use types::field_dimensions::FieldDimensions;

use crate::{panels::map::layer::Layer, robot::Robot, twix_painter::TwixPainter};

pub struct Field {}

impl Layer<coordinate_systems::Field> for Field {
    const NAME: &'static str = "Field";

    fn new(_robot: Arc<Robot>) -> Self {
        Self {}
    }

    fn paint(
        &self,
        painter: &TwixPainter<coordinate_systems::Field>,
        field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        painter.field(field_dimensions);
        Ok(())
    }
}
