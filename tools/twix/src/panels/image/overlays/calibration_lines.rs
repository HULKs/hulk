use std::str::FromStr;

use color_eyre::eyre::Result;
use eframe::{
    egui::{Align2, FontId},
    epaint::{Color32, Stroke},
};

use communication::client::{Cycler, CyclerOutput};
use coordinate_systems::Pixel;
use linear_algebra::Point2;

use crate::{
    panels::image::overlay::Overlay, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct CalibrationMeasurementDetection {
    circles_points_pixel: ValueBuffer,
    edge_points_pixel: ValueBuffer,
}

impl Overlay for CalibrationMeasurementDetection {
    const NAME: &'static str = "Calibration Measurements";

    fn new(nao: std::sync::Arc<crate::nao::Nao>, selected_cycler: Cycler) -> Self {
        Self {
            circles_points_pixel: nao.subscribe_output(
                CyclerOutput::from_str(&format!(
                    "{selected_cycler}.additional.calibration_circle_detection.circles_points_pixel"
                ))
                .unwrap(),
            ),
            edge_points_pixel: nao.subscribe_output(
                CyclerOutput::from_str(&format!(
                    "{selected_cycler}.additional.calibration_circle_detection.detected_edge_points"
                ))
                .unwrap(),
            ),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let edge_points_pixel: Vec<Point2<Pixel>> =
            self.edge_points_pixel.require_latest().unwrap_or_default();

        for edge_point in edge_points_pixel {
            painter.circle_stroke(edge_point, 1.0, Stroke::new(1.0, Color32::BLUE));
        }

        let circles_points_pixel: Vec<(Point2<Pixel>, Vec<Point2<Pixel>>)> = self
            .circles_points_pixel
            .require_latest()
            .unwrap_or_default();

        for (index, (circle_center_px, circle_points)) in circles_points_pixel.iter().enumerate() {
            painter.floating_text(
                *circle_center_px,
                Align2::LEFT_BOTTOM,
                format!("{:.2}", index),
                FontId::default(),
                Color32::BLUE,
            );
            painter.circle_stroke(*circle_center_px, 2.0, Stroke::new(1.0, Color32::GOLD));
            painter.circle_stroke(*circle_center_px, 1.0, Stroke::new(1.0, Color32::DARK_BLUE));
            for circle_point in circle_points {
                painter.circle_stroke(*circle_point, 2.0, Stroke::new(1.0, Color32::RED));
            }
        }

        Ok(())
    }
}
