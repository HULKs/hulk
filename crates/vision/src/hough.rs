use geometry::rectangle::Rectangle;
use itertools::Itertools;
use linear_algebra::{point, Point2};
use nalgebra::DMatrix;

// fn hough<T>(points: &[Point2<T>], roi: Option<Rectangle<T>>) {
//     let roi = roi.unwrap_or_else(|| get_center_circle_roi(points, (0.0, 0.0)));

//     let dmax = (roi.max - roi.min).norm();
//     let angles = (0..180).map(|a| (a as f32).to_radians());

//     let accum = DMatrix::<u32>::zeros(nrows, ncols)
// }

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
