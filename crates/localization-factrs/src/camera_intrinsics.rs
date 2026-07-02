use std::fmt::Display;

use factrs::{
    linalg::{Numeric, SupersetOf},
    traits::Variable,
    variables::VectorVar4,
};
use nalgebra::{Const, Vector2, VectorView3, vector};

#[derive(Clone, Debug)]
pub struct CameraIntrinsics<T: Numeric = f64> {
    storage: VectorVar4<T>,
}

impl<T: Numeric> CameraIntrinsics<T> {
    pub fn new(focals: Vector2<T>, optical_center: Vector2<T>) -> Self {
        let storage = VectorVar4::new(focals.x, focals.y, optical_center.x, optical_center.y);
        Self { storage }
    }

    pub fn focals(&self) -> Vector2<T> {
        self.storage.0.xy()
    }

    pub fn optical_center(&self) -> Vector2<T> {
        vector![self.storage.0.z, self.storage.0.w]
    }

    pub fn project<S: Numeric + SupersetOf<T>>(&self, point_camera: VectorView3<S>) -> Vector2<S> {
        let focals = self.focals().cast::<S>();
        let optical_center = self.optical_center().cast::<S>();

        let z_inv = S::one() / point_camera.z;
        focals.component_mul(&point_camera.xy()).scale(z_inv) + optical_center
    }

    pub fn project_checked<S: Numeric + SupersetOf<T>>(
        &self,
        point_camera: VectorView3<S>,
        min_depth: S,
    ) -> Option<Vector2<S>> {
        if point_camera.z <= min_depth {
            return None;
        }

        Some(self.project(point_camera))
    }
}

impl<T: Numeric> Display for CameraIntrinsics<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let precision = f.precision().unwrap_or(3);
        let focals = self.focals();
        let optical_center = self.optical_center();
        write!(
            f,
            "CameraIntrinsics(focals: [{:.precision$}, {:.precision$}], center: [{:.precision$}, {:.precision$}])",
            focals.x,
            focals.y,
            optical_center.x,
            optical_center.y,
            precision = precision
        )
    }
}

#[factrs::mark]
impl<T: Numeric> Variable for CameraIntrinsics<T> {
    type T = T;
    type Dim = Const<4>; // The 4 parameters: fx, fy, cx, cy
    type Alias<TT: Numeric> = CameraIntrinsics<TT>;

    fn identity() -> Self {
        Self {
            storage: VectorVar4::identity(),
        }
    }

    fn inverse(&self) -> Self {
        Self {
            storage: self.storage.inverse(),
        }
    }

    fn compose(&self, other_intrinsics: &Self) -> Self {
        Self {
            storage: self.storage.compose(&other_intrinsics.storage),
        }
    }

    fn exp(delta: factrs::linalg::VectorViewX<Self::T>) -> Self {
        Self {
            storage: VectorVar4::exp(delta),
        }
    }

    fn log(&self) -> factrs::linalg::VectorX<Self::T> {
        self.storage.log()
    }

    fn cast<TT: Numeric + SupersetOf<Self::T>>(&self) -> Self::Alias<TT> {
        CameraIntrinsics {
            storage: self.storage.cast(),
        }
    }
}
