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
            orientation: angle_penalty_derivative(pose.orientation, self.target_orientation),
            ..PoseGradient::zeros()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;

    use linear_algebra::point;
    use proptest::{prop_assume, proptest};

    use crate::{
        cost_fields::target_orientation::TargetOrientationField,
        geometry::{angle::Angle, pose::Pose},
        test_utils::is_roughly_opposite,
    };

    proptest!(
        #[test]
        fn verify_gradient(x in -5.0f32..5.0, y in -5.0f32..5.0, orientation in 0.0..TAU, target_orientation in 0.0..TAU) {
            prop_assume!(!is_roughly_opposite(orientation, target_orientation));
            verify_gradient_impl(x, y, orientation, target_orientation)
        }
    );

    fn verify_gradient_impl(x: f32, y: f32, orientation: f32, target_orientation: f32) {
        let cost_field = TargetOrientationField {
            target_orientation: Angle(target_orientation),
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
}
