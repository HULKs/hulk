use linear_algebra::Vector2;
use types::planned_path::Path;

use crate::{
    geometry::{angle::Angle, pose::PoseGradient, Pose},
    traits::{Length, PathProgress},
    utils::{angle_penalty, angle_penalty_derivative},
};

pub struct TargetOrientationField<'a> {
    pub target_orientation: Angle<f32>,
    pub path: &'a Path,
    pub alignment_start_distance: f32,
    pub ramp_width: f32,
}

impl TargetOrientationField<'_> {
    pub fn cost(&self, pose: Pose<f32>) -> f32 {
        let progress = self.path.progress(pose.position);
        let path_length = self.path.length();

        let distance_to_target = path_length - progress;

        angle_penalty(pose.orientation, self.target_orientation)
            * self.importance(distance_to_target)
    }

    pub fn grad(&self, pose: Pose<f32>) -> PoseGradient<f32> {
        let progress = self.path.progress(pose.position);
        let path_length = self.path.length();

        let distance_to_target = path_length - progress;

        PoseGradient {
            position: Vector2::zeros(),
            orientation: angle_penalty_derivative(pose.orientation, self.target_orientation)
                * self.importance(distance_to_target),
        }
    }
}

impl TargetOrientationField<'_> {
    fn importance(&self, distance_to_target: f32) -> f32 {
        if distance_to_target > self.alignment_start_distance - self.ramp_width {
            0.0
        } else if distance_to_target < self.alignment_start_distance + self.ramp_width {
            1.0
        } else {
            (distance_to_target - (self.alignment_start_distance - self.ramp_width))
                / (2.0 * self.ramp_width)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{PI, TAU};

    use linear_algebra::point;
    use proptest::proptest;

    use crate::{
        cost_fields::target_orientation::TargetOrientationField,
        geometry::{angle::Angle, Pose},
        test_utils::test_path,
    };

    proptest!(
        #[test]
        fn verify_gradient(x in -2.0f32..5.0, y in -2.0f32..5.0, orientation in 0.0..TAU) {
            let cost_field = TargetOrientationField {
                target_orientation: Angle(PI),
                path: &test_path(),
                alignment_start_distance: 1.0,
                ramp_width: 0.5,
            };

            let position = point![x, y];
            let orientation = Angle(orientation);

            let pose = Pose {
                position,
                orientation,
            };

            crate::test_utils::verify_gradient::verify_gradient(
                &|p| cost_field.cost(p),
                &|p| cost_field.grad(p),
                0.05,
                pose,
            )
        }
    );
}
