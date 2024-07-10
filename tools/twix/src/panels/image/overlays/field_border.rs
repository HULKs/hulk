use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Pixel;
use eframe::epaint::{Color32, Stroke};
use geometry::line::Line2;
use linear_algebra::Point2;

use crate::{
    panels::image::{cycler_selector::VisionCycler, overlay::Overlay},
    twix_painter::TwixPainter,
    value_buffer::BufferHandle,
};

pub struct FieldBorder {
    border_lines: BufferHandle<Option<Vec<Line2<Pixel>>>>,
    candidates: BufferHandle<Option<Vec<Point2<Pixel>>>>,
}

impl Overlay for FieldBorder {
    const NAME: &'static str = "Field Border";

    fn new(nao: Arc<crate::nao::Nao>, selected_cycler: VisionCycler) -> Self {
        let cycler_path = selected_cycler.as_path();
        Self {
            border_lines: nao.subscribe_value(format!("{cycler_path}.main_outputs.field_border")),
            candidates: nao.subscribe_value(format!(
                "{cycler_path}.additional_outputs.field_border_points"
            )),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let Some(border_lines_in_image) = self.border_lines.get_last_value()?.flatten() else {
            return Ok(());
        };
        for line in border_lines_in_image {
            painter.line_segment(
                line.0,
                line.1,
                Stroke::new(3.0, Color32::from_rgb(255, 0, 240)),
            );
        }

        let Some(candidates) = self.candidates.get_last_value()?.flatten() else {
            return Ok(());
        };
        for point in candidates {
            painter.circle_filled(point, 2.0, Color32::BLUE);
        }

        Ok(())
    }
}
