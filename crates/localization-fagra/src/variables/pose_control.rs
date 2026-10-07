use coordinate_systems::Local;
use fagra::{Jacobian, Tangent, Variable};
use linear_algebra::Pose3;
use nalgebra::{Const, DefaultAllocator, Isometry3, Matrix3, RealField};

use super::rotation::{self, RightJacobian};

/// Pose-only control for the split cubic spline, stored in the local frame.
///
/// Controls generally do not lie on the trajectory. Optimization uses right SE(3)
/// increments `[rotation xyz, translation xyz]` (radians, metres); spline evaluation
/// treats position and orientation separately. There is no independent velocity.
/// The principal rotation log is discontinuous at π; exponential Jacobians are
/// singular at nonzero multiples of 2π.
#[derive(Clone, Debug)]
pub struct PoseControl<R: RealField + Copy = f64> {
    pub pose: Pose3<Local, R>,
}

impl<R: RealField + Copy> Variable for PoseControl<R> {
    type Scalar = R;
    type Dim = Const<6>;
    type Allocator = DefaultAllocator;

    fn identity() -> Self {
        Self {
            pose: Pose3::wrap(Isometry3::identity()),
        }
    }

    fn compose(&self, other: &Self) -> Self {
        Self {
            pose: Pose3::wrap(self.pose.inner * other.pose.inner),
        }
    }

    fn inverse(&self) -> Self {
        Self {
            pose: Pose3::wrap(self.pose.inner.inverse()),
        }
    }

    fn exp(delta: &Tangent<Self>) -> Self {
        let omega = delta.fixed_rows::<3>(0).into_owned();
        let translation = delta.fixed_rows::<3>(3).into_owned();
        let [translation] = rotation::left_jacobian_actions(&omega, [translation]);
        Self {
            pose: Pose3::wrap(Isometry3::from_parts(
                translation.into(),
                rotation::exp(omega),
            )),
        }
    }

    fn log(&self) -> Tangent<Self> {
        let omega = rotation::log(&self.pose.inner.rotation);
        let translation = self.pose.inner.translation.vector;
        let [translation] = rotation::left_jacobian_inverse_actions(&omega, [translation]);
        let mut result = Tangent::<Self>::zeros();
        result.fixed_rows_mut::<3>(0).copy_from(&omega);
        result.fixed_rows_mut::<3>(3).copy_from(&translation);
        result
    }

    fn adjoint(&self) -> Jacobian<Self> {
        let rotation = self.pose.inner.rotation.to_rotation_matrix().into_inner();
        assemble(
            rotation,
            self.pose.inner.translation.vector.cross_matrix() * rotation,
        )
    }

    fn right_jacobian(delta: &Tangent<Self>) -> Jacobian<Self> {
        let blocks = RightJacobian::new(delta.fixed_rows::<3>(0).into_owned());
        assemble(
            blocks.diagonal,
            blocks.coupling(delta.fixed_rows::<3>(3).into_owned()),
        )
    }

    fn right_jacobian_inverse(delta: &Tangent<Self>) -> Jacobian<Self> {
        let blocks = RightJacobian::new(delta.fixed_rows::<3>(0).into_owned());
        let inverse = blocks.inverse_diagonal();
        assemble(
            inverse,
            -inverse * blocks.coupling(delta.fixed_rows::<3>(3).into_owned()) * inverse,
        )
    }
}

fn assemble<R: RealField + Copy>(
    rotation: Matrix3<R>,
    coupling: Matrix3<R>,
) -> Jacobian<PoseControl<R>> {
    let mut result = Jacobian::<PoseControl<R>>::zeros();
    result.fixed_view_mut::<3, 3>(0, 0).copy_from(&rotation);
    result.fixed_view_mut::<3, 3>(3, 3).copy_from(&rotation);
    result.fixed_view_mut::<3, 3>(3, 0).copy_from(&coupling);
    result
}
