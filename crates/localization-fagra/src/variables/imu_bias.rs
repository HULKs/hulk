use coordinate_systems::Robot;
use fagra::{Jacobian, Tangent, Variable};
use linear_algebra::Vector3;
use nalgebra::{Const, DefaultAllocator, RealField};

/// Residual sensor offsets after fixed calibration, in Robot axes.
/// Additive tangent order: gyroscope (rad/s), accelerometer (m/s²).
#[derive(Clone, Debug)]
pub struct ImuBias<R: RealField + Copy = f64> {
    pub gyroscope: Vector3<Robot, R>,
    pub accelerometer: Vector3<Robot, R>,
}

impl<R: RealField + Copy> Variable for ImuBias<R> {
    type Scalar = R;
    type Dim = Const<6>;
    type Allocator = DefaultAllocator;

    fn identity() -> Self {
        Self {
            gyroscope: Vector3::zeros(),
            accelerometer: Vector3::zeros(),
        }
    }
    fn compose(&self, other: &Self) -> Self {
        Self {
            gyroscope: self.gyroscope + other.gyroscope,
            accelerometer: self.accelerometer + other.accelerometer,
        }
    }
    fn inverse(&self) -> Self {
        Self {
            gyroscope: -self.gyroscope,
            accelerometer: -self.accelerometer,
        }
    }
    fn exp(delta: &Tangent<Self>) -> Self {
        Self {
            gyroscope: Vector3::wrap(delta.fixed_rows::<3>(0).into_owned()),
            accelerometer: Vector3::wrap(delta.fixed_rows::<3>(3).into_owned()),
        }
    }
    fn log(&self) -> Tangent<Self> {
        let mut value = Tangent::<Self>::zeros();
        value
            .fixed_rows_mut::<3>(0)
            .copy_from(&self.gyroscope.inner);
        value
            .fixed_rows_mut::<3>(3)
            .copy_from(&self.accelerometer.inner);
        value
    }
    fn adjoint(&self) -> Jacobian<Self> {
        Jacobian::<Self>::identity()
    }
    fn right_jacobian(_: &Tangent<Self>) -> Jacobian<Self> {
        Jacobian::<Self>::identity()
    }
    fn right_jacobian_inverse(_: &Tangent<Self>) -> Jacobian<Self> {
        Jacobian::<Self>::identity()
    }
}
