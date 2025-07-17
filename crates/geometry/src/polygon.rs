use linear_algebra::{distance, Point2};
use nalgebra::Matrix2;

use crate::{circle::Circle, convex_hull::reduce_to_convex_hull, line_segment::LineSegment};

pub fn is_inside_polygon<Frame>(points: &[Point2<Frame>], target_point: &Point2<Frame>) -> bool {
    if points.is_empty() {
        return false;
    }

    let mut crossings = 0;
    for i in 0..points.len() {
        let j = (i + 1) % points.len();

        if points[i].y() <= target_point.y() {
            if points[j].y() > target_point.y()
                && cross_product(target_point, &points[i], &points[j]) > 0.0
            {
                crossings += 1;
            }
        } else if points[j].y() <= target_point.y()
            && cross_product(target_point, &points[i], &points[j]) < 0.0
        {
            crossings -= 1;
        }
    }

    crossings != 0
}

pub fn is_inside_convex_hull<Frame>(
    points: &[Point2<Frame>],
    target_point: &Point2<Frame>,
) -> bool {
    let convex_hull = reduce_to_convex_hull(points);
    is_inside_polygon(&convex_hull, target_point)
}

pub fn circle_overlaps_polygon<Frame>(points: &[Point2<Frame>], circle: Circle<Frame>) -> bool {
    if points.is_empty() {
        return false;
    };

    let mut crossings = 0;
    for i in 0..points.len() {
        let j = (i + 1) % points.len();

        if points[i].y() <= circle.center.y() {
            if points[j].y() > circle.center.y()
                && cross_product(&circle.center, &points[i], &points[j]) > 0.0
            {
                crossings += 1;
            }
        } else if points[j].y() <= circle.center.y()
            && cross_product(&circle.center, &points[i], &points[j]) < 0.0
        {
            crossings -= 1;
        }

        let closest = LineSegment(points[i], points[j]).closest_point(circle.center);
        if distance(closest, circle.center) <= circle.radius {
            return true;
        }
    }

    crossings != 0
}

fn cross_product<Frame>(
    point: &Point2<Frame>,
    segment_start: &Point2<Frame>,
    segment_end: &Point2<Frame>,
) -> f32 {
    let point_to_segment_start = *segment_start - *point;
    let point_to_segment_end = *segment_end - *point;
    Matrix2::from_columns(&[point_to_segment_start.inner, point_to_segment_end.inner]).determinant()
}
