use color_eyre::Report;
use coordinate_systems::Pixel;
use eframe::egui::{Color32, Pos2, Stroke};
use linear_algebra::point;
use projection::camera_matrix::CameraMatrix;
use ros_z::time::Time;
use types::time_wrapper::TimeWrapper;

use crate::repaint::ObservationContext;
use twix_visualization::twix_painter::TwixPainter;

use super::super::image_overlay::{ImageOverlay, OverlayObservation};

pub(in crate::panels::image) struct HorizonOverlay {
    camera_matrix: OverlayObservation<TimeWrapper<CameraMatrix>>,
}

impl ImageOverlay for HorizonOverlay {
    type Sample = super::super::image_overlay::CameraSample;
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

    fn prepare(&self, image_time: Time) -> Option<Self::Sample> {
        self.camera_matrix.camera_at(image_time)
    }

    fn paint(painter: &TwixPainter<Pixel>, camera_matrix: &Self::Sample) {
        let Some(horizon) = camera_matrix.horizon else {
            return;
        };

        let rect = painter.pixel_rect();
        let left = painter.transform_pixel_to_world(Pos2::new(rect.left(), 0.0));
        let right = painter.transform_pixel_to_world(Pos2::new(rect.right(), 0.0));
        painter.line_segment(
            point![left.x(), horizon.y_at_x(left.x())],
            point![right.x(), horizon.y_at_x(right.x())],
            Stroke::new(3.0, Color32::GREEN),
        );
        painter.circle_stroke(
            horizon.vanishing_point,
            5.0,
            Stroke::new(3.0, Color32::GREEN),
        );
    }
}
