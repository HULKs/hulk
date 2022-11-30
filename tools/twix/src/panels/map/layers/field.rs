use std::sync::Arc;

use color_eyre::Result;
use types::FieldDimensions;

use crate::{nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter};

pub struct Field {}

impl Layer for Field {
    const NAME: &'static str = "Field";

    fn new(_nao: Arc<Nao>) -> Self {
        Self {}
    }

    fn paint(&self, painter: &TwixPainter, field_dimensions: &FieldDimensions) -> Result<()> {
        painter.field(field_dimensions);
        Ok(())
    }
}
