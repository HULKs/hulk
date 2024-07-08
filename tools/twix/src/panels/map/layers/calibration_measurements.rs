use std::str::FromStr;

use color_eyre::Result;
use communication::client::CyclerOutput;
use coordinate_systems::Ground;
use eframe::epaint::{Color32, Stroke};
use geometry::circle::Circle;
use linear_algebra::Point2;
use types::field_dimensions::FieldDimensions;

use crate::{panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer};

pub struct CalibrationMeasurementDetection {
    circle_used_points_robot: ValueBuffer,
    circle_robot: ValueBuffer,
}

impl Layer<Ground> for CalibrationMeasurementDetection {
    const NAME: &'static str = "Calibration Measurement Detection";

    fn new(nao: std::sync::Arc<crate::nao::Nao>) -> Self {
        Self {
            circle_used_points_robot: nao.subscribe_output(
                CyclerOutput::from_str(&format!(
                    "VisionTop.additional.calibration_center_circle_detection.circle_used_points_robot"
                ))
                .unwrap(),
            ),
            circle_robot: nao.subscribe_output(
                CyclerOutput::from_str(&format!(
                    "VisionTop.additional.calibration_center_circle_detection.circle_robot"
                ))
                .unwrap(),
            ),
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let used_points: Vec<Point2<Ground>> = self.circle_used_points_robot.require_latest()?;
        let circle_robot: Option<Circle<Ground>> = self.circle_robot.require_latest()?;

        if let Some(circle_robot) = circle_robot {
            painter.circle_stroke(
                circle_robot.center,
                circle_robot.radius,
                Stroke {
                    width: 3.0,
                    color: Color32::LIGHT_RED,
                },
            );
        }

        for circle_point in used_points {
            painter.circle_stroke(circle_point, 2.0, Stroke::new(1.0, Color32::YELLOW));
        }

        Ok(())
    }
}
