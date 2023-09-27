use std::str::FromStr;

use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput};
use eframe::epaint::{Color32, Stroke};
use types::line_data::ImageLines;
use types::line_data::LineDiscardReason;

use crate::{
    panels::image::overlay::Overlay, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct LineDetection {
    lines_in_image: ValueBuffer,
}

impl Overlay for LineDetection {
    const NAME: &'static str = "Line Detection";

    fn new(nao: std::sync::Arc<crate::nao::Nao>, selected_cycler: Cycler) -> Self {
        Self {
            lines_in_image: nao.subscribe_output(
                CyclerOutput::from_str(&format!("{selected_cycler}.additional.lines_in_image"))
                    .unwrap(),
            ),
        }
    }

    fn paint(&self, painter: &TwixPainter) -> Result<()> {
        let lines_in_image: ImageLines = self.lines_in_image.require_latest()?;
        for point in lines_in_image.points {
            painter.circle_stroke(point, 3.0, Stroke::new(1.0, Color32::RED))
        }
        for (line, reason) in lines_in_image.discarded_lines {
            let color = match reason {
                LineDiscardReason::TooFewPoints => Color32::YELLOW,
                LineDiscardReason::LineTooShort => Color32::GRAY,
                LineDiscardReason::LineTooLong => Color32::BROWN,
                LineDiscardReason::TooFarAway => Color32::BLACK,
            };
            painter.line_segment(line.0, line.1, Stroke::new(3.0, color));
        }
        for line in lines_in_image.lines {
            painter.line_segment(line.0, line.1, Stroke::new(3.0, Color32::BLUE));
        }
        Ok(())
    }
}
