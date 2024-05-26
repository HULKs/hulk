use linear_algebra::Point2;
use nalgebra::Matrix2;

pub fn reduce_to_convex_hull<Frame>(
    points: &[Point2<Frame>],
    only_top_half: bool,
) -> Vec<Point2<Frame>>
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
    let mut top_half_finished = false;
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
        let has_smaller_x = candidate_end_point.x() < point_on_hull.x();
        if has_smaller_x && !top_half_finished {
            if only_top_half {
                break;
            }
            top_half_finished = true
        }
        // end of modification
        point_on_hull = candidate_end_point;
        if candidate_end_point == *convex_hull.first().unwrap() {
            break;
        }
    }
    convex_hull
}
