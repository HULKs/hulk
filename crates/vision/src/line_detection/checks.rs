use std::ops::Range;

use coordinate_systems::Pixel;
use linear_algebra::{distance, vector, Point2, Vector2};
use projection::{camera_matrix::CameraMatrix, Projection};
use types::{
    image_segments::{EdgeType, GenericSegment},
    ycbcr422_image::YCbCr422Image,
};

pub fn is_non_field_segment(segment: &GenericSegment) -> bool {
    segment.start_edge_type == EdgeType::Rising && segment.end_edge_type == EdgeType::Falling
}

pub fn is_in_length_range(
    segment: &GenericSegment,
    camera_matrix: &CameraMatrix,
    allowed_projected_segment_length: &Range<f32>,
) -> bool {
    let Ok(start) = camera_matrix.pixel_to_ground(segment.start.cast()) else {
        return false;
    };
    let Ok(end) = camera_matrix.pixel_to_ground(segment.end.cast()) else {
        return false;
    };
    allowed_projected_segment_length.contains(&distance(start, end))
}

pub fn has_opposite_gradients(
    segment: &GenericSegment,
    image: &YCbCr422Image,
    gradient_alignment: f32,
    gradient_sobel_stride: u32,
) -> bool {
    // checking gradients on segments without rising/falling edges doesn't make sense, just accept those
    if segment.start_edge_type != EdgeType::Rising || segment.end_edge_type != EdgeType::Falling {
        return true;
    }

    // gradients (approximately) point in opposite directions if their dot product is (close to) -1
    let gradient_at_start = get_gradient(image, segment.start, gradient_sobel_stride);
    let gradient_at_end = get_gradient(image, segment.end, gradient_sobel_stride);
    gradient_at_start.dot(&gradient_at_end) < gradient_alignment
}

fn get_gradient(
    image: &YCbCr422Image,
    point: Point2<Pixel, u16>,
    gradient_sobel_stride: u32,
) -> Vector2<f32> {
    if point.x() < gradient_sobel_stride as u16
        || point.y() < gradient_sobel_stride as u16
        || point.x() > image.width() as u16 - 2 * gradient_sobel_stride as u16
        || point.y() > image.height() as u16 - 2 * gradient_sobel_stride as u16
    {
        return vector![0.0, 0.0];
    }
    let px = point.x() as u32;
    let py = point.y() as u32;
    // Sobel matrix x (transposed)
    // -1 -2 -1
    //  0  0  0
    //  1  2  1
    let gradient_x = (-1.0
        * image
            .at(px - gradient_sobel_stride, py - gradient_sobel_stride)
            .y as f32)
        + (-2.0 * image.at(px, py - gradient_sobel_stride).y as f32)
        + (-1.0
            * image
                .at(px + gradient_sobel_stride, py - gradient_sobel_stride)
                .y as f32)
        + (1.0
            * image
                .at(px - gradient_sobel_stride, py + gradient_sobel_stride)
                .y as f32)
        + (2.0 * image.at(px, py + gradient_sobel_stride).y as f32)
        + (1.0
            * image
                .at(px + gradient_sobel_stride, py + gradient_sobel_stride)
                .y as f32);
    // Sobel matrix y (transposed)
    //  1  0 -1
    //  2  0 -2
    //  1  0 -1
    let gradient_y = (1.0
        * image
            .at(px - gradient_sobel_stride, py - gradient_sobel_stride)
            .y as f32)
        + (-1.0
            * image
                .at(px + gradient_sobel_stride, py - gradient_sobel_stride)
                .y as f32)
        + (2.0 * image.at(px - gradient_sobel_stride, py).y as f32)
        + (-2.0 * image.at(px + gradient_sobel_stride, py).y as f32)
        + (1.0
            * image
                .at(px - gradient_sobel_stride, py + gradient_sobel_stride)
                .y as f32)
        + (-1.0
            * image
                .at(px + gradient_sobel_stride, py + gradient_sobel_stride)
                .y as f32);
    let gradient = vector![gradient_x, gradient_y];
    gradient
        .try_normalize(0.0001)
        .unwrap_or_else(Vector2::zeros)
}

#[cfg(test)]
mod tests {
    use linear_algebra::{point, IntoTransform, Rotation3};
    use nalgebra::{Isometry3, Translation, UnitQuaternion};

    use super::*;

    #[test]
    fn check_fixed_segment_size() {
        let image_size = vector![1.0, 1.0];
        let camera_matrix = CameraMatrix::from_normalized_focal_and_center(
            nalgebra::vector![2.0, 2.0],
            nalgebra::point![1.0, 1.0],
            image_size,
            Isometry3 {
                rotation: UnitQuaternion::from_euler_angles(0.0, std::f32::consts::PI / 4.0, 0.0),
                translation: Translation::from(nalgebra::point![0.0, 0.0, 0.5]),
            }
            .framed_transform(),
            Isometry3::identity().framed_transform(),
            Isometry3::identity().framed_transform(),
            Rotation3::from_euler_angles(0.0, 0.0, 0.0),
            Rotation3::from_euler_angles(0.0, 0.0, 0.0),
        );

        let segment = GenericSegment {
            start: point![40, 2],
            end: point![40, 202],
            start_edge_type: EdgeType::ImageBorder,
            end_edge_type: EdgeType::ImageBorder,
        };
        assert!(!is_in_length_range(&segment, &camera_matrix, &(0.0..0.3)));

        let segment = GenericSegment {
            start: point![40, 364],
            end: point![40, 366],
            start_edge_type: EdgeType::ImageBorder,
            end_edge_type: EdgeType::ImageBorder,
        };
        assert!(is_in_length_range(&segment, &camera_matrix, &(0.0..0.3)));
    }

    #[test]
    fn gradient_of_zero_image() {
        let image = YCbCr422Image::zero(4, 4);
        let point = point![1, 1];
        assert_eq!(get_gradient(&image, point, 1), vector![0.0, 0.0]);
    }
}
