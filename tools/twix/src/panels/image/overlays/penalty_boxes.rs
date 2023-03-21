use std::str::FromStr;

use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput};
use eframe::epaint::{Color32, Stroke};
use types::Line2;

use crate::{
    panels::image::overlay::Overlay, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct PenaltyBoxes {
    penalty_boxes: ValueBuffer,
}

impl Overlay for PenaltyBoxes {
    const NAME: &'static str = "Penalty Boxes";

    fn new(nao: std::sync::Arc<crate::nao::Nao>, selected_cycler: Cycler) -> Self {
        let top_or_bottom = match selected_cycler {
            Cycler::VisionTop => "top",
            _ => "bottom",
        };
        Self {
            penalty_boxes: nao.subscribe_output(
                CyclerOutput::from_str(&format!(
                    "Control.additional_outputs.projected_field_lines.{top_or_bottom}"
                ))
                .unwrap(),
            ),
        }
    }

    fn paint(&self, painter: &TwixPainter) -> Result<()> {
        let penalty_boxes_lines_in_image: Vec<Line2> = self.penalty_boxes.require_latest()?;
        for line in penalty_boxes_lines_in_image {
            painter.line_segment(line.0, line.1, Stroke::new(3.0, Color32::BLACK));
        }
        Ok(())
    }
}
