use std::mem::transmute;

use linear_algebra::Point2;
use parry2d::transformation::convex_hull;

pub fn reduce_to_convex_hull<Frame>(points: &[Point2<Frame>]) -> Vec<Point2<Frame>> {
    if points.len() < 3 {
        return points.to_vec();
    }

    // SAFETY: linear_algebra::Point2 has repr(transparent) and is guaranteed to have the same memory layout as nalgebra::Point2
    let points: &[_] = unsafe { transmute(points) };
    let convex_hull = convex_hull(points);
    // SAFETY: linear_algebra::Point2 has repr(transparent) and is guaranteed to have the same memory layout as nalgebra::Point2
    unsafe { transmute(convex_hull) }
}

#[cfg(test)]
mod test {
    use super::*;
    use coordinate_systems::Ground;
    use linear_algebra::point;
    use ordered_float::NotNan;

    fn assert_polygon_equality<Frame: std::fmt::Debug>(
        mut a: Vec<Point2<Frame>>,
        mut b: Vec<Point2<Frame>>,
    ) {
        a.sort_by_key(|point| {
            (
                NotNan::new(point.x()).unwrap(),
                NotNan::new(point.y()).unwrap(),
            )
        });
        b.sort_by_key(|point| {
            (
                NotNan::new(point.x()).unwrap(),
                NotNan::new(point.y()).unwrap(),
            )
        });

        assert_eq!(a, b)
    }

    #[test]
    fn test_convex_hull() {
        let hexagon = vec![
            point![-1.0, 0.0],
            point![-0.5, -0.86],
            point![0.5, -0.86],
            point![1.0, 0.0],
            point![0.5, 0.86],
            point![-0.5, 0.86],
        ];
        let hexagon_inner_points = vec![point![0.0, 0.0], point![0.1, 0.0], point![-0.1, 0.0]];
        assert_polygon_equality(hexagon.clone(), reduce_to_convex_hull::<Ground>(&hexagon));

        assert_polygon_equality(
            hexagon.clone(),
            reduce_to_convex_hull::<Ground>(&Vec::from_iter(
                hexagon.into_iter().chain(hexagon_inner_points),
            )),
        );
    }

    #[test]
    fn test_2_points() {
        let collinear_points = vec![point![0.0, 0.0], point![2.0, 0.0]];
        let result = reduce_to_convex_hull::<Ground>(&collinear_points);
        assert_polygon_equality(collinear_points, result);
    }
}
