use std::time::SystemTime;

use factrs::{
    core::{SE3, SO3},
    linalg::{ForwardProp, Numeric, VectorX},
    traits::{Residual, Variable},
    variables::SE23,
};
use nalgebra::{Matrix3, SVector};

use crate::{SE23Spline, utils::tau};

#[derive(Debug, Clone)]
pub struct OdometerMeasurement {
    pub previous_time: SystemTime,
    pub current_time: SystemTime,
    /// Planar transformation from the current robot frame to the previous robot frame.
    pub robot_delta: SE3,
}

#[derive(Debug, Clone)]
pub struct OdometerFactor {
    measurements: Vec<OdometerDelta>,
    information_root: Matrix3<f64>,
    duration: f64,
}

#[derive(Debug, Clone)]
pub struct AdjacentOdometerFactor {
    measurements: Vec<OdometerDelta>,
    information_root: Matrix3<f64>,
    duration: f64,
}

#[derive(Debug, Clone)]
pub struct OdometerDelta {
    start_tau: f64,
    end_tau: f64,
    robot_delta: SE3,
}

#[factrs::mark]
impl Residual for OdometerFactor {
    type Input = (SE23, SE23);
    type Differ = ForwardProp;

    fn dim_out(&self) -> usize {
        self.measurements.len() * 3
    }

    fn residual<T: Numeric>(&self, (start, end): (SE23<T>, SE23<T>)) -> VectorX<T> {
        self.residuals_on_spline(start, end)
    }
}

#[factrs::mark]
impl Residual for AdjacentOdometerFactor {
    type Input = (SE23, SE23, SE23);
    type Differ = ForwardProp;

    fn dim_out(&self) -> usize {
        self.measurements.len() * 3
    }

    fn residual<T: Numeric>(
        &self,
        (previous_start, middle, current_end): (SE23<T>, SE23<T>, SE23<T>),
    ) -> VectorX<T> {
        self.residuals_on_splines(previous_start, middle, current_end)
    }
}

impl OdometerFactor {
    pub fn new(
        measurements: Vec<OdometerDelta>,
        odometer_noise: Matrix3<f64>,
        duration: f64,
    ) -> Self {
        Self {
            measurements,
            information_root: information_root(odometer_noise),
            duration,
        }
    }

    fn residuals_on_spline<T: Numeric>(&self, start: SE23<T>, end: SE23<T>) -> VectorX<T> {
        let mut residuals = VectorX::<T>::zeros(self.dim_out());
        if self.measurements.is_empty() {
            return residuals;
        }

        let spline = SE23Spline::new(start, end, T::from(self.duration));
        let information_root = self.information_root.cast::<T>();

        for (index, measurement) in self.measurements.iter().enumerate() {
            let previous_pose = se23_pose_to_se3(spline.evaluate(T::from(measurement.start_tau)));
            let current_pose = se23_pose_to_se3(spline.evaluate(T::from(measurement.end_tau)));
            let whitened_error = planar_delta_residual(
                previous_pose,
                current_pose,
                &measurement.robot_delta,
                &information_root,
            );

            residuals
                .fixed_view_mut::<3, 1>(index * 3, 0)
                .copy_from(&whitened_error);
        }

        residuals
    }
}

impl AdjacentOdometerFactor {
    pub fn new(
        measurements: Vec<OdometerDelta>,
        odometer_noise: Matrix3<f64>,
        duration: f64,
    ) -> Self {
        Self {
            measurements,
            information_root: information_root(odometer_noise),
            duration,
        }
    }

    fn residuals_on_splines<T: Numeric>(
        &self,
        previous_start: SE23<T>,
        middle: SE23<T>,
        current_end: SE23<T>,
    ) -> VectorX<T> {
        let mut residuals = VectorX::<T>::zeros(self.dim_out());
        if self.measurements.is_empty() {
            return residuals;
        }

        let previous_spline =
            SE23Spline::new(previous_start, middle.clone(), T::from(self.duration));
        let current_spline = SE23Spline::new(middle, current_end, T::from(self.duration));
        let information_root = self.information_root.cast::<T>();

        for (index, measurement) in self.measurements.iter().enumerate() {
            let previous_pose =
                se23_pose_to_se3(previous_spline.evaluate(T::from(measurement.start_tau)));
            let current_pose =
                se23_pose_to_se3(current_spline.evaluate(T::from(measurement.end_tau)));
            let whitened_error = planar_delta_residual(
                previous_pose,
                current_pose,
                &measurement.robot_delta,
                &information_root,
            );

            residuals
                .fixed_view_mut::<3, 1>(index * 3, 0)
                .copy_from(&whitened_error);
        }

        residuals
    }
}

impl OdometerDelta {
    pub fn new(start_tau: f64, end_tau: f64, robot_delta: SE3) -> Self {
        Self {
            start_tau,
            end_tau,
            robot_delta,
        }
    }

    pub fn from_measurement(
        measurement: OdometerMeasurement,
        start_time: SystemTime,
        end_time: SystemTime,
    ) -> Self {
        Self {
            start_tau: tau::<f64>(start_time, end_time, measurement.previous_time),
            end_tau: tau::<f64>(start_time, end_time, measurement.current_time),
            robot_delta: measurement.robot_delta,
        }
    }
}

fn information_root(odometer_noise: Matrix3<f64>) -> Matrix3<f64> {
    odometer_noise
        .cholesky()
        .expect("odometer covariance must be positive definite")
        .l()
        .try_inverse()
        .expect("odometer covariance Cholesky factor must be invertible")
}

