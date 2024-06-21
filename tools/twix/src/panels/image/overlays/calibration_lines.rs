use std::str::FromStr;

use crate::{
    panels::image::overlay::Overlay, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};
use color_eyre::eyre::Result;
use communication::client::{Cycler, CyclerOutput};
use coordinate_systems::{Ground, Pixel};
use eframe::epaint::{Color32, Stroke};
use geometry::circle::Circle;
use linear_algebra::Point2;

pub struct CalibrationLineDetection {
    circles_and_used_points: ValueBuffer,
}

impl Overlay for CalibrationLineDetection {
    const NAME: &'static str = "Calibration Line Detection";

    fn new(nao: std::sync::Arc<crate::nao::Nao>, selected_cycler: Cycler) -> Self {
        Self {

            circles_and_used_points: nao.subscribe_output(
                CyclerOutput::from_str(&format!(
                    "{selected_cycler}.additional.calibration_line_detection.circles_and_used_points"
                ))
                .unwrap(),
            ),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let circles_and_used_points: Vec<(Circle<Ground>, Point2<Pixel>, Vec<Point2<Pixel>>)> =
            self.circles_and_used_points.require_latest()?;

        for (_circle, circle_center_px, circle_points) in circles_and_used_points {
            painter.circle_stroke(circle_center_px, 2.0, Stroke::new(1.0, Color32::GOLD));
            painter.circle_stroke(circle_center_px, 1.0, Stroke::new(1.0, Color32::RED));
            for circle_point in circle_points {
                painter.circle_stroke(circle_point, 2.0, Stroke::new(1.0, Color32::YELLOW));
            }
        }

        Ok(())
    }
}
