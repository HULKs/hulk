use coordinate_systems::{Field, Local};
use fagra::{Jacobian, Tangent, Variable};
use linear_algebra::Isometry2;
use nalgebra::{Const, DefaultAllocator, Matrix2, RealField, Vector2};

use super::rotation::{coefficients, inverse_coefficient, scalar};

/// Planar SE(2) alignment from local odometry coordinates to field coordinates.
///
/// Right-increment coordinates are `[yaw, translation x, translation y]`,
/// in radians and metres, expressed in the local frame.
/// The rotation logarithm uses the principal branch, discontinuous at pi.
/// Exponential Jacobians are singular at nonzero multiples of 2π rotation.
#[derive(Clone, Debug)]
pub struct FieldAlignment<R: RealField + Copy = f64> {
    pub local_to_field: Isometry2<Local, Field, R>,
}

impl<R: RealField + Copy> Variable for FieldAlignment<R> {
    type Scalar = R;
    type Dim = Const<3>;
    type Allocator = DefaultAllocator;

    fn identity() -> Self {
        Self {
            local_to_field: Isometry2::wrap(nalgebra::Isometry2::identity()),
        }
    }

    fn compose(&self, other: &Self) -> Self {
        Self {
            local_to_field: Isometry2::wrap(self.local_to_field.inner * other.local_to_field.inner),
        }
    }

    fn inverse(&self) -> Self {
        Self {
            local_to_field: Isometry2::wrap(self.local_to_field.inner.inverse()),
        }
    }

    fn exp(delta: &Tangent<Self>) -> Self {
        let theta = delta[0];
        let (b, c) = coefficients(theta * theta);
        let v = delta.fixed_rows::<2>(1).into_owned();
        let translation = v * (R::one() - theta * theta * c) + perpendicular(v) * (theta * b);
        Self {
            local_to_field: Isometry2::wrap(nalgebra::Isometry2::new(translation, theta)),
        }
    }

    fn log(&self) -> Tangent<Self> {
        let theta = self.local_to_field.inner.rotation.angle();
        let v = self.local_to_field.inner.translation.vector;
        let a = R::one() - theta * theta * inverse_coefficient(theta * theta);
        let translation = v * a - perpendicular(v) * (theta * scalar(0.5));
        nalgebra::Vector3::new(theta, translation.x, translation.y)
    }

    fn adjoint(&self) -> Jacobian<Self> {
        assemble(
            self.local_to_field
                .inner
                .rotation
                .to_rotation_matrix()
                .into_inner(),
            -perpendicular(self.local_to_field.inner.translation.vector),
        )
    }

    fn right_jacobian(delta: &Tangent<Self>) -> Jacobian<Self> {
        let theta = delta[0];
        let (b, c) = coefficients(theta * theta);
        let a = R::one() - theta * theta * c;
        let s = theta * b;
        let v = delta.fixed_rows::<2>(1).into_owned();
        assemble(
            Matrix2::new(a, s, -s, a),
            v * (theta * c) + perpendicular(v) * b,
        )
    }

    fn right_jacobian_inverse(delta: &Tangent<Self>) -> Jacobian<Self> {
        let theta = delta[0];
        let d = inverse_coefficient(theta * theta);
        let a = R::one() - theta * theta * d;
        let half = theta * scalar(0.5);
        let v = delta.fixed_rows::<2>(1).into_owned();
        assemble(
            Matrix2::new(a, -half, half, a),
            v * (theta * d) - perpendicular(v) * scalar::<R>(0.5),
        )
    }
}

fn perpendicular<R: RealField + Copy>(v: Vector2<R>) -> Vector2<R> {
    Vector2::new(-v.y, v.x)
}

fn assemble<R: RealField + Copy>(
    linear: Matrix2<R>,
    angular: Vector2<R>,
) -> Jacobian<FieldAlignment<R>> {
    let mut result = Jacobian::<FieldAlignment<R>>::zeros();
    result[(0, 0)] = R::one();
    result.fixed_view_mut::<2, 2>(1, 1).copy_from(&linear);
    result.fixed_view_mut::<2, 1>(1, 0).copy_from(&angular);
    result
}
