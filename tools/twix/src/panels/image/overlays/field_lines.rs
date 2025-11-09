use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Pixel;
use eframe::epaint::{Color32, Stroke};
use types::field_lines::ProjectedFieldLines;

use crate::{
    panels::image::overlay::Overlay, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct FieldLines {
    penalty_boxes: BufferHandle<Option<Option<ProjectedFieldLines>>>,
}

impl Overlay for FieldLines {
    const NAME: &'static str = "Field Lines";

    fn new(nao: Arc<crate::nao::Nao>) -> Self {
        Self {
            penalty_boxes: nao.subscribe_value("Control.additional_outputs.projected_field_lines"),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let Some(penalty_boxes_lines_in_image) =
            self.penalty_boxes.get_last_value()?.flatten().flatten()
        else {
            return Ok(());
        };
        for line in penalty_boxes_lines_in_image.field_lines {
            painter.line_segment(line.0, line.1, Stroke::new(3.0, Color32::GRAY));
        }
        Ok(())
    }
}
