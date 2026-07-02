use color_eyre::Report;
use eframe::egui::{Color32, Stroke};
use linear_algebra::point;
use projection::camera_matrix::CameraMatrix;
use types::time_wrapper::TimeWrapper;

use crate::repaint::ObservationContext;

use super::super::image_overlay::{ImageOverlay, ImageOverlayPainter, OverlayObservation};

pub(in crate::panels::image) struct HorizonOverlay {
    camera_matrix: OverlayObservation<TimeWrapper<CameraMatrix>>,
}

impl ImageOverlay for HorizonOverlay {
    const NAME: &'static str = "Horizon";
    const STORAGE_KEY: &'static str = "horizon";

    fn new<C>(context: &C) -> Result<Self, Report>
    where
        C: ObservationContext,
    {
        Ok(Self {
            camera_matrix: OverlayObservation::new(context, "camera_matrix")?,
        })
    }

    fn paint(&self, painter: &ImageOverlayPainter) {
        let Some(camera_matrix) = self.camera_matrix.latest() else {
            return;
        };
        let Some(horizon) = camera_matrix.value.inner.horizon else {
            return;
        };

        let left_horizon_height = horizon.y_at_x(0.0);
        let image_width = painter.image_width();
        let right_horizon_height = horizon.y_at_x(image_width);

        painter.line_segment(
            point![0.0, left_horizon_height],
            point![image_width, right_horizon_height],
            Stroke::new(3.0, Color32::GREEN),
        );
        painter.circle_stroke(
            horizon.vanishing_point,
            5.0,
            Stroke::new(3.0, Color32::GREEN),
        );
    }
}
