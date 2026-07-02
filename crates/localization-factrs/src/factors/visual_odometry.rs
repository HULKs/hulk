use std::time::SystemTime;

use factrs::{
    core::SE3,
    linalg::{ForwardProp, Numeric, VectorX},
    traits::{Residual, Variable},
    variables::SE23,
};
use nalgebra::{SMatrix, SVector};

use crate::{SE23Spline, utils::tau};

#[derive(Debug, Clone)]
pub struct VisualOdometryMeasurement {
    pub previous_time: SystemTime,
    pub current_time: SystemTime,
    /// Transformation from the current robot frame to the previous robot frame.
    pub robot_delta: SE3,
}

#[derive(Debug, Clone)]
pub struct VisualOdometryFactor {
    measurements: Vec<VisualOdometryDelta>,
    information_root: SMatrix<f64, 6, 6>,
    duration: f64,
}

#[derive(Debug, Clone)]
pub struct AdjacentVisualOdometryFactor {
    measurements: Vec<VisualOdometryDelta>,
    information_root: SMatrix<f64, 6, 6>,
    duration: f64,
}

#[derive(Debug, Clone)]
pub struct VisualOdometryDelta {
    start_tau: f64,
    end_tau: f64,
    robot_delta: SE3,
}

#[factrs::mark]
impl Residual for VisualOdometryFactor {
    type Input = (SE23, SE23);
    type Differ = ForwardProp;

    fn dim_out(&self) -> usize {
        self.measurements.len() * 6
    }

    fn residual<T: Numeric>(&self, (start, end): (SE23<T>, SE23<T>)) -> VectorX<T> {
        self.residuals_on_spline(start, end)
    }
}

#[factrs::mark]
impl Residual for AdjacentVisualOdometryFactor {
    type Input = (SE23, SE23, SE23);
    type Differ = ForwardProp;

    fn dim_out(&self) -> usize {
        self.measurements.len() * 6
    }

    fn residual<T: Numeric>(
        &self,
        (previous_start, middle, current_end): (SE23<T>, SE23<T>, SE23<T>),
    ) -> VectorX<T> {
        self.residuals_on_splines(previous_start, middle, current_end)
    }
}

impl VisualOdometryFactor {
    pub fn new(
        measurements: Vec<VisualOdometryDelta>,
        visual_odometry_noise: SMatrix<f64, 6, 6>,
        duration: f64,
    ) -> Self {
        Self {
            measurements,
            information_root: information_root(visual_odometry_noise),
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
            let whitened_error = delta_residual(
                previous_pose,
                current_pose,
                &measurement.robot_delta,
                &information_root,
            );

            residuals
                .fixed_view_mut::<6, 1>(index * 6, 0)
                .copy_from(&whitened_error);
        }

        residuals
    }
}

impl AdjacentVisualOdometryFactor {
    pub fn new(
        measurements: Vec<VisualOdometryDelta>,
        visual_odometry_noise: SMatrix<f64, 6, 6>,
        duration: f64,
    ) -> Self {
        Self {
            measurements,
            information_root: information_root(visual_odometry_noise),
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
            let whitened_error = delta_residual(
                previous_pose,
                current_pose,
                &measurement.robot_delta,
                &information_root,
            );

            residuals
                .fixed_view_mut::<6, 1>(index * 6, 0)
                .copy_from(&whitened_error);
        }

        residuals
    }
}

impl VisualOdometryDelta {
    pub fn new(start_tau: f64, end_tau: f64, robot_delta: SE3) -> Self {
        Self {
            start_tau,
            end_tau,
            robot_delta,
        }
    }

