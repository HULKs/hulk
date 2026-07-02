use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::{Ground, Pixel};
use geometry::line_segment::LineSegment;
use projection::{Projection, camera_matrix::CameraMatrix};
use ros_z_debug::TopicObservation;
use types::field_dimensions::FieldDimensions;

use crate::{backend::RobotBackend, panels::map::layer::Layer, twix_painter::TwixPainter};

pub struct Lines {
    lines_in_image: TopicObservation<Option<Vec<LineSegment<Pixel>>>>,
    camera_matrix: TopicObservation<Option<CameraMatrix>>,
}

impl Layer<Ground> for Lines {
    const NAME: &'static str = "Lines";

    fn new(backend: Arc<RobotBackend>) -> Self {
        let _runtime_handle = backend.runtime_handle().enter();

        let lines_in_image = backend
            .observer()
            .observe_typed("Vision.additional_outputs.lines_in_image")
            .expect("failed to create lines_in_image observation")
            .spawn();
        let camera_matrix = backend
            .observer()
            .observe_typed("WorldState.main_outputs.camera_matrix")
            .expect("failed to create camera_matrix observation")
            .spawn();

        Self {
            lines_in_image,
            camera_matrix,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let Some(lines_in_image) = self
            .lines_in_image
            .latest()
            .map(|sample| sample.value.clone())
        else {
            return Ok(());
        };
        let Some(camera_matrix) = self
            .camera_matrix
            .latest()
            .map(|sample| sample.value.clone())
        else {
            return Ok(());
        };
        paint_lines(painter, lines_in_image, camera_matrix);

        Ok(())
    }
}

fn paint_lines(
    painter: &TwixPainter<Ground>,
    lines_in_image: Option<Vec<LineSegment<Pixel>>>,
    camera_matrix: Option<CameraMatrix>,
) -> Option<()> {
    let camera_matrix = camera_matrix?;
    for line in lines_in_image? {
        let start = camera_matrix.pixel_to_ground(line.0);
        let end = camera_matrix.pixel_to_ground(line.1);
        painter.line_segment(start.ok()?, end.ok()?, Stroke::new(0.04_f32, Color32::BLUE));
    }
    Some(())
}
