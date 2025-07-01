use linear_algebra::Vector2;

use crate::{
    geometry::{angle::Angle, pose::Pose, pose::PoseGradient},
    utils::{angle_penalty, angle_penalty_derivative},
};

pub struct TargetOrientationField {
    pub target_orientation: Angle<f32>,
}

impl TargetOrientationField {
    pub fn cost(&self, pose: Pose<f32>) -> f32 {
        angle_penalty(pose.orientation, self.target_orientation)
    }

    pub fn grad(&self, pose: Pose<f32>) -> PoseGradient<f32> {
        PoseGradient {
            position: Vector2::zeros(),
            orientation: angle_penalty_derivative(pose.orientation, self.target_orientation),
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
        geometry::{angle::Angle, pose::Pose},
    };

    proptest!(
        #[test]
        fn verify_gradient(x in -2.0f32..5.0, y in -2.0f32..5.0, orientation in 0.0..TAU) {
            let cost_field = TargetOrientationField {
                target_orientation: Angle(PI),
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
