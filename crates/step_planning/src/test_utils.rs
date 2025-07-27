pub mod decompose;
pub mod gradient_type;
pub mod verify_gradient;

use std::f32::consts::{PI, TAU};

use approx::AbsDiffEq;
use coordinate_systems::Ground;
use linear_algebra::{point, vector, Orientation2, Point2};
use proptest::test_runner::Config as ProptestConfig;

pub fn proptest_config() -> ProptestConfig {
    ProptestConfig {
        cases: 1_000_000,
        max_global_rejects: 50_000,
        ..Default::default()
    }
}

pub fn is_near_test_path_segment_joins(query_point: Point2<Ground>) -> bool {
    is_near_ray(
        query_point,
        point![3.0, 1.0],
        Orientation2::from_vector(vector![1.0, 0.0]),
    ) || is_near_ray(
        query_point,
        point![3.0, 1.0],
        Orientation2::from_vector(vector![0.0, -1.0]),
    ) || is_near_test_path_progress_discontinuity(query_point)
}

pub fn is_near_test_path_progress_discontinuity(query_point: Point2<Ground>) -> bool {
    is_near_ray(
        query_point,
        point![3.0, 1.0],
        Orientation2::from_vector(vector![-1.0, 1.0]),
    )
}

fn is_near_ray(
    query_point: Point2<Ground>,
    start: Point2<Ground>,
    direction: Orientation2<Ground>,
) -> bool {
    let direction_vector = direction.as_unit_vector();
    let start_to_query = query_point - start;

    let t = start_to_query.dot(&direction_vector);
    let query_projected_onto_line = start + direction_vector * t.max(0.0);

    let squared_distance_to_ray = (query_point - query_projected_onto_line).norm_squared();

    squared_distance_to_ray < 1e-2
}

pub fn is_roughly_opposite(a: f32, b: f32) -> bool {
    (a - b).rem_euclid(TAU).abs_diff_eq(&PI, 1e-2)
}

#[cfg(test)]
mod tests {
    use linear_algebra::point;

    use crate::test_utils::is_near_test_path_progress_discontinuity;

    #[test]
    fn test_is_near_test_path_discontinuity() {
        assert!(is_near_test_path_progress_discontinuity(point![3.0, 1.0]));
        assert!(is_near_test_path_progress_discontinuity(point![2.0, 2.0]));
        assert!(is_near_test_path_progress_discontinuity(point![1.0, 3.0]));

        assert!(!is_near_test_path_progress_discontinuity(point![4.0, 0.0]));
        assert!(!is_near_test_path_progress_discontinuity(point![2.5, 2.5]));
    }
}