    pub fn from_measurement(
        measurement: VisualOdometryMeasurement,
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

fn information_root(visual_odometry_noise: SMatrix<f64, 6, 6>) -> SMatrix<f64, 6, 6> {
    visual_odometry_noise
        .cholesky()
        .expect("visual odometry covariance must be positive definite")
        .l()
        .try_inverse()
        .expect("visual odometry covariance Cholesky factor must be invertible")
}

fn delta_residual<T: Numeric>(
    previous_pose: SE3<T>,
    current_pose: SE3<T>,
    measured_delta: &SE3,
    information_root: &SMatrix<T, 6, 6>,
) -> SVector<T, 6> {
    let predicted_delta = previous_pose.inverse().compose(&current_pose);
    let measured_delta = measured_delta.cast::<T>();
    let raw_error = predicted_delta.ominus(&measured_delta);
    let raw_error = SVector::<T, 6>::from_column_slice(raw_error.as_slice());

    information_root * raw_error
}

fn se23_pose_to_se3<T: Numeric>(pose: SE23<T>) -> SE3<T> {
    SE3::from_rot_trans(pose.rot().clone(), pose.xyz().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use factrs::core::{SO3, Vector3};
    use nalgebra::vector;

    fn state(position: Vector3, velocity: Vector3) -> SE23 {
        SE23::from_rot_vel_trans(SO3::identity(), velocity, position)
    }

    fn translation(x: f64, y: f64, z: f64) -> SE3 {
        SE3::from_rot_trans(SO3::identity(), vector![x, y, z])
    }

    fn delta(start_tau: f64, end_tau: f64, robot_delta: SE3) -> VisualOdometryDelta {
        VisualOdometryDelta::new(start_tau, end_tau, robot_delta)
    }

    #[test]
    fn empty_deltas_have_empty_residual() {
        let factor = VisualOdometryFactor::new(vec![], SMatrix::<f64, 6, 6>::identity(), 1.0);

        let residual = factor.residuals_on_spline(
            state(Vector3::zeros(), Vector3::zeros()),
            state(Vector3::zeros(), Vector3::zeros()),
        );

        assert_eq!(residual.len(), 0);
    }

    #[test]
    fn residual_is_zero_for_matching_camera_motion() {
        let factor = VisualOdometryFactor::new(
            vec![delta(0.0, 1.0, translation(1.0, 0.0, 0.0))],
            SMatrix::<f64, 6, 6>::identity(),
            1.0,
        );

        let residual = factor.residuals_on_spline(
            state(Vector3::zeros(), vector![1.0, 0.0, 0.0]),
            state(vector![1.0, 0.0, 0.0], vector![1.0, 0.0, 0.0]),
        );

        assert!(
            residual.iter().all(|value| value.abs() < 1e-9),
            "expected zero residual, got {residual:?}"
        );
    }

    #[test]
    fn adjacent_residual_is_zero_for_matching_camera_motion() {
        let factor = AdjacentVisualOdometryFactor::new(
            vec![delta(0.5, 0.5, translation(1.0, 0.0, 0.0))],
            SMatrix::<f64, 6, 6>::identity(),
            1.0,
        );

        let residual = factor.residuals_on_splines(
            state(Vector3::zeros(), vector![1.0, 0.0, 0.0]),
            state(vector![1.0, 0.0, 0.0], vector![1.0, 0.0, 0.0]),
            state(vector![2.0, 0.0, 0.0], vector![1.0, 0.0, 0.0]),
        );

        assert!(
            residual.iter().all(|value| value.abs() < 1e-9),
            "expected zero residual, got {residual:?}"
        );
    }

    #[test]
    fn residual_is_nonzero_for_mismatching_odometer_motion() {
        let factor = VisualOdometryFactor::new(
            vec![delta(0.0, 1.0, translation(0.5, 0.0, 0.0))],
            SMatrix::<f64, 6, 6>::identity(),
            1.0,
        );

        let residual = factor.residuals_on_spline(
            state(Vector3::zeros(), vector![1.0, 0.0, 0.0]),
            state(vector![1.0, 0.0, 0.0], vector![1.0, 0.0, 0.0]),
        );

        assert!(
            residual.norm() > 0.1,
            "expected nonzero residual, got {residual:?}"
        );
    }
}
