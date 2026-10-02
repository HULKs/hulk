use coordinate_systems::Pixel;
use fagra::{Jacobian, Tangent, Variable};
use linear_algebra::{Point2, Vector2};
use nalgebra::{Const, DefaultAllocator, RealField};

/// Pinhole calibration as the additive group R⁴.
///
/// Tangent coordinates are `[fx, fy, cx, cy]`, all in pixels.
/// Focal lengths must be positive for a physical camera, although the additive
/// group's identity is zero. Projection factors must validate their domain.
#[derive(Clone, Debug)]
pub struct CameraIntrinsics<R: RealField + Copy = f64> {
    pub focal_lengths: Vector2<Pixel, R>,
    pub optical_center: Point2<Pixel, R>,
}

impl<R: RealField + Copy> Variable for CameraIntrinsics<R> {
    type Scalar = R;
    type Dim = Const<4>;
    type Allocator = DefaultAllocator;

    fn identity() -> Self {
        Self {
            focal_lengths: Vector2::wrap(nalgebra::Vector2::zeros()),
            optical_center: Point2::wrap(nalgebra::Point2::origin()),
        }
    }

    fn compose(&self, other: &Self) -> Self {
        Self {
            focal_lengths: self.focal_lengths + other.focal_lengths,
            optical_center: Point2::wrap(
                self.optical_center.inner + other.optical_center.inner.coords,
            ),
        }
    }

    fn inverse(&self) -> Self {
        Self {
            focal_lengths: -self.focal_lengths,
            optical_center: Point2::wrap((-self.optical_center.inner.coords).into()),
        }
    }

    fn exp(delta: &Tangent<Self>) -> Self {
        Self {
            focal_lengths: Vector2::wrap(delta.fixed_rows::<2>(0).into_owned()),
            optical_center: Point2::wrap(delta.fixed_rows::<2>(2).into_owned().into()),
        }
    }

    fn log(&self) -> Tangent<Self> {
        nalgebra::Vector4::new(
            self.focal_lengths.inner.x,
            self.focal_lengths.inner.y,
            self.optical_center.inner.x,
            self.optical_center.inner.y,
        )
    }

    fn adjoint(&self) -> Jacobian<Self> {
        Jacobian::<Self>::identity()
    }

    fn right_jacobian(_delta: &Tangent<Self>) -> Jacobian<Self> {
        Jacobian::<Self>::identity()
    }

    fn right_jacobian_inverse(_delta: &Tangent<Self>) -> Jacobian<Self> {
        Jacobian::<Self>::identity()
    }
}