fn planar_delta_residual<T: Numeric>(
    previous_pose: SE3<T>,
    current_pose: SE3<T>,
    measured_delta: &SE3,
    information_root: &Matrix3<T>,
) -> SVector<T, 3> {
    let measured_delta = measured_delta.cast::<T>();
    let previous_yaw = heading_yaw(previous_pose.rot());
    let current_yaw = heading_yaw(current_pose.rot());
    let dx = current_pose.xyz().x - previous_pose.xyz().x;
    let dy = current_pose.xyz().y - previous_pose.xyz().y;
    let previous_yaw_cos = previous_yaw.cos();
    let previous_yaw_sin = previous_yaw.sin();
    let predicted_x = previous_yaw_cos * dx + previous_yaw_sin * dy;
    let predicted_y = -previous_yaw_sin * dx + previous_yaw_cos * dy;
    let yaw_error =
        wrap_angle(wrap_angle(current_yaw - previous_yaw) - heading_yaw(measured_delta.rot()));
    let raw_error = SVector::<T, 3>::new(
        predicted_x - measured_delta.xyz().x,
        predicted_y - measured_delta.xyz().y,
        yaw_error,
    );

    information_root * raw_error
}

fn heading_yaw<T: Numeric>(rotation: &SO3<T>) -> T {
    let one = T::one();
    let two = T::from(2.0);

    let w = rotation.w();
    let x = rotation.x();
    let y = rotation.y();
    let z = rotation.z();

    (two * (w * z + x * y)).atan2(one - two * (y * y + z * z))
}

fn wrap_angle<T: Numeric>(angle: T) -> T {
    angle.sin().atan2(angle.cos())
}

fn se23_pose_to_se3<T: Numeric>(pose: SE23<T>) -> SE3<T> {
    SE3::from_rot_trans(pose.rot().clone(), pose.xyz().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use factrs::{core::Vector3, traits::Variable};
    use nalgebra::vector;

    fn state(position: Vector3, yaw: f64) -> SE23 {
        SE23::from_rot_vel_trans(so3_from_yaw(yaw), Vector3::zeros(), position)
    }

    fn state_with_rotation(position: Vector3, rotation: SO3) -> SE23 {
        SE23::from_rot_vel_trans(rotation, Vector3::zeros(), position)
    }

    fn translation(x: f64, y: f64, yaw: f64) -> SE3 {
        SE3::from_rot_trans(so3_from_yaw(yaw), vector![x, y, 0.0])
    }

    fn delta(start_tau: f64, end_tau: f64, robot_delta: SE3) -> OdometerDelta {
        OdometerDelta::new(start_tau, end_tau, robot_delta)
    }

    fn so3_from_yaw(yaw: f64) -> SO3 {
        let rotation = nalgebra::UnitQuaternion::from_euler_angles(0.0, 0.0, yaw);
        SO3::from_xyzw(rotation.i, rotation.j, rotation.k, rotation.w)
    }

    fn so3_from_roll_pitch(roll: f64, pitch: f64) -> SO3 {
        let rotation = nalgebra::UnitQuaternion::from_euler_angles(roll, pitch, 0.0);
        SO3::from_xyzw(rotation.i, rotation.j, rotation.k, rotation.w)
    }

    #[test]
    fn empty_deltas_have_empty_residual() {
        let factor = OdometerFactor::new(vec![], Matrix3::identity(), 1.0);

        let residual =
            factor.residuals_on_spline(state(Vector3::zeros(), 0.0), state(Vector3::zeros(), 0.0));

        assert_eq!(residual.len(), 0);
    }

    #[test]
    fn residual_is_zero_for_matching_planar_motion() {
        let factor = OdometerFactor::new(
            vec![delta(0.0, 1.0, translation(1.0, 0.0, 0.2))],
            Matrix3::identity(),
            1.0,
        );

        let residual = factor.residuals_on_spline(
            state(Vector3::zeros(), 0.0),
            state(vector![1.0, 0.0, 0.0], 0.2),
        );

        assert!(
            residual.iter().all(|value| value.abs() < 1e-9),
            "expected zero residual, got {residual:?}"
        );
    }

    #[test]
    fn residual_uses_level_flat_previous_heading() {
        let factor = OdometerFactor::new(
            vec![delta(0.0, 1.0, translation(1.0, 0.0, 0.0))],
            Matrix3::identity(),
            1.0,
        );

        let residual = factor.residuals_on_spline(
            state(Vector3::zeros(), std::f64::consts::FRAC_PI_2),
            state(vector![0.0, 1.0, 0.0], std::f64::consts::FRAC_PI_2),
        );

        assert!(
            residual.iter().all(|value| value.abs() < 1e-9),
            "expected zero residual, got {residual:?}"
        );
    }

    #[test]
    fn residual_ignores_non_planar_motion() {
        let factor = OdometerFactor::new(
            vec![delta(0.0, 1.0, SE3::identity())],
            Matrix3::identity(),
            1.0,
        );

        let residual = factor.residuals_on_spline(
            state_with_rotation(vector![0.0, 0.0, 0.4], so3_from_roll_pitch(0.2, -0.1)),
            state_with_rotation(vector![0.0, 0.0, 1.0], so3_from_roll_pitch(-0.1, 0.3)),
        );

        assert!(
            residual.iter().all(|value| value.abs() < 1e-9),
            "expected zero residual, got {residual:?}"
        );
    }

    #[test]
    fn residual_is_nonzero_for_mismatching_planar_motion() {
        let factor = OdometerFactor::new(
            vec![delta(0.0, 1.0, translation(0.5, 0.0, 0.0))],
            Matrix3::identity(),
            1.0,
        );

        let residual = factor.residuals_on_spline(
            state(Vector3::zeros(), 0.0),
            state(vector![1.0, 0.0, 0.0], 0.2),
        );

        assert!(
            residual.norm() > 0.1,
            "expected nonzero residual, got {residual:?}"
        );
    }
}
