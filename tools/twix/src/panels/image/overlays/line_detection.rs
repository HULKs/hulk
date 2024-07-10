use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Pixel;
use geometry::line::Line2;
use linear_algebra::Point2;
use types::line_data::LineDiscardReason;

use crate::{
    panels::image::{cycler_selector::VisionCycler, overlay::Overlay},
    twix_painter::TwixPainter,
    value_buffer::BufferHandle,
};

pub struct LineDetection {
    lines_in_image: BufferHandle<Option<Vec<Line2<Pixel>>>>,
    discarded_lines: BufferHandle<Option<Vec<(Line2<Pixel>, LineDiscardReason)>>>,
    ransac_input: BufferHandle<Option<Vec<Point2<Pixel>>>>,
}

impl Overlay for LineDetection {
    const NAME: &'static str = "Line Detection";

    fn new(nao: std::sync::Arc<crate::nao::Nao>, selected_cycler: VisionCycler) -> Self {
        let cycler_path = selected_cycler.as_path();
        Self {
            lines_in_image: nao
                .subscribe_value(format!("{cycler_path}.additional_outputs.lines_in_image")),
            discarded_lines: nao
                .subscribe_value(format!("{cycler_path}.additional_outputs.discarded_lines")),
            ransac_input: nao
                .subscribe_value(format!("{cycler_path}.additional_outputs.ransac_input")),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let Some(lines_in_image) = self.lines_in_image.get_last_value()?.flatten() else {
            return Ok(());
        };
        let Some(discarded_lines) = self.discarded_lines.get_last_value()?.flatten() else {
            return Ok(());
        };
        let Some(ransac_input) = self.ransac_input.get_last_value()?.flatten() else {
            return Ok(());
        };
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
            painter.line_segment(line.0, line.1, Stroke::new(3.0, color));
        }
        for line in lines_in_image {
            painter.line_segment(line.0, line.1, Stroke::new(3.0, Color32::BLUE));
        }
        Ok(())
    }
}
