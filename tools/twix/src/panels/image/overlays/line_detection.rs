use std::str::FromStr;

use communication::{Cycler, CyclerOutput};
use eframe::epaint::{Color32, Stroke};
use nalgebra::{point, Point2};
use types::ImageLines;

use crate::{panels::image::overlay::Overlay, value_buffer::ValueBuffer};

pub struct LineDetection {
    lines_in_image: ValueBuffer,
}

impl Overlay for LineDetection {
    const NAME: &'static str = "Line Detection";

    fn new(nao: std::sync::Arc<crate::nao::Nao>, selected_cycler: Cycler) -> Self {
        Self {
            lines_in_image: nao.subscribe_output(
                CyclerOutput::from_str(&format!("{}.additional.lines_in_image", selected_cycler))
                    .unwrap(),
            ),
        }
    }

    fn paint(&self, painter: &crate::twix_painter::TwixPainter) -> anyhow::Result<()> {
        let lines_in_image: ImageLines = self.lines_in_image.require_latest()?;
        for point in lines_in_image.points {
            painter.circle_stroke(to_444(point), 3.0, Stroke::new(1.0, Color32::RED))
        }
        for line in lines_in_image.lines {
            painter.line_segment(
                to_444(line.0),
                to_444(line.1),
                Stroke::new(3.0, Color32::BLUE),
            );
        }
        Ok(())
    }
}

fn to_444(point: Point2<f32>) -> Point2<f32> {
    point![point.x * 2.0, point.y]
}
