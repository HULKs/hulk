use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::{Ground, Pixel};
use geometry::line_segment::LineSegment;
use projection::{Projection, camera_matrix::CameraMatrix};
use types::{field_dimensions::FieldDimensions, time_wrapper::TimeWrapper};

use crate::{
    backend::TwixBackend, panels::map::layer::Layer, twix_painter::TwixPainter,
    value_buffer::BufferHandle,
};

pub struct Lines {
    lines_in_image: BufferHandle<Vec<LineSegment<Pixel>>>,
    camera_matrix: BufferHandle<TimeWrapper<CameraMatrix>>,
}

impl Layer<Ground> for Lines {
    const NAME: &'static str = "Lines";

    fn new(backend: Arc<TwixBackend>) -> Self {
        let lines_in_image = backend.subscribe_buffered_value_with_queue_depth(
            "line_detection/lines_in_image",
            std::time::Duration::ZERO,
            crate::backend::HIGH_RATE_SUBSCRIBER_QUEUE_DEPTH,
        );
        let camera_matrix = backend.subscribe_buffered_value_with_queue_depth(
            "camera_matrix",
            std::time::Duration::ZERO,
            crate::backend::HIGH_RATE_SUBSCRIBER_QUEUE_DEPTH,
        );
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
        let Some(lines_in_image) = self.lines_in_image.get_last_value()? else {
            return Ok(());
        };
        let Some(camera_matrix) = self
            .camera_matrix
            .get_last_value()?
            .map(|value| value.inner)
        else {
            return Ok(());
        };
        paint_lines(painter, lines_in_image, camera_matrix);

        Ok(())
    }
}

fn paint_lines(
    painter: &TwixPainter<Ground>,
    lines_in_image: Vec<LineSegment<Pixel>>,
    camera_matrix: CameraMatrix,
) -> Option<()> {
    for line in lines_in_image {
        let start = camera_matrix.pixel_to_ground(line.0);
        let end = camera_matrix.pixel_to_ground(line.1);
        painter.line_segment(start.ok()?, end.ok()?, Stroke::new(0.04_f32, Color32::BLUE));
    }
    Some(())
}
