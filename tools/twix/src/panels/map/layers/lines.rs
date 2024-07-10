use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::{Ground, Pixel};
use geometry::line::Line2;
use projection::{camera_matrix::CameraMatrix, Projection};
use types::field_dimensions::FieldDimensions;

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct Lines {
    lines_in_image_top: BufferHandle<Option<Vec<Line2<Pixel>>>>,
    camera_matrix_top: BufferHandle<Option<CameraMatrix>>,
    lines_in_image_bottom: BufferHandle<Option<Vec<Line2<Pixel>>>>,
    camera_matrix_bottom: BufferHandle<Option<CameraMatrix>>,
}

impl Layer<Ground> for Lines {
    const NAME: &'static str = "Lines";

    fn new(nao: Arc<Nao>) -> Self {
        let lines_in_image_top = nao.subscribe_value("VisionTop.additional_outputs.lines_in_image");
        let camera_matrix_top = nao.subscribe_value("VisionTop.main_outputs.camera_matrix");
        let lines_in_image_bottom =
            nao.subscribe_value("VisionBottom.additional_outputs.lines_in_image");
        let camera_matrix_bottom = nao.subscribe_value("VisionBottom.main_outputs.camera_matrix");
        Self {
            lines_in_image_top,
            camera_matrix_top,
            lines_in_image_bottom,
            camera_matrix_bottom,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let Some(lines_in_image_top) = self.lines_in_image_top.get_last_value()? else {
            return Ok(());
        };
        let Some(camera_matrix_top) = self.camera_matrix_top.get_last_value()? else {
            return Ok(());
        };
        paint_lines(painter, lines_in_image_top, camera_matrix_top);

        let Some(lines_in_image_bottom) = self.lines_in_image_bottom.get_last_value()? else {
            return Ok(());
        };
        let Some(camera_matrix_bottom) = self.camera_matrix_bottom.get_last_value()? else {
            return Ok(());
        };
        paint_lines(painter, lines_in_image_bottom, camera_matrix_bottom);
        Ok(())
    }
}

fn paint_lines(
    painter: &TwixPainter<Ground>,
    lines_in_image: Option<Vec<Line2<Pixel>>>,
    camera_matrix: Option<CameraMatrix>,
) -> Option<()> {
    let camera_matrix = camera_matrix?;
    for line in lines_in_image? {
        let start = camera_matrix.pixel_to_ground(line.0);
        let end = camera_matrix.pixel_to_ground(line.1);
        painter.line_segment(start.ok()?, end.ok()?, Stroke::new(0.04, Color32::RED));
    }
    Some(())
}
