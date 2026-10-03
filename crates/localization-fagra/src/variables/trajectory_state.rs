use coordinate_systems::Local;
use fagra::{Jacobian, Tangent, Variable};
use linear_algebra::{Pose3, Vector3};
use nalgebra::{Const, DefaultAllocator, Isometry3, Matrix3, RealField};

use super::rotation::{self, RightJacobian};

/// An evaluated SE₂(3) navigation state: robot pose and local linear velocity.
///
/// The spline optimizes [`super::PoseControl`] values and derives velocity from
/// position. This type represents its output and prior references, not independent
/// pose/velocity controls. Its Lie operations define navigation-state errors.
///
/// Right-increment coordinates are `[rotation xyz, velocity xyz, position xyz]`,
/// in radians, m/s, and metres, respectively. Increments are robot-frame quantities;
/// stored velocity and position are local-frame quantities. Rotation couples to
/// both velocity and position in the group exponential.
/// The rotation logarithm uses the principal branch, discontinuous at pi.
/// Exponential Jacobians are singular at nonzero multiples of 2π rotation.
#[derive(Clone, Debug)]
pub struct TrajectoryState<R: RealField + Copy = f64> {
    pub pose: Pose3<Local, R>,
    /// Linear velocity in m/s.
    pub velocity: Vector3<Local, R>,
}

impl<R: RealField + Copy> Variable for TrajectoryState<R> {
    type Scalar = R;
    type Dim = Const<9>;
    type Allocator = DefaultAllocator;

    fn identity() -> Self {
        Self {
            pose: Pose3::wrap(Isometry3::identity()),
            velocity: Vector3::wrap(nalgebra::Vector3::zeros()),
        }
    }

    fn compose(&self, other: &Self) -> Self {
        Self {
            pose: Pose3::wrap(self.pose.inner * other.pose.inner),
            velocity: Vector3::wrap(
                self.velocity.inner + self.pose.inner.rotation * other.velocity.inner,
            ),
        }
    }

    fn inverse(&self) -> Self {
        let pose = self.pose.inner.inverse();
        Self {
            velocity: Vector3::wrap(-(pose.rotation * self.velocity.inner)),
            pose: Pose3::wrap(pose),
        }
    }

    fn exp(delta: &Tangent<Self>) -> Self {
        let omega = delta.fixed_rows::<3>(0).into_owned();
        let [velocity, position] = rotation::left_jacobian_actions(
            &omega,
            [
                delta.fixed_rows::<3>(3).into_owned(),
                delta.fixed_rows::<3>(6).into_owned(),
            ],
        );
        Self {
            pose: Pose3::wrap(Isometry3::from_parts(position.into(), rotation::exp(omega))),
            velocity: Vector3::wrap(velocity),
        }
    }

    fn log(&self) -> Tangent<Self> {
        let omega = rotation::log(&self.pose.inner.rotation);
        let [velocity, position] = rotation::left_jacobian_inverse_actions(
            &omega,
            [self.velocity.inner, self.pose.inner.translation.vector],
        );
        let mut delta = Tangent::<Self>::zeros();
        delta.fixed_rows_mut::<3>(0).copy_from(&omega);
        delta.fixed_rows_mut::<3>(3).copy_from(&velocity);
        delta.fixed_rows_mut::<3>(6).copy_from(&position);
        delta
    }

    fn adjoint(&self) -> Jacobian<Self> {
        let rotation = self.pose.inner.rotation.to_rotation_matrix().into_inner();
        assemble(
            rotation,
            self.velocity.inner.cross_matrix() * rotation,
            self.pose.inner.translation.vector.cross_matrix() * rotation,
        )
    }

    fn right_jacobian(delta: &Tangent<Self>) -> Jacobian<Self> {
        let blocks = RightJacobian::new(delta.fixed_rows::<3>(0).into_owned());
        assemble(
            blocks.diagonal,
            blocks.coupling(delta.fixed_rows::<3>(3).into_owned()),
            blocks.coupling(delta.fixed_rows::<3>(6).into_owned()),
        )
    }

    fn right_jacobian_inverse(delta: &Tangent<Self>) -> Jacobian<Self> {
        let blocks = RightJacobian::new(delta.fixed_rows::<3>(0).into_owned());
        let inverse = blocks.inverse_diagonal();
        assemble(
            inverse,
            -inverse * blocks.coupling(delta.fixed_rows::<3>(3).into_owned()) * inverse,
            -inverse * blocks.coupling(delta.fixed_rows::<3>(6).into_owned()) * inverse,
        )
    }
}

fn assemble<R: RealField + Copy>(
    diagonal: Matrix3<R>,
    velocity: Matrix3<R>,
    position: Matrix3<R>,
) -> Jacobian<TrajectoryState<R>> {
    let mut result = Jacobian::<TrajectoryState<R>>::zeros();
    result.fixed_view_mut::<3, 3>(0, 0).copy_from(&diagonal);
    result.fixed_view_mut::<3, 3>(3, 3).copy_from(&diagonal);
    result.fixed_view_mut::<3, 3>(6, 6).copy_from(&diagonal);
    result.fixed_view_mut::<3, 3>(3, 0).copy_from(&velocity);
    result.fixed_view_mut::<3, 3>(6, 0).copy_from(&position);
    result
}
