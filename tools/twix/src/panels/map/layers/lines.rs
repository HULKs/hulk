use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use communication::client::CyclerOutput;
use coordinate_systems::{Ground, Pixel};
use geometry::line::Line2;
use projection::{camera_matrix::CameraMatrix, Projection};
use types::field_dimensions::FieldDimensions;

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct Lines {
    lines_in_image_top: ValueBuffer,
    camera_matrix_top: ValueBuffer,
    lines_in_image_bottom: ValueBuffer,
    camera_matrix_bottom: ValueBuffer,
}

impl Layer<Ground> for Lines {
    const NAME: &'static str = "Lines";

    fn new(nao: Arc<Nao>) -> Self {
        let lines_in_image_top = nao.subscribe_output(
            CyclerOutput::from_str("VisionTop.additional.lines_in_image").unwrap(),
        );
        let camera_matrix_top =
            nao.subscribe_output(CyclerOutput::from_str("VisionTop.main.camera_matrix").unwrap());
        let lines_in_image_bottom = nao.subscribe_output(
            CyclerOutput::from_str("VisionBottom.additional.lines_in_image").unwrap(),
        );
        let camera_matrix_bottom = nao
            .subscribe_output(CyclerOutput::from_str("VisionBottom.main.camera_matrix").unwrap());
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
        let lines_in_image_top: Option<Vec<Line2<Pixel>>> =
            self.lines_in_image_top.require_latest().ok();
        let camera_matrix_top: Option<CameraMatrix> = self.camera_matrix_top.require_latest().ok();
        paint_lines(painter, lines_in_image_top, camera_matrix_top);

        let lines_in_image_bottom: Option<Vec<Line2<Pixel>>> =
            self.lines_in_image_bottom.require_latest().ok();
        let camera_matrix_bottom: Option<CameraMatrix> =
            self.camera_matrix_bottom.require_latest().ok();
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
