//! Right-local covariance propagation for evaluated spline poses.

use fagra::Variable;
use nalgebra::{RealField, SMatrix};

use crate::{
    spline::LinearizedPose,
    variables::{PoseControl, rotation::scalar},
};

impl<R: RealField + Copy> LinearizedPose<R> {
    /// Propagate joint covariance of the four controls, in their spline order.
    /// Includes cross-correlations; the output tangent is [rotation xyz, translation xyz].
    pub fn covariance(&self, controls: &SMatrix<R, 24, 24>) -> SMatrix<R, 6, 6> {
        let j = self.control_jacobian();
        symmetric(j * controls * j.transpose())
    }

    /// Covariance of `local_to_field * self.pose`, in the output's right tangent.
    /// Joint coordinates are four 6D controls followed by the 3D field alignment
    /// [yaw, translation x, translation y]. No control/alignment correlations are dropped.
    /// The mean alignment does not appear: right perturbations map through Ad(pose^-1).
    pub fn field_covariance(&self, joint: &SMatrix<R, 27, 27>) -> SMatrix<R, 6, 6> {
        let mut embed = SMatrix::<R, 6, 3>::zeros();
        embed[(2, 0)] = R::one();
        embed[(3, 1)] = R::one();
        embed[(4, 2)] = R::one();
        let pose = PoseControl { pose: self.pose };
        let mut j = SMatrix::<R, 6, 27>::zeros();
        j.fixed_view_mut::<6, 24>(0, 0)
            .copy_from(&self.control_jacobian());
        j.fixed_view_mut::<6, 3>(0, 24)
            .copy_from(&(pose.inverse().adjoint() * embed));
        symmetric(j * joint * j.transpose())
    }

    fn control_jacobian(&self) -> SMatrix<R, 6, 24> {
        let mut j = SMatrix::<R, 6, 24>::zeros();
        for (index, block) in self.jacobians.iter().enumerate() {
            j.fixed_view_mut::<6, 6>(0, index * 6).copy_from(block);
        }
        j
    }
}

fn symmetric<R: RealField + Copy>(covariance: SMatrix<R, 6, 6>) -> SMatrix<R, 6, 6> {
    (covariance + covariance.transpose()) * scalar::<R>(0.5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{spline::PoseSpline, variables::FieldAlignment};
    use fagra::Tangent;
    use linear_algebra::{Framed, IntoTransform};

    #[test]
    fn joint_covariance_matches_finite_differences_of_composed_pose() {
        let controls = std::array::from_fn::<_, 4, _>(|i| PoseControl {
            pose: Framed::wrap(nalgebra::Isometry3::new(
                nalgebra::Vector3::new(i as f64 * 0.1, 0.2, 0.5),
                nalgebra::Vector3::new(0.1, -0.05, i as f64 * 0.07),
            )),
        });
        let alignment = FieldAlignment {
            local_to_field: nalgebra::Isometry2::new(nalgebra::Vector2::new(2.0, -1.0), 0.3)
                .framed_transform(),
        };
        let evaluate = |controls: &[PoseControl; 4], alignment: &FieldAlignment| {
            let pose = PoseSpline::new(controls.each_ref(), 0.2)
                .unwrap()
                .pose(0.4)
                .unwrap();
            PoseControl {
                pose: Framed::wrap((alignment.local_to_field.to_3d() * pose).inner),
            }
        };
        let reference = evaluate(&controls, &alignment);
        let mut numerical = SMatrix::<f64, 6, 27>::zeros();
        let epsilon = 1.0e-6;
        for column in 0..27 {
            let perturb = |sign| {
                let mut controls = controls.clone();
                let mut alignment = alignment.clone();
                if column < 24 {
                    let mut delta = Tangent::<PoseControl>::zeros();
                    delta[column % 6] = sign * epsilon;
                    controls[column / 6] = controls[column / 6].retract(&delta);
                } else {
                    let mut delta = Tangent::<FieldAlignment>::zeros();
                    delta[column - 24] = sign * epsilon;
                    alignment = alignment.retract(&delta);
                }
                reference.local(&evaluate(&controls, &alignment))
            };
            numerical
                .column_mut(column)
                .copy_from(&((perturb(1.0) - perturb(-1.0)) / (2.0 * epsilon)));
        }
        let mut root = SMatrix::<f64, 27, 27>::identity();
        root[(0, 24)] = 0.3;
        root[(8, 26)] = -0.2;
        root[(4, 9)] = 0.4;
        let joint = root * root.transpose();
        let spline = PoseSpline::new(controls.each_ref(), 0.2).unwrap();
        let sample = spline.linearize().unwrap().pose(0.4).unwrap();
        assert!(
            (sample.field_covariance(&joint) - numerical * joint * numerical.transpose()).amax()
                < 1.0e-7
        );
        let controls_covariance = joint.fixed_view::<24, 24>(0, 0).into_owned();
        let j = numerical.fixed_columns::<24>(0);
        assert!(
            (sample.covariance(&controls_covariance) - j * controls_covariance * j.transpose())
                .amax()
                < 1.0e-7
        );
    }
}
