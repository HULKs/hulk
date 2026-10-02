//! Shared rotation coefficients. Series in squared angle also work at dual-number zero.

use nalgebra::{Matrix3, Quaternion, RealField, UnitQuaternion, Vector3};

#[inline]
pub(crate) fn scalar<R: RealField>(value: f64) -> R {
    R::from_f64(value).expect("geometry constant is representable")
}

/// B = (1 - cos θ)/θ² and C = (θ - sin θ)/θ³.
/// Adapted from fagra's SLAM geometry; the wide series region protects f32 and
/// radial derivatives against cancellation. The omitted B term at θ = 0.5 is < 3e-15.
pub(super) fn coefficients<R: RealField + Copy>(t2: R) -> (R, R) {
    let c = scalar::<R>;
    if t2 < c(0.25) {
        (
            c(0.5)
                + t2 * (c(-1.0 / 24.0)
                    + t2 * (c(1.0 / 720.0)
                        + t2 * (c(-1.0 / 40320.0)
                            + t2 * (c(1.0 / 3628800.0) - t2 * c(1.0 / 479001600.0))))),
            c(1.0 / 6.0)
                + t2 * (c(-1.0 / 120.0)
                    + t2 * (c(1.0 / 5040.0)
                        + t2 * (c(-1.0 / 362880.0)
                            + t2 * (c(1.0 / 39916800.0) - t2 * c(1.0 / 6227020800.0))))),
        )
    } else {
        let angle = t2.sqrt();
        let half_sinc = (angle * c(0.5)).sin() / angle;
        (
            c(2.0) * half_sinc * half_sinc,
            (R::one() - angle.sin() / angle) / t2,
        )
    }
}

/// D in Jl⁻¹ = I - Ω/2 + D Ω². Singular at nonzero multiples of 2π.
pub(super) fn inverse_coefficient<R: RealField + Copy>(t2: R) -> R {
    let c = scalar::<R>;
    if t2 < c(0.25) {
        c(1.0 / 12.0)
            + t2 * (c(1.0 / 720.0)
                + t2 * (c(1.0 / 30240.0)
                    + t2 * (c(1.0 / 1209600.0)
                        + t2 * (c(1.0 / 47900160.0) + t2 * c(691.0 / 1307674368000.0)))))
    } else {
        let half = t2.sqrt() * c(0.5);
        (R::one() - half / half.tan()) / t2
    }
}

pub(crate) fn exp<R: RealField + Copy>(omega: Vector3<R>) -> UnitQuaternion<R> {
    let c = scalar::<R>;
    let t2 = omega.norm_squared();
    let (w, scale) = if t2 < c(0.01) {
        (
            R::one() + t2 * (c(-1.0 / 8.0) + t2 * (c(1.0 / 384.0) - t2 * c(1.0 / 46080.0))),
            c(0.5) + t2 * (c(-1.0 / 48.0) + t2 * (c(1.0 / 3840.0) - t2 * c(1.0 / 645120.0))),
        )
    } else {
        let angle = t2.sqrt();
        let (sin, cos) = (angle * c(0.5)).sin_cos();
        (cos, sin / angle)
    };
    UnitQuaternion::new_normalize(Quaternion::from_parts(w, omega * scale))
}

pub(crate) fn log<R: RealField + Copy>(rotation: &UnitQuaternion<R>) -> Vector3<R> {
    let c = scalar::<R>;
    let q = rotation.quaternion();
    let q = if q.w < R::zero() { -*q } else { *q };
    let vector = q.imag();
    let s2 = vector.norm_squared();
    let scale = if s2 < c(1e-4) {
        c(2.0) + s2 * (c(1.0 / 3.0) + s2 * (c(3.0 / 20.0) + s2 * c(5.0 / 56.0)))
    } else {
        let s = s2.sqrt();
        c(2.0) * s.atan2(q.w) / s
    };
    vector * scale
}

