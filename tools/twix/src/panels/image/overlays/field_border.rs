use std::sync::Arc;

use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput, Output};
use coordinate_systems::Pixel;
use eframe::epaint::{Color32, Stroke};
use geometry::line::Line2;
use linear_algebra::Point2;

use crate::{
    panels::image::overlay::Overlay, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct FieldBorder {
    border_lines: ValueBuffer,
    candidates: ValueBuffer,
}

impl Overlay for FieldBorder {
    const NAME: &'static str = "Field Border";

    fn new(nao: Arc<crate::nao::Nao>, selected_cycler: Cycler) -> Self {
        Self {
            border_lines: nao.subscribe_output(CyclerOutput {
                cycler: selected_cycler,
                output: Output::Main {
                    path: "field_border.border_lines".into(),
                },
            }),
            candidates: nao.subscribe_output(CyclerOutput {
                cycler: selected_cycler,
                output: Output::Additional {
                    path: "field_border_points".to_string(),
                },
            }),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let border_lines_in_image: Vec<Line2<Pixel>> = self.border_lines.require_latest()?;
        for line in border_lines_in_image {
            painter.line_segment(
                line.0,
                line.1,
                Stroke::new(3.0, Color32::from_rgb(255, 0, 240)),
            );
        }

        let candidates: Vec<Point2<Pixel>> = self.candidates.require_latest()?;
        for point in candidates {
            painter.circle_filled(point, 2.0, Color32::BLUE);
        }

        Ok(())
    }
}
