use color_eyre::Report;
use coordinate_systems::Pixel;
use eframe::egui::{Color32, Stroke};
use geometry::circle::Circle;
use ros_z::time::Time;
use serde_json::{Map, Value};
use twix_visualization::twix_painter::TwixPainter;

use crate::repaint::ObservationContext;

use super::super::image_overlay::{ImageOverlay, OverlayObservation};

pub(in crate::panels::image) struct BallDetectionOverlay {
    filtered_balls: OverlayObservation<Vec<Circle<Pixel>>>,
}

impl ImageOverlay for BallDetectionOverlay {
    const NAME: &'static str = "Ball Detection";
    const STORAGE_KEY: &'static str = "ball_detection";

    fn new<C>(context: &C, _settings: &Map<String, Value>) -> Result<Self, Report>
    where
        C: ObservationContext,
    {
        Ok(Self {
            filtered_balls: OverlayObservation::new(
                context,
                "ball_filter/filtered_balls_in_image",
            )?,
        })
    }

    fn paint(&self, painter: &TwixPainter<Pixel>, _image_time: Time) {
        let Some(filtered_balls) = self.filtered_balls.latest() else {
            return;
        };
        for circle in &filtered_balls.value {
            painter.circle_stroke(circle.center, circle.radius, Stroke::new(3.0, Color32::RED));
        }
    }
}
