use std::sync::Arc;

use types::FieldDimensions;

use crate::{nao::Nao, panels::Layer, twix_paint::TwixPainter};

pub struct Field {}

impl Layer for Field {
    const NAME: &'static str = "Field";

    fn new(_nao: Arc<Nao>) -> Self {
        Self {}
    }

    fn paint(&self, painter: &TwixPainter, field_dimensions: &FieldDimensions) {
        painter.field(field_dimensions);
    }
}
