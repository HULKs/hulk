use coordinate_systems::Ground;
use linear_algebra::Vector2;

use crate::utils::{smoothmin, smoothmin_derivative};

pub struct PathProgressField {
    pub smoothness: f32,
}

impl PathProgressField {
    pub fn cost(&self, progress: f32, path_length: f32) -> f32 {
        let clamped_progress = smoothmin(progress, path_length, self.smoothness);

        path_length - clamped_progress
    }

    pub fn grad(
        &self,
        progress: f32,
        forward: Vector2<Ground>,
        path_length: f32,
    ) -> Vector2<Ground> {
        let forward_scale = smoothmin_derivative(progress, path_length, self.smoothness);

        -forward * forward_scale
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

    use approx::assert_abs_diff_eq;
    use proptest::{prop_assume, proptest};

    use linear_algebra::{point, vector};

    use crate::{
        cost_fields::path_progress::PathProgressField,
        test_utils::{is_near_test_path_segment_joins, test_path},
        traits::{Length, PathProgress},
    };

    #[test]
    fn test_path_progress() {
        let cost_field = PathProgressField { smoothness: 1.0 };
        let path = &test_path();
        let path_length = path.length();

        // Start
        let sample_point_1 = point![0.0, 0.0];
        let progress = path.progress(sample_point_1);
        let forward = path.forward(sample_point_1);
        let cost_1 = cost_field.cost(progress, path_length);
        let grad_1 = cost_field.grad(progress, forward, path_length);

        assert_abs_diff_eq!(grad_1, vector![-1.0, 0.0]);

        // Before start
        let sample_point_2 = point![-1.0, 0.0];
        let progress = path.progress(sample_point_2);
        let forward = path.forward(sample_point_2);
        let cost_2 = cost_field.cost(progress, path_length);
        let grad_2 = cost_field.grad(progress, forward, path_length);

        assert_abs_diff_eq!(cost_2 - cost_1, 1.0, epsilon = 1e-6);
        assert_abs_diff_eq!(grad_2, vector![-1.0, 0.0]);

        // End of first line segment, start of arc
        let sample_point_3 = point![3.0, 0.0];
        let progress = path.progress(sample_point_3);
        let forward = path.forward(sample_point_3);
        let cost_3 = cost_field.cost(progress, path_length);
        let grad_3 = cost_field.grad(progress, forward, path_length);

        assert_abs_diff_eq!(cost_3 - cost_1, -3.0);
        assert_abs_diff_eq!(grad_3, vector![-1.0, 0.0]);

        // Below start of arc
        let sample_point_4 = point![3.0, -1.0];
        let progress = path.progress(sample_point_4);
        let forward = path.forward(sample_point_4);
        let cost_4 = cost_field.cost(progress, path_length);
        let grad_4 = cost_field.grad(progress, forward, path_length);

        assert_abs_diff_eq!(cost_4, cost_3);
        assert_abs_diff_eq!(grad_4, grad_3);

        // End of arc
        let sample_point_5 = point![4.0, 1.0];
        let progress = path.progress(sample_point_5);
        let forward = path.forward(sample_point_5);
        let cost_5 = cost_field.cost(progress, path_length);
        let grad_5 = cost_field.grad(progress, forward, path_length);

        assert_abs_diff_eq!(cost_5, cost_3 - FRAC_PI_2);
        assert_abs_diff_eq!(grad_5, vector![0.0, -1.0]);

        // End
        let sample_point_6 = point![4.0, 4.0];
        let progress = path.progress(sample_point_6);
        let forward = path.forward(sample_point_6);
        let cost_6 = cost_field.cost(progress, path_length);
        let grad_6 = cost_field.grad(progress, forward, path_length);

        assert!(((cost_5 - 3.0)..(cost_5 - 2.0)).contains(&cost_6));
        assert_abs_diff_eq!(grad_6, vector![0.0, 0.0]);

        // Outside of arc
        let sample_point_7 = point![4.0, 0.0];
        let progress = path.progress(sample_point_7);
        let forward = path.forward(sample_point_7);
        let cost_7 = cost_field.cost(progress, path_length);
        let grad_7 = cost_field.grad(progress, forward, path_length);

        assert_abs_diff_eq!(cost_7, cost_3 - FRAC_PI_4, epsilon = 1e-6);
        assert_abs_diff_eq!(grad_7, vector![-0.5, -0.5]);
    }

    proptest!(
        #[test]
        fn verify_gradient(x in -2.0f32..5.0, y in -2.0f32..5.0) {
            prop_assume!(!is_near_test_path_segment_joins(point![x, y]));
            verify_gradient_impl(x, y)
        }
    );

    fn verify_gradient_impl(x: f32, y: f32) {
        let path = test_path();
        let cost_field = PathProgressField { smoothness: 0.5 };

        let point = point![x, y];

        crate::test_utils::verify_gradient::verify_gradient(
            &|p| {
                let progress = path.progress(p);
                let path_length = path.length();
                cost_field.cost(progress, path_length)
            },
            &|p| {
                let progress = path.progress(p);
                let forward = path.forward(p);
                let path_length = path.length();

                cost_field.grad(progress, forward, path_length)
            },
            0.05,
            point,
        )
    }
}
