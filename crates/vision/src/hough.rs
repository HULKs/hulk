use std::fmt::Debug;

use geometry::{line::Line2, rectangle::Rectangle};
use image::{GrayImage, Luma};
use imageproc::hough::{detect_lines, LineDetectionOptions};
use itertools::Itertools;
use linear_algebra::{point, vector, Point2};
use nalgebra::DMatrix;

pub(crate) struct HoughParams {
    // point_inclusion_distance: f32,
    pub peak_threshold: u32,
    pub rho_bin_size: usize,
    pub suppression_radius: usize, // radius *2 + 1 square region around each parameter
}

pub(crate) fn get_hough_line_with_edges_imgproc<T>(
    points: &[Point2<T>],
    roi: Option<Rectangle<T>>,
    params: &HoughParams,
) -> Vec<(Line2<T>, u32)>
where
    T: Debug,
{
    let roi = roi.unwrap_or_else(|| get_center_circle_roi(points, (0.0, 0.0)));

    let mut grayscaleImage = GrayImage::new(roi.max.x() as u32 + 1, roi.max.y() as u32 + 1);
    points.iter().for_each(|point| {
        grayscaleImage.put_pixel(point.x() as u32, point.y() as u32, Luma([255u8]));
    });

    let lines = detect_lines(
        &grayscaleImage,
        LineDetectionOptions {
            vote_threshold: params.peak_threshold,
            suppression_radius: 8,
        },
    );

    lines
        .into_iter()
        .map(|line| {
            let distance = line.r;
            let angle = line.angle_in_degrees as f32;

            (polar_line_to_line(angle.to_radians(), distance), 2)
        })
        .collect()
}

pub(crate) fn get_hough_line_with_edges<T>(
    points: &[Point2<T>],
    roi: Option<Rectangle<T>>,
    params: &HoughParams,
) -> Vec<(Line2<T>, u32)>
where
    T: Debug,
{
    // let point_inclusion_distance_squared = params.point_inclusion_distance.powi(2);
    let roi = roi.unwrap_or_else(|| get_center_circle_roi(points, (0.0, 0.0)));

    let max_distance = (roi.max - roi.min).norm();
    let rho_min = roi.min.coords().norm().floor();
    let rho_bins = (max_distance * 2.0 / params.rho_bin_size as f32).ceil() as usize;
    // let rho_bins = max_distance as usize * 2;

    let angles: Vec<_> = (0..180).map(|a| (a as f32).to_radians()).collect();
    let sin_cos = angles.iter().map(|a| a.sin_cos()).collect_vec();

    let mut accumulator = DMatrix::<u32>::zeros(sin_cos.len(), rho_bins);

    assert!(accumulator.nrows() == sin_cos.len());
    points.into_iter().for_each(|point| {
        let shifted_point = *point - roi.min;
        for (i, (sin, cos)) in sin_cos.iter().enumerate() {
            let rho = shifted_point.x() * cos + shifted_point.y() * sin + max_distance;
            let rho_index =( rho /params.rho_bin_size as f32).floor()as usize;
            // let rho_index=rho as usize;
            assert!(
                rho_index < rho_bins,
                "rho_index: {rho_index}, theta:{} rho_bins: {rho_bins}, roi min: {:?},rho min:{rho_min} dist max: {max_distance}, shifted_max_distance: {}"
                , sin.asin().to_degrees(),roi.min.inner, max_distance
            );
            accumulator[(i, rho_index)] += 1;
        }
    });

    // Find peaks in the accumulator
    // let mut peaks: Vec<(f32, f32)> = Vec::with_capacity(100);
    let mut peaks = Vec::with_capacity(100);
    for j in 0..accumulator.ncols() {
        for (i, (sin, cos)) in sin_cos.iter().enumerate() {
            let score = accumulator[(i, j)];
            if accumulator[(i, j)] > params.peak_threshold {
                let rho = (j as f32 * params.rho_bin_size as f32) - max_distance;
                // let rho = j as f32 - max_distance;
                // peaks.push((*theta, rho));
                peaks.push((polar_line_to_line_sincos(*sin, *cos, rho), score));
            }
        }
    }

    peaks.sort_by_key(|v| v.1);
    peaks
}

pub(crate) fn get_center_circle_roi<T>(
    center_circle_points: &[Point2<T>],
    roi_padding: (f32, f32),
) -> Rectangle<T> {
    let (x_min, x_max) = center_circle_points
        .iter()
        .map(|point| point.x())
        .minmax()
        .into_option()
        .unwrap();
    let (y_min, y_max) = center_circle_points
        .iter()
        .map(|point| point.y())
        .minmax()
        .into_option()
        .unwrap();
    Rectangle {
        min: point![x_min - roi_padding.0, y_min - roi_padding.1],
        max: point![x_max + roi_padding.0, y_max + roi_padding.1],
    }
}

fn polar_line_to_line<T>(angle: f32, rho: f32) -> Line2<T>
where
    T: Debug,
{
    let (sin, cos) = angle.sin_cos();
    polar_line_to_line_sincos(sin, cos, rho)
}

#[inline(always)]
fn polar_line_to_line_sincos<T>(sin: f32, cos: f32, rho: f32) -> Line2<T> {
    Line2 {
        point: (vector![rho * cos, rho * sin]).as_point(),
        // rho is orthogonal to the line direction
        direction: vector![-sin, cos],
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[derive(Debug)]
    struct MyFrame {}

    #[test]
    fn test_polar_to_line_conversion_basic() {
        let values = [(0, 10.0), (90, 10.0), (180, 10.0)];
        let expected_direction = [(0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)];

        for ((theta, rho), (expected_direction_x, expected_direction_y)) in
            values.iter().zip(expected_direction.iter())
        {
            // The rho long line with theta angle is orthogonal to the line direction
            let line2: Line2<MyFrame> = polar_line_to_line((*theta as f32).to_radians(), *rho);

            // y must be near 1
            assert_relative_eq!(line2.direction.x(), expected_direction_x, epsilon = 0.01);
            assert_relative_eq!(line2.direction.y(), expected_direction_y, epsilon = 0.01);
            // assert_relative_eq!(line2.point.y(), 0.0, epsilon = 0.01);
            // assert_relative_eq!(line2.point.x(), rho, epsilon = 0.01);
        }
    }
}
