use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Pixel;
use eframe::epaint::{Color32, Stroke};
use geometry::line::Line2;

use crate::{
    panels::image::{cycler_selector::VisionCycler, overlay::Overlay},
    twix_painter::TwixPainter,
    value_buffer::BufferHandle,
};

pub struct PenaltyBoxes {
    penalty_boxes: BufferHandle<Vec<Line2<Pixel>>>,
}

impl Overlay for PenaltyBoxes {
    const NAME: &'static str = "Penalty Boxes";

    fn new(nao: Arc<crate::nao::Nao>, selected_cycler: VisionCycler) -> Self {
        let camera_position = match selected_cycler {
            VisionCycler::Top => "top",
            VisionCycler::Bottom => "bottom",
        };
        Self {
            penalty_boxes: nao.subscribe_value(format!(
                "Control.additional_outputs.projected_field_lines.{camera_position}"
            )),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let Some(penalty_boxes_lines_in_image) = self.penalty_boxes.get_last()? else {
            return Ok(());
        };
        for line in penalty_boxes_lines_in_image.value {
            painter.line_segment(line.0, line.1, Stroke::new(3.0, Color32::BLACK));
        }
        Ok(())
    }
}
