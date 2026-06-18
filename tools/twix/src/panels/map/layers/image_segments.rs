use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Ground;
use linear_algebra::{Point2, center, point};
use projection::{Projection, camera_matrix::CameraMatrix};
use ros_z_debug::RetentionPolicy;
use types::{color::Rgb, field_dimensions::FieldDimensions, time_wrapper::TimeWrapper};

use crate::{
    backend::{TwixBackend, retained_subscription::TypedSubscription},
    panels::map::{latest_value, layer::Layer},
    twix_painter::TwixPainter,
};

pub struct ImageSegments {
    camera_matrix: TypedSubscription<TimeWrapper<CameraMatrix>>,
    image_segments: TypedSubscription<TimeWrapper<types::image_segments::ImageSegments>>,
}

impl Layer<Ground> for ImageSegments {
    const NAME: &'static str = "Image Segments";

    fn new(backend: std::sync::Arc<TwixBackend>) -> Self {
        let camera_matrix = backend.subscribe_typed_retained(
            "camera_matrix",
            RetentionPolicy::LatestOnly,
            crate::backend::HIGH_RATE_SUBSCRIBER_QUEUE_DEPTH,
        );
        let image_segments = backend.subscribe_typed_retained(
            "image_segments",
            RetentionPolicy::LatestOnly,
            crate::backend::HIGH_RATE_SUBSCRIBER_QUEUE_DEPTH,
        );
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
        let Some(camera_matrix) = latest_value(&self.camera_matrix).map(|value| value.inner) else {
            return Ok(());
        };
        let Some(image_segments) = latest_value(&self.image_segments).map(|value| value.inner)
        else {
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
