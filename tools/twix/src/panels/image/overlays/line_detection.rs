use std::str::FromStr;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use communication::client::{Cycler, CyclerOutput};
use coordinate_systems::Pixel;
use geometry::line::Line2;
use linear_algebra::Point2;
use types::line_data::LineDiscardReason;

use crate::{
    panels::image::overlay::Overlay, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct LineDetection {
    lines_in_image: ValueBuffer,
    discarded_lines: ValueBuffer,
    ransac_input: ValueBuffer,
}

impl Overlay for LineDetection {
    const NAME: &'static str = "Line Detection";

    fn new(nao: std::sync::Arc<crate::nao::Nao>, selected_cycler: Cycler) -> Self {
        Self {
            lines_in_image: nao.subscribe_output(
                CyclerOutput::from_str(&format!("{selected_cycler}.additional.lines_in_image"))
                    .unwrap(),
            ),
            discarded_lines: nao.subscribe_output(
                CyclerOutput::from_str(&format!("{selected_cycler}.additional.discarded_lines"))
                    .unwrap(),
            ),
            ransac_input: nao.subscribe_output(
                CyclerOutput::from_str(&format!("{selected_cycler}.additional.ransac_input"))
                    .unwrap(),
            ),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let lines_in_image: Vec<Line2<Pixel>> = self.lines_in_image.require_latest()?;
        let discarded_lines: Vec<(Line2<Pixel>, LineDiscardReason)> =
            self.discarded_lines.require_latest()?;
        let ransac_input: Vec<Point2<Pixel>> = self.ransac_input.require_latest()?;
        for point in ransac_input {
            painter.circle_stroke(point, 3.0, Stroke::new(1.0, Color32::RED))
        }
        for (line, reason) in discarded_lines {
            let color = match reason {
                LineDiscardReason::TooFewPoints => Color32::YELLOW,
                LineDiscardReason::LineTooShort => Color32::GRAY,
                LineDiscardReason::LineTooLong => Color32::BROWN,
                LineDiscardReason::TooFarAway => Color32::BLACK,
            };
            painter.line_segment(line.first, line.second, Stroke::new(3.0, color));
        }
        for line in lines_in_image {
            painter.line_segment(line.first, line.second, Stroke::new(3.0, Color32::BLUE));
        }
        Ok(())
    }
}
