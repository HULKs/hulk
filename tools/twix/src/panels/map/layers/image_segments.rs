use std::str::FromStr;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use communication::client::CyclerOutput;
use coordinate_systems::Ground;
use linear_algebra::{center, point, Point2};
use projection::Projection;
use types::{camera_matrix::CameraMatrix, color::Rgb, field_dimensions::FieldDimensions};

use crate::{panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer};

pub struct ImageSegments {
    image_segments_bottom: ValueBuffer,
    camera_matrix_bottom: ValueBuffer,
    image_segments_top: ValueBuffer,
    camera_matrix_top: ValueBuffer,
}

impl Layer<Ground> for ImageSegments {
    const NAME: &'static str = "Image Segments";

    fn new(nao: std::sync::Arc<crate::nao::Nao>) -> Self {
        let image_segments_bottom = nao
            .subscribe_output(CyclerOutput::from_str("VisionBottom.main.image_segments").unwrap());
        let camera_matrix_bottom = nao
            .subscribe_output(CyclerOutput::from_str("VisionBottom.main.camera_matrix").unwrap());
        let image_segments_top =
            nao.subscribe_output(CyclerOutput::from_str("VisionTop.main.image_segments").unwrap());
        let camera_matrix_top =
            nao.subscribe_output(CyclerOutput::from_str("VisionTop.main.camera_matrix").unwrap());
        Self {
            image_segments_bottom,
            camera_matrix_bottom,
            image_segments_top,
            camera_matrix_top,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        paint_segments(
            painter,
            &self.camera_matrix_bottom.require_latest()?,
            &self.image_segments_bottom.require_latest()?,
        )?;
        paint_segments(
            painter,
            &self.camera_matrix_top.require_latest()?,
            &self.image_segments_top.require_latest()?,
        )?;
        Ok(())
    }
}

fn paint_segments(
    painter: &TwixPainter<Ground>,
    camera_matrix: &CameraMatrix,
    segments: &types::image_segments::ImageSegments,
) -> Result<()> {
    for scanline in &segments.scan_grid.vertical_scan_lines {
        let x = scanline.position as f32;
        for segment in scanline.segments.iter().rev() {
            let ycbcr_color = segment.color;
            let rgb_color = Rgb::from(ycbcr_color);
            let original_color = Color32::from_rgb(rgb_color.r, rgb_color.g, rgb_color.b);

            let segment_in_field = match project_segment_to_ground(x, segment, camera_matrix) {
                Ok(result) => result,
                Err(_error) => break,
            };

            painter.line_segment(
                segment_in_field.start,
                segment_in_field.end,
                Stroke::new(
                    segment_in_field.line_width.clamp(0.001, 0.1),
                    original_color,
                ),
            );
        }
    }
    Ok(())
}

struct SegmentInGround {
    start: Point2<Ground>,
    end: Point2<Ground>,
    line_width: f32,
}

fn project_segment_to_ground(
    x: f32,
    segment: &types::image_segments::Segment,
    camera_matrix: &CameraMatrix,
) -> Result<SegmentInGround> {
    let start = point![x, segment.start as f32];
    let end = point![x, segment.end as f32];

    let start_in_ground = camera_matrix.pixel_to_ground(start)?;
    let end_in_ground = camera_matrix.pixel_to_ground(end)?;

    let midpoint = center(start, end);
    let pixel_radius = 100.0 * camera_matrix.get_pixel_radius(0.01, midpoint)?;
    let line_width = 3.0 / pixel_radius;

    Ok(SegmentInGround {
        start: start_in_ground,
        end: end_in_ground,
        line_width,
    })
}
