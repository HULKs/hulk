use factrs::{core::SO3, linalg::Numeric, variables::SE23};
use nalgebra::SVector;

use crate::splines::{cubic_hermite_spline::CubicHermiteSpline, geodesic::GeodesicSpline};

pub struct SE23Spline<T: Numeric> {
    geodesic: GeodesicSpline<SO3<T>>,
    position: CubicHermiteSpline<T, 3>,
    dt: T,
}

pub struct SE23Kinematics<T: Numeric> {
    pub angular_velocity_local: SVector<T, 3>,
    pub linear_acceleration_global: SVector<T, 3>,
}

impl<T: Numeric> SE23Spline<T> {
    pub fn new(start: SE23<T>, end: SE23<T>, dt: T) -> Self {
        Self {
            geodesic: GeodesicSpline::new(start.rot().clone(), end.rot()),
            position: CubicHermiteSpline::new(
                start.xyz().into_owned(),
                end.xyz().into_owned(),
                start.uvw() * dt,
                end.uvw() * dt,
            ),
            dt,
        }
    }

    pub fn evaluate(&self, tau: T) -> SE23<T> {
        let rotation = self.geodesic.evaluate(tau);
        let position = self.position.evaluate(tau);
        let velocity = self.position.evaluate_derivative(tau) / self.dt;
        SE23::from_rot_vel_trans(rotation, velocity, position)
    }

    pub fn evaluate_derivative(&self, tau: T) -> SE23Kinematics<T> {
        let angular_velocity_local = self.geodesic.evaluate_time_derivative(self.dt);
        let angular_velocity_local = SVector::from_column_slice(angular_velocity_local.as_slice());

        let linear_acceleration_global =
            self.position.evaluate_second_derivative(tau) / self.dt.powi(2);

        SE23Kinematics {
            angular_velocity_local,
            linear_acceleration_global,
        }
    }
}
