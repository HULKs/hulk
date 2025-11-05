use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::{Ground, Pixel};
use geometry::line_segment::LineSegment;
use projection::{camera_matrix::CameraMatrix, Projection};
use types::field_dimensions::FieldDimensions;

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct Lines {
    lines_in_image: BufferHandle<Option<Vec<LineSegment<Pixel>>>>,
    camera_matrix: BufferHandle<Option<CameraMatrix>>,
}

impl Layer<Ground> for Lines {
    const NAME: &'static str = "Lines";

    fn new(nao: Arc<Nao>) -> Self {
        let lines_in_image = nao.subscribe_value("Vision.additional_outputs.lines_in_image");
        let camera_matrix = nao.subscribe_value("Vision.main_outputs.camera_matrix");
        Self {
            lines_in_image: lines_in_image,
            camera_matrix: camera_matrix,
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
        let Some(camera_matrix) = self.camera_matrix.get_last_value()? else {
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
        painter.line_segment(start.ok()?, end.ok()?, Stroke::new(0.04, Color32::BLUE));
    }
    Some(())
}
