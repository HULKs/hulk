use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Field;
use geometry::line::Line2;
use types::field_dimensions::FieldDimensions;

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct LineCorrespondences {
    correspondence_lines: BufferHandle<Option<Vec<Line2<Field>>>>,
    lines_in_field: BufferHandle<Option<Vec<Line2<Field>>>>,
}

impl Layer<Field> for LineCorrespondences {
    const NAME: &'static str = "Line Correspondences";

    fn new(nao: Arc<Nao>) -> Self {
        let correspondence_lines =
            nao.subscribe_value("Control.additional_outputs.localization.correspondence_lines");
        let lines_in_field =
            nao.subscribe_value("Control.additional_outputs.localization.measured_lines_in_field");
        Self {
            correspondence_lines,
            lines_in_field,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        if let Some(lines) = self.correspondence_lines.get_last_value()?.flatten() {
            for line in lines {
                painter.line_segment(line.0, line.1, Stroke::new(0.02, Color32::YELLOW));
            }
        }

        if let Some(lines) = self.lines_in_field.get_last_value()?.flatten() {
            for line in lines {
                painter.line_segment(line.0, line.1, Stroke::new(0.04, Color32::RED));
            }
        }
        Ok(())
    }
}
