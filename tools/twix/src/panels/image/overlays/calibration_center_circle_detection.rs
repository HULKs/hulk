use calibration::center_circle::circle_points::CenterCirclePoints;
use color_eyre::eyre::Result;
use eframe::{
    egui::{Align2, FontId},
    epaint::{Color32, Stroke},
};

use coordinate_systems::Pixel;
use linear_algebra::Point2;

use crate::{
    nao::Nao,
    panels::image::{cycler_selector::VisionCycler, overlay::Overlay},
    twix_painter::TwixPainter,
    value_buffer::BufferHandle,
};

pub struct CalibrationMeasurementDetection {
    center_circle_points: BufferHandle<Option<CenterCirclePoints<Pixel>>>,
    edge_points: BufferHandle<Vec<Point2<Pixel>>>,
}

impl Overlay for CalibrationMeasurementDetection {
    const NAME: &'static str = "Calibration Measurements";

    fn new(nao: std::sync::Arc<Nao>, selected_cycler: VisionCycler) -> Self {
        let cycler_path = selected_cycler.as_path();
        Self {
            center_circle_points: nao.subscribe_value(format!(
                "{cycler_path}.main_outputs.calibration_center_circle"
            )),
            edge_points: nao.subscribe_value(format!(
                "{cycler_path}.additional.calibration_center_circle_detection.detected_edge_points"
            )),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        if let Some(edge_points) = self.edge_points.get_last_value().ok().flatten() {
            for edge_point in edge_points {
                painter.circle_stroke(edge_point, 1.0, Stroke::new(1.0, Color32::BLUE));
            }
        }

        if let Some(center_circle_detections) =
            self.center_circle_points.get_last_value().ok().flatten()
        {
            for (index, center_circle) in center_circle_detections.iter().enumerate() {
                painter.floating_text(
                    center_circle.center,
                    Align2::LEFT_BOTTOM,
                    format!("{:.2}", index),
                    FontId::default(),
                    Color32::BLUE,
                );
                painter.circle_stroke(center_circle.center, 2.0, Stroke::new(1.0, Color32::GOLD));
                painter.circle_stroke(
                    center_circle.center,
                    1.0,
                    Stroke::new(1.0, Color32::DARK_BLUE),
                );
                for circle_point in &center_circle.points {
                    painter.circle_stroke(*circle_point, 2.0, Stroke::new(1.0, Color32::RED));
                }
            }
        }

        Ok(())
    }
}
