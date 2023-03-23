use std::sync::Arc;

use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput, Output};
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

    fn new(nao: Arc<crate::nao::Nao>, selected_cycler: Cycler) -> Self {
        let top_or_bottom = match selected_cycler {
            Cycler::VisionTop => "top",
            Cycler::VisionBottom => "bottom",
            cycler => panic!("Invalid vision cycler: {cycler}"),
        };
        Self {
            penalty_boxes: nao.subscribe_output(CyclerOutput {
                cycler: Cycler::Control,
                output: Output::Additional {
                    path: format!("projected_field_lines.{top_or_bottom}"),
                },
            }),
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
