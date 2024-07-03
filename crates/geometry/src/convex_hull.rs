use linear_algebra::Point2;
use nalgebra::Matrix2;

pub enum Range {
    Full,
    OnlyBottomHalf,
}

pub fn reduce_to_convex_hull<Frame>(points: &[Point2<Frame>], range: Range) -> Vec<Point2<Frame>>
where
    Frame: Copy,
{
    // https://en.wikipedia.org/wiki/Gift_wrapping_algorithm
    // Modification: This implementation iterates from left to right until a smaller x value is found
    if points.is_empty() {
        return vec![];
    }
    let mut point_on_hull = *points
        .iter()
        .min_by(|a, b| a.x().total_cmp(&b.x()))
        .unwrap();
    let mut convex_hull = vec![];
    loop {
        convex_hull.push(point_on_hull);
        let mut candidate_end_point = points[0];
        for point in points.iter() {
            let last_point_on_hull_to_candidate_end_point = candidate_end_point - point_on_hull;
            let last_point_on_hull_to_point = *point - point_on_hull;
            let determinant = Matrix2::from_columns(&[
                last_point_on_hull_to_candidate_end_point.inner,
                last_point_on_hull_to_point.inner,
            ])
            .determinant();
            let point_is_left_of_candidate_end_point = determinant < 0.0;
            if candidate_end_point == point_on_hull || point_is_left_of_candidate_end_point {
                candidate_end_point = *point;
            }
        }
        // begin of modification
        if matches!(range, Range::OnlyBottomHalf) && candidate_end_point.x() < point_on_hull.x() {
            break;
        }
        // end of modification
        point_on_hull = candidate_end_point;
        if candidate_end_point == *convex_hull.first().unwrap() {
            break;
        }
    }
    convex_hull
}

#[cfg(test)]
mod test {
    use super::*;
    use coordinate_systems::Ground;
    use linear_algebra::point;

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
        let bottom_half_hexagon = vec![
            point![-1.0, 0.0],
            point![-0.5, -0.86],
            point![0.5, -0.86],
            point![1.0, 0.0],
        ];
        assert_eq!(
            hexagon,
            reduce_to_convex_hull::<Ground>(&hexagon, Range::Full)
        );
        assert_eq!(
            bottom_half_hexagon,
            reduce_to_convex_hull::<Ground>(&hexagon, Range::OnlyBottomHalf)
        );
    }
}
