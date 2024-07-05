use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Pixel;
use eframe::epaint::{Color32, Stroke};
use types::field_lines::ProjectedFieldLines;

use crate::{
    panels::image::{cycler_selector::VisionCycler, overlay::Overlay},
    twix_painter::TwixPainter,
    value_buffer::BufferHandle,
};

pub struct PenaltyBoxes {
    penalty_boxes: BufferHandle<Option<ProjectedFieldLines>>,
    cycler: VisionCycler,
}

impl Overlay for PenaltyBoxes {
    const NAME: &'static str = "Penalty Boxes";

    fn new(nao: Arc<crate::nao::Nao>, selected_cycler: VisionCycler) -> Self {
        Self {
            penalty_boxes: nao.subscribe_value("Control.additional_outputs.projected_field_lines"),
            cycler: selected_cycler,
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let Some(penalty_boxes_lines_in_image) = self.penalty_boxes.get_last_value()?.flatten()
        else {
            return Ok(());
        };
        let lines = match self.cycler {
            VisionCycler::Top => &penalty_boxes_lines_in_image.top,
            VisionCycler::Bottom => &penalty_boxes_lines_in_image.bottom,
        };
        for line in lines {
            painter.line_segment(line.0, line.1, Stroke::new(3.0, Color32::BLACK));
        }
        Ok(())
    }
}
