use coordinate_systems::Ground;
use linear_algebra::{Point2, Vector2};
use types::planned_path::Path;

use crate::{
    traits::{Length, PathProgress},
    utils::{smoothmin, smoothmin_derivative},
};

pub struct PathProgressField<'a> {
    pub path: &'a Path,
    pub smoothness: f32,
}

impl PathProgressField<'_> {
    pub fn cost(&self, point: Point2<Ground>) -> f32 {
        let progress = self.path.progress(point);

        let clamped_progress = smoothmin(progress, self.path.length(), self.smoothness);

        self.path.length() - clamped_progress
    }

    pub fn grad(&self, point: Point2<Ground>) -> Vector2<Ground> {
        let progress = self.path.progress(point);
        let forward = self.path.forward(point);

        let forward_scale = smoothmin_derivative(progress, self.path.length(), self.smoothness);

        -forward * forward_scale
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

    use approx::assert_abs_diff_eq;

    use geometry::{arc::Arc, circle::Circle, direction::Direction, line_segment::LineSegment};
    use linear_algebra::{point, vector, Orientation2};
    use types::planned_path::{Path, PathSegment};

    use crate::cost_fields::path_progress::PathProgressField;

    fn test_path() -> Path {
        Path {
            segments: vec![
                PathSegment::LineSegment(LineSegment(point![0.0, 0.0], point![3.0, 0.0])),
                PathSegment::Arc(Arc {
                    circle: Circle {
                        center: point![3.0, 1.0],
                        radius: 1.0,
                    },
                    start: Orientation2::new(3.0 * FRAC_PI_2),
                    end: Orientation2::new(0.0),
                    direction: Direction::Counterclockwise,
                }),
                PathSegment::LineSegment(LineSegment(point![4.0, 1.0], point![4.0, 4.0])),
            ],
        }
    }

    #[test]
    fn test_path_progress() {
        let cost_field = PathProgressField {
            path: &test_path(),
            smoothness: 1.0,
        };

        // Start
        let sample_point_1 = point![0.0, 0.0];
        let cost_1 = cost_field.cost(sample_point_1);
        let grad_1 = cost_field.grad(sample_point_1);

        assert_abs_diff_eq!(grad_1, vector![-1.0, 0.0]);

        // Before start
        let sample_point_2 = point![-1.0, 0.0];
        let cost_2 = cost_field.cost(sample_point_2);
        let grad_2 = cost_field.grad(sample_point_2);

        assert_abs_diff_eq!(cost_2 - cost_1, 1.0, epsilon = 1e-6);
        assert_abs_diff_eq!(grad_2, vector![-1.0, 0.0]);

        // End of first line segment, start of arc
        let sample_point_3 = point![3.0, 0.0];
        let cost_3 = cost_field.cost(sample_point_3);
        let grad_3 = cost_field.grad(sample_point_3);

        assert_abs_diff_eq!(cost_3 - cost_1, -3.0);
        assert_abs_diff_eq!(grad_3, vector![-1.0, 0.0]);

        // Below start of arc
        let sample_point_4 = point![3.0, -1.0];
        let cost_4 = cost_field.cost(sample_point_4);
        let grad_4 = cost_field.grad(sample_point_4);

        assert_abs_diff_eq!(cost_4, cost_3);
        assert_abs_diff_eq!(grad_4, grad_3);

        // End of arc
        let sample_point_5 = point![4.0, 1.0];
        let cost_5 = cost_field.cost(sample_point_5);
        let grad_5 = cost_field.grad(sample_point_5);

        assert_abs_diff_eq!(cost_5, cost_3 - FRAC_PI_2);
        assert_abs_diff_eq!(grad_5, vector![0.0, -1.0]);

        // End
        let sample_point_6 = point![4.0, 4.0];
        let cost_6 = cost_field.cost(sample_point_6);
        let grad_6 = cost_field.grad(sample_point_6);

        assert!(((cost_5 - 3.0)..(cost_5 - 2.0)).contains(&cost_6));
        assert_abs_diff_eq!(grad_6, vector![0.0, 0.0]);

        // Outside of arc
        let sample_point_7 = point![4.0, 0.0];
        let cost_7 = cost_field.cost(sample_point_7);
        let grad_7 = cost_field.grad(sample_point_7);

        assert_abs_diff_eq!(cost_7, cost_3 - FRAC_PI_4, epsilon = 1e-6);
        assert_abs_diff_eq!(grad_7, vector![-0.5, -0.5]);
    }
}
