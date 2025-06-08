use coordinate_systems::Ground;
use linear_algebra::{Point2, Vector2};
use types::planned_path::Path;

use crate::traits::Project;

pub struct PathDistanceField<'a> {
    pub path: Path<'a>,
}

impl PathDistanceField<'_> {
    pub fn cost(&self, point: Point2<Ground>) -> f32 {
        let projection = self.path.project(point);

        let projection_to_point = point - projection;

        projection_to_point.norm_squared()
    }

    pub fn grad(&self, point: Point2<Ground>) -> Vector2<Ground> {
        let projection = self.path.project(point);

        let projection_to_point = point - projection;

        projection_to_point * 2.0
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::SQRT_2;

    use approx::assert_abs_diff_eq;
    use proptest::proptest;

    use linear_algebra::{point, vector, Vector2};
    use types::planned_path::Path;

    use crate::{cost_fields::path_distance::PathDistanceField, test_utils::test_path};

    #[test]
    fn test_path_distance() {
        let cost_field = PathDistanceField {
            path: Path {
                segments: &test_path(),
            },
        };

        // Start
        let sample_point_1 = point![0.0, 0.0];
        let cost_1 = cost_field.cost(sample_point_1);
        let grad_1 = cost_field.grad(sample_point_1);

        assert_abs_diff_eq!(cost_1, 0.0);
        assert_abs_diff_eq!(grad_1, Vector2::zeros());

        // Before start
        let sample_point_2 = point![-1.0, 0.0];
        let cost_2 = cost_field.cost(sample_point_2);
        let grad_2 = cost_field.grad(sample_point_2);

        assert_abs_diff_eq!(cost_2, 1.0);
        assert_abs_diff_eq!(grad_2, vector![-2.0, 0.0]);

        // End of first line segment, start of arc
        let sample_point_3 = point![3.0, 0.0];
        let cost_3 = cost_field.cost(sample_point_3);
        let grad_3 = cost_field.grad(sample_point_3);

        assert_abs_diff_eq!(cost_3, 0.0);
        assert_abs_diff_eq!(grad_3, Vector2::zeros());

        // Below start of arc
        let sample_point_4 = point![3.0, -1.0];
        let cost_4 = cost_field.cost(sample_point_4);
        let grad_4 = cost_field.grad(sample_point_4);

        assert_abs_diff_eq!(cost_4, 1.0);
        assert_abs_diff_eq!(grad_4, vector![0.0, -2.0]);

        // End of arc
        let sample_point_5 = point![4.0, 1.0];
        let cost_5 = cost_field.cost(sample_point_5);
        let grad_5 = cost_field.grad(sample_point_5);

        assert_abs_diff_eq!(cost_5, 0.0);
        assert_abs_diff_eq!(grad_5, Vector2::zeros());

        // End
        let sample_point_6 = point![4.0, 4.0];
        let cost_6 = cost_field.cost(sample_point_6);
        let grad_6 = cost_field.grad(sample_point_6);

        assert_abs_diff_eq!(cost_6, 0.0);
        assert_abs_diff_eq!(grad_6, Vector2::zeros());

        // Outside of arc
        let sample_point_7 = point![4.0, 0.0];
        let cost_7 = cost_field.cost(sample_point_7);
        let grad_7 = cost_field.grad(sample_point_7);

        assert_abs_diff_eq!(cost_7, (SQRT_2 - 1.0).powi(2));
        assert_abs_diff_eq!(grad_7, vector![2.0 - SQRT_2, -(2.0 - SQRT_2)]);
    }

    proptest! {
        #[test]
        fn verify_gradient(x in -2.0f32..5.0, y in -2.0f32..5.0) {
            let cost_field = PathDistanceField {
                path: Path {
                    segments: &test_path(),
                },
            };

            let point = point![x, y];

            crate::test_utils::verify_gradient::verify_gradient(
                &|p| cost_field.cost(p),
                &|p| cost_field.grad(p),
                0.05,
                point,
            )
        }
    }
}
