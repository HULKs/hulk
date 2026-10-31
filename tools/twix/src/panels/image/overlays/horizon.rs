use std::time::Duration;

use color_eyre::Report;
use coordinate_systems::Pixel;
use eframe::egui::{Color32, Pos2, Stroke};
use linear_algebra::point;
use projection::camera_matrix::CameraMatrix;
use ros_z::time::Time;
use ros_z_debug::query::plan::Selection;
use twix_visualization::twix_painter::TwixPainter;
use types::time_wrapper::TimeWrapper;

use super::{
    super::{
        image_overlay::ImageOverlay,
        inputs::{OverlayInput, OverlayInputs},
    },
    camera::{CameraSample, projected_camera_at},
};

#[derive(Default)]
pub(in crate::panels::image) struct HorizonOverlay {
    sample: Option<CameraSample>,
}

impl ImageOverlay for HorizonOverlay {
    fn inputs(&self) -> Vec<OverlayInput> {
        vec![OverlayInput::new::<TimeWrapper<CameraMatrix>>(
            "camera_matrix",
            Selection::Bracket {
                max_gap: Duration::from_millis(100),
            },
        )]
    }

    fn prepare(&mut self, time: Time, inputs: &mut OverlayInputs) -> Result<(), Report> {
        self.sample = None;
        self.sample = inputs
            .decode(0, time)?
            .and_then(|records| projected_camera_at(&records, time));
        Ok(())
    }

    fn image_overlay(&self, painter: &TwixPainter<Pixel>) {
        let Some(camera_matrix) = &self.sample else {
            return;
        };
        let Some(horizon) = camera_matrix.horizon else {
            return;
        };

        let painter_rect = painter.pixel_rect();
        let left = painter.transform_pixel_to_world(Pos2::new(painter_rect.left(), 0.0));
        let left_horizon_height = horizon.y_at_x(left.x());
        let right = painter.transform_pixel_to_world(Pos2::new(painter_rect.right(), 0.0));
        let right_horizon_height = horizon.y_at_x(right.x());

        painter.line_segment(
            point![left.x(), left_horizon_height],
            point![right.x(), right_horizon_height],
            Stroke::new(3.0, Color32::GREEN),
        );
        painter.circle_stroke(
            horizon.vanishing_point,
            5.0,
            Stroke::new(3.0, Color32::GREEN),
        );
    }
}