pub(super) fn left_jacobian_actions<R: RealField + Copy, const N: usize>(
    omega: &Vector3<R>,
    vectors: [Vector3<R>; N],
) -> [Vector3<R>; N] {
    let (b, c) = coefficients(omega.norm_squared());
    vectors.map(|vector| {
        let cross = omega.cross(&vector);
        vector + cross * b + omega.cross(&cross) * c
    })
}

pub(super) fn left_jacobian_inverse_actions<R: RealField + Copy, const N: usize>(
    omega: &Vector3<R>,
    vectors: [Vector3<R>; N],
) -> [Vector3<R>; N] {
    let d = inverse_coefficient(omega.norm_squared());
    vectors.map(|vector| {
        let cross = omega.cross(&vector);
        vector - cross * scalar::<R>(0.5) + omega.cross(&cross) * d
    })
}

pub(crate) fn right_jacobian<R: RealField + Copy>(omega: Vector3<R>) -> Matrix3<R> {
    let (b, c) = coefficients(omega.norm_squared());
    let w = omega.cross_matrix();
    Matrix3::identity() - w * b + w * w * c
}

pub(crate) fn right_jacobian_inverse<R: RealField + Copy>(omega: Vector3<R>) -> Matrix3<R> {
    let w = omega.cross_matrix();
    Matrix3::<R>::identity()
        + w * scalar::<R>(0.5)
        + w * w * inverse_coefficient(omega.norm_squared())
}

/// Rotation-dependent work shared by both SE₂(3) vector blocks.
pub(super) struct RightJacobian<R: RealField + Copy> {
    pub diagonal: Matrix3<R>,
    omega: Vector3<R>,
    rotation_inverse: Matrix3<R>,
    b: R,
    c: R,
    db: R,
    dc: R,
}

impl<R: RealField + Copy> RightJacobian<R> {
    pub(super) fn new(omega: Vector3<R>) -> Self {
        let k = scalar::<R>;
        let t2 = omega.norm_squared();
        let (b, c) = coefficients(t2);
        // B'(θ)/θ and C'(θ)/θ, differentiated in squared-angle coordinates.
        let (db, dc) = if t2 < k(0.25) {
            (
                k(-1.0 / 12.0)
                    + t2 * (k(1.0 / 180.0)
                        + t2 * (k(-1.0 / 6720.0)
                            + t2 * (k(1.0 / 453600.0) - t2 * k(1.0 / 47900160.0)))),
                k(-1.0 / 60.0)
                    + t2 * (k(1.0 / 1260.0)
                        + t2 * (k(-1.0 / 60480.0)
                            + t2 * (k(1.0 / 4989600.0) - t2 * k(1.0 / 622702080.0)))),
            )
        } else {
            ((R::one() - t2 * c - k(2.0) * b) / t2, (b - k(3.0) * c) / t2)
        };
        let w = omega.cross_matrix();
        let w2 = w * w;
        Self {
            diagonal: Matrix3::identity() - w * b + w2 * c,
            rotation_inverse: Matrix3::identity() - w * (R::one() - t2 * c) + w2 * b,
            omega,
            b,
            c,
            db,
            dc,
        }
    }

    /// Rᵀ d(Jl(ω) v)/dω; placed below the rotational diagonal block.
    pub(super) fn coupling(&self, vector: Vector3<R>) -> Matrix3<R> {
        let w = self.omega;
        let cross = w.cross(&vector);
        let derivative = -vector.cross_matrix() * self.b
            + (w * vector.transpose() + Matrix3::identity() * w.dot(&vector)
                - vector * w.transpose() * scalar::<R>(2.0))
                * self.c
            + (cross * self.db + w.cross(&cross) * self.dc) * w.transpose();
        self.rotation_inverse * derivative
    }

    pub(super) fn inverse_diagonal(&self) -> Matrix3<R> {
        right_jacobian_inverse(self.omega)
    }
}
