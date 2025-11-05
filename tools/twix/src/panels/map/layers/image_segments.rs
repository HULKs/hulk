use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Ground;
use linear_algebra::{center, point, Point2};
use projection::{camera_matrix::CameraMatrix, Projection};
use types::{color::Rgb, field_dimensions::FieldDimensions};

use crate::{panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle};

pub struct ImageSegments {
    camera_matrix: BufferHandle<Option<CameraMatrix>>,
    image_segments: BufferHandle<types::image_segments::ImageSegments>,
}

impl Layer<Ground> for ImageSegments {
    const NAME: &'static str = "Image Segments";

    fn new(nao: std::sync::Arc<crate::nao::Nao>) -> Self {
        let camera_matrix = nao.subscribe_value("Vision.main_outputs.camera_matrix");
        let image_segments = nao.subscribe_value("Vision.main_outputs.image_segments");
        Self {
            camera_matrix,
            image_segments,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let Some(camera_matrix) = self.camera_matrix.get_last_value()?.flatten() else {
            return Ok(());
        };
        let Some(image_segments) = self.image_segments.get_last_value()? else {
            return Ok(());
        };

        paint_segments(painter, &camera_matrix, &image_segments)?;
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
            let original_color = Color32::from_rgb(rgb_color.red, rgb_color.green, rgb_color.blue);

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
